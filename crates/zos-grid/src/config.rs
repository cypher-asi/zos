//! Persisted runtime configuration at `<data_dir>/config.json`.
//!
//! Currently stores a single field — the GRID multiaddr — but the file is a
//! JSON object so we can grow the schema without breaking existing
//! installations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zero_sdk::ZeroConfig;

use crate::error::GridFacadeError;

const CONFIG_FILE: &str = "config.json";

/// On-disk runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedConfig {
    /// Multiaddr of the GRID relay to dial on connect.
    pub grid_multiaddr: String,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            grid_multiaddr: ZeroConfig::default().grid_multiaddr,
        }
    }
}

impl PersistedConfig {
    /// Path of the on-disk config file under `data_dir`.
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join(CONFIG_FILE)
    }

    /// Read `config.json` from disk, creating it with defaults if missing.
    pub fn load_or_init(data_dir: &Path) -> Result<Self, GridFacadeError> {
        std::fs::create_dir_all(data_dir)?;
        let path = Self::path(data_dir);
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| GridFacadeError::Config(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Self::default();
                cfg.save(data_dir)?;
                Ok(cfg)
            }
            Err(e) => Err(GridFacadeError::Io(e)),
        }
    }

    /// Atomically persist `self` to `<data_dir>/config.json`.
    pub fn save(&self, data_dir: &Path) -> Result<(), GridFacadeError> {
        std::fs::create_dir_all(data_dir)?;
        let path = Self::path(data_dir);
        let tmp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| GridFacadeError::Config(format!("encode: {e}")))?;
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_uses_sdk_default_multiaddr() {
        let cfg = PersistedConfig::default();
        assert_eq!(cfg.grid_multiaddr, ZeroConfig::default().grid_multiaddr);
    }

    #[test]
    fn load_or_init_creates_file_when_absent() {
        let dir = tempdir().unwrap();
        let cfg = PersistedConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(cfg, PersistedConfig::default());
        assert!(PersistedConfig::path(dir.path()).exists());
    }

    #[test]
    fn round_trip_preserves_fields() {
        let dir = tempdir().unwrap();
        let cfg = PersistedConfig {
            grid_multiaddr: "/ip4/10.0.0.5/udp/4242/quic-v1".into(),
        };
        cfg.save(dir.path()).unwrap();
        let loaded = PersistedConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn load_or_init_idempotent_on_second_call() {
        let dir = tempdir().unwrap();
        let first = PersistedConfig::load_or_init(dir.path()).unwrap();
        let second = PersistedConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn corrupt_json_is_reported_as_config_error() {
        let dir = tempdir().unwrap();
        std::fs::write(PersistedConfig::path(dir.path()), b"not json").unwrap();
        let err = PersistedConfig::load_or_init(dir.path()).unwrap_err();
        assert!(matches!(err, GridFacadeError::Config(_)));
    }
}
