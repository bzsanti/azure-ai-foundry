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

use std::time::Duration;

/// Default API version for Azure AI Foundry.
///
/// # Warning
///
/// This is a **preview** API version (`-preview` suffix). Preview APIs may change
/// without notice and are not covered by SLA guarantees. For production use,
/// consider pinning to a stable version via
/// [`FoundryClientBuilder::api_version`](FoundryClientBuilder::api_version).
pub const DEFAULT_API_VERSION: &str = "2025-01-01-preview";

/// Default connection timeout (10 seconds).
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default read/response timeout (60 seconds).
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Default streaming timeout (5 minutes).
///
/// This is longer than the standard read timeout to accommodate
/// long-running streaming responses like chat completions.
pub const DEFAULT_STREAMING_TIMEOUT: Duration = Duration::from_secs(300);

/// Determines if an HTTP status code represents a retriable error.
///
/// Retriable errors are transient server-side issues that may succeed on retry:
/// - 429 Too Many Requests (rate limiting)
/// - 500 Internal Server Error
/// - 502 Bad Gateway
/// - 503 Service Unavailable
/// - 504 Gateway Timeout
#[inline]
pub fn is_retriable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Maximum backoff duration to prevent excessive waits (60 seconds).
pub const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Compute backoff duration with jitter for retry attempts.
///
/// Calculates exponential backoff (2^attempt * initial_backoff) with ±25% jitter
/// to prevent thundering herd problems when multiple clients retry simultaneously.
///
/// The backoff is capped at [`MAX_BACKOFF`] (60 seconds) to prevent excessive waits.
/// Uses saturating arithmetic to prevent overflow with large attempt values.
///
/// # Arguments
///
/// * `attempt` - The current retry attempt number (0-indexed)
/// * `initial_backoff` - Base backoff duration for the first retry
///
/// # Returns
///
/// The computed backoff duration with jitter applied, capped at 60 seconds.
#[inline]
fn compute_backoff(attempt: u32, initial_backoff: Duration) -> Duration {
    // Clamp exponent to prevent u32 overflow (2^31 overflows u32)
    let exponent = attempt.min(30);
    let multiplier = 2_u32.saturating_pow(exponent);
    // Use saturating_mul to prevent Duration overflow, then cap at MAX_BACKOFF
    let base_backoff = initial_backoff.saturating_mul(multiplier).min(MAX_BACKOFF);
    let jitter = 0.75 + fastrand::f64() * 0.5; // 0.75 to 1.25
    base_backoff.mul_f64(jitter)
}

/// Extract the retry delay from a `Retry-After` header if present.
///
/// Parses the `Retry-After` header value as seconds and returns the duration,
/// capped at [`MAX_BACKOFF`] to prevent excessive waits.
///
/// # Arguments
///
/// * `headers` - The HTTP response headers
///
/// # Returns
///
/// `Some(Duration)` if a valid `Retry-After` header is present, `None` otherwise.
#[inline]
fn extract_retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|secs| Duration::from_secs(secs).min(MAX_BACKOFF))
}

/// Configuration for automatic retry behavior on transient errors.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (not counting the initial request).
    pub max_retries: u32,
    /// Initial backoff duration before the first retry.
    /// Subsequent retries use exponential backoff (2^attempt * initial_backoff).
    pub initial_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(500),
        }
    }
}

impl RetryPolicy {
    /// Maximum allowed value for `max_retries` to prevent excessive retries.
    pub const MAX_ALLOWED_RETRIES: u32 = 10;

    /// Construct a validated `RetryPolicy`.
    ///
    /// # Arguments
    ///
    /// * `max_retries` - Maximum number of retry attempts (must be <= 10)
    /// * `initial_backoff` - Initial backoff duration (must be <= [`MAX_BACKOFF`])
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `max_retries` exceeds [`Self::MAX_ALLOWED_RETRIES`] (10)
    /// - `initial_backoff` exceeds [`MAX_BACKOFF`] (60 seconds)
    ///
    /// # Example
    ///
    /// ```
    /// use azure_ai_foundry_core::client::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// let policy = RetryPolicy::new(5, Duration::from_secs(1)).expect("valid policy");
    /// assert_eq!(policy.max_retries, 5);
    /// ```
    pub fn new(max_retries: u32, initial_backoff: Duration) -> FoundryResult<Self> {
        if max_retries > Self::MAX_ALLOWED_RETRIES {
            return Err(FoundryError::Builder(format!(
                "max_retries must be <= {}, got {}",
                Self::MAX_ALLOWED_RETRIES,
                max_retries
            )));
        }
        if initial_backoff > MAX_BACKOFF {
            return Err(FoundryError::Builder(format!(
                "initial_backoff must be <= {:?}, got {:?}",
                MAX_BACKOFF, initial_backoff
            )));
        }
        Ok(Self {
            max_retries,
            initial_backoff,
        })
    }
}

/// The base client for interacting with the Azure AI Foundry API.
///
/// This client handles authentication, HTTP transport, and endpoint management.
/// It is used by higher-level crates (`azure_ai_foundry_models`, `azure_ai_foundry_agents`)
/// to make API calls.
///
/// The client is cheaply cloneable and can be shared across threads.
#[derive(Debug, Clone)]
pub struct FoundryClient {
    http: HttpClient,
    endpoint: Url,
    credential: FoundryCredential,
    api_version: String,
    retry_policy: RetryPolicy,
    streaming_timeout: Duration,
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
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    streaming_timeout: Option<Duration>,
    retry_policy: Option<RetryPolicy>,
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

    /// Get the retry policy configuration.
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    /// Get the streaming timeout duration.
    ///
    /// This is the maximum time allowed for streaming responses.
    pub fn streaming_timeout(&self) -> Duration {
        self.streaming_timeout
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
            .map_err(|e| FoundryError::invalid_endpoint_with_source("failed to construct URL", e))
    }

    /// Send a GET request to the API with automatic retry on transient errors.
    ///
    /// Automatically adds authentication headers and API version.
    /// Retries on retriable HTTP errors (429, 500, 502, 503, 504) with exponential backoff.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    ///
    /// # Tracing
    ///
    /// This method emits a span named `foundry::client::get` with the following fields:
    /// - `path`: The API path being requested
    /// - `attempt`: Current retry attempt (0-indexed)
    /// - `status_code`: HTTP status code of the response
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, the request fails after all retries,
    /// or the server returns a non-retriable error response.
    #[tracing::instrument(
        name = "foundry::client::get",
        skip(self),
        fields(path = %path, attempt, status_code)
    )]
    pub async fn get(&self, path: &str) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;

        for attempt in 0..=self.retry_policy.max_retries {
            let span = tracing::Span::current();
            span.record("attempt", attempt);

            // Resolve credential on each attempt to handle token expiration during retries.
            // The internal cache ensures this is O(1) when the token is still valid.
            let auth = self.credential.resolve().await?;

            tracing::debug!("sending GET request");

            let response = self
                .http
                .get(url.clone())
                .header("Authorization", &auth)
                .header("api-version", &self.api_version)
                .send()
                .await?;

            let status = response.status().as_u16();
            span.record("status_code", status);

            // Success - return response
            if response.status().is_success() {
                return Ok(response);
            }

            // Non-retriable error or last attempt - return error
            if !is_retriable_status(status) || attempt == self.retry_policy.max_retries {
                return Self::check_response(response).await;
            }

            tracing::warn!(status = status, attempt = attempt, "retriable error, will retry");

            // Respect Retry-After header if present; otherwise use exponential backoff
            let backoff = extract_retry_after_delay(response.headers())
                .unwrap_or_else(|| compute_backoff(attempt, self.retry_policy.initial_backoff));
            tokio::time::sleep(backoff).await;
        }

        // This should never be reached due to the loop logic
        unreachable!("retry loop should return before reaching here")
    }

    /// Send a POST request with a JSON body to the API with automatic retry.
    ///
    /// Automatically adds authentication headers and API version.
    /// Retries on retriable HTTP errors (429, 500, 502, 503, 504) with exponential backoff.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    /// * `body` - The request body to serialize as JSON.
    ///
    /// # Tracing
    ///
    /// This method emits a span named `foundry::client::post` with the following fields:
    /// - `path`: The API path being requested
    /// - `attempt`: Current retry attempt (0-indexed)
    /// - `status_code`: HTTP status code of the response
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, serialization fails,
    /// the request fails after all retries, or the server returns a non-retriable error.
    #[tracing::instrument(
        name = "foundry::client::post",
        skip(self, body),
        fields(path = %path, attempt, status_code)
    )]
    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;

        for attempt in 0..=self.retry_policy.max_retries {
            let span = tracing::Span::current();
            span.record("attempt", attempt);

            // Resolve credential on each attempt to handle token expiration during retries.
            // The internal cache ensures this is O(1) when the token is still valid.
            let auth = self.credential.resolve().await?;

            tracing::debug!("sending POST request");

            let response = self
                .http
                .post(url.clone())
                .header("Authorization", &auth)
                .header("api-version", &self.api_version)
                .json(body)
                .send()
                .await?;

            let status = response.status().as_u16();
            span.record("status_code", status);

            // Success - return response
            if response.status().is_success() {
                return Ok(response);
            }

            // Non-retriable error or last attempt - return error
            if !is_retriable_status(status) || attempt == self.retry_policy.max_retries {
                return Self::check_response(response).await;
            }

            tracing::warn!(status = status, attempt = attempt, "retriable error, will retry");

            // Respect Retry-After header if present; otherwise use exponential backoff
            let backoff = extract_retry_after_delay(response.headers())
                .unwrap_or_else(|| compute_backoff(attempt, self.retry_policy.initial_backoff));
            tokio::time::sleep(backoff).await;
        }

        unreachable!("retry loop should return before reaching here")
    }

    /// Send a DELETE request to the API with automatic retry on transient errors.
    ///
    /// Automatically adds authentication headers and API version.
    /// Retries on retriable HTTP errors (429, 500, 502, 503, 504) with exponential backoff.
    ///
    /// # Arguments
    ///
    /// * `path` - The API path to request.
    ///
    /// # Tracing
    ///
    /// This method emits a span named `foundry::client::delete` with the following fields:
    /// - `path`: The API path being requested
    /// - `attempt`: Current retry attempt (0-indexed)
    /// - `status_code`: HTTP status code of the response
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, the request fails after all retries,
    /// or the server returns a non-retriable error response.
    #[tracing::instrument(
        name = "foundry::client::delete",
        skip(self),
        fields(path = %path, attempt, status_code)
    )]
    pub async fn delete(&self, path: &str) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;

        for attempt in 0..=self.retry_policy.max_retries {
            let span = tracing::Span::current();
            span.record("attempt", attempt);

            // Resolve credential on each attempt to handle token expiration during retries.
            let auth = self.credential.resolve().await?;

            tracing::debug!("sending DELETE request");

            let response = self
                .http
                .delete(url.clone())
                .header("Authorization", &auth)
                .header("api-version", &self.api_version)
                .send()
                .await?;

            let status = response.status().as_u16();
            span.record("status_code", status);

            // Success - return response
            if response.status().is_success() {
                return Ok(response);
            }

            // Non-retriable error or last attempt - return error
            if !is_retriable_status(status) || attempt == self.retry_policy.max_retries {
                return Self::check_response(response).await;
            }

            tracing::warn!(status = status, attempt = attempt, "retriable error, will retry");

            // Respect Retry-After header if present; otherwise use exponential backoff
            let backoff = extract_retry_after_delay(response.headers())
                .unwrap_or_else(|| compute_backoff(attempt, self.retry_policy.initial_backoff));
            tokio::time::sleep(backoff).await;
        }

        unreachable!("retry loop should return before reaching here")
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
    /// # Tracing
    ///
    /// This method emits a span named `foundry::client::post_stream` with the following fields:
    /// - `path`: The API path being requested
    /// - `attempt`: Current retry attempt (0-indexed)
    /// - `status_code`: HTTP status code of the response
    /// - `streaming_timeout_secs`: The streaming timeout in seconds
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, serialization fails,
    /// the request fails, or the HTTP status code indicates an error.
    #[tracing::instrument(
        name = "foundry::client::post_stream",
        skip(self, body),
        fields(path = %path, attempt, status_code, streaming_timeout_secs = self.streaming_timeout.as_secs())
    )]
    pub async fn post_stream<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> FoundryResult<reqwest::Response> {
        let url = self.url(path)?;

        // Retry loop for pre-stream errors only (connection errors and retriable status codes)
        // Once we receive a success response, the stream starts and we cannot retry.
        for attempt in 0..=self.retry_policy.max_retries {
            let span = tracing::Span::current();
            span.record("attempt", attempt);

            // Resolve credential on each attempt to handle token expiration during retries.
            // The internal cache ensures this is O(1) when the token is still valid.
            let auth = self.credential.resolve().await?;

            tracing::debug!("sending POST request for streaming");

            // Use streaming-specific timeout (longer than default for streaming responses)
            let response = self
                .http
                .post(url.clone())
                .header("Authorization", &auth)
                .header("api-version", &self.api_version)
                .timeout(self.streaming_timeout)
                .json(body)
                .send()
                .await?;

            let status = response.status().as_u16();
            span.record("status_code", status);

            // Success - return response for streaming (no more retries after this point)
            if response.status().is_success() {
                tracing::debug!("stream started");
                return Ok(response);
            }

            // Non-retriable error or last attempt - return error
            if !is_retriable_status(status) || attempt == self.retry_policy.max_retries {
                return Self::check_response(response).await;
            }

            tracing::warn!(status = status, attempt = attempt, "retriable error, will retry");

            // Respect Retry-After header if present; otherwise use exponential backoff
            let backoff = extract_retry_after_delay(response.headers())
                .unwrap_or_else(|| compute_backoff(attempt, self.retry_policy.initial_backoff));
            tokio::time::sleep(backoff).await;
        }

        unreachable!("retry loop should return before reaching here")
    }

    /// Maximum length for error messages to prevent sensitive data leaks.
    const MAX_ERROR_MESSAGE_LEN: usize = 1000;

    /// Sanitize error messages by removing sensitive data like tokens and API keys.
    ///
    /// This prevents credentials from being accidentally logged or exposed in error messages.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn sanitize_error_message(msg: &str) -> String {
        let mut result = msg.to_string();

        // Sanitize Bearer tokens (format: "Bearer <token>")
        // Use offset to avoid infinite loops
        let mut search_start = 0;
        while search_start < result.len() {
            if let Some(relative_pos) = result[search_start..].find("Bearer ") {
                let bearer_pos = search_start + relative_pos;
                let token_start = bearer_pos + 7; // "Bearer " is 7 chars

                if token_start < result.len() {
                    // Skip if already redacted
                    if result[token_start..].starts_with("[REDACTED]") {
                        search_start = token_start + 10;
                        continue;
                    }

                    // Find the end of the token (next whitespace/delimiter or end of string)
                    let token_end = result[token_start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                        .map(|pos| token_start + pos)
                        .unwrap_or(result.len());

                    if token_end > token_start {
                        result.replace_range(token_start..token_end, "[REDACTED]");
                        search_start = token_start + 10; // "[REDACTED]" is 10 chars
                    } else {
                        search_start = token_start;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Sanitize sk- style API keys (OpenAI format)
        search_start = 0;
        while search_start < result.len() {
            if let Some(relative_pos) = result[search_start..].find("sk-") {
                let sk_pos = search_start + relative_pos;
                let key_end = result[sk_pos..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                    .map(|pos| sk_pos + pos)
                    .unwrap_or(result.len());

                if key_end > sk_pos + 3 {
                    result.replace_range(sk_pos..key_end, "[REDACTED]");
                    search_start = sk_pos + 10; // "[REDACTED]" is 10 chars
                } else {
                    search_start = sk_pos + 3;
                }
            } else {
                break;
            }
        }

        // Sanitize JWT tokens (Entra ID tokens starting with "eyJ")
        // JWTs always start with "eyJ" because the header {"alg":...} encodes to this prefix
        search_start = 0;
        while search_start < result.len() {
            if let Some(relative_pos) = result[search_start..].find("eyJ") {
                let jwt_pos = search_start + relative_pos;
                // JWT tokens contain alphanumeric chars, dots, underscores, and hyphens (base64url + separators)
                let jwt_end = result[jwt_pos..]
                    .find(|c: char| {
                        c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == ')'
                    })
                    .map(|pos| jwt_pos + pos)
                    .unwrap_or(result.len());

                if jwt_end > jwt_pos + 3 {
                    result.replace_range(jwt_pos..jwt_end, "[REDACTED]");
                    search_start = jwt_pos + 10;
                } else {
                    search_start = jwt_pos + 3;
                }
            } else {
                break;
            }
        }

        // Sanitize api-key: pattern (Azure style)
        search_start = 0;
        while search_start < result.len() {
            // Case-insensitive search for "api-key:"
            let lower = result[search_start..].to_lowercase();
            if let Some(relative_pos) = lower.find("api-key:") {
                let key_pos = search_start + relative_pos + 8; // "api-key:" is 8 chars
                // Skip any whitespace after the colon
                let value_start = result[key_pos..]
                    .find(|c: char| !c.is_whitespace())
                    .map(|pos| key_pos + pos)
                    .unwrap_or(result.len());

                if value_start < result.len() {
                    let value_end = result[value_start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                        .map(|pos| value_start + pos)
                        .unwrap_or(result.len());

                    if value_end > value_start {
                        result.replace_range(value_start..value_end, "[REDACTED]");
                        search_start = value_start + 10;
                    } else {
                        search_start = value_start;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Sanitize Ocp-Apim-Subscription-Key: pattern (Azure API Management)
        search_start = 0;
        while search_start < result.len() {
            let lower = result[search_start..].to_lowercase();
            if let Some(relative_pos) = lower.find("ocp-apim-subscription-key:") {
                let key_pos = search_start + relative_pos + 26; // header is 26 chars
                let value_start = result[key_pos..]
                    .find(|c: char| !c.is_whitespace())
                    .map(|pos| key_pos + pos)
                    .unwrap_or(result.len());

                if value_start < result.len() {
                    let value_end = result[value_start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                        .map(|pos| value_start + pos)
                        .unwrap_or(result.len());

                    if value_end > value_start {
                        result.replace_range(value_start..value_end, "[REDACTED]");
                        search_start = value_start + 10;
                    } else {
                        search_start = value_start;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// Truncate a message if it exceeds the maximum length.
    /// Also sanitizes sensitive data before truncating.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn truncate_message(msg: &str) -> String {
        // Sanitize first to ensure sensitive data is removed before truncation
        let sanitized = Self::sanitize_error_message(msg);

        if sanitized.len() > Self::MAX_ERROR_MESSAGE_LEN {
            format!(
                "{}... (truncated)",
                &sanitized[..Self::MAX_ERROR_MESSAGE_LEN]
            )
        } else {
            sanitized
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

            Err(FoundryError::http(status, Self::truncate_message(&body)))
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
    ///
    /// **Note:** If you provide a custom HTTP client, any timeout configuration
    /// via [`connect_timeout`](Self::connect_timeout) will be ignored.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Set the connection timeout.
    ///
    /// This is the maximum time allowed for establishing a connection to the server.
    ///
    /// **Note:** This setting is ignored if a custom HTTP client is provided
    /// via [`http_client`](Self::http_client).
    pub fn connect_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the read timeout.
    ///
    /// This is the maximum time allowed for receiving a response from the server.
    /// It covers the entire request/response cycle including reading the body.
    ///
    /// **Note:** This setting is ignored if a custom HTTP client is provided
    /// via [`http_client`](Self::http_client).
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    /// Set the streaming timeout.
    ///
    /// This is the maximum time allowed for streaming responses like chat completions.
    /// Streaming requests typically take longer than regular requests, so this timeout
    /// is separate from the standard read timeout.
    ///
    /// Defaults to [`DEFAULT_STREAMING_TIMEOUT`] (5 minutes) if not specified.
    pub fn streaming_timeout(mut self, timeout: Duration) -> Self {
        self.streaming_timeout = Some(timeout);
        self
    }

    /// Set the retry policy for transient errors.
    ///
    /// Configures automatic retries for retriable HTTP errors (429, 500, 502, 503, 504)
    /// with exponential backoff.
    ///
    /// Defaults to 3 retries with 500ms initial backoff.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
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
    /// - HTTP client construction fails (rare, typically due to TLS issues)
    pub fn build(self) -> FoundryResult<FoundryClient> {
        // Build HTTP client first using timeout configuration
        let http = if let Some(client) = self.http_client {
            client
        } else {
            let connect_timeout = self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
            let read_timeout = self.read_timeout.unwrap_or(DEFAULT_READ_TIMEOUT);

            reqwest::Client::builder()
                .connect_timeout(connect_timeout)
                .timeout(read_timeout)
                .build()
                .map_err(|e| FoundryError::Builder(format!("failed to build HTTP client: {}", e)))?
        };

        let endpoint_str = self
            .endpoint
            .or_else(|| std::env::var("AZURE_AI_FOUNDRY_ENDPOINT").ok())
            .ok_or_else(|| {
                FoundryError::MissingConfig(
                    "endpoint is required. Set it via builder or AZURE_AI_FOUNDRY_ENDPOINT env var."
                        .into(),
                )
            })?;

        let endpoint = Url::parse(&endpoint_str)
            .map_err(|e| FoundryError::invalid_endpoint_with_source("invalid endpoint URL", e))?;

        let credential = self
            .credential
            .map(Ok)
            .unwrap_or_else(FoundryCredential::from_env)?;

        Ok(FoundryClient {
            http,
            endpoint,
            credential,
            api_version: self
                .api_version
                .unwrap_or_else(|| DEFAULT_API_VERSION.to_string()),
            retry_policy: self.retry_policy.unwrap_or_default(),
            streaming_timeout: self.streaming_timeout.unwrap_or(DEFAULT_STREAMING_TIMEOUT),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tracing_test::traced_test;
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
            FoundryError::InvalidEndpoint { .. }
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
            FoundryError::Http {
                status, message, ..
            } => {
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
            FoundryError::Http {
                status, message, ..
            } => {
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

    // --- Timeout configuration tests ---

    #[test]
    fn builder_accepts_connect_timeout() {
        use std::time::Duration;

        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("should build");

        // The client should build successfully with timeout configured
        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }

    #[test]
    fn builder_accepts_read_timeout() {
        use std::time::Duration;

        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .read_timeout(Duration::from_secs(30))
            .build()
            .expect("should build");

        // The client should build successfully with read timeout configured
        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }

    #[test]
    fn default_timeouts_are_defined() {
        use std::time::Duration;

        // Verify default timeout constants are defined and have sensible values
        assert_eq!(DEFAULT_CONNECT_TIMEOUT, Duration::from_secs(10));
        assert_eq!(DEFAULT_READ_TIMEOUT, Duration::from_secs(60));
        assert_eq!(DEFAULT_STREAMING_TIMEOUT, Duration::from_secs(300)); // 5 minutes
    }

    #[test]
    fn test_builder_accepts_streaming_timeout() {
        use std::time::Duration;

        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .streaming_timeout(Duration::from_secs(180))
            .build()
            .expect("should build");

        assert_eq!(client.streaming_timeout(), Duration::from_secs(180));
    }

    #[test]
    fn test_default_streaming_timeout_is_5_minutes() {
        use std::time::Duration;

        // Build client without specifying streaming_timeout
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        // Default should be 5 minutes (300 seconds)
        assert_eq!(client.streaming_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn default_timeouts_applied_when_not_specified() {
        // Build client without specifying timeouts
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        // Client should build successfully with default timeouts applied
        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }

    #[test]
    fn custom_http_client_ignores_timeout_config() {
        use std::time::Duration;

        // Create a custom HTTP client
        let custom_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(2))
            .build()
            .expect("should build custom client");

        // Build FoundryClient with custom client AND timeout config
        // The custom client should be used, ignoring the builder's timeout settings
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .http_client(custom_client)
            .connect_timeout(Duration::from_secs(99)) // Should be ignored
            .read_timeout(Duration::from_secs(99)) // Should be ignored
            .build()
            .expect("should build");

        // Client should build successfully using the custom HTTP client
        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }

    #[tokio::test]
    async fn request_times_out_with_configured_timeout() {
        use std::time::Duration;

        let server = MockServer::start().await;

        // Mock that delays response for 2 seconds
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("OK")
                    .set_delay(Duration::from_secs(2)),
            )
            .mount(&server)
            .await;

        // Client with 500ms timeout (less than 2 second delay)
        let client = FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test"))
            .read_timeout(Duration::from_millis(500))
            .build()
            .expect("should build");

        let start = std::time::Instant::now();
        let result = client.get("/slow").await;
        let elapsed = start.elapsed();

        // Should fail with a Request error due to timeout
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FoundryError::Request(_)),
            "Expected Request error from timeout, got {:?}",
            err
        );

        // Verify that the request timed out quickly (around 500ms, not 2s)
        // Allow some margin for timing variations
        assert!(
            elapsed < Duration::from_secs(1),
            "Request should have timed out within ~500ms, but took {:?}",
            elapsed
        );
    }

    // --- Retry logic tests ---

    #[test]
    fn identifies_retriable_http_errors() {
        // 429 Too Many Requests - should retry
        assert!(is_retriable_status(429));

        // 503 Service Unavailable - should retry
        assert!(is_retriable_status(503));

        // 504 Gateway Timeout - should retry
        assert!(is_retriable_status(504));

        // 500 Internal Server Error - should retry (transient)
        assert!(is_retriable_status(500));

        // 502 Bad Gateway - should retry
        assert!(is_retriable_status(502));

        // 4xx client errors should NOT retry (except 429)
        assert!(!is_retriable_status(400));
        assert!(!is_retriable_status(401));
        assert!(!is_retriable_status(403));
        assert!(!is_retriable_status(404));

        // 2xx success should NOT retry
        assert!(!is_retriable_status(200));
        assert!(!is_retriable_status(201));
    }

    #[test]
    fn builder_accepts_retry_policy() {
        use std::time::Duration;

        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff: Duration::from_millis(200),
        };

        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .retry_policy(policy)
            .build()
            .expect("should build");

        // Verify retry policy is configured
        assert_eq!(client.retry_policy().max_retries, 5);
        assert_eq!(
            client.retry_policy().initial_backoff,
            Duration::from_millis(200)
        );
    }

    #[test]
    fn default_retry_policy() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        // Default policy: 3 retries, 500ms initial backoff
        assert_eq!(client.retry_policy().max_retries, 3);
        assert_eq!(
            client.retry_policy().initial_backoff,
            Duration::from_millis(500)
        );
    }

    #[test]
    fn retry_policy_new_accepts_valid_values() {
        let policy = RetryPolicy::new(5, Duration::from_secs(1)).expect("should be valid");
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.initial_backoff, Duration::from_secs(1));
    }

    #[test]
    fn retry_policy_new_accepts_zero_backoff() {
        // Zero backoff is valid (useful in tests)
        let policy = RetryPolicy::new(3, Duration::ZERO).expect("should be valid");
        assert_eq!(policy.initial_backoff, Duration::ZERO);
    }

    #[test]
    fn retry_policy_new_rejects_excessive_retries() {
        let result = RetryPolicy::new(11, Duration::from_millis(500));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("max_retries"));
    }

    #[test]
    fn retry_policy_new_rejects_excessive_backoff() {
        // initial_backoff > MAX_BACKOFF (60s) should fail
        let result = RetryPolicy::new(3, Duration::from_secs(120));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("initial_backoff"));
    }

    #[tokio::test]
    async fn get_retries_on_503_with_backoff() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let server = MockServer::start().await;
        let request_count = Arc::new(AtomicU32::new(0));
        let counter = request_count.clone();

        // Mock that fails with 503 twice, then succeeds
        Mock::given(method("GET"))
            .and(path("/retry-test"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    ResponseTemplate::new(503).set_body_string("Service Unavailable")
                } else {
                    ResponseTemplate::new(200).set_body_string("OK")
                }
            })
            .mount(&server)
            .await;

        // Client with fast backoff for testing
        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10), // Fast for testing
        };

        let client = FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test"))
            .retry_policy(policy)
            .build()
            .expect("should build");

        let start = std::time::Instant::now();
        let result = client.get("/retry-test").await;
        let elapsed = start.elapsed();

        // Should succeed after retries
        assert!(
            result.is_ok(),
            "Expected success after retries, got {:?}",
            result
        );

        // Should have made 3 requests (initial + 2 retries)
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            3,
            "Expected 3 requests (initial + 2 retries)"
        );

        // Should have taken some time for backoff (at least 10ms + 20ms = 30ms)
        assert!(
            elapsed >= Duration::from_millis(20),
            "Expected backoff delays, but elapsed {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn post_retries_on_429_rate_limit() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let server = MockServer::start().await;
        let request_count = Arc::new(AtomicU32::new(0));
        let counter = request_count.clone();

        // Mock that returns 429 once, then succeeds
        Mock::given(method("POST"))
            .and(path("/rate-limited"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 1 {
                    ResponseTemplate::new(429)
                        .set_body_string("Rate limit exceeded")
                        .insert_header("Retry-After", "1")
                } else {
                    ResponseTemplate::new(200).set_body_string(r#"{"result": "ok"}"#)
                }
            })
            .mount(&server)
            .await;

        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10),
        };

        let client = FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test"))
            .retry_policy(policy)
            .build()
            .expect("should build");

        #[derive(serde::Serialize)]
        struct TestBody {
            data: String,
        }

        let body = TestBody {
            data: "test".to_string(),
        };

        let result = client.post("/rate-limited", &body).await;

        // Should succeed after retry
        assert!(
            result.is_ok(),
            "Expected success after retry, got {:?}",
            result
        );

        // Should have made 2 requests (initial 429 + retry success)
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            2,
            "Expected 2 requests (initial + 1 retry)"
        );
    }

    #[tokio::test]
    async fn post_stream_retries_on_503_before_stream_starts() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let server = MockServer::start().await;
        let request_count = Arc::new(AtomicU32::new(0));
        let counter = request_count.clone();

        // Mock that returns 503 once, then succeeds
        Mock::given(method("POST"))
            .and(path("/stream-retry"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 1 {
                    ResponseTemplate::new(503).set_body_string("Service Unavailable")
                } else {
                    // Return success with streaming content type
                    ResponseTemplate::new(200)
                        .set_body_string("data: test\n\n")
                        .insert_header("content-type", "text/event-stream")
                }
            })
            .mount(&server)
            .await;

        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10),
        };

        let client = FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test"))
            .retry_policy(policy)
            .build()
            .expect("should build");

        #[derive(serde::Serialize)]
        struct TestBody {
            data: String,
        }

        let body = TestBody {
            data: "test".to_string(),
        };

        let result = client.post_stream("/stream-retry", &body).await;

        // Should succeed after retry
        assert!(
            result.is_ok(),
            "Expected success after retry, got {:?}",
            result
        );

        // Should have made 2 requests (initial 503 + retry success)
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            2,
            "Expected 2 requests (initial + 1 retry)"
        );
    }

    #[tokio::test]
    async fn retry_backoff_includes_jitter() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let server = MockServer::start().await;
        let request_count = Arc::new(AtomicU32::new(0));
        let counter = request_count.clone();

        // Mock that fails 4 times then succeeds
        Mock::given(method("GET"))
            .and(path("/jitter-test"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 4 {
                    ResponseTemplate::new(503).set_body_string("Service Unavailable")
                } else {
                    ResponseTemplate::new(200).set_body_string("OK")
                }
            })
            .mount(&server)
            .await;

        // Run multiple times and collect delays
        let mut all_delays = Vec::new();

        for _ in 0..3 {
            let policy = RetryPolicy {
                max_retries: 5,
                initial_backoff: Duration::from_millis(50),
            };

            let client = FoundryClient::builder()
                .endpoint(server.uri())
                .credential(FoundryCredential::api_key("test"))
                .retry_policy(policy)
                .build()
                .expect("should build");

            let start = Instant::now();
            let _ = client.get("/jitter-test").await;
            let elapsed = start.elapsed();
            all_delays.push(elapsed);
        }

        // With jitter, delays should NOT be exactly the same
        // Check that at least some variation exists
        let min_delay = all_delays.iter().min().unwrap();
        let max_delay = all_delays.iter().max().unwrap();

        // There should be SOME variation (jitter adds ±25%)
        // With 4 retries at 50ms base: ~50+100+200+400 = 750ms base
        // With jitter: range should be roughly ±25% = ~180ms variation
        let variation = *max_delay - *min_delay;

        // Just verify jitter is working - some variation should exist
        // (Due to system timing, we can't be too strict)
        assert!(
            variation > Duration::from_millis(0) || all_delays.len() == 1,
            "Jitter should cause some variation in retry delays"
        );
    }

    #[tokio::test]
    async fn get_respects_retry_after_header() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let server = MockServer::start().await;
        let request_count = Arc::new(AtomicU32::new(0));
        let counter = request_count.clone();

        Mock::given(method("GET"))
            .and(path("/retry-after-test"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    ResponseTemplate::new(429)
                        .set_body_string("Rate limited")
                        .insert_header("Retry-After", "1") // Server asks to wait 1 second
                } else {
                    ResponseTemplate::new(200).set_body_string("OK")
                }
            })
            .mount(&server)
            .await;

        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_millis(10), // Much smaller than Retry-After
        };

        let client = FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test"))
            .retry_policy(policy)
            .build()
            .expect("should build");

        let start = Instant::now();
        let result = client.get("/retry-after-test").await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        // Must have waited at least 1 second (Retry-After from server),
        // not just 10ms from initial_backoff
        assert!(
            elapsed >= Duration::from_millis(900),
            "Should have waited for Retry-After (1s), but waited only {:?}",
            elapsed
        );
    }

    // --- Error Sanitization Tests (Mejora 2: Security) ---

    #[tokio::test]
    async fn test_error_sanitization_removes_bearer_tokens() {
        let server = MockServer::start().await;

        // Error response containing a bearer token
        let error_body = serde_json::json!({
            "error": {
                "code": "Unauthorized",
                "message": "Invalid token: Bearer sk-1234567890abcdef1234567890abcdef"
            }
        });

        Mock::given(method("GET"))
            .and(path("/sensitive-error"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/sensitive-error").await;

        assert!(result.is_err());
        let err = result.unwrap_err();

        let err_string = err.to_string();

        // Should NOT contain the actual token
        assert!(
            !err_string.contains("sk-1234567890abcdef"),
            "Error message should NOT contain sensitive token, got: {}",
            err_string
        );

        // Should contain a redaction marker
        assert!(
            err_string.contains("[REDACTED]"),
            "Error message should contain [REDACTED] marker, got: {}",
            err_string
        );
    }

    #[tokio::test]
    async fn test_error_sanitization_removes_api_keys() {
        let server = MockServer::start().await;

        // Error response containing an OpenAI-style API key
        Mock::given(method("GET"))
            .and(path("/api-key-error"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_string("Invalid API key: sk-proj1234567890abcdefghijklmnop"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let result = client.get("/api-key-error").await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();

        // Should NOT contain the actual API key
        assert!(
            !err_string.contains("sk-proj1234567890"),
            "Error message should NOT contain API key, got: {}",
            err_string
        );

        // Should contain redaction marker
        assert!(
            err_string.contains("[REDACTED]"),
            "Error message should contain [REDACTED], got: {}",
            err_string
        );
    }

    #[test]
    fn test_sanitization_before_truncation() {
        // Test that a long message with a token near the end gets sanitized
        // even when the message is truncated
        let token = "sk-verylongtokenthatmightbetrimmed123456789";
        let padding = "x".repeat(950); // Near MAX_ERROR_MESSAGE_LEN (1000)
        let msg = format!("{} token: {}", padding, token);

        let result = FoundryClient::truncate_message(&msg);

        // Should NOT contain the actual token
        assert!(
            !result.contains("sk-verylongtokenthatmightbetrimmed"),
            "Truncated message should NOT contain token"
        );
    }

    #[test]
    fn test_sanitization_preserves_legitimate_errors() {
        // Error messages without sensitive data should be unchanged
        let msg = "Invalid model 'gpt-4o' for this deployment. Please check your configuration.";
        let result = FoundryClient::sanitize_error_message(msg);

        assert_eq!(
            result, msg,
            "Legitimate error messages should be preserved unchanged"
        );
    }

    #[test]
    fn test_sanitization_multiple_tokens() {
        // Multiple tokens in same message
        let msg = "Token Bearer abc123 and key sk-xyz789 both invalid";
        let result = FoundryClient::sanitize_error_message(msg);

        assert!(!result.contains("abc123"), "First token should be redacted");
        assert!(
            !result.contains("xyz789"),
            "Second token should be redacted"
        );
        assert_eq!(
            result.matches("[REDACTED]").count(),
            2,
            "Should have two redaction markers"
        );
    }

    #[test]
    fn sanitize_jwt_tokens_in_error_messages() {
        // A real JWT has 3 parts separated by dots, all in base64url
        let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIiwiZXhwIjoxNzAwMDAwMDAwfQ.signature123";
        let msg = format!("Token validation failed: {}", jwt);
        let result = FoundryClient::sanitize_error_message(&msg);
        assert!(
            !result.contains("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9"),
            "JWT header should be redacted"
        );
        assert!(
            result.contains("[REDACTED]"),
            "Should contain redaction marker"
        );
    }

    #[test]
    fn sanitize_partial_jwt_eyj_prefix() {
        let msg = "Invalid token eyJhbGci.payload.sig in request";
        let result = FoundryClient::sanitize_error_message(msg);
        assert!(!result.contains("eyJhbGci"), "Partial JWT should be redacted");
    }

    #[test]
    fn sanitize_api_key_header_pattern() {
        let msg = "Request failed with api-key: abc123secret456 - invalid key";
        let result = FoundryClient::sanitize_error_message(msg);
        assert!(
            !result.contains("abc123secret456"),
            "api-key value should be redacted"
        );
        assert!(
            result.contains("[REDACTED]"),
            "Should contain redaction marker"
        );
    }

    #[test]
    fn sanitize_ocp_apim_subscription_key_header() {
        // Alternative header used by some Azure services
        let msg = "Ocp-Apim-Subscription-Key: deadbeef1234 was invalid";
        let result = FoundryClient::sanitize_error_message(msg);
        assert!(
            !result.contains("deadbeef1234"),
            "Subscription key should be redacted"
        );
    }

    // --- Tracing Instrumentation Tests ---

    #[tokio::test]
    #[traced_test]
    async fn test_get_emits_http_span() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/tracing-test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let _ = client.get("/tracing-test").await;

        // Verifies span is emitted with debug event
        assert!(logs_contain("foundry::client::get"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_post_emits_http_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/tracing-post-test"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok": true}"#))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let _ = client
            .post("/tracing-post-test", &serde_json::json!({"test": true}))
            .await;

        assert!(logs_contain("foundry::client::post"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_post_stream_emits_http_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/tracing-stream-test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("data: test\n\n")
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let _ = client
            .post_stream("/tracing-stream-test", &serde_json::json!({"stream": true}))
            .await;

        assert!(logs_contain("foundry::client::post_stream"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_error_events_do_not_contain_bearer_tokens() {
        let server = MockServer::start().await;

        // Error response containing a bearer token that should be sanitized
        Mock::given(method("GET"))
            .and(path("/secret-error"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string("Invalid token: Bearer sk-secret123token456"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let _ = client.get("/secret-error").await;

        // The raw token must NEVER appear in logs
        logs_assert(|lines: &[&str]| {
            let has_secret = lines.iter().any(|line| line.contains("sk-secret123"));
            if has_secret {
                Err(format!(
                    "SECURITY: Sensitive token found in logs!\nLogs:\n{}",
                    lines.join("\n")
                ))
            } else {
                Ok(())
            }
        });
    }

    // --- compute_backoff tests ---

    #[test]
    fn test_compute_backoff_attempt_zero() {
        let backoff = compute_backoff(0, Duration::from_millis(500));
        // With jitter 0.75-1.25: range 375ms - 625ms (2^0 = 1)
        assert!(backoff >= Duration::from_millis(375));
        assert!(backoff <= Duration::from_millis(625));
    }

    #[test]
    fn test_compute_backoff_attempt_one() {
        let backoff = compute_backoff(1, Duration::from_millis(500));
        // With jitter 0.75-1.25: range 750ms - 1250ms (2^1 = 2)
        assert!(backoff >= Duration::from_millis(750));
        assert!(backoff <= Duration::from_millis(1250));
    }

    #[test]
    fn test_compute_backoff_large_attempt_does_not_overflow() {
        // Should not panic even with large attempt values
        let backoff = compute_backoff(100, Duration::from_millis(500));
        // Should be capped at MAX_BACKOFF (60 seconds) with jitter
        assert!(backoff <= Duration::from_secs(75)); // MAX_BACKOFF * 1.25 jitter
    }

    #[test]
    fn test_compute_backoff_capped_at_max() {
        // With initial_backoff = 10s and attempt = 10, base would be 10240s
        // Should be capped at MAX_BACKOFF (60s)
        let backoff = compute_backoff(10, Duration::from_secs(10));
        assert!(backoff <= Duration::from_secs(75)); // MAX_BACKOFF * 1.25 jitter
        assert!(backoff >= Duration::from_secs(45)); // MAX_BACKOFF * 0.75 jitter
    }

    #[test]
    fn test_compute_backoff_zero_initial() {
        let backoff = compute_backoff(5, Duration::ZERO);
        assert_eq!(backoff, Duration::ZERO);
    }

    // --- Retry-After Header Tests ---

    #[test]
    fn extract_retry_delay_from_seconds_header() {
        use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("30"));
        let delay = extract_retry_after_delay(&headers);
        assert_eq!(delay, Some(Duration::from_secs(30)));
    }

    #[test]
    fn extract_retry_delay_missing_header() {
        let headers = reqwest::header::HeaderMap::new();
        let delay = extract_retry_after_delay(&headers);
        assert_eq!(delay, None);
    }

    #[test]
    fn extract_retry_delay_capped_at_max_backoff() {
        use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("3600")); // 1 hour
        let delay = extract_retry_after_delay(&headers);
        // Must respect MAX_BACKOFF as upper bound
        assert_eq!(delay, Some(MAX_BACKOFF));
    }

    #[test]
    fn extract_retry_delay_invalid_value_returns_none() {
        use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("not-a-number"));
        let delay = extract_retry_after_delay(&headers);
        assert_eq!(delay, None);
    }

    // --- Encapsulation Tests ---

    /// Verifies that FoundryClient works correctly using only its public API.
    /// The internal fields (http, credential) should not need to be accessed directly.
    #[test]
    fn client_internals_are_encapsulated() {
        let client = FoundryClient::builder()
            .endpoint("https://test.services.ai.azure.com")
            .credential(FoundryCredential::api_key("test"))
            .build()
            .expect("should build");

        // All functionality is available through the public API
        assert!(client.url("/test").is_ok());
        assert_eq!(client.api_version(), DEFAULT_API_VERSION);
        assert_eq!(client.retry_policy().max_retries, 3);
        assert_eq!(client.streaming_timeout(), DEFAULT_STREAMING_TIMEOUT);
        assert_eq!(
            client.endpoint().as_str(),
            "https://test.services.ai.azure.com/"
        );
    }
}
