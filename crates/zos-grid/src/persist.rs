//! Local on-disk persistence for things the upstream `zero-sdk-10` does
//! not currently track itself.
//!
//! The new SDK exposes [`zero_sdk::NeuralKey`] only as an opaque value with
//! no public byte accessors, and its [`zero_sdk::MachineKeyStore`] is
//! in-memory. To preserve the HTTP-facing semantics of the older `zero-sdk`
//! façade (an identity that survives across restarts, machine keys with
//! capability bitflags, monotonically-increasing epoch), this module owns
//! two small JSON files:
//!
//! * `<data_dir>/identity/identity.json` — the active [`PersistedIdentity`].
//! * `<data_dir>/identity/devices.json`  — the [`PersistedDevices`] list.
//!
//! The `NeuralKey` itself is round-tripped through Shamir 2-of-2 because
//! that's the only public path back from raw bytes to a `NeuralKey` value
//! in the new SDK.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zero_identity::neural_key::{
    recover_neural_key, split_neural_key, NeuralKey, NeuralKeyShares, ShareConfig,
};

use crate::error::GridFacadeError;

const IDENTITY_DIR: &str = "identity";
const IDENTITY_FILE: &str = "identity.json";
const DEVICES_FILE: &str = "devices.json";

/// 2-of-2 Shamir split is used purely as a public API for serialising the
/// otherwise-opaque [`NeuralKey`] bytes. Storing both shares side-by-side
/// is equivalent to storing the raw 32-byte secret; the security model
/// here is "trust the local data dir" exactly like the older
/// `FsIdentityStore`.
const SHAMIR_TOTAL: u8 = 2;
const SHAMIR_THRESHOLD: u8 = 2;

/// On-disk identity record. Drives both DTO conversion and Neural Key
/// recovery on every `ensure_started`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedIdentity {
    /// Hex-encoded 16-byte identity id.
    pub identity_id: String,
    /// Monotonic epoch counter (bumped on every machine-key registration).
    pub epoch: u64,
    /// Unix-ms timestamp at which the identity was created.
    pub created_at_ms: u64,
    /// Hex-encoded Shamir shares; pass through to [`recover`] to rebuild
    /// the [`NeuralKey`].
    pub shares: Vec<String>,
    /// Threshold needed to recombine shares (always [`SHAMIR_THRESHOLD`]
    /// for newly-created identities).
    pub threshold: u8,
}

/// On-disk machine-key entry. Mirrors the previous `MachineKeyRecord`
/// shape so the wire DTO is unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedDevice {
    /// Hex-encoded 16-byte machine id.
    pub machine_id: String,
    /// Hex-encoded owning identity id.
    pub identity_id: String,
    /// Caller-provided label (`POST /api/devices` body), currently unused.
    pub label: Option<String>,
    /// Capability bitflags chosen by the caller.
    pub capabilities: u32,
    /// Epoch at which this machine was registered.
    pub epoch: u64,
    /// Unix-ms timestamp at which the device was registered.
    pub created_at_ms: u64,
    /// Hex-encoded Ed25519 verifying key (32 bytes).
    pub ed25519_pub: String,
    /// Hex-encoded ML-DSA-65 verifying key (1952 bytes).
    pub mldsa65_pub: String,
}

/// Container for the on-disk device list (so the file is always a JSON
/// object even when empty).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedDevices {
    pub devices: Vec<PersistedDevice>,
}

// ── Path helpers ──────────────────────────────────────────────────────────

pub fn identity_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(IDENTITY_DIR)
}

pub fn identity_path(data_dir: &Path) -> PathBuf {
    identity_dir(data_dir).join(IDENTITY_FILE)
}

pub fn devices_path(data_dir: &Path) -> PathBuf {
    identity_dir(data_dir).join(DEVICES_FILE)
}

/// Path that the old `zero_sdk::runtime` used to mark "identity exists".
/// Read-only here; we never write it. See migration test in `runtime.rs`.
const LEGACY_CURRENT_ID_FILE: &str = "current_id";

pub fn legacy_current_id_path(data_dir: &Path) -> PathBuf {
    identity_dir(data_dir).join(LEGACY_CURRENT_ID_FILE)
}

// ── Identity I/O ──────────────────────────────────────────────────────────

/// Serialise `key` into the on-disk Shamir share representation.
pub fn split(key: &NeuralKey) -> Result<(Vec<String>, u8), GridFacadeError> {
    let cfg = ShareConfig {
        threshold: SHAMIR_THRESHOLD,
        total: SHAMIR_TOTAL,
    };
    let NeuralKeyShares { shares, threshold } = split_neural_key(key, cfg)
        .map_err(|e| GridFacadeError::Identity(format!("shamir split failed: {e}")))?;
    Ok((shares.iter().map(hex::encode).collect(), threshold))
}

/// Reconstruct the [`NeuralKey`] from a [`PersistedIdentity`] previously
/// produced by [`split`].
pub fn recover(record: &PersistedIdentity) -> Result<NeuralKey, GridFacadeError> {
    let mut decoded = Vec::with_capacity(record.shares.len());
    for s in &record.shares {
        let raw = hex::decode(s)
            .map_err(|e| GridFacadeError::Identity(format!("hex decode share: {e}")))?;
        decoded.push(raw);
    }
    recover_neural_key(&decoded, record.threshold)
        .map_err(|e| GridFacadeError::Identity(format!("shamir recover failed: {e}")))
}

pub fn read_identity(data_dir: &Path) -> Result<Option<PersistedIdentity>, GridFacadeError> {
    let path = identity_path(data_dir);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let record: PersistedIdentity = serde_json::from_slice(&bytes)
                .map_err(|e| GridFacadeError::Identity(format!("parse {}: {e}", path.display())))?;
            Ok(Some(record))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(GridFacadeError::Io(e)),
    }
}

pub fn write_identity(data_dir: &Path, record: &PersistedIdentity) -> Result<(), GridFacadeError> {
    std::fs::create_dir_all(identity_dir(data_dir))?;
    let path = identity_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(record)
        .map_err(|e| GridFacadeError::Identity(format!("encode identity: {e}")))?;
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

// ── Devices I/O ───────────────────────────────────────────────────────────

pub fn read_devices(data_dir: &Path) -> Result<PersistedDevices, GridFacadeError> {
    let path = devices_path(data_dir);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| GridFacadeError::Identity(format!("parse {}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PersistedDevices::default()),
        Err(e) => Err(GridFacadeError::Io(e)),
    }
}

pub fn write_devices(data_dir: &Path, devices: &PersistedDevices) -> Result<(), GridFacadeError> {
    std::fs::create_dir_all(identity_dir(data_dir))?;
    let path = devices_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(devices)
        .map_err(|e| GridFacadeError::Identity(format!("encode devices: {e}")))?;
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn split_and_recover_round_trip() {
        let key = NeuralKey::generate().unwrap();
        let id_a = key.identity_id_bytes();
        let (shares, threshold) = split(&key).unwrap();
        assert_eq!(shares.len(), SHAMIR_TOTAL as usize);
        assert_eq!(threshold, SHAMIR_THRESHOLD);

        let record = PersistedIdentity {
            identity_id: hex::encode(id_a),
            epoch: 0,
            created_at_ms: 0,
            shares,
            threshold,
        };
        let recovered = recover(&record).unwrap();
        assert_eq!(recovered.identity_id_bytes(), id_a);
    }

    #[test]
    fn read_identity_returns_none_on_fresh_dir() {
        let dir = tempdir().unwrap();
        assert!(read_identity(dir.path()).unwrap().is_none());
    }

    #[test]
    fn write_then_read_identity_round_trip() {
        let dir = tempdir().unwrap();
        let record = PersistedIdentity {
            identity_id: "00".repeat(16),
            epoch: 7,
            created_at_ms: 1_700_000_000_000,
            shares: vec!["aa".into(), "bb".into()],
            threshold: 2,
        };
        write_identity(dir.path(), &record).unwrap();
        let loaded = read_identity(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, record);
    }

    #[test]
    fn read_devices_returns_empty_on_fresh_dir() {
        let dir = tempdir().unwrap();
        let devs = read_devices(dir.path()).unwrap();
        assert!(devs.devices.is_empty());
    }

    #[test]
    fn write_then_read_devices_round_trip() {
        let dir = tempdir().unwrap();
        let devs = PersistedDevices {
            devices: vec![PersistedDevice {
                machine_id: "11".repeat(16),
                identity_id: "22".repeat(16),
                label: Some("laptop".into()),
                capabilities: 3,
                epoch: 1,
                created_at_ms: 1_700_000_001_000,
                ed25519_pub: "ee".repeat(32),
                mldsa65_pub: "dd".repeat(1952),
            }],
        };
        write_devices(dir.path(), &devs).unwrap();
        let loaded = read_devices(dir.path()).unwrap();
        assert_eq!(loaded, devs);
    }
}
