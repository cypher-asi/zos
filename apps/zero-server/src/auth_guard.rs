use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::Json;
use tracing::warn;

use zos_auth::{AuthError, ZeroAuthSession};

use crate::error::ApiError;
use crate::state::{AppState, AuthJwt, AuthSession, CachedSession};

const AUTH_REFRESH_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

fn map_auth_error(e: AuthError) -> (StatusCode, Json<ApiError>) {
    match e {
        AuthError::ZosApi {
            status: 401,
            message,
            ..
        } => ApiError::unauthorized(if message.is_empty() {
            "session expired or invalid".to_string()
        } else {
            message
        }),
        AuthError::Http(err) => {
            ApiError::service_unavailable(format!("unable to reach zOS API: {err}"))
        }
        other => ApiError::internal(other.to_string()),
    }
}

/// Extract a JWT from the request: checks the `Authorization: Bearer` header
/// first, then falls back to the `?token=` query parameter (for WebSocket
/// connections where browsers cannot send custom headers).
fn extract_request_token(req: &Request) -> Option<String> {
    if let Some(token) = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|val| val.strip_prefix("Bearer "))
    {
        return Some(token.to_string());
    }

    req.uri()
        .query()
        .and_then(|q| q.split('&').find_map(|pair| pair.strip_prefix("token=")))
        .map(|t| t.to_string())
}

fn get_cached_session(state: &AppState, jwt: &str) -> Option<ZeroAuthSession> {
    let entry = state.validation_cache.get(jwt)?;
    if entry.validated_at.elapsed() < AUTH_REFRESH_TTL {
        Some(entry.session.clone())
    } else {
        None
    }
}

async fn validate_and_cache(
    state: &AppState,
    jwt: &str,
) -> Result<ZeroAuthSession, (StatusCode, Json<ApiError>)> {
    let result = state
        .auth_service
        .validate_token(jwt)
        .await
        .map_err(map_auth_error)?;

    state.validation_cache.insert(
        jwt.to_string(),
        CachedSession {
            session: result.session.clone(),
            validated_at: Instant::now(),
        },
    );

    Ok(result.session)
}

/// Resolve a session from a JWT: check cache first, then validate with zOS.
/// On zOS network failure, falls back to a stale cached entry if available.
async fn resolve_session_from_jwt(
    state: &AppState,
    jwt: &str,
) -> Result<ZeroAuthSession, (StatusCode, Json<ApiError>)> {
    if let Some(session) = get_cached_session(state, jwt) {
        return Ok(session);
    }

    match validate_and_cache(state, jwt).await {
        Ok(session) => Ok(session),
        Err(err) if err.0 == StatusCode::UNAUTHORIZED => Err(err),
        Err(err) => {
            if let Some(entry) = state.validation_cache.get(jwt) {
                warn!(
                    user_id = %entry.session.user_id,
                    "zOS unreachable, using stale cached session"
                );
                Ok(entry.session.clone())
            } else {
                Err(err)
            }
        }
    }
}

pub(crate) async fn require_verified_session(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiError>)> {
    let token = extract_request_token(&req)
        .ok_or_else(|| ApiError::unauthorized("missing authorization token"))?;
    let session = resolve_session_from_jwt(&state, &token).await?;

    req.extensions_mut().insert(AuthJwt(token));
    req.extensions_mut().insert(AuthSession(session));

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;
    use zos_auth::ZeroAuthSession;

    fn make_session() -> ZeroAuthSession {
        let now = Utc::now();
        ZeroAuthSession {
            user_id: "u1".into(),
            display_name: "Test User".into(),
            profile_image: String::new(),
            primary_zid: "0://tester".into(),
            zero_wallet: "0xabc".into(),
            wallets: vec![],
            access_token: "test-jwt-token".into(),
            is_zero_pro: false,
            created_at: now,
            validated_at: now,
        }
    }

    // --- Token extraction tests ---

    fn request_with_auth_header(value: &str) -> Request {
        axum::http::Request::builder()
            .header("Authorization", value)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    fn request_with_query(query: &str) -> Request {
        axum::http::Request::builder()
            .uri(format!("/api/test?{query}"))
            .body(axum::body::Body::empty())
            .unwrap()
    }

    fn request_bare() -> Request {
        axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[test]
    fn extract_token_from_bearer_header() {
        let req = request_with_auth_header("Bearer my-jwt-token");
        assert_eq!(extract_request_token(&req).unwrap(), "my-jwt-token");
    }

    #[test]
    fn extract_token_missing_bearer_prefix() {
        let req = request_with_auth_header("Basic abc123");
        assert!(extract_request_token(&req).is_none());
    }

    #[test]
    fn extract_token_from_query_param() {
        let req = request_with_query("token=ws-jwt-token");
        assert_eq!(extract_request_token(&req).unwrap(), "ws-jwt-token");
    }

    #[test]
    fn extract_token_from_query_with_other_params() {
        let req = request_with_query("foo=bar&token=my-token&baz=1");
        assert_eq!(extract_request_token(&req).unwrap(), "my-token");
    }

    #[test]
    fn extract_token_prefers_header_over_query() {
        let req = axum::http::Request::builder()
            .uri("/api/test?token=query-token")
            .header("Authorization", "Bearer header-token")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_request_token(&req).unwrap(), "header-token");
    }

    #[test]
    fn extract_token_returns_none_when_absent() {
        let req = request_bare();
        assert!(extract_request_token(&req).is_none());
    }

    #[test]
    fn extract_token_empty_bearer_value() {
        let req = request_with_auth_header("Bearer ");
        assert_eq!(extract_request_token(&req).unwrap(), "");
    }

    // --- Validation cache tests ---

    fn make_cache() -> crate::state::ValidationCache {
        Arc::new(dashmap::DashMap::new())
    }

    fn make_state_with_cache(cache: crate::state::ValidationCache) -> AppState {
        AppState {
            auth_service: Arc::new(zos_auth::AuthService::new()),
            validation_cache: cache,
            db: Arc::new(tokio::sync::Mutex::new(vec![])),
        }
    }

    fn insert_cached(
        cache: &crate::state::ValidationCache,
        jwt: &str,
        age: std::time::Duration,
    ) {
        cache.insert(
            jwt.to_string(),
            CachedSession {
                session: make_session(),
                validated_at: Instant::now() - age,
            },
        );
    }

    #[test]
    fn get_cached_session_returns_fresh_entry() {
        let cache = make_cache();
        insert_cached(&cache, "jwt-1", std::time::Duration::from_secs(60));

        let state = make_state_with_cache(cache);
        let result = get_cached_session(&state, "jwt-1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().user_id, "u1");
    }

    #[test]
    fn get_cached_session_returns_none_for_stale_entry() {
        let cache = make_cache();
        insert_cached(&cache, "jwt-1", std::time::Duration::from_secs(6 * 60));

        let state = make_state_with_cache(cache);
        assert!(get_cached_session(&state, "jwt-1").is_none());
    }

    #[test]
    fn get_cached_session_returns_none_for_missing_entry() {
        let cache = make_cache();
        let state = make_state_with_cache(cache);
        assert!(get_cached_session(&state, "nonexistent").is_none());
    }

    // --- Auth error mapping tests ---

    #[test]
    fn map_auth_error_401_zos_gives_unauthorized() {
        let err = AuthError::ZosApi {
            status: 401,
            code: "INVALID_EMAIL_PASSWORD".into(),
            message: "Bad creds".into(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "Bad creds");
    }

    #[test]
    fn map_auth_error_401_empty_message_gives_default() {
        let err = AuthError::ZosApi {
            status: 401,
            code: String::new(),
            message: String::new(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "session expired or invalid");
    }

    #[test]
    fn map_auth_error_non_401_zos_gives_internal() {
        let err = AuthError::ZosApi {
            status: 500,
            code: String::new(),
            message: "server error".into(),
        };
        let (status, _) = map_auth_error(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
