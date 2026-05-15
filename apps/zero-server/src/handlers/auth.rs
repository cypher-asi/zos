use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use zos_auth::AuthError;

use crate::dto::{
    AuthLoginRequest, AuthRegisterRequest, AuthSessionResponse, PasswordResetRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, AuthJwt, AuthSession, CachedSession};

fn map_auth_error(e: AuthError) -> (StatusCode, Json<ApiError>) {
    match &e {
        AuthError::ZosApi {
            status,
            code,
            message,
        } if *status == 401 || code == "INVALID_EMAIL_PASSWORD" => {
            ApiError::unauthorized(if message.is_empty() {
                "Invalid email or password".to_string()
            } else {
                message.clone()
            })
        }
        AuthError::ZosApi { message, .. } => ApiError::bad_request(if message.is_empty() {
            "Authentication request failed".to_string()
        } else {
            message.clone()
        }),
        _ => ApiError::internal(format!("authentication failed: {e}")),
    }
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(req): Json<AuthLoginRequest>,
) -> ApiResult<Json<AuthSessionResponse>> {
    let result = state
        .auth_service
        .login(&req.email, &req.password)
        .await
        .map_err(map_auth_error)?;

    state.validation_cache.insert(
        result.session.access_token.clone(),
        CachedSession {
            session: result.session.clone(),
            validated_at: Instant::now(),
        },
    );

    Ok(Json(AuthSessionResponse::from_auth_result(result)))
}

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(req): Json<AuthRegisterRequest>,
) -> ApiResult<Json<AuthSessionResponse>> {
    let result = state
        .auth_service
        .register(&req.email, &req.password, &req.name, &req.invite_code)
        .await
        .map_err(map_auth_error)?;

    state.validation_cache.insert(
        result.session.access_token.clone(),
        CachedSession {
            session: result.session.clone(),
            validated_at: Instant::now(),
        },
    );

    Ok(Json(AuthSessionResponse::from_auth_result(result)))
}

pub(crate) async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> ApiResult<StatusCode> {
    state
        .auth_service
        .request_password_reset(&req.email)
        .await
        .map_err(map_auth_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn logout(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> ApiResult<StatusCode> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if let Some(ref jwt) = token {
        state.validation_cache.remove(jwt);
    }

    state
        .auth_service
        .logout(token.as_deref())
        .await
        .map_err(map_auth_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn get_session(
    AuthSession(session): AuthSession,
) -> ApiResult<Json<AuthSessionResponse>> {
    Ok(Json(AuthSessionResponse::from(session)))
}

pub(crate) async fn validate(
    State(state): State<AppState>,
    AuthJwt(jwt): AuthJwt,
) -> ApiResult<Json<AuthSessionResponse>> {
    let result = state
        .auth_service
        .validate_token(&jwt)
        .await
        .map_err(map_auth_error)?;

    state.validation_cache.insert(
        jwt,
        CachedSession {
            session: result.session.clone(),
            validated_at: Instant::now(),
        },
    );

    Ok(Json(AuthSessionResponse::from_auth_result(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_auth_error_401_invalid_password() {
        let err = AuthError::ZosApi {
            status: 401,
            code: "INVALID_EMAIL_PASSWORD".into(),
            message: "Wrong password".into(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "Wrong password");
    }

    #[test]
    fn map_auth_error_401_empty_message() {
        let err = AuthError::ZosApi {
            status: 401,
            code: String::new(),
            message: String::new(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "Invalid email or password");
    }

    #[test]
    fn map_auth_error_invalid_email_password_code_non_401() {
        let err = AuthError::ZosApi {
            status: 400,
            code: "INVALID_EMAIL_PASSWORD".into(),
            message: "Bad request".into(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "Bad request");
    }

    #[test]
    fn map_auth_error_generic_zos_error() {
        let err = AuthError::ZosApi {
            status: 422,
            code: "VALIDATION".into(),
            message: "email taken".into(),
        };
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.error, "email taken");
    }

    #[test]
    fn map_auth_error_generic_zos_empty_message() {
        let err = AuthError::ZosApi {
            status: 422,
            code: "UNKNOWN".into(),
            message: String::new(),
        };
        let (_, body) = map_auth_error(err);
        assert_eq!(body.error, "Authentication request failed");
    }

    #[test]
    fn map_auth_error_serialization() {
        let inner = serde_json::from_str::<String>("bad").unwrap_err();
        let err = AuthError::Serialization(inner);
        let (status, body) = map_auth_error(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.error.starts_with("authentication failed:"));
    }
}
