use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ApiError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ApiError {
    pub(crate) fn unauthorized(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: msg.into(),
                code: Some("unauthorized".into()),
            }),
        )
    }

    pub(crate) fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: msg.into(),
                code: Some("bad_request".into()),
            }),
        )
    }

    pub(crate) fn internal(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: msg.into(),
                code: Some("internal_error".into()),
            }),
        )
    }

    pub(crate) fn service_unavailable(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                error: msg.into(),
                code: Some("service_unavailable".into()),
            }),
        )
    }
}

pub(crate) type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_status_code() {
        let (status, body) = ApiError::unauthorized("nope");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "nope");
        assert_eq!(body.code.as_deref(), Some("unauthorized"));
    }

    #[test]
    fn bad_request_status_code() {
        let (status, body) = ApiError::bad_request("invalid");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.error, "invalid");
    }

    #[test]
    fn internal_status_code() {
        let (status, _) = ApiError::internal("boom");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn service_unavailable_status_code() {
        let (status, body) = ApiError::service_unavailable("down");
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.code.as_deref(), Some("service_unavailable"));
    }

    #[test]
    fn api_error_serializes_to_json() {
        let err = ApiError {
            error: "test error".into(),
            code: Some("test_code".into()),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "test error");
        assert_eq!(json["code"], "test_code");
    }

    #[test]
    fn api_error_omits_none_code() {
        let err = ApiError {
            error: "test".into(),
            code: None,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert!(json.get("code").is_none());
    }
}
