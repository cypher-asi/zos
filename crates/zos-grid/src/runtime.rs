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
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Mutex, RwLock};

use zero_identity::neural_key::NeuralKey;
use zero_sdk::{RealGridClient, ZeroSdk};

use crate::config::PersistedConfig;
use crate::dto::GridStatusDto;
use crate::error::GridFacadeError;
use crate::persist::{self, PersistedDevice, PersistedDevices, PersistedIdentity};

/// Subdirectory under `data_dir` where the SDK opens its RocksDB.
const SDK_DB_SUBDIR: &str = "db";

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
}

impl ZeroRuntime {
    /// Set up on-disk layout (`data_dir`, `config.json`) but do **not**
    /// open the SDK or dial GRID. Use [`Self::ensure_started`] /
    /// [`Self::connect`] to actually bring the runtime up.
    pub fn new(data_dir: PathBuf) -> Result<Arc<Self>, GridFacadeError> {
        std::fs::create_dir_all(&data_dir)?;
        // ensure config.json exists so future reads always succeed
        let _ = PersistedConfig::load_or_init(&data_dir)?;
        Ok(Arc::new(Self {
            data_dir,
            inner: RwLock::new(None),
            last_error: Mutex::new(None),
            connected: AtomicBool::new(false),
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

        let identity =
            persist::read_identity(&self.data_dir)?.ok_or(GridFacadeError::IdentityMissing)?;
        let neural_key = persist::recover(&identity)?;

        let cfg = PersistedConfig::load_or_init(&self.data_dir)?;
        let db_path = self.data_dir.join(SDK_DB_SUBDIR);

        match Self::bring_up(db_path, &neural_key, &cfg.grid_multiaddr).await {
            Ok(live) => {
                let sdk = Arc::clone(&live.sdk);
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

    /// Open the local DB and dial the GRID multiaddr, packaging both
    /// into a [`LiveSdk`] for the runtime to hold onto. Errors produced
    /// here are stringified into [`GridFacadeError::Bootstrap`] by the
    /// caller.
    async fn bring_up(
        db_path: PathBuf,
        neural_key: &NeuralKey,
        multiaddr: &str,
    ) -> Result<LiveSdk, String> {
        let sdk = ZeroSdk::open(&db_path, neural_key).map_err(|e| e.to_string())?;
        let grid = RealGridClient::connect(multiaddr)
            .await
            .map_err(|e| e.to_string())?;
        Ok(LiveSdk {
            sdk: Arc::new(sdk),
            grid: Arc::new(grid),
        })
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
    pub async fn set_multiaddr(&self, multiaddr: String) -> Result<(), GridFacadeError> {
        let cfg = PersistedConfig {
            grid_multiaddr: multiaddr,
        };
        cfg.save(&self.data_dir)?;
        self.disconnect().await;
        *self.last_error.lock().await = None;
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
        // Also reset the device list so a re-created identity doesn't
        // inherit stale machine keys from a prior install.
        persist::write_devices(&self.data_dir, &PersistedDevices::default())?;
        Ok(record)
    }

    /// List all persisted devices for the current identity.
    pub fn list_devices(&self) -> Result<Vec<PersistedDevice>, GridFacadeError> {
        if persist::read_identity(&self.data_dir)?.is_none() {
            return Ok(Vec::new());
        }
        Ok(persist::read_devices(&self.data_dir)?.devices)
    }

    /// Derive + persist a new machine key for the current identity.
    ///
    /// Bumps the identity's epoch and stores the public verifying-keys
    /// alongside the caller-supplied `capabilities` bitflags so the
    /// existing wire DTO is unchanged.
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

        let store = zero_sdk::MachineKeyStore::new();
        let label_for_store = label.clone().unwrap_or_default();
        let entry = store
            .generate_machine_key(&neural_key, label_for_store)
            .map_err(|e| GridFacadeError::Identity(format!("machine key generate: {e}")))?;

        identity.epoch = identity.epoch.saturating_add(1);
        let device = PersistedDevice {
            machine_id: hex::encode(entry.machine_id.as_bytes()),
            identity_id: identity.identity_id.clone(),
            label,
            capabilities,
            epoch: identity.epoch,
            created_at_ms: entry.created_at.saturating_mul(1_000),
            ed25519_pub: hex::encode(entry.ed25519_pub),
            mldsa65_pub: hex::encode(&entry.mldsa65_pub),
        };

        let mut devices = persist::read_devices(&self.data_dir)?;
        devices.devices.push(device.clone());
        persist::write_devices(&self.data_dir, &devices)?;
        persist::write_identity(&self.data_dir, &identity)?;

        Ok(device)
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
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
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
