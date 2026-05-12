#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("zOS API error {status}: {message}")]
    ZosApi {
        status: u16,
        code: String,
        message: String,
    },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zos_api_error_display() {
        let err = AuthError::ZosApi {
            status: 401,
            code: "INVALID_EMAIL_PASSWORD".into(),
            message: "Invalid credentials".into(),
        };
        assert_eq!(err.to_string(), "zOS API error 401: Invalid credentials");
    }

    #[test]
    fn serialization_error_display() {
        let inner = serde_json::from_str::<String>("not json").unwrap_err();
        let err = AuthError::Serialization(inner);
        assert!(err.to_string().starts_with("serialization error:"));
    }
}
