use std::sync::Arc;
use std::time::Instant;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;
use dashmap::DashMap;
use tokio::sync::Mutex;

use zos_auth::{AuthService, ZeroAuthSession};

use crate::dto::Project;
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Per-request auth extractors (set by `require_verified_session` middleware)
// ---------------------------------------------------------------------------

/// JWT access token extracted from the `Authorization: Bearer <token>` header.
#[derive(Clone, Debug)]
pub(crate) struct AuthJwt(pub String);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthJwt {
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthJwt>()
            .cloned()
            .ok_or_else(|| ApiError::unauthorized("missing auth token"))
    }
}

/// Full authenticated session, available after middleware validation.
#[derive(Clone, Debug)]
pub(crate) struct AuthSession(pub ZeroAuthSession);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthSession {
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthSession>()
            .cloned()
            .ok_or_else(|| ApiError::unauthorized("missing auth session"))
    }
}

// ---------------------------------------------------------------------------
// Validation cache
// ---------------------------------------------------------------------------

pub(crate) struct CachedSession {
    pub session: ZeroAuthSession,
    pub validated_at: Instant,
}

pub(crate) type ValidationCache = Arc<DashMap<String, CachedSession>>;
pub(crate) type Db = Arc<Mutex<Vec<Project>>>;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct AppState {
    pub auth_service: Arc<AuthService>,
    pub validation_cache: ValidationCache,
    pub db: Db,
    pub grid: Arc<zos_grid::ZeroRuntime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_session() -> ZeroAuthSession {
        let now = Utc::now();
        ZeroAuthSession {
            user_id: "u1".into(),
            display_name: "Test".into(),
            profile_image: String::new(),
            primary_zid: "0://test".into(),
            zero_wallet: "0xabc".into(),
            wallets: vec![],
            access_token: "jwt".into(),
            is_zero_pro: false,
            created_at: now,
            validated_at: now,
        }
    }

    #[test]
    fn cached_session_stores_instant() {
        let cached = CachedSession {
            session: make_session(),
            validated_at: Instant::now(),
        };
        assert!(cached.validated_at.elapsed().as_secs() < 1);
    }

    #[test]
    fn validation_cache_insert_and_get() {
        let cache: ValidationCache = Arc::new(DashMap::new());
        cache.insert(
            "jwt-1".into(),
            CachedSession {
                session: make_session(),
                validated_at: Instant::now(),
            },
        );
        assert!(cache.get("jwt-1").is_some());
        assert!(cache.get("jwt-2").is_none());
    }

    #[test]
    fn validation_cache_remove() {
        let cache: ValidationCache = Arc::new(DashMap::new());
        cache.insert(
            "jwt-1".into(),
            CachedSession {
                session: make_session(),
                validated_at: Instant::now(),
            },
        );
        cache.remove("jwt-1");
        assert!(cache.get("jwt-1").is_none());
    }

    #[test]
    fn app_state_is_clone() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            auth_service: Arc::new(AuthService::new()),
            validation_cache: Arc::new(DashMap::new()),
            db: Arc::new(Mutex::new(vec![])),
            grid: zos_grid::ZeroRuntime::new(dir.path().to_path_buf()).unwrap(),
        };
        let _cloned = state.clone();
    }
}
