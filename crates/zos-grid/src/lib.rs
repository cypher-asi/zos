//! Thin façade around the [`zero_sdk::Zero`] runtime for the `zero-server`
//! HTTP layer.
//!
//! Owns:
//!
//! * A lazily-bootstrapped `Arc<Zero>` so the server can start before any
//!   GRID node is reachable.
//! * A small persisted `config.json` storing the user-chosen GRID
//!   multiaddr.
//! * A flat DTO layer the HTTP handlers can hand straight to `axum::Json`.
//!
//! Both `cargo run -p zero-server` and the embedded server inside
//! `cargo run -p zero-desktop` use this façade and share the same on-disk
//! state via the `ZERO_DATA_DIR` env var.

#![forbid(unsafe_code)]

pub mod config;
pub mod dto;
mod error;
mod runtime;

pub use config::PersistedConfig;
pub use dto::{CreateDeviceRequest, DeviceDto, GridStatusDto, IdentityDto, SetMultiaddrRequest};
pub use error::GridFacadeError;
pub use runtime::ZeroRuntime;
