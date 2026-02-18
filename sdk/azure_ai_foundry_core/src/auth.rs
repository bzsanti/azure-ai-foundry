//! Authentication types for Azure AI Foundry.
//!
//! This module provides credential types that integrate with the Azure SDK for Rust.
//! It supports both API key authentication and Microsoft Entra ID (Azure AD) token-based
//! authentication via [`azure_identity`].
//!
//! # Examples
//!
//! ## Using API Key
//! ```rust,no_run
//! use azure_ai_foundry_core::auth::FoundryCredential;
//!
//! let credential = FoundryCredential::api_key("your-api-key");
//! ```
//!
//! ## Using Azure CLI Credential
//! ```rust,no_run
//! use azure_ai_foundry_core::auth::FoundryCredential;
//!
//! let credential = FoundryCredential::azure_cli().expect("Failed to create credential");
//! ```
//!
//! ## Using a Custom TokenCredential
//! ```rust,no_run
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_identity::ClientSecretCredential;
//! use azure_core::credentials::Secret;
//!
//! let credential = ClientSecretCredential::new(
//!     "tenant-id",
//!     "client-id".to_string(),
//!     Secret::new("client-secret"),
//!     None,
//! ).expect("Failed to create credential");
//!
//! let foundry_cred = FoundryCredential::token_credential(credential);
//! ```

use crate::error::{FoundryError, FoundryResult};
use azure_core::credentials::{AccessToken, TokenCredential, TokenRequestOptions};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Buffer time before token expiration to trigger proactive refresh.
/// Tokens will be refreshed when they have less than this duration remaining.
pub const TOKEN_EXPIRY_BUFFER: Duration = Duration::from_secs(60);

/// The scope required for Azure AI Foundry / Cognitive Services APIs.
pub const COGNITIVE_SERVICES_SCOPE: &str = "https://cognitiveservices.azure.com/.default";

/// Credential types supported by the Azure AI Foundry SDK.
///
/// This enum wraps either an API key or an Azure SDK [`TokenCredential`] implementation.
/// Use the convenience constructors to create credentials for common scenarios.
pub enum FoundryCredential {
    /// API key authentication (for OpenAI-compatible endpoints).
    ///
    /// The key is stored securely and not exposed in debug output.
    ApiKey(SecretString),

    /// Microsoft Entra ID (Azure AD) token-based authentication.
    ///
    /// Wraps any [`TokenCredential`] implementation from `azure_identity`.
    /// Includes an internal cache to avoid redundant token requests.
    TokenCredential {
        /// The underlying credential provider.
        credential: Arc<dyn TokenCredential>,
        /// Cached access token (if available).
        cache: Arc<Mutex<Option<AccessToken>>>,
    },
}

impl FoundryCredential {
    /// Returns a static string describing the credential type for tracing.
    ///
    /// This method is used internally to populate span fields without
    /// exposing sensitive information.
    fn credential_type_name(&self) -> &'static str {
        match self {
            Self::ApiKey(_) => "api_key",
            Self::TokenCredential { .. } => "token_credential",
        }
    }

    /// Create a credential from environment variables.
    ///
    /// Checks `AZURE_AI_FOUNDRY_API_KEY` first. If not set or empty,
    /// falls back to [`DeveloperToolsCredential`](azure_identity::DeveloperToolsCredential)
    /// which tries Azure CLI and Azure Developer CLI.
    ///
    /// # Errors
    ///
    /// Returns an error if Entra ID credential creation fails.
    pub fn from_env() -> FoundryResult<Self> {
        match std::env::var("AZURE_AI_FOUNDRY_API_KEY") {
            Ok(key) if !key.is_empty() => Ok(Self::ApiKey(SecretString::from(key))),
            _ => Self::developer_tools(),
        }
    }

    /// Create an API key credential.
    ///
    /// # Arguments
    ///
    /// * `key` - The API key string.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(SecretString::from(key.into()))
    }

    /// Create a credential from any [`TokenCredential`] implementation.
    ///
    /// This allows using any credential type from `azure_identity`, including:
    /// - [`ClientSecretCredential`](azure_identity::ClientSecretCredential)
    /// - [`ManagedIdentityCredential`](azure_identity::ManagedIdentityCredential)
    /// - [`AzureCliCredential`](azure_identity::AzureCliCredential)
    /// - [`DeveloperToolsCredential`](azure_identity::DeveloperToolsCredential)
    /// - Custom implementations
    ///
    /// # Arguments
    ///
    /// * `credential` - An `Arc` wrapping a `TokenCredential` implementation.
    pub fn token_credential(credential: Arc<dyn TokenCredential>) -> Self {
        Self::TokenCredential {
            credential,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a credential using [`DeveloperToolsCredential`](azure_identity::DeveloperToolsCredential).
    ///
    /// This tries Azure CLI and Azure Developer CLI in order.
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn developer_tools() -> FoundryResult<Self> {
        let credential = azure_identity::DeveloperToolsCredential::new(None).map_err(|e| {
            FoundryError::auth_with_source("failed to create developer tools credential", e)
        })?;
        Ok(Self::TokenCredential {
            credential,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// Create a credential using [`AzureCliCredential`](azure_identity::AzureCliCredential).
    ///
    /// Requires the Azure CLI to be installed and logged in.
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn azure_cli() -> FoundryResult<Self> {
        let credential = azure_identity::AzureCliCredential::new(None).map_err(|e| {
            FoundryError::auth_with_source("failed to create Azure CLI credential", e)
        })?;
        Ok(Self::TokenCredential {
            credential,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// Create a credential using [`ManagedIdentityCredential`](azure_identity::ManagedIdentityCredential).
    ///
    /// For use in Azure-hosted environments (VMs, App Service, AKS, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn managed_identity() -> FoundryResult<Self> {
        let credential = azure_identity::ManagedIdentityCredential::new(None).map_err(|e| {
            FoundryError::auth_with_source("failed to create managed identity credential", e)
        })?;
        Ok(Self::TokenCredential {
            credential,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// Resolve the credential to an authorization header value.
    ///
    /// For API keys, returns `Bearer <key>`.
    /// For token credentials, acquires a token for the Cognitive Services scope
    /// and returns `Bearer <token>`. Tokens are cached to avoid redundant requests,
    /// and automatically refreshed before expiration (with a 60-second buffer).
    ///
    /// This method is thread-safe: concurrent calls will wait for a single token
    /// acquisition rather than making duplicate requests.
    ///
    /// # Tracing
    ///
    /// This method emits a span named `foundry::auth::resolve` with the following fields:
    /// - `credential_type`: Either "api_key" or "token_credential"
    ///
    /// # Errors
    ///
    /// Returns an error if token acquisition fails.
    #[tracing::instrument(name = "foundry::auth::resolve", skip(self), fields(credential_type = self.credential_type_name()))]
    pub async fn resolve(&self) -> FoundryResult<String> {
        tracing::debug!("resolving credential");
        match self {
            Self::ApiKey(key) => Ok(format!("Bearer {}", key.expose_secret())),
            Self::TokenCredential { credential, cache } => {
                // Hold lock for the entire operation to prevent race conditions
                let mut cached = cache.lock().await;

                // Check if we have a valid cached token (with expiry buffer)
                if let Some(ref token) = *cached {
                    let now = azure_core::time::OffsetDateTime::now_utc();
                    let buffer = azure_core::time::Duration::try_from(TOKEN_EXPIRY_BUFFER)
                        .expect("buffer duration should be valid");
                    let refresh_at = token.expires_on - buffer;

                    if now < refresh_at {
                        return Ok(format!("Bearer {}", token.token.secret()));
                    }
                    // Token expired or within buffer - will refresh below
                }

                // Cache miss or needs refresh - acquire new token while holding lock
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                let token = credential
                    .get_token(scopes, None)
                    .await
                    .map_err(|e| FoundryError::auth_with_source("failed to acquire token", e))?;

                // Store in cache and return
                let auth_header = format!("Bearer {}", token.token.secret());
                *cached = Some(token);

                Ok(auth_header)
            }
        }
    }

    /// Get an access token for the Cognitive Services scope.
    ///
    /// This is useful when you need the raw token and expiration time,
    /// for example for caching or monitoring token lifetimes.
    ///
    /// Note: This method bypasses the internal cache and always fetches a fresh token.
    /// Use `resolve()` for normal authentication which benefits from caching.
    ///
    /// # Errors
    ///
    /// Returns an error if this is an API key credential (use `resolve()` instead)
    /// or if token acquisition fails.
    pub async fn get_token(&self) -> FoundryResult<AccessToken> {
        match self {
            Self::ApiKey(_) => Err(FoundryError::auth(
                "Cannot get token from API key credential. Use resolve() instead.",
            )),
            Self::TokenCredential { credential, .. } => {
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                credential
                    .get_token(scopes, None)
                    .await
                    .map_err(|e| FoundryError::auth_with_source("failed to acquire token", e))
            }
        }
    }

    /// Get an access token with custom options.
    ///
    /// Note: This method bypasses the internal cache and always fetches a fresh token.
    ///
    /// # Arguments
    ///
    /// * `options` - Token request options for advanced scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error if this is an API key credential or if token acquisition fails.
    pub async fn get_token_with_options(
        &self,
        options: TokenRequestOptions<'_>,
    ) -> FoundryResult<AccessToken> {
        match self {
            Self::ApiKey(_) => Err(FoundryError::auth(
                "Cannot get token from API key credential.",
            )),
            Self::TokenCredential { credential, .. } => {
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                credential
                    .get_token(scopes, Some(options))
                    .await
                    .map_err(|e| FoundryError::auth_with_source("failed to acquire token", e))
            }
        }
    }
}

impl Clone for FoundryCredential {
    fn clone(&self) -> Self {
        match self {
            Self::ApiKey(key) => Self::ApiKey(key.clone()),
            Self::TokenCredential { credential, cache } => Self::TokenCredential {
                credential: Arc::clone(credential),
                cache: Arc::clone(cache),
            },
        }
    }
}

impl std::fmt::Debug for FoundryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => write!(f, "FoundryCredential::ApiKey(****)"),
            Self::TokenCredential { .. } => write!(f, "FoundryCredential::TokenCredential(...)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use tracing_test::traced_test;

    // Mock TokenCredential for testing
    #[derive(Debug)]
    struct MockTokenCredential {
        token: String,
        should_fail: bool,
    }

    impl MockTokenCredential {
        fn new(token: impl Into<String>) -> Arc<Self> {
            Arc::new(Self {
                token: token.into(),
                should_fail: false,
            })
        }

        fn failing() -> Arc<Self> {
            Arc::new(Self {
                token: String::new(),
                should_fail: true,
            })
        }
    }

    /// Mock credential that counts calls to get_token
    #[derive(Debug)]
    struct CountingTokenCredential {
        token: String,
        call_count: AtomicU32,
        expires_in_secs: u64,
        delay_ms: u64,
    }

    impl CountingTokenCredential {
        fn new(token: impl Into<String>, expires_in_secs: u64) -> Arc<Self> {
            Arc::new(Self {
                token: token.into(),
                call_count: AtomicU32::new(0),
                expires_in_secs,
                delay_ms: 0,
            })
        }

        fn new_with_delay(
            token: impl Into<String>,
            expires_in_secs: u64,
            delay_ms: u64,
        ) -> Arc<Self> {
            Arc::new(Self {
                token: token.into(),
                call_count: AtomicU32::new(0),
                expires_in_secs,
                delay_ms,
            })
        }

        fn call_count(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl TokenCredential for CountingTokenCredential {
        async fn get_token(
            &self,
            scopes: &[&str],
            _options: Option<TokenRequestOptions<'_>>,
        ) -> azure_core::Result<AccessToken> {
            self.call_count.fetch_add(1, Ordering::SeqCst);

            // Simulate network latency to increase race condition probability
            if self.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            }

            assert!(
                scopes.contains(&COGNITIVE_SERVICES_SCOPE),
                "Expected scope {}, got {:?}",
                COGNITIVE_SERVICES_SCOPE,
                scopes
            );

            Ok(AccessToken::new(
                self.token.clone(),
                (std::time::SystemTime::now() + Duration::from_secs(self.expires_in_secs)).into(),
            ))
        }
    }

    #[async_trait::async_trait]
    impl TokenCredential for MockTokenCredential {
        async fn get_token(
            &self,
            scopes: &[&str],
            _options: Option<TokenRequestOptions<'_>>,
        ) -> azure_core::Result<AccessToken> {
            // Verify correct scope is passed
            assert!(
                scopes.contains(&COGNITIVE_SERVICES_SCOPE),
                "Expected scope {}, got {:?}",
                COGNITIVE_SERVICES_SCOPE,
                scopes
            );

            if self.should_fail {
                return Err(azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Credential,
                    "Mock credential failure",
                ));
            }

            Ok(AccessToken::new(
                self.token.clone(),
                (std::time::SystemTime::now() + Duration::from_secs(3600)).into(),
            ))
        }
    }

    #[test]
    fn api_key_credential_debug_hides_secret() {
        let cred = FoundryCredential::api_key("secret-key");
        let debug = format!("{:?}", cred);
        assert!(!debug.contains("secret-key"));
        assert!(debug.contains("****"));
    }

    #[test]
    fn token_credential_debug() {
        let mock = MockTokenCredential::new("test-token");
        let cred = FoundryCredential::token_credential(mock);
        let debug = format!("{:?}", cred);
        assert!(debug.contains("TokenCredential"));
        assert!(!debug.contains("test-token"));
    }

    #[test]
    fn api_key_is_cloneable() {
        let cred = FoundryCredential::api_key("test-key");
        let cloned = cred.clone();
        assert_eq!(format!("{:?}", cred), format!("{:?}", cloned));
    }

    #[test]
    fn token_credential_is_cloneable() {
        let mock = MockTokenCredential::new("test-token");
        let cred = FoundryCredential::token_credential(mock);
        let cloned = cred.clone();
        // Both should be TokenCredential variants
        assert!(matches!(cred, FoundryCredential::TokenCredential { .. }));
        assert!(matches!(cloned, FoundryCredential::TokenCredential { .. }));
    }

    #[test]
    #[serial]
    fn from_env_with_api_key() {
        // Save original value
        let original = std::env::var("AZURE_AI_FOUNDRY_API_KEY").ok();

        // Set env var
        std::env::set_var("AZURE_AI_FOUNDRY_API_KEY", "test-api-key-123");

        let cred = FoundryCredential::from_env().expect("should create credential");
        assert!(
            matches!(cred, FoundryCredential::ApiKey(_)),
            "Expected ApiKey, got {:?}",
            cred
        );

        // Restore original value
        match original {
            Some(val) => std::env::set_var("AZURE_AI_FOUNDRY_API_KEY", val),
            None => std::env::remove_var("AZURE_AI_FOUNDRY_API_KEY"),
        }
    }

    #[test]
    #[serial]
    fn from_env_with_empty_api_key_falls_back() {
        // Save original value
        let original = std::env::var("AZURE_AI_FOUNDRY_API_KEY").ok();

        // Set empty env var - should fall back to developer tools
        std::env::set_var("AZURE_AI_FOUNDRY_API_KEY", "");

        // This may fail if Azure CLI is not installed, which is expected
        let result = FoundryCredential::from_env();
        // Either succeeds with TokenCredential or fails with auth error
        match result {
            Ok(cred) => assert!(matches!(cred, FoundryCredential::TokenCredential { .. })),
            Err(e) => assert!(matches!(e, FoundryError::Auth { .. })),
        }

        // Restore original value
        match original {
            Some(val) => std::env::set_var("AZURE_AI_FOUNDRY_API_KEY", val),
            None => std::env::remove_var("AZURE_AI_FOUNDRY_API_KEY"),
        }
    }

    #[test]
    fn token_credential_constructor() {
        let mock = MockTokenCredential::new("my-token");
        let cred = FoundryCredential::token_credential(mock);
        assert!(matches!(cred, FoundryCredential::TokenCredential { .. }));
    }

    #[tokio::test]
    async fn resolve_with_api_key() {
        let cred = FoundryCredential::api_key("my-secret-key");
        let auth_header = cred.resolve().await.expect("should resolve");
        assert_eq!(auth_header, "Bearer my-secret-key");
    }

    #[tokio::test]
    async fn resolve_with_token_credential() {
        let mock = MockTokenCredential::new("mock-access-token");
        let cred = FoundryCredential::token_credential(mock);

        let auth_header = cred.resolve().await.expect("should resolve");
        assert_eq!(auth_header, "Bearer mock-access-token");
    }

    #[tokio::test]
    async fn resolve_with_failing_credential() {
        let mock = MockTokenCredential::failing();
        let cred = FoundryCredential::token_credential(mock);

        let result = cred.resolve().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FoundryError::Auth { .. }));
        assert!(err.to_string().contains("failed to acquire token"));
    }

    #[tokio::test]
    async fn get_token_with_api_key_fails() {
        let cred = FoundryCredential::api_key("my-key");
        let result = cred.get_token().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FoundryError::Auth { .. }));
        assert!(err.to_string().contains("API key credential"));
    }

    #[tokio::test]
    async fn get_token_with_token_credential() {
        let mock = MockTokenCredential::new("access-token-123");
        let cred = FoundryCredential::token_credential(mock);

        let token = cred.get_token().await.expect("should get token");
        assert_eq!(token.token.secret(), "access-token-123");
        // Token should expire in the future
        assert!(token.expires_on > azure_core::time::OffsetDateTime::now_utc());
    }

    #[tokio::test]
    async fn get_token_with_options_api_key_fails() {
        let cred = FoundryCredential::api_key("my-key");
        let options = TokenRequestOptions::default();
        let result = cred.get_token_with_options(options).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FoundryError::Auth { .. }));
    }

    #[tokio::test]
    async fn get_token_with_options_token_credential() {
        let mock = MockTokenCredential::new("token-with-options");
        let cred = FoundryCredential::token_credential(mock);

        let options = TokenRequestOptions::default();
        let token = cred
            .get_token_with_options(options)
            .await
            .expect("should get token");
        assert_eq!(token.token.secret(), "token-with-options");
    }

    #[test]
    fn cognitive_services_scope_is_correct() {
        assert_eq!(
            COGNITIVE_SERVICES_SCOPE,
            "https://cognitiveservices.azure.com/.default"
        );
    }

    // ========== Token Caching Tests ==========

    #[tokio::test]
    async fn test_token_cache_stores_valid_token() {
        // Setup: Create a counting mock that expires in 1 hour
        let mock = CountingTokenCredential::new("cached-token", 3600);
        let cred = FoundryCredential::token_credential(mock.clone());

        // Action: Call resolve() twice
        let result1 = cred.resolve().await.expect("first resolve should succeed");
        let result2 = cred.resolve().await.expect("second resolve should succeed");

        // Assert: Both return the same token
        assert_eq!(result1, "Bearer cached-token");
        assert_eq!(result2, "Bearer cached-token");

        // Assert: get_token was called only ONCE (second call used cache)
        assert_eq!(
            mock.call_count(),
            1,
            "get_token should be called only once, second call should use cache"
        );
    }

    #[tokio::test]
    async fn test_token_cache_expires_after_ttl() {
        // Setup: Create a counting mock that expires in 1 second
        let mock = CountingTokenCredential::new("short-lived-token", 1);
        let cred = FoundryCredential::token_credential(mock.clone());

        // Action: Call resolve() to populate cache
        let result1 = cred.resolve().await.expect("first resolve should succeed");
        assert_eq!(result1, "Bearer short-lived-token");
        assert_eq!(mock.call_count(), 1, "first call should fetch token");

        // Wait for token to expire (slightly more than 1 second)
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Call resolve again - should need to refresh
        let result2 = cred.resolve().await.expect("second resolve should succeed");
        assert_eq!(result2, "Bearer short-lived-token");

        // Assert: get_token was called TWICE (second refresh due to expiration)
        assert_eq!(
            mock.call_count(),
            2,
            "get_token should be called twice, second call refreshes expired token"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_token_cache_thread_safe() {
        // Setup: Create a counting mock that expires in 1 hour
        // Adding a small delay to simulate network latency and increase race condition probability
        let mock = CountingTokenCredential::new_with_delay("concurrent-token", 3600, 10);
        let cred = Arc::new(FoundryCredential::token_credential(mock.clone()));

        // Action: Spawn 10 concurrent tasks that all call resolve()
        let mut handles = Vec::new();
        for _ in 0..10 {
            let cred_clone = Arc::clone(&cred);
            handles.push(tokio::spawn(async move {
                cred_clone.resolve().await.expect("resolve should succeed")
            }));
        }

        // Wait for all tasks to complete
        let results: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("task should not panic"))
            .collect();

        // Assert: All tasks got the same token
        for result in &results {
            assert_eq!(result, "Bearer concurrent-token");
        }

        // Assert: get_token was called only ONCE (no race conditions)
        assert_eq!(
            mock.call_count(),
            1,
            "get_token should be called only once, even with concurrent access"
        );
    }

    #[tokio::test]
    async fn test_token_cache_refreshes_before_expiry() {
        // Setup: Token that "expires" in 30 seconds, which is within the 60s buffer
        // This should trigger a refresh even though the token hasn't technically expired
        let mock = CountingTokenCredential::new("almost-expired-token", 30);
        let cred = FoundryCredential::token_credential(mock.clone());

        // First call - should fetch token
        let result1 = cred.resolve().await.expect("first resolve should succeed");
        assert_eq!(result1, "Bearer almost-expired-token");
        assert_eq!(mock.call_count(), 1, "first call should fetch token");

        // Second call - token is within expiry buffer, should refresh
        let result2 = cred.resolve().await.expect("second resolve should succeed");
        assert_eq!(result2, "Bearer almost-expired-token");

        // Assert: get_token was called TWICE because token is within expiry buffer
        assert_eq!(
            mock.call_count(),
            2,
            "get_token should be called twice, token is within 60s expiry buffer"
        );
    }

    #[tokio::test]
    async fn test_api_key_credential_no_cache() {
        // Setup: API key credential
        let cred = FoundryCredential::api_key("test-api-key");

        // Action: Call resolve() multiple times
        let result1 = cred.resolve().await.expect("first resolve should succeed");
        let result2 = cred.resolve().await.expect("second resolve should succeed");
        let result3 = cred.resolve().await.expect("third resolve should succeed");

        // Assert: All calls return the same API key immediately
        assert_eq!(result1, "Bearer test-api-key");
        assert_eq!(result2, "Bearer test-api-key");
        assert_eq!(result3, "Bearer test-api-key");

        // The test passes if we get here without errors - API keys don't use caching
        // and should return immediately without any async waits
    }

    // ========== High Concurrency Tests ==========

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_100_concurrent_token_refreshes() {
        // Setup: 100 concurrent tasks, cache empty, all calling resolve() simultaneously
        // Adding delay to simulate real token acquisition latency
        let mock = CountingTokenCredential::new_with_delay("high-concurrency-token", 3600, 50);
        let cred = Arc::new(FoundryCredential::token_credential(mock.clone()));

        // Action: Spawn 100 concurrent tasks
        let mut handles = Vec::new();
        for _ in 0..100 {
            let cred_clone = Arc::clone(&cred);
            handles.push(tokio::spawn(async move {
                cred_clone.resolve().await.expect("resolve should succeed")
            }));
        }

        // Wait for all tasks to complete
        let results: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("task should not panic"))
            .collect();

        // Assert: All 100 tasks got the same token
        assert_eq!(results.len(), 100);
        for result in &results {
            assert_eq!(result, "Bearer high-concurrency-token");
        }

        // Assert: get_token was called only ONCE despite 100 concurrent calls
        // This verifies the mutex correctly serializes access
        assert_eq!(
            mock.call_count(),
            1,
            "get_token should be called only once, even with 100 concurrent tasks"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_no_deadlock_with_repeated_concurrent_access() {
        // Setup: Simulate high load with repeated concurrent access
        let mock = CountingTokenCredential::new_with_delay("repeated-access-token", 3600, 10);
        let cred = Arc::new(FoundryCredential::token_credential(mock.clone()));

        // Action: 50 tasks, each calling resolve() 10 times
        let mut handles = Vec::new();
        for _ in 0..50 {
            let cred_clone = Arc::clone(&cred);
            handles.push(tokio::spawn(async move {
                for _ in 0..10 {
                    let _ = cred_clone.resolve().await.expect("resolve should succeed");
                }
            }));
        }

        // All tasks should complete without deadlock (timeout would indicate deadlock)
        let timeout_result =
            tokio::time::timeout(Duration::from_secs(10), futures::future::join_all(handles)).await;

        assert!(
            timeout_result.is_ok(),
            "Tasks should complete within timeout (no deadlock)"
        );

        // All 500 total calls should have succeeded
        let results = timeout_result.unwrap();
        for result in results {
            assert!(result.is_ok(), "Task should not have panicked");
        }
    }

    // --- Tracing Instrumentation Tests ---

    #[tokio::test]
    #[traced_test]
    async fn test_resolve_emits_auth_span() {
        let cred = FoundryCredential::api_key("test-key");
        let _ = cred.resolve().await;

        // Verify span was emitted with debug event
        assert!(logs_contain("foundry::auth::resolve"));
        assert!(logs_contain("resolving credential"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_resolve_emits_credential_type_field() {
        // Test API key credential type
        let api_key_cred = FoundryCredential::api_key("test-key");
        let _ = api_key_cred.resolve().await;
        assert!(logs_contain("credential_type"));
        assert!(logs_contain("api_key"));
    }
}
