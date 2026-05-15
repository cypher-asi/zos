mod auth_guard;
mod dto;
mod error;
mod handlers;
mod router;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

use dto::Project;
use state::AppState;

fn seed_projects() -> Vec<Project> {
    let now = Utc::now().to_rfc3339();
    vec![
        Project {
            id: Uuid::new_v4().to_string(),
            name: "my-website".into(),
            description: "Personal portfolio built with React".into(),
            updated_at: now.clone(),
        },
        Project {
            id: Uuid::new_v4().to_string(),
            name: "api-service".into(),
            description: "REST API microservice in Rust".into(),
            updated_at: now.clone(),
        },
        Project {
            id: Uuid::new_v4().to_string(),
            name: "mobile-app".into(),
            description: "Cross-platform mobile application".into(),
            updated_at: now,
        },
    ]
}

/// Resolve the on-disk data directory, in order:
///
/// 1. `ZERO_DATA_DIR` environment variable (set by `zero-desktop`).
/// 2. `dirs::data_local_dir()/zero` (matches the desktop binary's default).
/// 3. `./.zero-data` as a last resort if neither is available.
pub fn resolve_data_dir() -> PathBuf {
    if let Some(env) = std::env::var_os("ZERO_DATA_DIR") {
        return PathBuf::from(env);
    }
    dirs::data_local_dir()
        .map(|d| d.join("zero"))
        .unwrap_or_else(|| PathBuf::from(".zero-data"))
}

pub fn create_router(interface_dir: Option<PathBuf>) -> axum::Router {
    let data_dir = resolve_data_dir();
    tracing::info!(data_dir = %data_dir.display(), "zero-server using data dir");

    let grid = zos_grid::ZeroRuntime::new(data_dir).expect("failed to initialise zos-grid runtime");

    let state = AppState {
        auth_service: Arc::new(zos_auth::AuthService::new()),
        validation_cache: Arc::new(DashMap::new()),
        db: Arc::new(Mutex::new(seed_projects())),
        grid,
    };

    router::create_router(state, interface_dir)
}
