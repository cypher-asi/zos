//! Persisted runtime configuration at `<data_dir>/config.json`.
//!
//! Currently stores a single field — the GRID multiaddr — but the file is a
//! JSON object so we can grow the schema without breaking existing
//! installations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::GridFacadeError;

const CONFIG_FILE: &str = "config.json";

/// Default GRID multiaddr the UI seeds into a fresh `config.json`.
///
/// This mirrors the value the older `zero-sdk` exposed via
/// `ZeroConfig::default().grid_multiaddr` so existing installs and screen
/// captures keep matching after the migration to `zero-sdk-10`.
pub const DEFAULT_GRID_MULTIADDR: &str =
    "/ip4/3.129.15.45/tcp/3691/p2p/12D3KooWHvyFJm77ZAUR7DzAhRCjyhGgcwNxhQAoCptScBhQCs2b/p2p-circuit/p2p/12D3KooWHMNU9wSoHWUxW13tpz94h27L1F4ij26ytp9XsCPPhJNd/p2p/Zx12D3KooWHMNU9wSoHWUxW13tpz94h27L1F4ij26ytp9XsCPPhJNd";

/// On-disk runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedConfig {
    /// Multiaddr of the GRID relay to dial on connect.
    pub grid_multiaddr: String,
    /// Optional override for the libp2p connect-timeout, in milliseconds.
    /// `None` means "use the SDK default" (currently 30s) — written into
    /// `config.json` only after the user picks a custom value, so older
    /// installs without this key continue to deserialize cleanly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            grid_multiaddr: DEFAULT_GRID_MULTIADDR.to_owned(),
            connect_timeout_ms: None,
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
    fn default_uses_baked_in_multiaddr() {
        let cfg = PersistedConfig::default();
        assert_eq!(cfg.grid_multiaddr, DEFAULT_GRID_MULTIADDR);
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
            connect_timeout_ms: Some(45_000),
        };
        cfg.save(dir.path()).unwrap();
        let loaded = PersistedConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn legacy_config_without_timeout_field_parses_with_none() {
        let dir = tempdir().unwrap();
        std::fs::write(
            PersistedConfig::path(dir.path()),
            br#"{"grid_multiaddr":"/ip4/10.0.0.5/udp/4242/quic-v1"}"#,
        )
        .unwrap();
        let loaded = PersistedConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(loaded.grid_multiaddr, "/ip4/10.0.0.5/udp/4242/quic-v1");
        assert_eq!(loaded.connect_timeout_ms, None);
    }

    #[test]
    fn default_omits_timeout_field_in_serialized_form() {
        let cfg = PersistedConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("connect_timeout_ms"),
            "default should not write the field: {json}"
        );
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
