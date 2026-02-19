use thiserror::Error;

/// Errors that can occur when interacting with the Azure AI Foundry API.
#[derive(Error, Debug)]
pub enum FoundryError {
    /// The request failed due to an HTTP error.
    #[error("HTTP error: {status} - {message}")]
    Http {
        status: u16,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication failed.
    #[error("Authentication failed: {message}")]
    Auth {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The request payload could not be serialized.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The HTTP request failed at the transport level.
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// The endpoint URL is invalid.
    #[error("Invalid endpoint URL: {message}")]
    InvalidEndpoint {
        message: String,
        #[source]
        source: Option<url::ParseError>,
    },

    /// A required configuration value is missing.
    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    /// The API returned an error response.
    #[error("API error ({code}): {message}")]
    Api { code: String, message: String },

    /// The streaming response could not be parsed.
    #[error("Stream error: {message}")]
    Stream {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error from the Azure SDK.
    #[error("Azure SDK error: {message}")]
    AzureSdk {
        message: String,
        #[source]
        source: azure_core::Error,
    },

    /// A required builder field is missing.
    #[error("Builder error: {0}")]
    Builder(String),
}

impl From<azure_core::Error> for FoundryError {
    fn from(err: azure_core::Error) -> Self {
        Self::AzureSdk {
            message: err.to_string(),
            source: err,
        }
    }
}

/// Result type alias for Foundry operations.
pub type FoundryResult<T> = std::result::Result<T, FoundryError>;

impl FoundryError {
    /// Creates an authentication error without a source error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an authentication error with a source error for error chain preservation.
    pub fn auth_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Auth {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates an HTTP error without a source error.
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        Self::Http {
            status,
            message: message.into(),
            source: None,
        }
    }

    /// Creates an HTTP error with a source error for error chain preservation.
    pub fn http_with_source<E>(status: u16, message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Http {
            status,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates an invalid endpoint error without a source error.
    pub fn invalid_endpoint(message: impl Into<String>) -> Self {
        Self::InvalidEndpoint {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an invalid endpoint error with the underlying parse error.
    pub fn invalid_endpoint_with_source(
        message: impl Into<String>,
        source: url::ParseError,
    ) -> Self {
        Self::InvalidEndpoint {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Creates a stream error without a source error.
    pub fn stream(message: impl Into<String>) -> Self {
        Self::Stream {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a stream error with a source error for error chain preservation.
    pub fn stream_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Stream {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_error_display() {
        let err = FoundryError::http(404, "Not found");
        assert_eq!(err.to_string(), "HTTP error: 404 - Not found");
    }

    #[test]
    fn auth_error_display() {
        let err = FoundryError::auth("Invalid credentials");
        assert_eq!(
            err.to_string(),
            "Authentication failed: Invalid credentials"
        );
    }

    #[test]
    fn invalid_endpoint_error_display() {
        let err = FoundryError::invalid_endpoint("bad url");
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
        let err = FoundryError::stream("connection lost");
        assert_eq!(err.to_string(), "Stream error: connection lost");
    }

    #[test]
    fn azure_sdk_error_display() {
        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "credential error",
        );
        let err: FoundryError = azure_err.into();
        assert_eq!(err.to_string(), "Azure SDK error: credential error");
    }

    #[test]
    fn from_azure_core_error() {
        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "token expired",
        );
        let foundry_err: FoundryError = azure_err.into();
        assert!(matches!(foundry_err, FoundryError::AzureSdk { .. }));
        assert!(foundry_err.to_string().contains("token expired"));
    }

    #[test]
    fn azure_sdk_error_preserves_source() {
        use std::error::Error;

        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "token expired",
        );
        let foundry_err: FoundryError = azure_err.into();

        // source() must NOT be None - this is the critical assertion
        assert!(
            foundry_err.source().is_some(),
            "AzureSdk must preserve source chain"
        );
        assert!(foundry_err
            .source()
            .unwrap()
            .to_string()
            .contains("token expired"));
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

    #[test]
    fn auth_error_preserves_source() {
        use std::error::Error;

        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "token expired",
        );

        let foundry_err = FoundryError::auth_with_source("token expired", azure_err);

        // Verify source chain is preserved
        let source = foundry_err.source().expect("should have source");
        assert!(source.to_string().contains("token expired"));
    }

    #[test]
    fn http_error_preserves_source() {
        use std::error::Error;
        use std::io;

        // Use an io::Error as a stand-in for any source error
        let io_err = io::Error::new(io::ErrorKind::TimedOut, "connection timed out");

        let foundry_err = FoundryError::http_with_source(503, "Service Unavailable", io_err);

        // Verify display message
        assert_eq!(
            foundry_err.to_string(),
            "HTTP error: 503 - Service Unavailable"
        );

        // Verify source chain is preserved
        let source = foundry_err.source().expect("should have source");
        assert!(source.to_string().contains("connection timed out"));
    }

    #[test]
    fn error_chain_preserves_all_sources() {
        use std::error::Error;

        // Create a chain: azure_core::Error → FoundryError::Auth
        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "token expired from Azure",
        );

        let foundry_err = FoundryError::auth_with_source("authentication failed", azure_err);

        // Verify we can walk the entire error chain
        let level1 = foundry_err.source().expect("should have level 1 source");
        assert!(level1.to_string().contains("token expired from Azure"));

        // azure_core::Error may or may not have a source depending on how it was created
        // The important thing is that our error preserves its immediate source
    }

    #[test]
    fn error_without_source_returns_none() {
        use std::error::Error;

        let err = FoundryError::auth("simple error message");
        assert!(err.source().is_none());

        let http_err = FoundryError::http(404, "Not Found");
        assert!(http_err.source().is_none());
    }

    #[test]
    fn invalid_endpoint_preserves_parse_error() {
        use std::error::Error;
        use url::Url;

        // Try to parse an invalid URL
        let parse_result = Url::parse("not a valid url");
        assert!(parse_result.is_err());
        let parse_err = parse_result.unwrap_err();

        let foundry_err = FoundryError::invalid_endpoint_with_source("bad URL format", parse_err);

        // Verify display message
        assert_eq!(
            foundry_err.to_string(),
            "Invalid endpoint URL: bad URL format"
        );

        // Verify source chain is preserved
        let source = foundry_err.source().expect("should have source");
        assert!(source.to_string().contains("relative URL without a base"));
    }

    #[test]
    fn stream_error_preserves_source() {
        use std::error::Error;

        // Create a serde_json deserialization error
        let json_result: Result<serde_json::Value, _> = serde_json::from_str("{ invalid json }");
        assert!(json_result.is_err());
        let json_err = json_result.unwrap_err();

        let foundry_err = FoundryError::stream_with_source("failed to parse SSE event", json_err);

        // Verify display message
        assert_eq!(
            foundry_err.to_string(),
            "Stream error: failed to parse SSE event"
        );

        // Verify source chain is preserved
        let source = foundry_err.source().expect("should have source");
        assert!(source.to_string().contains("key must be a string"));
    }

    /// Backward compatibility test: verify error message formats are unchanged
    /// when using constructors without source (the common case for existing code).
    #[test]
    fn error_display_backward_compatible() {
        // Auth error format unchanged
        let auth = FoundryError::auth("Invalid credentials");
        assert_eq!(
            auth.to_string(),
            "Authentication failed: Invalid credentials"
        );

        // HTTP error format unchanged
        let http = FoundryError::http(404, "Not found");
        assert_eq!(http.to_string(), "HTTP error: 404 - Not found");

        // InvalidEndpoint format unchanged
        let endpoint = FoundryError::invalid_endpoint("bad url");
        assert_eq!(endpoint.to_string(), "Invalid endpoint URL: bad url");

        // Stream error format unchanged
        let stream = FoundryError::stream("connection lost");
        assert_eq!(stream.to_string(), "Stream error: connection lost");

        // API error format unchanged (this variant wasn't modified)
        let api = FoundryError::Api {
            code: "InvalidRequest".into(),
            message: "Bad request body".into(),
        };
        assert_eq!(
            api.to_string(),
            "API error (InvalidRequest): Bad request body"
        );

        // MissingConfig format unchanged (not modified)
        let config = FoundryError::MissingConfig("endpoint required".into());
        assert_eq!(
            config.to_string(),
            "Missing configuration: endpoint required"
        );

        // Builder format unchanged (not modified)
        let builder = FoundryError::Builder("model is required".into());
        assert_eq!(builder.to_string(), "Builder error: model is required");

        // AzureSdk format unchanged
        let azure_err = azure_core::Error::with_message(
            azure_core::error::ErrorKind::Credential,
            "credential error",
        );
        let sdk: FoundryError = azure_err.into();
        assert_eq!(sdk.to_string(), "Azure SDK error: credential error");
    }
}
