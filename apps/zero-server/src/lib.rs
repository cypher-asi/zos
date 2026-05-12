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

pub fn create_router(interface_dir: Option<PathBuf>) -> axum::Router {
    let state = AppState {
        auth_service: Arc::new(zos_auth::AuthService::new()),
        validation_cache: Arc::new(DashMap::new()),
        db: Arc::new(Mutex::new(seed_projects())),
    };

    router::create_router(state, interface_dir)
}
