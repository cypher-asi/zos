//! `/api/grid/*` handlers — status / config / connect / disconnect.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use zos_grid::{GridFacadeError, GridStatusDto, SetMultiaddrRequest};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Map a façade error to the standard ApiError envelope.
fn map_err(e: GridFacadeError) -> (StatusCode, Json<ApiError>) {
    match e {
        GridFacadeError::IdentityExists => (
            StatusCode::CONFLICT,
            Json(ApiError {
                error: "identity already exists".into(),
                code: Some("exists".into()),
            }),
        ),
        GridFacadeError::IdentityMissing => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: e.to_string(),
                code: Some("not_found".into()),
            }),
        ),
        other => ApiError::internal(other.to_string()),
    }
}

pub(crate) async fn status(State(state): State<AppState>) -> ApiResult<Json<GridStatusDto>> {
    let s = state.grid.status().await.map_err(map_err)?;
    Ok(Json(s))
}

pub(crate) async fn set_config(
    State(state): State<AppState>,
    Json(req): Json<SetMultiaddrRequest>,
) -> ApiResult<Json<GridStatusDto>> {
    state
        .grid
        .set_multiaddr(req.multiaddr)
        .await
        .map_err(map_err)?;
    let s = state.grid.status().await.map_err(map_err)?;
    Ok(Json(s))
}

pub(crate) async fn connect(State(state): State<AppState>) -> ApiResult<Json<GridStatusDto>> {
    // Bootstrap may fail (no GRID node reachable). We surface that via the
    // `last_error` field in the returned status rather than a 5xx.
    let _ = state.grid.ensure_started().await;
    let s = state.grid.status().await.map_err(map_err)?;
    Ok(Json(s))
}

pub(crate) async fn disconnect(State(state): State<AppState>) -> ApiResult<Json<GridStatusDto>> {
    state.grid.disconnect().await;
    let s = state.grid.status().await.map_err(map_err)?;
    Ok(Json(s))
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

    fn make_session() -> ZeroAuthSession {
        let now = Utc::now();
        ZeroAuthSession {
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
        }
    }

    fn make_state(data_dir: std::path::PathBuf) -> AppState {
        let cache: ValidationCache = Arc::new(DashMap::new());
        cache.insert(
            "jwt".into(),
            CachedSession {
                session: make_session(),
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
    async fn status_on_fresh_data_dir_is_disconnected() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(s) = status(State(state)).await.unwrap();
        assert!(!s.connected);
        assert!(s.identity_id.is_none());
        assert!(s.last_error.is_none());
    }

    #[tokio::test]
    async fn set_config_persists_multiaddr() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let req = SetMultiaddrRequest {
            multiaddr: "/ip4/127.0.0.1/udp/12345/quic-v1".into(),
        };
        let Json(s) = set_config(State(state), Json(req)).await.unwrap();
        assert_eq!(s.multiaddr, "/ip4/127.0.0.1/udp/12345/quic-v1");
        assert!(!s.connected);
    }

    #[tokio::test]
    async fn disconnect_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(s1) = disconnect(State(state.clone())).await.unwrap();
        let Json(s2) = disconnect(State(state)).await.unwrap();
        assert!(!s1.connected);
        assert!(!s2.connected);
    }
}
