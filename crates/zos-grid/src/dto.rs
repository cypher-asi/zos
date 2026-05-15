//! Wire-format DTOs shared between the façade and the HTTP handlers.
//!
//! `MachineKeyRecord` and `IdentityRecord` themselves contain `serde_bytes`
//! byte fields that don't round-trip cleanly through JSON. The DTOs in this
//! module flatten them to a friendly `{ machine_id, identity_id, ... }` shape
//! suited to the React client.

use serde::{Deserialize, Serialize};
use zero_sdk::identity::{IdentityRecord, MachineKeyRecord};

/// `GET /api/grid/status`.
#[derive(Debug, Clone, Serialize)]
pub struct GridStatusDto {
    /// `true` when the runtime currently holds an `Arc<Zero>`. The SDK does
    /// not expose a transport-level liveness check, so this only reflects
    /// whether bootstrap has succeeded since the last (re)connect.
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

impl From<&IdentityRecord> for IdentityDto {
    fn from(r: &IdentityRecord) -> Self {
        Self {
            identity_id: r.id.to_string(),
            epoch: r.epoch.as_u64(),
            created_at: u64::try_from(r.created_at.as_i64().max(0)).unwrap_or(0),
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
    /// Capability bitflags (see `zero_identity::MachineKeyCapabilities`).
    pub capabilities: u32,
    /// Unix milliseconds at which the machine key was created.
    pub created_at: u64,
}

impl From<&MachineKeyRecord> for DeviceDto {
    fn from(m: &MachineKeyRecord) -> Self {
        Self {
            machine_id: m.machine_id.to_string(),
            identity_id: m.identity_id.to_string(),
            epoch: m.epoch.as_u64(),
            capabilities: m.capabilities.bits(),
            created_at: u64::try_from(m.created_at.as_i64().max(0)).unwrap_or(0),
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
    /// Optional human-friendly label. Currently unused on disk; reserved so
    /// the UI can start sending it without a follow-up server change.
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
}
