//! `/api/devices` handlers — list and derive machine keys for the current
//! identity.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use zos_grid::{CreateDeviceRequest, DeviceDto, GridFacadeError};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn map_err(e: GridFacadeError) -> (StatusCode, Json<ApiError>) {
    match e {
        GridFacadeError::IdentityMissing => (
            StatusCode::CONFLICT,
            Json(ApiError {
                error: e.to_string(),
                code: Some("identity_missing".into()),
            }),
        ),
        other => ApiError::internal(other.to_string()),
    }
}

pub(crate) async fn list_devices(State(state): State<AppState>) -> ApiResult<Json<Vec<DeviceDto>>> {
    let machines = state.grid.list_devices().map_err(map_err)?;
    let dtos = machines.iter().map(DeviceDto::from).collect();
    Ok(Json(dtos))
}

pub(crate) async fn create_device(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> ApiResult<(StatusCode, Json<DeviceDto>)> {
    if let Some(label) = req.label.as_deref() {
        tracing::debug!(label, "device label received (currently unused)");
    }
    let machine = state
        .grid
        .register_machine(req.capabilities)
        .map_err(map_err)?;
    Ok((StatusCode::CREATED, Json(DeviceDto::from(&machine))))
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
    async fn list_devices_is_empty_without_identity() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(list) = list_devices(State(state)).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn create_device_without_identity_returns_409() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let req = CreateDeviceRequest {
            label: None,
            capabilities: 3,
        };
        let err = create_device(State(state), Json(req)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_then_list_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        // create identity first using the runtime directly
        state.grid.create_identity().unwrap();

        let req = CreateDeviceRequest {
            label: Some("laptop".into()),
            capabilities: 3,
        };
        let (status, Json(created)) = create_device(State(state.clone()), Json(req))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.capabilities, 3);

        let Json(list) = list_devices(State(state)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].machine_id, created.machine_id);
    }
}
