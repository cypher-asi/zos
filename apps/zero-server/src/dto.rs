use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zos_auth::{AuthSessionResult, ZeroAuthSession};

// ---------------------------------------------------------------------------
// Auth DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct AuthLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthRegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub invite_code: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AuthSessionResponse {
    pub user_id: String,
    pub display_name: String,
    pub profile_image: String,
    pub primary_zid: String,
    pub zero_wallet: String,
    pub wallets: Vec<String>,
    pub is_zero_pro: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zero_pro_refresh_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub validated_at: DateTime<Utc>,
}

impl From<ZeroAuthSession> for AuthSessionResponse {
    fn from(s: ZeroAuthSession) -> Self {
        let token = s.access_token.clone();
        Self {
            user_id: s.user_id,
            display_name: s.display_name,
            profile_image: s.profile_image,
            primary_zid: s.primary_zid,
            zero_wallet: s.zero_wallet,
            wallets: s.wallets,
            is_zero_pro: s.is_zero_pro,
            zero_pro_refresh_error: None,
            access_token: Some(token),
            created_at: s.created_at,
            validated_at: s.validated_at,
        }
    }
}

impl AuthSessionResponse {
    pub(crate) fn from_auth_result(result: AuthSessionResult) -> Self {
        let mut response = Self::from(result.session);
        response.zero_pro_refresh_error = result.zero_pro_refresh_error;
        response
    }
}

// ---------------------------------------------------------------------------
// Project DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_session() -> ZeroAuthSession {
        let now = Utc::now();
        ZeroAuthSession {
            user_id: "u1".into(),
            display_name: "Test".into(),
            profile_image: String::new(),
            primary_zid: "0://test".into(),
            zero_wallet: "0xabc".into(),
            wallets: vec!["0xabc".into()],
            access_token: "jwt".into(),
            is_zero_pro: true,
            created_at: now,
            validated_at: now,
        }
    }

    #[test]
    fn auth_session_response_from_session() {
        let session = sample_session();
        let resp = AuthSessionResponse::from(session.clone());
        assert_eq!(resp.user_id, session.user_id);
        assert_eq!(resp.display_name, session.display_name);
        assert_eq!(resp.access_token, Some("jwt".into()));
        assert!(resp.zero_pro_refresh_error.is_none());
        assert!(resp.is_zero_pro);
    }

    #[test]
    fn auth_session_response_from_result_with_error() {
        let result = AuthSessionResult {
            session: sample_session(),
            zero_pro_refresh_error: Some("could not verify".into()),
        };
        let resp = AuthSessionResponse::from_auth_result(result);
        assert_eq!(
            resp.zero_pro_refresh_error.as_deref(),
            Some("could not verify")
        );
    }

    #[test]
    fn auth_session_response_serializes_without_none_fields() {
        let session = sample_session();
        let resp = AuthSessionResponse::from(session);
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("zero_pro_refresh_error").is_none());
        assert!(json.get("access_token").is_some());
    }

    #[test]
    fn auth_login_request_deserialize() {
        let json = r#"{"email":"a@b.com","password":"pass"}"#;
        let req: AuthLoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "a@b.com");
        assert_eq!(req.password, "pass");
    }

    #[test]
    fn auth_register_request_deserialize() {
        let json = r#"{"email":"a@b.com","password":"pass","name":"Alice","invite_code":"INV"}"#;
        let req: AuthRegisterRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Alice");
        assert_eq!(req.invite_code, "INV");
    }

    #[test]
    fn project_roundtrip() {
        let p = Project {
            id: "1".into(),
            name: "proj".into(),
            description: "desc".into(),
            updated_at: "2025-01-01".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(p.id, parsed.id);
    }

    #[test]
    fn create_project_default_description() {
        let json = r#"{"name":"test"}"#;
        let req: CreateProject = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, "");
    }
}
