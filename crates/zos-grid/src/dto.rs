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
    /// Custom connect-timeout (milliseconds) the user has configured, or
    /// `null` to indicate the SDK default (currently 30 000 ms) is in
    /// effect. Applied on the next `connect` only.
    pub connect_timeout_ms: Option<u64>,
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

/// Body of `POST /api/grid/timeout`.
///
/// `timeout_ms == None` (i.e. the JSON value `null`) clears any custom
/// override and falls back to the SDK default. Any positive integer is
/// taken as the connect-timeout in milliseconds. Range validation is
/// performed in the HTTP handler / UI rather than here so the DTO stays
/// a pure wire shape.
#[derive(Debug, Deserialize)]
pub struct SetTimeoutRequest {
    /// New connect-timeout in milliseconds, or `null` to revert to the
    /// SDK default.
    pub timeout_ms: Option<u64>,
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

// ── Chat DTOs ─────────────────────────────────────────────────────────────
//
// The chat DTOs project the upstream `zero-sdk-10` `Conversation` /
// `Message` / `Contact` types onto a flat snake_case JSON shape the React
// chat UI consumes via `/api/chat/...`. All ids are hex-encoded:
//
// * `conversation_id` — 64 hex chars (32 bytes).
// * `message_id`      — 32 hex chars (16 bytes, UUID v7).
// * `identity_id` / `machine_id` — 32 hex chars (16 bytes).

/// One row in `GET /api/chat/conversations`. Synthesised by combining
/// `ContactStore` (for the peer label / identity) with `DmService::history`
/// (for the latest message timestamp) and `InboxService` (for unread
/// counts when the inbox has been populated).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationDto {
    /// Hex-encoded `ConversationId`.
    pub id: String,
    /// `"dm"` or `"group"`. Phase 5 only emits DMs.
    pub kind: String,
    /// Hex-encoded peer `IdentityId` (`Some` for DMs).
    pub contact_id: Option<String>,
    /// Display name (the contact label for DMs).
    pub name: Option<String>,
    /// Unix milliseconds at which the latest message was created, or `None`
    /// when the conversation has no messages yet.
    pub last_message_at: Option<u64>,
    /// Snippet of the last message body (capped to ~140 chars).
    pub last_message_preview: Option<String>,
    /// Number of unread messages in this conversation.
    pub unread_count: u64,
}

/// One message in `GET /api/chat/conversations/:id/messages`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageDto {
    /// Hex-encoded `MessageId`.
    pub id: String,
    /// Hex-encoded `ConversationId`.
    pub conversation_id: String,
    /// Hex-encoded sender `MachineId`.
    pub sender_machine_id: String,
    /// Hex-encoded sender `IdentityId`.
    pub sender_identity_id: String,
    /// Plain-text body.
    pub body: String,
    /// Unix milliseconds at which the message was sent.
    pub sent_at: u64,
    /// `"queued" | "sent" | "delivered" | "read"`.
    pub status: String,
}

/// Per-machine public key bundle for a contact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactMachineKeyDto {
    /// Hex-encoded `MachineId`.
    pub machine_id: String,
    /// Hex-encoded Ed25519 verifying key (32 bytes).
    pub ed25519_pub_hex: String,
    /// Hex-encoded ML-DSA-65 verifying key (1952 bytes).
    pub mldsa65_pub_hex: String,
}

/// One row in `GET /api/chat/contacts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactDto {
    /// Hex-encoded `IdentityId` of the contact (also acts as the row id).
    pub id: String,
    /// Caller-chosen label, capped at 64 chars by the SDK.
    pub label: String,
    /// Hex-encoded `IdentityId` (duplicated as `id` for clarity at call
    /// sites that want a strongly-named field).
    pub identity_id: String,
    /// Per-machine public keys known for this contact, possibly empty.
    pub machine_keys: Vec<ContactMachineKeyDto>,
    /// Unix milliseconds at which the contact was added.
    pub added_at: u64,
}

/// Body of `POST /api/chat/contacts`.
#[derive(Debug, Deserialize)]
pub struct AddContactRequest {
    /// Caller-friendly label (≤64 chars per `ContactStore::add_contact`).
    pub label: String,
    /// Hex-encoded `IdentityId` of the contact (32 chars).
    pub identity_id: String,
}

/// Body of `POST /api/chat/conversations`.
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    /// Hex-encoded contact `IdentityId` to open / resume a DM with.
    pub contact_id: String,
}

/// Body of `POST /api/chat/conversations/:id/messages`.
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Plain-text body of the new message.
    pub body: String,
}

/// Query params for `GET /api/chat/conversations`.
#[derive(Debug, Deserialize, Default)]
pub struct ListConversationsQuery {
    /// Optional cap on the number of conversations returned.
    pub limit: Option<usize>,
}

/// Query params for `GET /api/chat/conversations/:id/messages`.
#[derive(Debug, Deserialize, Default)]
pub struct ListMessagesQuery {
    /// Hex-encoded `MessageId` cursor — page starts strictly before this id
    /// (exclusive upper bound). Omit to load the latest page.
    pub before: Option<String>,
    /// Page size cap, defaults to 50.
    pub limit: Option<usize>,
}

/// Frame pushed over the WebSocket `/api/chat/stream`. Tagged so we can
/// add `conversation_updated` / `presence` etc. without a breaking change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum MessageEnvelopeDto {
    /// A new message landed in (or was sent from) one of our conversations.
    Message {
        /// The full DTO of the new message.
        message: MessageDto,
    },
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
    fn set_timeout_request_accepts_positive_number() {
        let req: SetTimeoutRequest = serde_json::from_str(r#"{"timeout_ms":45000}"#).unwrap();
        assert_eq!(req.timeout_ms, Some(45_000));
    }

    #[test]
    fn set_timeout_request_accepts_null() {
        let req: SetTimeoutRequest = serde_json::from_str(r#"{"timeout_ms":null}"#).unwrap();
        assert_eq!(req.timeout_ms, None);
    }

    #[test]
    fn grid_status_serializes_null_optionals() {
        let s = GridStatusDto {
            connected: false,
            multiaddr: "/ip4/0.0.0.0/udp/3690/quic-v1".into(),
            connect_timeout_ms: None,
            identity_id: None,
            last_error: None,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["connected"], false);
        assert!(v["connect_timeout_ms"].is_null());
        assert!(v["identity_id"].is_null());
        assert!(v["last_error"].is_null());
    }

    #[test]
    fn grid_status_serializes_some_timeout_as_number() {
        let s = GridStatusDto {
            connected: true,
            multiaddr: "/ip4/0.0.0.0/udp/3690/quic-v1".into(),
            connect_timeout_ms: Some(60_000),
            identity_id: None,
            last_error: None,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["connect_timeout_ms"], 60_000);
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
