//! `ZeroRuntime` — the lazy `Arc<Zero>` holder + identity / device façade.
//!
//! The runtime is intentionally cheap to construct (`new`) so the HTTP
//! server can boot even when the GRID multiaddr points at nothing. The
//! actual `Zero::bootstrap` call happens on the first `ensure_started` /
//! `connect`, and any failure is captured in `last_error` so the UI can
//! surface it via `GET /api/grid/status`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use zero_sdk::crypto::NeuralKey;
use zero_sdk::identity::{
    FsIdentityStore, Identity, IdentityRecord, IdentityStore as _, MachineKeyCapabilities,
    MachineKeyRecord,
};
use zero_sdk::types::IdentityId;
use zero_sdk::{Zero, ZeroConfig};

use crate::config::PersistedConfig;
use crate::dto::GridStatusDto;
use crate::error::GridFacadeError;

/// File at the root of the identity directory holding the active identity
/// id as raw 16 bytes. Mirrors `zero_sdk::runtime::CURRENT_ID_FILE`.
const CURRENT_ID_FILE: &str = "current_id";

/// Lazy holder for an `Arc<Zero>` runtime sharing the on-disk state in
/// `data_dir` with any other process pointed at the same directory
/// (typically the embedded server inside `zero-desktop`).
pub struct ZeroRuntime {
    data_dir: PathBuf,
    inner: RwLock<Option<Arc<Zero>>>,
    last_error: Mutex<Option<String>>,
    connected: AtomicBool,
}

impl ZeroRuntime {
    /// Set up on-disk layout (`data_dir`, `config.json`) but do **not**
    /// dial GRID. Use [`Self::ensure_started`] / [`Self::connect`] to
    /// actually bootstrap a `Zero`.
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

    /// Build the `Arc<Zero>` if it isn't already up. Idempotent: a second
    /// concurrent call will block on the write-lock and observe the
    /// already-built handle.
    pub async fn ensure_started(&self) -> Result<Arc<Zero>, GridFacadeError> {
        if let Some(zero) = self.inner.read().await.as_ref() {
            return Ok(Arc::clone(zero));
        }
        let mut guard = self.inner.write().await;
        if let Some(zero) = guard.as_ref() {
            return Ok(Arc::clone(zero));
        }

        let cfg = PersistedConfig::load_or_init(&self.data_dir)?;
        let zero_cfg = ZeroConfig {
            data_dir: self.data_dir.clone(),
            grid_multiaddr: cfg.grid_multiaddr,
        };

        match Zero::bootstrap(zero_cfg).await {
            Ok(z) => {
                let arc = Arc::new(z);
                *guard = Some(Arc::clone(&arc));
                self.connected.store(true, Ordering::SeqCst);
                *self.last_error.lock().await = None;
                tracing::info!("zos-grid: Zero bootstrap succeeded");
                Ok(arc)
            }
            Err(e) => {
                self.connected.store(false, Ordering::SeqCst);
                let msg = e.to_string();
                tracing::warn!(error = %msg, "zos-grid: Zero bootstrap failed");
                *self.last_error.lock().await = Some(msg.clone());
                Err(GridFacadeError::Bootstrap(msg))
            }
        }
    }

    /// Drop the active `Arc<Zero>` (if any). Subsequent
    /// [`Self::ensure_started`] calls will rebuild it from `config.json`.
    pub async fn disconnect(&self) {
        let mut guard = self.inner.write().await;
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);
    }

    /// Persist a new GRID multiaddr and tear down any active connection so
    /// the next `connect` dials the new endpoint.
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
        let identity_id = self.current_identity_id()?.map(|id| id.to_string());
        Ok(GridStatusDto {
            connected: self.connected.load(Ordering::SeqCst),
            multiaddr: cfg.grid_multiaddr,
            identity_id,
            last_error: self.last_error.lock().await.clone(),
        })
    }

    fn identity_dir(&self) -> PathBuf {
        self.data_dir.join("identity")
    }

    fn open_store(&self) -> Result<FsIdentityStore, GridFacadeError> {
        FsIdentityStore::open(self.identity_dir())
            .map_err(|e| GridFacadeError::Identity(e.to_string()))
    }

    fn current_id_path(&self) -> PathBuf {
        self.identity_dir().join(CURRENT_ID_FILE)
    }

    /// Read the `current_id` marker file written by either us or
    /// `zero_sdk::runtime::load_or_create_identity`.
    pub fn current_identity_id(&self) -> Result<Option<IdentityId>, GridFacadeError> {
        let path = self.current_id_path();
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read(&path)?;
        let id_bytes: [u8; 16] = raw
            .as_slice()
            .try_into()
            .map_err(|_| GridFacadeError::Identity("corrupt identity id file".into()))?;
        Ok(Some(IdentityId::new(id_bytes)))
    }

    /// Read the public [`IdentityRecord`] for the current identity, or
    /// `None` if no identity has been created yet.
    pub fn get_identity(&self) -> Result<Option<IdentityRecord>, GridFacadeError> {
        let Some(id) = self.current_identity_id()? else {
            return Ok(None);
        };
        let store = self.open_store()?;
        store
            .get_identity(&id)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))
    }

    /// Generate + persist a brand new Neural Key identity.
    ///
    /// We bypass `Zero::bootstrap` here so identity creation works even
    /// when the GRID multiaddr is unreachable. This mirrors the
    /// `load_or_create_identity` helper in
    /// `zero_sdk::runtime`.
    pub fn create_identity(&self) -> Result<IdentityRecord, GridFacadeError> {
        if self.current_id_path().exists() {
            return Err(GridFacadeError::IdentityExists);
        }
        let store = self.open_store()?;

        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| GridFacadeError::Identity(format!("getrandom failed: {e}")))?;
        let identity = Identity::new(NeuralKey::new(bytes));

        let record = identity.to_record();
        let secret = identity.to_secret_record();
        store
            .put_identity(&record)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?;
        store
            .put_identity_secret(&secret)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?;

        write_current_id_atomic(&self.current_id_path(), identity.id().as_bytes())?;
        Ok(record)
    }

    /// List all `MachineKeyRecord`s for the current identity.
    pub fn list_devices(&self) -> Result<Vec<MachineKeyRecord>, GridFacadeError> {
        let Some(id) = self.current_identity_id()? else {
            return Ok(Vec::new());
        };
        let store = self.open_store()?;
        store
            .list_machines(&id)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))
    }

    /// Derive + persist a new machine key for the current identity.
    pub fn register_machine(&self, capabilities: u32) -> Result<MachineKeyRecord, GridFacadeError> {
        let Some(id) = self.current_identity_id()? else {
            return Err(GridFacadeError::IdentityMissing);
        };
        let store = self.open_store()?;

        let record = store
            .get_identity(&id)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?
            .ok_or_else(|| GridFacadeError::Identity("identity record missing".into()))?;
        let secret = store
            .get_identity_secret(&id)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?
            .ok_or_else(|| GridFacadeError::Identity("identity secret missing".into()))?;
        let neural_bytes: [u8; 32] = secret
            .neural_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| GridFacadeError::Identity("corrupt neural key length".into()))?;

        let mut identity = Identity::load(&store, record, neural_bytes)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?;

        let caps = MachineKeyCapabilities::from_bits_truncate(capabilities);
        let machine = identity
            .register_machine(caps, &store)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?;

        // Persist the updated identity record so `primary_machine` matches.
        let updated = identity.to_record();
        store
            .put_identity(&updated)
            .map_err(|e| GridFacadeError::Identity(e.to_string()))?;

        Ok(machine)
    }
}

/// Write the active identity id atomically to `path` (mirrors the helper
/// in `zero_sdk::runtime`).
fn write_current_id_atomic(path: &Path, id: &[u8; 16]) -> Result<(), GridFacadeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, id)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
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
        assert_eq!(st.multiaddr, ZeroConfig::default().grid_multiaddr);
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
        let again = runtime.create_identity();
        assert!(matches!(again, Err(GridFacadeError::IdentityExists)));
        // reload should give the same id
        let reloaded = runtime.get_identity().unwrap().unwrap();
        assert_eq!(reloaded.id, first.id);
    }

    #[test]
    fn create_identity_then_register_machine_shows_in_list() {
        let dir = tempdir().unwrap();
        let runtime = rt(dir.path());
        runtime.create_identity().unwrap();
        let caps = MachineKeyCapabilities::SEND_MESSAGES | MachineKeyCapabilities::RECEIVE_MESSAGES;
        let machine = runtime.register_machine(caps.bits()).unwrap();
        let list = runtime.list_devices().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].machine_id, machine.machine_id);
        assert_eq!(list[0].capabilities, caps);
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
}
