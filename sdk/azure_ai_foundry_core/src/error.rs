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
}

/// Result type alias for Foundry operations.
pub type FoundryResult<T> = std::result::Result<T, FoundryError>;
