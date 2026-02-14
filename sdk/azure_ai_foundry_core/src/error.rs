use thiserror::Error;

/// Errors that can occur when interacting with the Azure AI Foundry API.
#[derive(Error, Debug)]
pub enum FoundryError {
    /// The request failed due to an HTTP error.
    #[error("HTTP error: {status} - {message}")]
    Http { status: u16, message: String },

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// The request payload could not be serialized.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The HTTP request failed at the transport level.
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// The endpoint URL is invalid.
    #[error("Invalid endpoint URL: {0}")]
    InvalidEndpoint(String),

    /// A required configuration value is missing.
    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    /// The API returned an error response.
    #[error("API error ({code}): {message}")]
    Api { code: String, message: String },

    /// The streaming response could not be parsed.
    #[error("Stream error: {0}")]
    Stream(String),

    /// An error from the Azure SDK.
    #[error("Azure SDK error: {0}")]
    AzureSdk(String),

    /// A required builder field is missing.
    #[error("Builder error: {0}")]
    Builder(String),
}

impl From<azure_core::Error> for FoundryError {
    fn from(err: azure_core::Error) -> Self {
        Self::AzureSdk(err.to_string())
    }
}

/// Result type alias for Foundry operations.
pub type FoundryResult<T> = std::result::Result<T, FoundryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_error_display() {
        let err = FoundryError::Http {
            status: 404,
            message: "Not found".into(),
        };
        assert_eq!(err.to_string(), "HTTP error: 404 - Not found");
    }

    #[test]
    fn auth_error_display() {
        let err = FoundryError::Auth("Invalid credentials".into());
        assert_eq!(
            err.to_string(),
            "Authentication failed: Invalid credentials"
        );
    }

    #[test]
    fn invalid_endpoint_error_display() {
        let err = FoundryError::InvalidEndpoint("bad url".into());
        assert_eq!(err.to_string(), "Invalid endpoint URL: bad url");
    }

    #[test]
    fn missing_config_error_display() {
        let err = FoundryError::MissingConfig("endpoint required".into());
        assert_eq!(err.to_string(), "Missing configuration: endpoint required");
    }

    #[test]
    fn api_error_display() {
        let err = FoundryError::Api {
            code: "InvalidRequest".into(),
            message: "Bad request body".into(),
        };
        assert_eq!(
            err.to_string(),
            "API error (InvalidRequest): Bad request body"
        );
    }

    #[test]
    fn stream_error_display() {
        let err = FoundryError::Stream("connection lost".into());
        assert_eq!(err.to_string(), "Stream error: connection lost");
    }

    #[test]
    fn azure_sdk_error_display() {
        let err = FoundryError::AzureSdk("credential error".into());
        assert_eq!(err.to_string(), "Azure SDK error: credential error");
    }

    #[test]
    fn from_azure_core_error() {
        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "token expired",
        );
        let foundry_err: FoundryError = azure_err.into();
        assert!(matches!(foundry_err, FoundryError::AzureSdk(_)));
        assert!(foundry_err.to_string().contains("token expired"));
    }

    #[test]
    fn from_serde_json_error() {
        let json_err =
            serde_json::from_str::<serde_json::Value>("invalid json").expect_err("should fail");
        let foundry_err: FoundryError = json_err.into();
        assert!(matches!(foundry_err, FoundryError::Serialization(_)));
    }

    #[test]
    fn builder_error_display() {
        let err = FoundryError::Builder("model is required".into());
        assert_eq!(err.to_string(), "Builder error: model is required");
    }
}
