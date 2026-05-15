//! Wire-format DTOs shared between the façade and the HTTP handlers.
//!
//! The previous SDK returned `IdentityRecord` / `MachineKeyRecord` types
//! whose `serde_bytes` fields didn't round-trip cleanly through JSON. The
//! new SDK doesn't expose comparable types at all, so the DTOs here are
//! simply a flat projection of the `crate::persist` records onto the
//! shape the React client already consumes.

use serde::{Deserialize, Serialize};

use crate::persist::{PersistedDevice, PersistedIdentity};

/// `GET /api/grid/status`.
#[derive(Debug, Clone, Serialize)]
pub struct GridStatusDto {
    /// `true` when the runtime currently holds a live `ZeroSdk` and a
    /// dialled `RealGridClient`. The real upstream GRID library is still
    /// stubbed in `zero-sdk-10`, so this only reflects whether the local
    /// RocksDB opened and `RealGridClient::connect` returned `Ok`.
    pub connected: bool,
    /// Multiaddr that will be dialed on the next connect attempt.
    pub multiaddr: String,
    /// Hex-encoded identity id, if one has been created on disk.
    pub identity_id: Option<String>,
    /// Last bootstrap error, cleared when a connect succeeds or the user
    /// disconnects/sets a new multiaddr.
    pub last_error: Option<String>,
}

/// `GET /api/identity` (or `null` body when no identity exists yet).
#[derive(Debug, Clone, Serialize)]
pub struct IdentityDto {
    /// Hex-encoded `IdentityId` (32 chars, 16 bytes).
    pub identity_id: String,
    /// Current key-rotation epoch.
    pub epoch: u64,
    /// Unix milliseconds at which the identity was created.
    pub created_at: u64,
}

impl From<&PersistedIdentity> for IdentityDto {
    fn from(r: &PersistedIdentity) -> Self {
        Self {
            identity_id: r.identity_id.clone(),
            epoch: r.epoch,
            created_at: r.created_at_ms,
        }
    }
}

/// One row in `GET /api/devices`, also the response for `POST /api/devices`.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceDto {
    /// Hex-encoded `MachineId` (32 chars, 16 bytes).
    pub machine_id: String,
    /// Hex-encoded owning `IdentityId`.
    pub identity_id: String,
    /// Epoch at which this machine key was registered.
    pub epoch: u64,
    /// Capability bitflags (see `zid::keys::machine::MachineKeyCapabilities`).
    pub capabilities: u32,
    /// Unix milliseconds at which the machine key was created.
    pub created_at: u64,
}

impl From<&PersistedDevice> for DeviceDto {
    fn from(m: &PersistedDevice) -> Self {
        Self {
            machine_id: m.machine_id.clone(),
            identity_id: m.identity_id.clone(),
            epoch: m.epoch,
            capabilities: m.capabilities,
            created_at: m.created_at_ms,
        }
    }
}

/// Body of `POST /api/grid/config`.
#[derive(Debug, Deserialize)]
pub struct SetMultiaddrRequest {
    /// New GRID multiaddr to persist + dial on next connect.
    pub multiaddr: String,
}

/// Body of `POST /api/devices`.
#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    /// Optional human-friendly label persisted alongside the machine key.
    #[serde(default)]
    pub label: Option<String>,
    /// Capability bitflags (`SEND_MESSAGES | RECEIVE_MESSAGES = 3`).
    pub capabilities: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_device_request_label_optional() {
        let req: CreateDeviceRequest = serde_json::from_str(r#"{"capabilities":3}"#).unwrap();
        assert!(req.label.is_none());
        assert_eq!(req.capabilities, 3);
    }

    #[test]
    fn set_multiaddr_round_trip() {
        let req: SetMultiaddrRequest =
            serde_json::from_str(r#"{"multiaddr":"/ip4/127.0.0.1/udp/3690/quic-v1"}"#).unwrap();
        assert_eq!(req.multiaddr, "/ip4/127.0.0.1/udp/3690/quic-v1");
    }

    #[test]
    fn grid_status_serializes_null_optionals() {
        let s = GridStatusDto {
            connected: false,
            multiaddr: "/ip4/0.0.0.0/udp/3690/quic-v1".into(),
            identity_id: None,
            last_error: None,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["connected"], false);
        assert!(v["identity_id"].is_null());
        assert!(v["last_error"].is_null());
    }

    #[test]
    fn identity_dto_from_persisted() {
        let p = PersistedIdentity {
            identity_id: "ab".repeat(16),
            epoch: 5,
            created_at_ms: 1_700_000_000_000,
            shares: vec![],
            threshold: 2,
        };
        let dto = IdentityDto::from(&p);
        assert_eq!(dto.identity_id, p.identity_id);
        assert_eq!(dto.epoch, 5);
        assert_eq!(dto.created_at, 1_700_000_000_000);
    }

    #[test]
    fn device_dto_from_persisted() {
        let d = PersistedDevice {
            machine_id: "11".repeat(16),
            identity_id: "22".repeat(16),
            label: None,
            capabilities: 3,
            epoch: 2,
            created_at_ms: 1_700_000_001_000,
            ed25519_pub: "ee".repeat(32),
            mldsa65_pub: "dd".repeat(1952),
        };
        let dto = DeviceDto::from(&d);
        assert_eq!(dto.machine_id, d.machine_id);
        assert_eq!(dto.identity_id, d.identity_id);
        assert_eq!(dto.capabilities, 3);
        assert_eq!(dto.epoch, 2);
        assert_eq!(dto.created_at, 1_700_000_001_000);
    }
}
