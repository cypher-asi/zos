//! Error type for the grid façade.

use thiserror::Error;

/// Errors surfaced by [`crate::ZeroRuntime`] and the persisted-config layer.
#[derive(Debug, Error)]
pub enum GridFacadeError {
    /// Filesystem I/O failure while reading or writing on-disk state.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Reading or writing the persisted `config.json` failed.
    #[error("config error: {0}")]
    Config(String),

    /// Identity-store layer failure (CBOR encode/decode, missing record,
    /// corrupt data on disk, ...).
    #[error("identity error: {0}")]
    Identity(String),

    /// `POST /api/identity` was called but an identity already exists.
    #[error("identity already exists")]
    IdentityExists,

    /// An operation requires an identity but none has been created yet.
    #[error("identity does not exist; create one first")]
    IdentityMissing,

    /// `Zero::bootstrap` failed (typically because the GRID multiaddr is
    /// unreachable). The error message is the SDK's own `Display` output.
    #[error("bootstrap failed: {0}")]
    Bootstrap(String),
}
