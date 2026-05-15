//! `/api/identity` handlers — read or create the local Neural Key identity.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use zos_grid::{GridFacadeError, IdentityDto};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn map_err(e: GridFacadeError) -> (StatusCode, Json<ApiError>) {
    match e {
        GridFacadeError::IdentityExists => (
            StatusCode::CONFLICT,
            Json(ApiError {
                error: "identity already exists".into(),
                code: Some("exists".into()),
            }),
        ),
        other => ApiError::internal(other.to_string()),
    }
}

pub(crate) async fn get_identity(
    State(state): State<AppState>,
) -> ApiResult<Json<Option<IdentityDto>>> {
    let record = state.grid.get_identity().map_err(map_err)?;
    Ok(Json(record.as_ref().map(IdentityDto::from)))
}

pub(crate) async fn create_identity(
    State(state): State<AppState>,
) -> ApiResult<(StatusCode, Json<IdentityDto>)> {
    let record = state.grid.create_identity().map_err(map_err)?;
    let dto = IdentityDto::from(&record);
    Ok((StatusCode::CREATED, Json(dto)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CachedSession, ValidationCache};
    use chrono::Utc;
    use dashmap::DashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;
    use zos_auth::ZeroAuthSession;

    fn make_state(data_dir: std::path::PathBuf) -> AppState {
        let cache: ValidationCache = Arc::new(DashMap::new());
        let now = Utc::now();
        cache.insert(
            "jwt".into(),
            CachedSession {
                session: ZeroAuthSession {
                    user_id: "u".into(),
                    display_name: "T".into(),
                    profile_image: String::new(),
                    primary_zid: "0://t".into(),
                    zero_wallet: "0xabc".into(),
                    wallets: vec![],
                    access_token: "jwt".into(),
                    is_zero_pro: false,
                    created_at: now,
                    validated_at: now,
                },
                validated_at: Instant::now(),
            },
        );
        AppState {
            auth_service: Arc::new(zos_auth::AuthService::new()),
            validation_cache: cache,
            db: Arc::new(Mutex::new(vec![])),
            grid: zos_grid::ZeroRuntime::new(data_dir).unwrap(),
        }
    }

    #[tokio::test]
    async fn get_identity_is_null_on_fresh_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(opt) = get_identity(State(state)).await.unwrap();
        assert!(opt.is_none());
    }

    #[tokio::test]
    async fn create_then_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let (status, Json(created)) = create_identity(State(state.clone())).await.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        let Json(read) = get_identity(State(state)).await.unwrap();
        assert_eq!(read.unwrap().identity_id, created.identity_id);
    }

    #[tokio::test]
    async fn create_twice_returns_409() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let _ = create_identity(State(state.clone())).await.unwrap();
        let err = create_identity(State(state)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::CONFLICT);
        assert_eq!(err.1.code.as_deref(), Some("exists"));
    }
}
