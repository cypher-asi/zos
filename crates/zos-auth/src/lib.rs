mod error;
mod session;

pub use error::AuthError;
pub use session::ZeroAuthSession;

use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, error, warn};

const ZOS_API_URL: &str = "https://zosapi.zero.tech";

#[derive(Debug, Deserialize)]
struct ZosErrorBody {
    code: Option<String>,
    message: Option<String>,
}

fn parse_zos_error(status: u16, body: &str) -> AuthError {
    let (code, message) = match serde_json::from_str::<ZosErrorBody>(body) {
        Ok(parsed) => (
            parsed.code.unwrap_or_default(),
            parsed.message.unwrap_or_else(|| body.to_string()),
        ),
        Err(_) => (String::new(), body.to_string()),
    };
    error!(status, %code, %message, "zOS API error");
    AuthError::ZosApi {
        status,
        code,
        message,
    }
}

#[derive(Debug, Deserialize)]
struct ZosLoginResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[allow(dead_code)]
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct ZosProfileSummary {
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZosWallet {
    #[serde(rename = "publicAddress")]
    public_address: String,
}

#[derive(Debug, Deserialize)]
struct ZosUserResponse {
    id: String,
    #[serde(rename = "profileSummary")]
    profile_summary: Option<ZosProfileSummary>,
    #[serde(rename = "primaryZID")]
    primary_zid: Option<String>,
    #[serde(rename = "primaryWalletAddress")]
    primary_wallet_address: Option<String>,
    wallets: Option<Vec<ZosWallet>>,
}

#[derive(Debug, Deserialize)]
struct ZosProfileResponse {
    #[serde(rename = "isZeroProSubscriber", default)]
    is_zero_pro: bool,
}

pub struct AuthSessionResult {
    pub session: ZeroAuthSession,
    pub zero_pro_refresh_error: Option<String>,
}

pub struct AuthService {
    http: Client,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("failed to build auth http client"),
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthSessionResult, AuthError> {
        debug!("Logging in via zOS-api");
        let res = self
            .http
            .post(format!("{ZOS_API_URL}/api/v2/accounts/login"))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(parse_zos_error(status, &body));
        }

        let login_data: ZosLoginResponse = res.json().await.map_err(AuthError::Http)?;
        self.build_session_from_token(&login_data.access_token)
            .await
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        name: &str,
        invite_code: &str,
    ) -> Result<AuthSessionResult, AuthError> {
        debug!("Registering via zOS-api");

        let res = self
            .http
            .post(format!("{ZOS_API_URL}/api/v2/accounts/createAndAuthorize"))
            .json(&serde_json::json!({
                "user": {
                    "email": email.to_lowercase(),
                    "password": password,
                    "handle": email.to_lowercase(),
                },
                "inviteSlug": invite_code,
            }))
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(parse_zos_error(status, &body));
        }

        let login_data: ZosLoginResponse = res.json().await.map_err(AuthError::Http)?;
        let token = &login_data.access_token;

        let user = self.fetch_user_info(token).await?;

        let finalize_res = self
            .http
            .post(format!("{ZOS_API_URL}/api/v2/accounts/finalize"))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "userId": user.id,
                "name": name,
                "inviteCode": invite_code,
            }))
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !finalize_res.status().is_success() {
            let status = finalize_res.status().as_u16();
            let body = finalize_res.text().await.unwrap_or_default();
            warn!(status, body = %body, "Failed to finalize account, continuing with session");
        }

        self.build_session_from_token(token).await
    }

    pub async fn request_password_reset(&self, email: &str) -> Result<(), AuthError> {
        debug!("Requesting password reset via zOS-api");
        let res = self
            .http
            .post(format!(
                "{ZOS_API_URL}/api/v2/accounts/request-password-reset"
            ))
            .json(&serde_json::json!({ "email": email.to_lowercase() }))
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(parse_zos_error(status, &body));
        }

        Ok(())
    }

    pub async fn logout(&self, token: Option<&str>) -> Result<(), AuthError> {
        if let Some(jwt) = token {
            debug!("Logging out via zOS-api");
            let _ = self
                .http
                .delete(format!("{ZOS_API_URL}/authentication/session"))
                .bearer_auth(jwt)
                .send()
                .await;
        }
        Ok(())
    }

    /// Validate a JWT token against zOS.
    pub async fn validate_token(&self, token: &str) -> Result<AuthSessionResult, AuthError> {
        debug!("Validating token against zOS-api");
        self.build_session_from_token(token).await
    }

    async fn fetch_user_info(&self, token: &str) -> Result<ZosUserResponse, AuthError> {
        let res = self
            .http
            .get(format!("{ZOS_API_URL}/api/users/current"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(parse_zos_error(status, &body));
        }

        res.json().await.map_err(AuthError::Http)
    }

    async fn fetch_is_zero_pro(&self, token: &str) -> Result<bool, AuthError> {
        let res = self
            .http
            .get(format!("{ZOS_API_URL}/api/v2/users/me"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AuthError::Http)?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(parse_zos_error(status, &body));
        }

        res.json::<ZosProfileResponse>()
            .await
            .map(|p| p.is_zero_pro)
            .map_err(AuthError::Http)
    }

    async fn build_session_from_token(
        &self,
        access_token: &str,
    ) -> Result<AuthSessionResult, AuthError> {
        let user = self.fetch_user_info(access_token).await?;
        let now = Utc::now();
        let mut session = ZeroAuthSession {
            user_id: user.id,
            display_name: build_display_name(&user.profile_summary, &user.primary_zid),
            profile_image: String::new(),
            primary_zid: user.primary_zid.unwrap_or_default(),
            zero_wallet: user.primary_wallet_address.unwrap_or_default(),
            wallets: user
                .wallets
                .unwrap_or_default()
                .into_iter()
                .map(|w| w.public_address)
                .collect(),
            access_token: access_token.to_string(),
            is_zero_pro: false,
            created_at: now,
            validated_at: now,
        };
        let mut zero_pro_refresh_error = None;

        match self.fetch_is_zero_pro(access_token).await {
            Ok(is_zero_pro) => {
                session.is_zero_pro = is_zero_pro;
            }
            Err(err) => {
                zero_pro_refresh_error =
                    Some("Unable to verify ZERO Pro status right now.".to_string());
                warn!(
                    error = %err,
                    user_id = %session.user_id,
                    "authenticated session but could not verify ZERO Pro entitlement"
                );
            }
        }

        Ok(AuthSessionResult {
            session,
            zero_pro_refresh_error,
        })
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

fn build_display_name(profile: &Option<ZosProfileSummary>, primary_zid: &Option<String>) -> String {
    if let Some(p) = profile {
        let first = p.first_name.as_deref().unwrap_or("");
        let last = p.last_name.as_deref().unwrap_or("");
        let full = format!("{first} {last}").trim().to_string();
        if !full.is_empty() {
            return full;
        }
    }
    if let Some(zid) = primary_zid {
        if !zid.is_empty() {
            return zid.clone();
        }
    }
    "User".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_from_full_profile() {
        let profile = Some(ZosProfileSummary {
            first_name: Some("Alice".into()),
            last_name: Some("Smith".into()),
        });
        assert_eq!(build_display_name(&profile, &None), "Alice Smith");
    }

    #[test]
    fn display_name_first_name_only() {
        let profile = Some(ZosProfileSummary {
            first_name: Some("Alice".into()),
            last_name: None,
        });
        assert_eq!(build_display_name(&profile, &None), "Alice");
    }

    #[test]
    fn display_name_falls_back_to_zid() {
        let profile = Some(ZosProfileSummary {
            first_name: None,
            last_name: None,
        });
        let zid = Some("0://alice".into());
        assert_eq!(build_display_name(&profile, &zid), "0://alice");
    }

    #[test]
    fn display_name_falls_back_to_user() {
        assert_eq!(build_display_name(&None, &None), "User");
    }

    #[test]
    fn display_name_empty_zid_falls_back_to_user() {
        assert_eq!(build_display_name(&None, &Some(String::new())), "User");
    }

    #[test]
    fn parse_zos_error_valid_json() {
        let body = r#"{"code":"INVALID_EMAIL_PASSWORD","message":"Bad creds"}"#;
        let err = parse_zos_error(401, body);
        match err {
            AuthError::ZosApi {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 401);
                assert_eq!(code, "INVALID_EMAIL_PASSWORD");
                assert_eq!(message, "Bad creds");
            }
            _ => panic!("expected ZosApi variant"),
        }
    }

    #[test]
    fn parse_zos_error_invalid_json_uses_raw_body() {
        let body = "plain text error";
        let err = parse_zos_error(500, body);
        match err {
            AuthError::ZosApi {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 500);
                assert_eq!(code, "");
                assert_eq!(message, "plain text error");
            }
            _ => panic!("expected ZosApi variant"),
        }
    }

    #[test]
    fn parse_zos_error_partial_json() {
        let body = r#"{"code":"SOME_CODE"}"#;
        let err = parse_zos_error(400, body);
        match err {
            AuthError::ZosApi {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, "SOME_CODE");
                assert_eq!(message, body);
            }
            _ => panic!("expected ZosApi variant"),
        }
    }

    #[test]
    fn auth_service_default_creates_instance() {
        let _service = AuthService::default();
    }

    #[test]
    fn zos_login_response_deserialize() {
        let json = r#"{"accessToken":"abc","identityToken":"def"}"#;
        let resp: ZosLoginResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "abc");
    }

    #[test]
    fn zos_user_response_minimal() {
        let json = r#"{"id":"u1"}"#;
        let resp: ZosUserResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "u1");
        assert!(resp.profile_summary.is_none());
        assert!(resp.primary_zid.is_none());
        assert!(resp.wallets.is_none());
    }

    #[test]
    fn zos_profile_response_defaults_false() {
        let json = r#"{}"#;
        let resp: ZosProfileResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.is_zero_pro);
    }
}
