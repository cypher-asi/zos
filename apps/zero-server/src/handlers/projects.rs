use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::dto::{CreateProject, Project, UpdateProject};
use crate::state::AppState;

pub(crate) async fn list_projects(State(state): State<AppState>) -> Json<Vec<Project>> {
    Json(state.db.lock().await.clone())
}

pub(crate) async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Json<Project> {
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: body.name,
        description: body.description,
        updated_at: Utc::now().to_rfc3339(),
    };
    state.db.lock().await.push(project.clone());
    Json(project)
}

pub(crate) async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Project>, StatusCode> {
    state
        .db
        .lock()
        .await
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub(crate) async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProject>,
) -> Result<Json<Project>, StatusCode> {
    let mut projects = state.db.lock().await;
    let project = projects
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    if let Some(name) = body.name {
        project.name = name;
    }
    if let Some(desc) = body.description {
        project.description = desc;
    }
    project.updated_at = Utc::now().to_rfc3339();
    Ok(Json(project.clone()))
}

pub(crate) async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let mut projects = state.db.lock().await;
    let len = projects.len();
    projects.retain(|p| p.id != id);
    if projects.len() < len {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
