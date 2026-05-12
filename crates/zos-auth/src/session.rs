use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZeroAuthSession {
    pub user_id: String,
    pub display_name: String,
    pub profile_image: String,
    pub primary_zid: String,
    pub zero_wallet: String,
    pub wallets: Vec<String>,
    pub access_token: String,
    #[serde(default)]
    pub is_zero_pro: bool,
    pub created_at: DateTime<Utc>,
    pub validated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_session() -> ZeroAuthSession {
        let now = Utc::now();
        ZeroAuthSession {
            user_id: "user-123".into(),
            display_name: "Test User".into(),
            profile_image: String::new(),
            primary_zid: "0://tester".into(),
            zero_wallet: "0xabc".into(),
            wallets: vec!["0xabc".into()],
            access_token: "jwt-token".into(),
            is_zero_pro: true,
            created_at: now,
            validated_at: now,
        }
    }

    #[test]
    fn session_roundtrip_json() {
        let session = sample_session();
        let json = serde_json::to_string(&session).unwrap();
        let parsed: ZeroAuthSession = serde_json::from_str(&json).unwrap();
        assert_eq!(session, parsed);
    }

    #[test]
    fn session_deserialize_missing_is_zero_pro_defaults_false() {
        let json = r#"{
            "user_id": "u1",
            "display_name": "User",
            "profile_image": "",
            "primary_zid": "",
            "zero_wallet": "",
            "wallets": [],
            "access_token": "tok",
            "created_at": "2025-01-01T00:00:00Z",
            "validated_at": "2025-01-01T00:00:00Z"
        }"#;
        let session: ZeroAuthSession = serde_json::from_str(json).unwrap();
        assert!(!session.is_zero_pro);
    }

    #[test]
    fn session_clone_is_equal() {
        let session = sample_session();
        let cloned = session.clone();
        assert_eq!(session, cloned);
    }
}
