//! Error type for the grid façade.

use thiserror::Error;

use zero_messaging::contacts::types::ContactError;
use zero_sdk::{DmError, InboxError};

/// Errors surfaced by [`crate::ZeroRuntime`] and the persisted-config layer.
#[derive(Debug, Error)]
pub enum GridFacadeError {
    /// Filesystem I/O failure while reading or writing on-disk state.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Reading or writing the persisted `config.json` failed.
    #[error("config error: {0}")]
    Config(String),

    /// Identity-store layer failure (encode/decode, missing record,
    /// corrupt data on disk, ...).
    #[error("identity error: {0}")]
    Identity(String),

    /// `POST /api/identity` was called but an identity already exists.
    #[error("identity already exists")]
    IdentityExists,

    /// An operation requires an identity but none has been created yet.
    #[error("identity does not exist; create one first")]
    IdentityMissing,

    /// Booting the `ZeroSdk` (RocksDB open or GRID dial) failed. The
    /// message is the SDK's own `Display` output.
    #[error("bootstrap failed: {0}")]
    Bootstrap(String),

    /// Caller passed a hex-encoded id that did not decode to the expected
    /// length (e.g. a 16-byte `IdentityId` or 32-byte `ConversationId`).
    #[error("invalid hex id: {0}")]
    BadHex(String),

    /// Caller-addressable resource (conversation / contact / message) does
    /// not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// Error originating from the upstream DM service.
    #[error("dm error: {0}")]
    Dm(String),

    /// Error originating from the upstream inbox service.
    #[error("inbox error: {0}")]
    Inbox(String),

    /// Error originating from the upstream contact store.
    #[error("contact error: {0}")]
    Contact(String),
}

impl From<DmError> for GridFacadeError {
    fn from(e: DmError) -> Self {
        match e {
            DmError::ContactNotFound(_)
            | DmError::UnknownConversation(_)
            | DmError::MessageNotFound(_) => GridFacadeError::NotFound(e.to_string()),
            other => GridFacadeError::Dm(other.to_string()),
        }
    }
}

impl From<InboxError> for GridFacadeError {
    fn from(e: InboxError) -> Self {
        GridFacadeError::Inbox(e.to_string())
    }
}

impl From<ContactError> for GridFacadeError {
    fn from(e: ContactError) -> Self {
        match e {
            ContactError::NotFound(_) => GridFacadeError::NotFound(e.to_string()),
            other => GridFacadeError::Contact(other.to_string()),
        }
    }
}
