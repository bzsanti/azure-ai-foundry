//! HTTP client for Azure AI Foundry.
//!
//! This module provides [`FoundryClient`], the main entry point for interacting
//! with Azure AI Foundry APIs. The client handles authentication, HTTP transport,
//! and endpoint management.
//!
//! # Examples
//!
//! ## Using API Key
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Azure CLI Credential
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::azure_cli()?)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using a Custom TokenCredential
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_identity::ClientSecretCredential;
//! use azure_core::credentials::Secret;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let credential = ClientSecretCredential::new(
//!     "tenant-id",
//!     "client-id".to_string(),
//!     Secret::new("client-secret"),
//!     None,
//! )?;
//!
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::token_credential(credential))
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use crate::auth::FoundryCredential;
use crate::error::{FoundryError, FoundryResult};
use reqwest::Client as HttpClient;
use url::Url;

/// Default API version for Azure AI Foundry.
pub const DEFAULT_API_VERSION: &str = "2025-01-01-preview";

/// The base client for interacting with the Azure AI Foundry API.
///
/// This client handles authentication, HTTP transport, and endpoint management.
/// It is used by higher-level crates (`azure_ai_foundry_models`, `azure_ai_foundry_agents`)
/// to make API calls.
///
/// The client is cheaply cloneable and can be shared across threads.
#[derive(Debug, Clone)]
pub struct FoundryClient {
    pub(crate) http: HttpClient,
    pub(crate) endpoint: Url,
    pub(crate) credential: FoundryCredential,
    pub(crate) api_version: String,
}

/// Builder for constructing a [`FoundryClient`].
///
/// Use [`FoundryClient::builder()`] to create a new builder.
#[derive(Debug, Default)]
pub struct FoundryClientBuilder {
    endpoint: Option<String>,
    credential: Option<FoundryCredential>,
    api_version: Option<String>,
    http_client: Option<HttpClient>,
}

impl FoundryClient {
    /// Create a new builder for configuring a `FoundryClient`.
    pub fn builder() -> FoundryClientBuilder {
        FoundryClientBuilder::default()
    }

    /// Get the base endpoint URL.
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Get the API version being used.
    pub fn api_version(&self) -> &str {
        &self.api_version
    }

    /// Build a full URL for an API path.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to append to the base endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be joined to the endpoint URL.
    pub fn url(&self, path: &str) -> FoundryResult<Url> {
        self.endpoint
            .join(path)
            .map_err(|e| FoundryError::InvalidEndpoint(e.to_string()))
    }

    /// Send a GET request to the API.
    ///
    /// Automatically adds authentication headers and API version.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, the request fails,
    /// or the server returns an error response.
    pub async fn get(&self, path: &str) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;
        let auth = self.credential.resolve().await?;

        let response = self
            .http
            .get(url)
            .header("Authorization", &auth)
            .header("api-version", &self.api_version)
            .send()
            .await?;

        Self::check_response(response).await
    }

    /// Send a POST request with a JSON body to the API.
    ///
    /// Automatically adds authentication headers and API version.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    /// * `body` - The request body to serialize as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, serialization fails,
    /// the request fails, or the server returns an error response.
    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;
        let auth = self.credential.resolve().await?;

        let response = self
            .http
            .post(url)
            .header("Authorization", &auth)
            .header("api-version", &self.api_version)
            .json(body)
            .send()
            .await?;

        Self::check_response(response).await
    }

    /// Send a POST request for streaming responses.
    ///
    /// Unlike [`Self::post`], this method does not consume the response body
    /// for error checking. The caller is responsible for handling the stream.
    /// Only checks the HTTP status code, not the body content.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    /// * `body` - The request body to serialize as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, serialization fails,
    /// the request fails, or the HTTP status code indicates an error.
    pub async fn post_stream<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;
        let auth = self.credential.resolve().await?;

        let response = self
            .http
            .post(url)
            .header("Authorization", &auth)
            .header("api-version", &self.api_version)
            .json(body)
            .send()
            .await?;

        // For streaming, only check status code, don't consume body
        if response.status().is_success() {
            Ok(response)
        } else {
            // For errors, consume body to get error message
            Self::check_response(response).await
        }
    }

    /// Maximum length for error messages to prevent sensitive data leaks.
    const MAX_ERROR_MESSAGE_LEN: usize = 1000;

    /// Truncate a message if it exceeds the maximum length.
    fn truncate_message(msg: &str) -> String {
        if msg.len() > Self::MAX_ERROR_MESSAGE_LEN {
            format!("{}... (truncated)", &msg[..Self::MAX_ERROR_MESSAGE_LEN])
        } else {
            msg.to_string()
        }
    }

    /// Check the response status and return an error if not successful.
    async fn check_response(response: reqwest::Response) -> FoundryResult<reqwest::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();

            // Try to parse as API error
            if let Ok(error) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(err_obj) = error.get("error") {
                    return Err(FoundryError::Api {
                        code: err_obj
                            .get("code")
                            .and_then(|c| c.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        message: Self::truncate_message(
                            err_obj
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or(&body),
                        ),
                    });
                }
            }

            Err(FoundryError::Http {
                status,
                message: Self::truncate_message(&body),
            })
        }
    }
}

impl FoundryClientBuilder {
    /// Set the Azure AI Foundry endpoint URL.
    ///
    /// This should be in the format:
    /// `https://<resource-name>.services.ai.azure.com`
    ///
    /// If not set, the builder will check the `AZURE_AI_FOUNDRY_ENDPOINT`
    /// environment variable.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the credential to use for authentication.
    ///
    /// If not set, the builder will use [`FoundryCredential::from_env()`]
    /// which checks for an API key in `AZURE_AI_FOUNDRY_API_KEY` and
    /// falls back to developer tools credentials.
    pub fn credential(mut self, credential: FoundryCredential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Set the API version.
    ///
    /// Defaults to [`DEFAULT_API_VERSION`] (`2025-01-01-preview`).
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Set a custom HTTP client.
    ///
    /// Use this to configure timeouts, proxies, or other HTTP settings.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the `FoundryClient`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No endpoint is provided and `AZURE_AI_FOUNDRY_ENDPOINT` is not set
    /// - The endpoint URL is invalid
    /// - Credential creation fails (when using environment-based credentials)
    pub fn build(self) -> FoundryResult<FoundryClient> {
        let endpoint_str = self
            .endpoint
            .or_else(|| std::env::var("AZURE_AI_FOUNDRY_ENDPOINT").ok())
            .ok_or_else(|| {
                FoundryError::MissingConfig(
                    "endpoint is required. Set it via builder or AZURE_AI_FOUNDRY_ENDPOINT env var."
                        .into(),
                )
            })?;

        let endpoint =
            Url::parse(&endpoint_str).map_err(|e| FoundryError::InvalidEndpoint(e.to_string()))?;

        let credential = self
            .credential
            .map(Ok)
            .unwrap_or_else(FoundryCredential::from_env)?;

        Ok(FoundryClient {
            http: self.http_client.unwrap_or_default(),
            endpoint,
            credential,
            api_version: self
                .api_version
                .unwrap_or_else(|| DEFAULT_API_VERSION.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    #[serial]
    fn builder_requires_endpoint() {
        // Clear env var to ensure test isolation
        std::env::remove_var("AZURE_AI_FOUNDRY_ENDPOINT");

        let result = FoundryClient::builder()
            .credential(FoundryCredential::api_key("test"))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FoundryError::MissingConfig(_)));
    }

    #[test]
    fn builder_accepts_endpoint() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }

    #[test]
    fn builder_uses_default_api_version() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        assert_eq!(client.api_version(), DEFAULT_API_VERSION);
    }

    #[test]
    fn builder_accepts_custom_api_version() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .api_version("2024-01-01")
            .build()
            .expect("should build");

        assert_eq!(client.api_version(), "2024-01-01");
    }

    #[test]
    #[serial]
    fn builder_uses_endpoint_from_env() {
        // Save original value
        let original = std::env::var("AZURE_AI_FOUNDRY_ENDPOINT").ok();

        std::env::set_var(
            "AZURE_AI_FOUNDRY_ENDPOINT",
            "https://env.services.ai.azure.com",
        );

        let client = FoundryClient::builder()
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        assert_eq!(
            client.endpoint().as_str(),
            "https://env.services.ai.azure.com/"
        );

        // Restore original value
        match original {
            Some(val) => std::env::set_var("AZURE_AI_FOUNDRY_ENDPOINT", val),
            None => std::env::remove_var("AZURE_AI_FOUNDRY_ENDPOINT"),
        }
    }

    #[test]
    #[serial]
    fn builder_endpoint_overrides_env() {
        // Save original value
        let original = std::env::var("AZURE_AI_FOUNDRY_ENDPOINT").ok();

        std::env::set_var(
            "AZURE_AI_FOUNDRY_ENDPOINT",
            "https://env.services.ai.azure.com",
        );

        let client = FoundryClient::builder()
            .endpoint("https://explicit.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        assert_eq!(
            client.endpoint().as_str(),
            "https://explicit.services.ai.azure.com/"
        );

        // Restore original value
        match original {
            Some(val) => std::env::set_var("AZURE_AI_FOUNDRY_ENDPOINT", val),
            None => std::env::remove_var("AZURE_AI_FOUNDRY_ENDPOINT"),
        }
    }

    #[test]
    fn builder_invalid_endpoint_url() {
        let result = FoundryClient::builder()
            .endpoint("not a valid url")
            .credential(FoundryCredential::api_key("test"))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FoundryError::InvalidEndpoint(_)
        ));
    }

    #[test]
    fn url_joins_path() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        let url = client.url("/openai/deployments/gpt-4o/chat/completions");
        assert!(url.is_ok());
        assert_eq!(
            url.unwrap().as_str(),
            "https://test.services.ai.azure.com/openai/deployments/gpt-4o/chat/completions"
        );
    }

    #[test]
    fn url_joins_path_without_leading_slash() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        let url = client.url("openai/v1/models");
        assert!(url.is_ok());
        assert_eq!(
            url.unwrap().as_str(),
            "https://test.services.ai.azure.com/openai/v1/models"
        );
    }

    #[test]
    fn client_is_cloneable() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        let cloned = client.clone();
        assert_eq!(client.endpoint(), cloned.endpoint());
    }

    // --- Wiremock integration tests ---

    async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test-api-key"))
            .api_version("2025-01-01-preview")
            .build()
            .expect("should build client")
    }

    #[tokio::test]
    async fn get_request_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(header("api-version", "2025-01-01-preview"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let response = client.get("/test/endpoint").await.expect("should succeed");

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn get_request_401_unauthorized() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/test/endpoint").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Http { status, message } => {
                assert_eq!(status, 401);
                assert_eq!(message, "Unauthorized");
            }
            _ => panic!("Expected Http error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn get_request_500_with_api_error_format() {
        let server = MockServer::start().await;

        let error_body = serde_json::json!({
            "error": {
                "code": "InternalServerError",
                "message": "Something went wrong on the server"
            }
        });

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(500).set_body_json(error_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/test/endpoint").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Api { code, message } => {
                assert_eq!(code, "InternalServerError");
                assert_eq!(message, "Something went wrong on the server");
            }
            _ => panic!("Expected Api error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn post_request_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(header("api-version", "2025-01-01-preview"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "choices": [{"message": {"content": "Hello!"}}]
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let request_body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hi"}]
        });

        let response = client
            .post("/openai/v1/chat/completions", &request_body)
            .await
            .expect("should succeed");

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["id"], "chatcmpl-123");
    }

    #[tokio::test]
    async fn post_request_400_bad_request() {
        let server = MockServer::start().await;

        let error_body = serde_json::json!({
            "error": {
                "code": "BadRequest",
                "message": "Invalid request body"
            }
        });

        Mock::given(method("POST"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(400).set_body_json(error_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.post("/test/endpoint", &serde_json::json!({})).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Api { code, message } => {
                assert_eq!(code, "BadRequest");
                assert_eq!(message, "Invalid request body");
            }
            _ => panic!("Expected Api error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn post_request_429_rate_limit() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.post("/test/endpoint", &serde_json::json!({})).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Http { status, .. } => {
                assert_eq!(status, 429);
            }
            _ => panic!("Expected Http error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn get_request_201_created_is_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(serde_json::json!({"created": true})),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let response = client.get("/test/endpoint").await.expect("should succeed");

        assert_eq!(response.status(), 201);
    }

    #[tokio::test]
    async fn get_request_204_no_content_is_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let response = client.get("/test/endpoint").await.expect("should succeed");

        assert_eq!(response.status(), 204);
    }

    #[tokio::test]
    async fn error_response_with_non_json_body() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/test/endpoint").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Http { status, message } => {
                assert_eq!(status, 503);
                assert_eq!(message, "Service Unavailable");
            }
            _ => panic!("Expected Http error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn error_response_with_partial_error_object() {
        let server = MockServer::start().await;

        // Error object without message field
        let error_body = serde_json::json!({
            "error": {
                "code": "SomeError"
            }
        });

        Mock::given(method("GET"))
            .and(path("/test/endpoint"))
            .respond_with(ResponseTemplate::new(500).set_body_json(&error_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/test/endpoint").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            FoundryError::Api { code, message } => {
                assert_eq!(code, "SomeError");
                // Message should fall back to the raw body
                assert!(message.contains("SomeError"));
            }
            _ => panic!("Expected Api error, got {:?}", err),
        }
    }
}
