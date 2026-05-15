//! Thin façade around the [`zero_sdk::ZeroSdk`] runtime for the
//! `zero-server` HTTP layer.
//!
//! Owns:
//!
//! * A lazily-bootstrapped pair of `Arc<ZeroSdk>` + `Arc<RealGridClient>`
//!   so the server can start before any GRID node is reachable.
//! * A small persisted `config.json` storing the user-chosen GRID
//!   multiaddr.
//! * A local on-disk identity / device store (see [`persist`]) sitting
//!   alongside the SDK's RocksDB. The new `zero-sdk-10` does not persist
//!   `NeuralKey` material itself, so this façade keeps that bookkeeping.
//! * A flat DTO layer the HTTP handlers can hand straight to `axum::Json`.
//!
//! Both `cargo run -p zero-server` and the embedded server inside
//! `cargo run -p zero-desktop` use this façade and share the same on-disk
//! state via the `ZERO_DATA_DIR` env var.

#![forbid(unsafe_code)]

pub mod config;
pub mod dto;
mod error;
pub mod persist;
mod runtime;

pub use config::{PersistedConfig, DEFAULT_GRID_MULTIADDR};
pub use dto::{CreateDeviceRequest, DeviceDto, GridStatusDto, IdentityDto, SetMultiaddrRequest};
pub use error::GridFacadeError;
pub use persist::{PersistedDevice, PersistedDevices, PersistedIdentity};
pub use runtime::ZeroRuntime;
