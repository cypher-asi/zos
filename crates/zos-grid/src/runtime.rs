//! `ZeroRuntime` — façade over the new `zero-sdk-10` stack.
//!
//! Compared to the older `zero-sdk` integration this layer kept around
//! before, two shifts are notable:
//!
//! * `zero-sdk-10` does **not** persist `NeuralKey` material, machine
//!   keys, or "current identity" markers itself. We carry that on local
//!   JSON files (`crate::persist`), reusing `<data_dir>/identity/` so the
//!   on-disk shape is migration-friendly.
//! * `zero_sdk::ZeroSdk::open` opens a RocksDB at `<data_dir>/db` and
//!   needs an in-hand `NeuralKey` to bind it to a stable identity. The
//!   runtime stays "cold" until `ensure_started`, at which point we
//!   recover the key from the persisted Shamir shares.
//!
//! `connected` reflects whether we currently hold a live `ZeroSdk` plus
//! a successfully-dialled `RealGridClient`. The real upstream GRID library
//! is still a stub in `zero-sdk-10`, so for now this is "did the local DB
//! open and did `RealGridClient::connect` return Ok" — exactly the level
//! of liveness the React UI was already coded against.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::{broadcast, Mutex, RwLock};

use zero_identity::neural_key::NeuralKey;
use zero_sdk::{MachineKeyStore, RealGridClient, ZeroSdk};

use crate::chat;
use crate::config::PersistedConfig;
use crate::dto::{
    ContactDto, ConversationDto, GridStatusDto, MessageDto, MessageEnvelopeDto,
};
use crate::error::GridFacadeError;
use crate::persist::{self, PersistedDevice, PersistedIdentity};

/// Capacity of the chat WebSocket broadcast channel. Receivers that
/// can't keep up will see `RecvError::Lagged` and reconnect; 256 is
/// enough headroom that a single-page reload doesn't blow the buffer.
const CHAT_BROADCAST_CAPACITY: usize = 256;

/// Subdirectory under `data_dir` where the SDK opens its RocksDB.
const SDK_DB_SUBDIR: &str = "db";

/// Default connect-timeout used when `PersistedConfig::connect_timeout_ms`
/// is `None`. Mirrors the upstream `RealGridClient` default so the UI's
/// "use default" choice matches what the SDK would have applied anyway.
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30_000;

/// Live SDK + GRID client owned together so they're swapped atomically on
/// disconnect / reconnect.
struct LiveSdk {
    sdk: Arc<ZeroSdk>,
    grid: Arc<RealGridClient>,
}

/// Lazy holder for the new `ZeroSdk` runtime, sharing on-disk state in
/// `data_dir` with any other process pointed at the same directory
/// (typically the embedded server inside `zero-desktop`).
pub struct ZeroRuntime {
    data_dir: PathBuf,
    inner: RwLock<Option<LiveSdk>>,
    last_error: Mutex<Option<String>>,
    connected: AtomicBool,
    /// Chat fan-out channel, fed by [`chat::run_chat_poll_loop`] and
    /// consumed by every connected `/api/chat/stream` WebSocket. Created
    /// in [`Self::new`] so subscribers can attach before the SDK is
    /// brought up.
    chat_tx: broadcast::Sender<MessageEnvelopeDto>,
    /// Guards the one-shot spawn of [`chat::run_chat_poll_loop`]. Flipped
    /// from `false` to `true` the first time anything inside the runtime
    /// requests chat services.
    chat_loop_started: AtomicBool,
    /// Local-only `ZeroSdk` cache used by the chat surface. Distinct
    /// from [`LiveSdk`] (which also owns a dialled `RealGridClient`)
    /// because chat reads/writes are pure local-DB operations and must
    /// keep working when the upstream GRID node is unreachable. Filled
    /// lazily on the first chat call after an identity exists; never
    /// torn down.
    local_sdk: RwLock<Option<Arc<ZeroSdk>>>,
    /// Guards the one-time `tracing::warn!` emitted when a legacy
    /// `devices.json` is observed on disk. Set on the first
    /// [`Self::list_devices`] call that finds a non-empty legacy file.
    legacy_devices_warned: AtomicBool,
}

impl ZeroRuntime {
    /// Set up on-disk layout (`data_dir`, `config.json`) but do **not**
    /// open the SDK or dial GRID. Use [`Self::ensure_started`] /
    /// [`Self::connect`] to actually bring the runtime up.
    pub fn new(data_dir: PathBuf) -> Result<Arc<Self>, GridFacadeError> {
        std::fs::create_dir_all(&data_dir)?;
        // ensure config.json exists so future reads always succeed
        let _ = PersistedConfig::load_or_init(&data_dir)?;
        let (chat_tx, _initial_rx) = broadcast::channel(CHAT_BROADCAST_CAPACITY);
        Ok(Arc::new(Self {
            data_dir,
            inner: RwLock::new(None),
            last_error: Mutex::new(None),
            connected: AtomicBool::new(false),
            chat_tx,
            chat_loop_started: AtomicBool::new(false),
            local_sdk: RwLock::new(None),
            legacy_devices_warned: AtomicBool::new(false),
        }))
    }

    /// On-disk root used by this runtime.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Read (or initialise) the persisted runtime config.
    pub fn read_config(&self) -> Result<PersistedConfig, GridFacadeError> {
        PersistedConfig::load_or_init(&self.data_dir)
    }

    /// Open the `ZeroSdk` + dial GRID if it isn't already up. Idempotent:
    /// a second concurrent call will block on the write-lock and observe
    /// the already-built handle.
    ///
    /// Returns [`GridFacadeError::IdentityMissing`] if no identity has
    /// been created yet — the SDK can't bind to a database without a
    /// `NeuralKey`.
    pub async fn ensure_started(&self) -> Result<Arc<ZeroSdk>, GridFacadeError> {
        if let Some(live) = self.inner.read().await.as_ref() {
            return Ok(Arc::clone(&live.sdk));
        }
        let mut guard = self.inner.write().await;
        if let Some(live) = guard.as_ref() {
            return Ok(Arc::clone(&live.sdk));
        }

        // Open (or reuse) the local-only SDK first. Doing this before
        // we touch the network keeps the chat surface usable even if
        // the GRID dial below fails -- the same `Arc<ZeroSdk>` will
        // be handed back to chat callers and the eventual `LiveSdk`.
        let sdk = self.ensure_local_sdk().await?;

        let cfg = PersistedConfig::load_or_init(&self.data_dir)?;
        let connect_timeout = Duration::from_millis(
            cfg.connect_timeout_ms.unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS),
        );

        match Self::dial_grid(&cfg.grid_multiaddr, connect_timeout).await {
            Ok(grid) => {
                let live = LiveSdk {
                    sdk: Arc::clone(&sdk),
                    grid: Arc::new(grid),
                };
                *guard = Some(live);
                self.connected.store(true, Ordering::SeqCst);
                *self.last_error.lock().await = None;
                tracing::info!("zos-grid: ZeroSdk bootstrap succeeded");
                Ok(sdk)
            }
            Err(e) => {
                self.connected.store(false, Ordering::SeqCst);
                let msg = e.to_string();
                tracing::warn!(error = %msg, "zos-grid: ZeroSdk bootstrap failed");
                *self.last_error.lock().await = Some(msg.clone());
                Err(GridFacadeError::Bootstrap(msg))
            }
        }
    }

    /// Dial the GRID multiaddr with the configured timeout. Errors are
    /// stringified into [`GridFacadeError::Bootstrap`] by the caller.
    async fn dial_grid(
        multiaddr: &str,
        connect_timeout: Duration,
    ) -> Result<RealGridClient, String> {
        RealGridClient::connect_with_timeout(multiaddr, connect_timeout)
            .await
            .map_err(|e| e.to_string())
    }

    /// Open the local-only `ZeroSdk` (RocksDB + neural key) without any
    /// GRID connectivity. Cached in `local_sdk` and shared with
    /// [`Self::ensure_started`] so the same handle backs both the live
    /// path and the chat fallback path -- avoiding a second RocksDB
    /// open on the same directory, which would deadlock.
    async fn ensure_local_sdk(&self) -> Result<Arc<ZeroSdk>, GridFacadeError> {
        if let Some(sdk) = self.local_sdk.read().await.as_ref() {
            return Ok(Arc::clone(sdk));
        }
        let mut guard = self.local_sdk.write().await;
        if let Some(sdk) = guard.as_ref() {
            return Ok(Arc::clone(sdk));
        }
        let identity =
            persist::read_identity(&self.data_dir)?.ok_or(GridFacadeError::IdentityMissing)?;
        let neural_key = persist::recover(&identity)?;
        let db_path = self.data_dir.join(SDK_DB_SUBDIR);
        let sdk = ZeroSdk::open(&db_path, &neural_key)
            .map_err(|e| GridFacadeError::Bootstrap(e.to_string()))?;
        let arc = Arc::new(sdk);
        *guard = Some(Arc::clone(&arc));
        Ok(arc)
    }

    /// Drop the live SDK + GRID client (if any). Subsequent
    /// [`Self::ensure_started`] calls will rebuild them from the
    /// persisted state.
    pub async fn disconnect(&self) {
        let mut guard = self.inner.write().await;
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);
    }

    /// Persist a new GRID multiaddr and tear down any active connection
    /// so the next `connect` dials the new endpoint.
    ///
    /// Reads the existing `PersistedConfig` first so other persisted
    /// fields (e.g. `connect_timeout_ms`) survive the write.
    pub async fn set_multiaddr(&self, multiaddr: String) -> Result<(), GridFacadeError> {
        let mut cfg = PersistedConfig::load_or_init(&self.data_dir)?;
        cfg.grid_multiaddr = multiaddr;
        cfg.save(&self.data_dir)?;
        self.disconnect().await;
        *self.last_error.lock().await = None;
        Ok(())
    }

    /// Persist a new connect-timeout (in milliseconds), or `None` to fall
    /// back to the SDK default.
    ///
    /// The new value only takes effect on the *next* `connect` — any
    /// currently-live connection is left in place so the user can adjust
    /// the timeout while debugging a flaky link without losing the
    /// session. Use [`Self::disconnect`] explicitly if you also want to
    /// re-dial.
    pub async fn set_connect_timeout(
        &self,
        timeout_ms: Option<u64>,
    ) -> Result<(), GridFacadeError> {
        let mut cfg = PersistedConfig::load_or_init(&self.data_dir)?;
        cfg.connect_timeout_ms = timeout_ms;
        cfg.save(&self.data_dir)?;
        Ok(())
    }

    /// Snapshot of the current connection status, suitable for
    /// `GET /api/grid/status`. Never triggers bootstrap.
    pub async fn status(&self) -> Result<GridStatusDto, GridFacadeError> {
        let cfg = self.read_config()?;
        let identity_id = persist::read_identity(&self.data_dir)?.map(|r| r.identity_id);
        Ok(GridStatusDto {
            connected: self.connected.load(Ordering::SeqCst),
            multiaddr: cfg.grid_multiaddr,
            connect_timeout_ms: cfg.connect_timeout_ms,
            identity_id,
            last_error: self.last_error.lock().await.clone(),
        })
    }

    /// Read the public identity record for the current identity, or
    /// `None` if no identity has been created yet.
    pub fn get_identity(&self) -> Result<Option<PersistedIdentity>, GridFacadeError> {
        persist::read_identity(&self.data_dir)
    }

    /// Generate + persist a brand new Neural Key identity. Bypasses the
    /// SDK so identity creation works even when GRID is unreachable.
    pub fn create_identity(&self) -> Result<PersistedIdentity, GridFacadeError> {
        if persist::read_identity(&self.data_dir)?.is_some() {
            return Err(GridFacadeError::IdentityExists);
        }

        let key = NeuralKey::generate()
            .map_err(|e| GridFacadeError::Identity(format!("neural key generate: {e}")))?;
        let id_bytes = key.identity_id_bytes();
        let (shares, threshold) = persist::split(&key)?;

        let record = PersistedIdentity {
            identity_id: hex::encode(id_bytes),
            epoch: 0,
            created_at_ms: now_unix_ms(),
            shares,
            threshold,
        };
        persist::write_identity(&self.data_dir, &record)?;
        // Reset the sealed device store so a re-created identity does
        // not inherit stale machine keys from a prior install. The
        // legacy `devices.json` (if present) stays put -- it can only
        // be loaded for display once and never round-trips secrets.
        persist::remove_sealed_devices(&self.data_dir)?;
        Ok(record)
    }

    /// List all persisted devices for the current identity.
    ///
    /// Reads the sealed device store (Phase D1: full key pairs + seeds
    /// behind ChaCha20-Poly1305) and projects each entry into the
    /// existing [`PersistedDevice`] DTO shape. The HTTP DTO surface is
    /// unchanged from the pre-D1 `devices.json` path.
    ///
    /// If the legacy `devices.json` is present alongside an absent
    /// sealed blob (an upgraded install), its public summaries are
    /// also returned -- those rows can be displayed but cannot sign,
    /// since their secret halves were never persisted. A `register`
    /// call writes the new sealed store and effectively "owns" the
    /// device list going forward; the legacy file is left in place
    /// for read-only display until the user clears state.
    pub fn list_devices(&self) -> Result<Vec<PersistedDevice>, GridFacadeError> {
        let identity = match persist::read_identity(&self.data_dir)? {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };
        let neural_key = persist::recover(&identity)?;

        let mut by_id: std::collections::HashMap<String, PersistedDevice> =
            std::collections::HashMap::new();

        if let Some(store) = persist::read_sealed_devices(&self.data_dir, &neural_key)? {
            let records = store
                .list_machine_records(&neural_key)
                .map_err(|e| GridFacadeError::Identity(format!("list machine records: {e}")))?;
            for record in records {
                let device = persisted_device_from_record(&identity.identity_id, &record);
                by_id.insert(device.machine_id.clone(), device);
            }
        }

        // Legacy fallback: rows whose secret halves were lost in the
        // pre-D1 scheme are surfaced read-only so the UI doesn't
        // suddenly show an empty device list after upgrade.
        let legacy = persist::read_legacy_devices(&self.data_dir)?;
        if !legacy.devices.is_empty() {
            self.warn_legacy_devices_once();
        }
        for legacy_device in legacy.devices {
            by_id
                .entry(legacy_device.machine_id.clone())
                .or_insert(legacy_device);
        }

        Ok(by_id.into_values().collect())
    }

    /// Derive + persist a new machine key for the current identity.
    ///
    /// Bumps the identity's epoch and persists the new entry into the
    /// sealed device store so it survives across process restart.
    /// Capability bitflags + label are wired straight through to the
    /// HTTP DTO so the wire shape is unchanged from the pre-D1 path.
    pub fn register_machine(&self, capabilities: u32) -> Result<PersistedDevice, GridFacadeError> {
        self.register_machine_with_label(capabilities, None)
    }

    /// Variant of [`Self::register_machine`] that also persists a
    /// human-friendly label.
    pub fn register_machine_with_label(
        &self,
        capabilities: u32,
        label: Option<String>,
    ) -> Result<PersistedDevice, GridFacadeError> {
        let mut identity =
            persist::read_identity(&self.data_dir)?.ok_or(GridFacadeError::IdentityMissing)?;
        let neural_key = persist::recover(&identity)?;

        let store = persist::read_sealed_devices(&self.data_dir, &neural_key)?
            .unwrap_or_else(MachineKeyStore::new);

        let label_for_store = label.clone().unwrap_or_default();
        identity.epoch = identity.epoch.saturating_add(1);
        let record = store
            .generate_machine_key_with(&neural_key, label_for_store, capabilities, identity.epoch)
            .map_err(|e| GridFacadeError::Identity(format!("machine key generate: {e}")))?;

        let device = PersistedDevice {
            machine_id: hex::encode(record.entry.machine_id.as_bytes()),
            identity_id: identity.identity_id.clone(),
            label,
            capabilities: record.capabilities,
            epoch: record.epoch,
            created_at_ms: record.entry.created_at.saturating_mul(1_000),
            ed25519_pub: hex::encode(record.entry.ed25519_pub),
            mldsa65_pub: hex::encode(&record.entry.mldsa65_pub),
        };

        persist::write_sealed_devices(&self.data_dir, &neural_key, &store)?;
        persist::write_identity(&self.data_dir, &identity)?;

        Ok(device)
    }

    /// Emit a one-time `tracing::warn!` when we observe a populated
    /// legacy `devices.json` alongside (or without) a sealed store.
    /// Idempotent: subsequent calls are no-ops thanks to
    /// `legacy_devices_warned`.
    fn warn_legacy_devices_once(&self) {
        if self
            .legacy_devices_warned
            .swap(true, Ordering::SeqCst)
        {
            return;
        }
        tracing::warn!(
            "zos-grid: legacy devices.json detected; secret halves from the pre-D1 scheme are unrecoverable across restart. New `derive_machine_key` calls populate `devices.sealed` going forward."
        );
    }

    /// Multiaddr of the currently-connected GRID client, if any. Mostly
    /// useful in tests; HTTP handlers go through [`Self::status`].
    pub async fn current_multiaddr(&self) -> Option<String> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|live| live.grid.multiaddr().to_owned())
    }

    // ── Chat API ──────────────────────────────────────────────────────
    //
    // All chat methods take `self: &Arc<Self>` so the lazily-spawned
    // poll loop can hold a `Weak<ZeroRuntime>` and shut itself down
    // when the runtime is dropped (e.g. test teardown).

    /// Snapshot the live `ZeroSdk` for direct read/write access from the
    /// chat helpers. Returns `None` when the runtime is "cold" (no
    /// identity yet, or `disconnect()` was called and nothing has
    /// re-bootstrapped). Used internally by [`chat::run_chat_poll_loop`]
    /// and the public chat methods below.
    pub(crate) async fn live_sdk_snapshot(&self) -> Option<Arc<ZeroSdk>> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|live| Arc::clone(&live.sdk))
    }

    /// Acquire a `ZeroSdk` for chat operations.
    ///
    /// Chat reads/writes (`DmService`, `InboxService`, `ContactStore`)
    /// are pure local-DB operations, so we deliberately do **not** call
    /// [`Self::ensure_started`] here -- requiring a live GRID dial
    /// would render chat unusable on cold-start or while offline. We
    /// piggy-back on the live SDK if one already exists, otherwise we
    /// lazily open the local-only SDK.
    async fn chat_sdk(&self) -> Result<Arc<ZeroSdk>, GridFacadeError> {
        if let Some(sdk) = self.live_sdk_snapshot().await {
            return Ok(sdk);
        }
        self.ensure_local_sdk().await
    }

    /// Best-effort variant of [`Self::chat_sdk`] for the polling task:
    /// returns `None` instead of an error so the loop can quietly
    /// no-op while there is no identity or while the DB hasn't been
    /// opened yet.
    pub(crate) async fn chat_sdk_for_poll(&self) -> Option<Arc<ZeroSdk>> {
        if let Some(sdk) = self.live_sdk_snapshot().await {
            return Some(sdk);
        }
        if let Some(sdk) = self.local_sdk.read().await.as_ref() {
            return Some(Arc::clone(sdk));
        }
        None
    }

    /// Subscribe to the chat broadcast channel. Idempotently spawns the
    /// background poll loop on the first call so a `/api/chat/stream`
    /// WebSocket starts seeing live updates without first having to hit
    /// any other endpoint.
    pub fn subscribe_messages(self: &Arc<Self>) -> broadcast::Receiver<MessageEnvelopeDto> {
        self.maybe_start_chat_loop();
        self.chat_tx.subscribe()
    }

    /// Spawn the chat poll loop exactly once over the lifetime of the
    /// runtime. Subsequent calls are no-ops.
    fn maybe_start_chat_loop(self: &Arc<Self>) {
        if self.chat_loop_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = Arc::downgrade(self);
        let tx = self.chat_tx.clone();
        tokio::spawn(chat::run_chat_poll_loop(weak, tx));
    }

    /// `GET /api/chat/conversations` body. Returns an empty list (rather
    /// than a 5xx) when no identity exists yet so the chat UI can render
    /// its empty state without first making a status call.
    pub async fn list_conversations(
        self: &Arc<Self>,
        limit: Option<usize>,
    ) -> Result<Vec<ConversationDto>, GridFacadeError> {
        match self.chat_sdk().await {
            Ok(sdk) => chat::list_conversations(&sdk, limit),
            Err(GridFacadeError::IdentityMissing) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// `GET /api/chat/conversations/:id/messages` body.
    pub async fn list_messages(
        self: &Arc<Self>,
        conversation_id_hex: &str,
        before: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<MessageDto>, GridFacadeError> {
        match self.chat_sdk().await {
            Ok(sdk) => chat::list_messages(&sdk, conversation_id_hex, before, limit),
            Err(GridFacadeError::IdentityMissing) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// `POST /api/chat/conversations/:id/messages` body. Persists the
    /// message via `DmService::send_text`, broadcasts an envelope on the
    /// chat channel for any connected WebSocket clients, and returns
    /// the freshly-stored DTO.
    pub async fn send_message(
        self: &Arc<Self>,
        conversation_id_hex: &str,
        body: String,
    ) -> Result<MessageDto, GridFacadeError> {
        let sdk = self.chat_sdk().await?;
        let dto = chat::send_message(&sdk, conversation_id_hex, body)?;
        // Best-effort fan-out; ignore "no subscribers" errors.
        let _ = self.chat_tx.send(MessageEnvelopeDto::Message {
            message: dto.clone(),
        });
        // Make sure the poll loop is running so future inbound messages
        // also get pushed (no-op if already started).
        self.maybe_start_chat_loop();
        Ok(dto)
    }

    /// `POST /api/chat/conversations` body.
    pub async fn create_conversation(
        self: &Arc<Self>,
        contact_id_hex: String,
    ) -> Result<ConversationDto, GridFacadeError> {
        let sdk = self.chat_sdk().await?;
        chat::create_conversation(&sdk, contact_id_hex)
    }

    /// `GET /api/chat/contacts` body. Returns an empty list when no
    /// identity exists yet.
    pub async fn list_contacts(
        self: &Arc<Self>,
    ) -> Result<Vec<ContactDto>, GridFacadeError> {
        match self.chat_sdk().await {
            Ok(sdk) => chat::list_contacts(&sdk),
            Err(GridFacadeError::IdentityMissing) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// `POST /api/chat/contacts` body.
    pub async fn add_contact(
        self: &Arc<Self>,
        label: String,
        identity_id_hex: String,
    ) -> Result<ContactDto, GridFacadeError> {
        let sdk = self.chat_sdk().await?;
        chat::add_contact(&sdk, label, identity_id_hex)
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Project a [`zero_sdk::MachineKeyRecord`] onto the on-wire
/// [`PersistedDevice`] DTO shape so the HTTP surface is unchanged
/// from the pre-D1 `devices.json` flow.
///
/// The record's `label` lands in `PersistedDevice.label` only when
/// non-empty, mirroring the wire convention where an absent / blank
/// label is serialized as `null`.
fn persisted_device_from_record(
    identity_id_hex: &str,
    record: &zero_sdk::MachineKeyRecord,
) -> PersistedDevice {
    let label = if record.entry.label.is_empty() {
        None
    } else {
        Some(record.entry.label.clone())
    };
    PersistedDevice {
        machine_id: hex::encode(record.entry.machine_id.as_bytes()),
        identity_id: identity_id_hex.to_owned(),
        label,
        capabilities: record.capabilities,
        epoch: record.epoch,
        created_at_ms: record.entry.created_at.saturating_mul(1_000),
        ed25519_pub: hex::encode(record.entry.ed25519_pub),
        mldsa65_pub: hex::encode(&record.entry.mldsa65_pub),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn rt(dir: &Path) -> Arc<ZeroRuntime> {
        ZeroRuntime::new(dir.to_path_buf()).unwrap()
    }

    #[tokio::test]
    async fn status_before_any_action_is_disconnected_with_no_identity() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        let st = runtime.status().await.unwrap();
        assert!(!st.connected);
        assert!(st.identity_id.is_none());
        assert!(st.last_error.is_none());
        assert_eq!(st.multiaddr, PersistedConfig::default().grid_multiaddr);
    }

    #[test]
    fn get_identity_returns_none_on_fresh_dir() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        assert!(runtime.get_identity().unwrap().is_none());
    }

    #[test]
    fn create_identity_persists_and_is_idempotent_at_409() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        let first = runtime.create_identity().unwrap();
        assert_eq!(first.identity_id.len(), 32);
        let again = runtime.create_identity();
        assert!(matches!(again, Err(GridFacadeError::IdentityExists)));
        // reload should give the same id
        let reloaded = runtime.get_identity().unwrap().unwrap();
        assert_eq!(reloaded.identity_id, first.identity_id);
        assert_eq!(reloaded.epoch, 0);
    }

    #[test]
    fn create_identity_then_register_machine_shows_in_list_and_bumps_epoch() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        let identity = runtime.create_identity().unwrap();
        assert_eq!(identity.epoch, 0);

        let machine = runtime.register_machine(3).unwrap();
        assert_eq!(machine.capabilities, 3);
        assert_eq!(machine.epoch, 1);
        assert_eq!(machine.identity_id, identity.identity_id);

        let list = runtime.list_devices().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].machine_id, machine.machine_id);
        assert_eq!(list[0].capabilities, 3);

        // identity record should reflect the bumped epoch
        let reloaded = runtime.get_identity().unwrap().unwrap();
        assert_eq!(reloaded.epoch, 1);

        // a second registration bumps the epoch again
        let m2 = runtime.register_machine(1).unwrap();
        assert_eq!(m2.epoch, 2);
        assert_eq!(runtime.list_devices().unwrap().len(), 2);
    }

    #[test]
    fn register_machine_with_label_persists_label() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.create_identity().unwrap();
        let m = runtime
            .register_machine_with_label(3, Some("laptop".into()))
            .unwrap();
        assert_eq!(m.label.as_deref(), Some("laptop"));
        let list = runtime.list_devices().unwrap();
        assert_eq!(list[0].label.as_deref(), Some("laptop"));
    }

    /// Phase D1 acceptance at the runtime layer: machine keys
    /// registered before a "restart" must still be present (with the
    /// same machine ids, labels, and capabilities) after a fresh
    /// `ZeroRuntime::new` against the same `data_dir`.
    #[test]
    fn registered_machines_survive_runtime_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        let runtime = rt(path);
        runtime.create_identity().unwrap();
        let alpha = runtime
            .register_machine_with_label(3, Some("alpha".into()))
            .unwrap();
        let beta = runtime
            .register_machine_with_label(7, Some("beta".into()))
            .unwrap();
        drop(runtime);

        let runtime_after = rt(path);
        let list = runtime_after.list_devices().unwrap();
        assert_eq!(list.len(), 2, "both devices should survive restart");
        let by_id: std::collections::HashMap<&str, &PersistedDevice> =
            list.iter().map(|d| (d.machine_id.as_str(), d)).collect();
        let restored_alpha = by_id
            .get(alpha.machine_id.as_str())
            .expect("alpha must be present");
        assert_eq!(restored_alpha.label.as_deref(), Some("alpha"));
        assert_eq!(restored_alpha.capabilities, 3);
        assert_eq!(restored_alpha.ed25519_pub, alpha.ed25519_pub);
        assert_eq!(restored_alpha.mldsa65_pub, alpha.mldsa65_pub);
        let restored_beta = by_id
            .get(beta.machine_id.as_str())
            .expect("beta must be present");
        assert_eq!(restored_beta.label.as_deref(), Some("beta"));
        assert_eq!(restored_beta.capabilities, 7);
    }

    /// Phase D1 acceptance: `create_identity` resets the sealed store
    /// so a freshly-created identity doesn't inherit machine keys
    /// from a prior install.
    #[test]
    fn create_identity_clears_prior_sealed_store() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        let runtime = rt(path);
        runtime.create_identity().unwrap();
        runtime.register_machine(3).unwrap();
        assert_eq!(runtime.list_devices().unwrap().len(), 1);

        // Tear down the identity files so we can create a new one.
        std::fs::remove_file(persist::identity_path(path)).unwrap();
        // Recreate the runtime and the identity; the sealed store
        // should be wiped.
        drop(runtime);
        let runtime = rt(path);
        runtime.create_identity().unwrap();
        assert!(runtime.list_devices().unwrap().is_empty());
    }

    #[test]
    fn register_machine_without_identity_errors() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        let err = runtime.register_machine(3).unwrap_err();
        assert!(matches!(err, GridFacadeError::IdentityMissing));
    }

    #[tokio::test]
    async fn set_multiaddr_persists_and_clears_error() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        // seed a fake last_error
        *runtime.last_error.lock().await = Some("boom".into());
        runtime
            .set_multiaddr("/ip4/127.0.0.1/udp/9999/quic-v1".into())
            .await
            .unwrap();
        let cfg = runtime.read_config().unwrap();
        assert_eq!(cfg.grid_multiaddr, "/ip4/127.0.0.1/udp/9999/quic-v1");
        assert!(runtime.last_error.lock().await.is_none());
    }

    #[tokio::test]
    async fn set_connect_timeout_round_trips_some_and_none() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.set_connect_timeout(Some(60_000)).await.unwrap();
        assert_eq!(runtime.read_config().unwrap().connect_timeout_ms, Some(60_000));
        runtime.set_connect_timeout(None).await.unwrap();
        assert_eq!(runtime.read_config().unwrap().connect_timeout_ms, None);
    }

    #[tokio::test]
    async fn set_multiaddr_preserves_existing_timeout() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.set_connect_timeout(Some(45_000)).await.unwrap();
        runtime
            .set_multiaddr("/ip4/127.0.0.1/udp/9999/quic-v1".into())
            .await
            .unwrap();
        let cfg = runtime.read_config().unwrap();
        assert_eq!(cfg.grid_multiaddr, "/ip4/127.0.0.1/udp/9999/quic-v1");
        assert_eq!(cfg.connect_timeout_ms, Some(45_000));
    }

    #[tokio::test]
    async fn status_surfaces_connect_timeout() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        let s = runtime.status().await.unwrap();
        assert_eq!(s.connect_timeout_ms, None);
        runtime.set_connect_timeout(Some(12_345)).await.unwrap();
        let s = runtime.status().await.unwrap();
        assert_eq!(s.connect_timeout_ms, Some(12_345));
    }

    #[tokio::test]
    async fn ensure_started_without_identity_returns_identity_missing() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        // `Arc<ZeroSdk>` doesn't impl `Debug`, so unwrap_err is unavailable.
        let res = runtime.ensure_started().await.map(|_| ());
        assert!(matches!(res, Err(GridFacadeError::IdentityMissing)));
    }

    /// `RealGridClient::connect` now actually dials the upstream Zode via
    /// `grid-net`, so this test no longer exercises the happy path (we
    /// have no live node in CI). Instead we verify the *bookkeeping* when
    /// connect fails: `ensure_started` surfaces the upstream error as
    /// `GridFacadeError::Bootstrap`, `status.connected` flips back to
    /// `false`, and `last_error` is populated.
    ///
    /// The live-node round-trip is covered by the `#[ignore]`d
    /// integration test below — run it manually against a real zode.
    #[tokio::test]
    async fn ensure_started_with_unreachable_multiaddr_records_bootstrap_error() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.create_identity().unwrap();
        // Force a parse-time failure inside `RealGridClient::connect` so the
        // test fails deterministically without waiting for a libp2p dial
        // timeout.
        runtime
            .set_multiaddr("not-a-valid-multiaddr".into())
            .await
            .unwrap();

        let err = runtime.ensure_started().await.map(|_| ()).unwrap_err();
        assert!(
            matches!(err, GridFacadeError::Bootstrap(_)),
            "unexpected error variant: {err:?}"
        );

        let st = runtime.status().await.unwrap();
        assert!(!st.connected);
        assert!(st.identity_id.is_some());
        let last = st.last_error.expect("last_error should be populated on dial failure");
        assert!(
            last.contains("invalid multiaddr") || last.contains("grid"),
            "unexpected last_error: {last}"
        );
    }

    /// Live-node smoke test for the full bring-up path. Requires a real
    /// zode reachable at the multiaddr stored in `PersistedConfig::default`.
    /// Run with `cargo test -p zos-grid -- --ignored
    /// ensure_started_against_live_node`.
    #[tokio::test]
    #[ignore = "requires a live GRID zode at the default multiaddr"]
    async fn ensure_started_against_live_node() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.create_identity().unwrap();

        drop(runtime.ensure_started().await.unwrap());
        let st = runtime.status().await.unwrap();
        assert!(st.connected);
        assert!(st.last_error.is_none());
        assert!(st.identity_id.is_some());
        assert_eq!(
            runtime.current_multiaddr().await.as_deref(),
            Some(PersistedConfig::default().grid_multiaddr.as_str())
        );

        runtime.disconnect().await;
        assert!(!runtime.status().await.unwrap().connected);
        drop(runtime.ensure_started().await.unwrap());
        assert!(runtime.status().await.unwrap().connected);
    }
}
