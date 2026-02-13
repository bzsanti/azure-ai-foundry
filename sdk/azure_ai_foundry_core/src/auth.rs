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
    TokenCredential(Arc<dyn TokenCredential>),
}

impl FoundryCredential {
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
        Self::TokenCredential(credential)
    }

    /// Create a credential using [`DeveloperToolsCredential`](azure_identity::DeveloperToolsCredential).
    ///
    /// This tries Azure CLI and Azure Developer CLI in order.
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn developer_tools() -> FoundryResult<Self> {
        let credential = azure_identity::DeveloperToolsCredential::new(None)
            .map_err(|e| FoundryError::Auth(e.to_string()))?;
        Ok(Self::TokenCredential(credential))
    }

    /// Create a credential using [`AzureCliCredential`](azure_identity::AzureCliCredential).
    ///
    /// Requires the Azure CLI to be installed and logged in.
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn azure_cli() -> FoundryResult<Self> {
        let credential = azure_identity::AzureCliCredential::new(None)
            .map_err(|e| FoundryError::Auth(e.to_string()))?;
        Ok(Self::TokenCredential(credential))
    }

    /// Create a credential using [`ManagedIdentityCredential`](azure_identity::ManagedIdentityCredential).
    ///
    /// For use in Azure-hosted environments (VMs, App Service, AKS, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if credential creation fails.
    pub fn managed_identity() -> FoundryResult<Self> {
        let credential = azure_identity::ManagedIdentityCredential::new(None)
            .map_err(|e| FoundryError::Auth(e.to_string()))?;
        Ok(Self::TokenCredential(credential))
    }

    /// Resolve the credential to an authorization header value.
    ///
    /// For API keys, returns `Bearer <key>`.
    /// For token credentials, acquires a token for the Cognitive Services scope
    /// and returns `Bearer <token>`.
    ///
    /// # Errors
    ///
    /// Returns an error if token acquisition fails.
    pub async fn resolve(&self) -> FoundryResult<String> {
        match self {
            Self::ApiKey(key) => Ok(format!("Bearer {}", key.expose_secret())),
            Self::TokenCredential(credential) => {
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                let token = credential
                    .get_token(scopes, None)
                    .await
                    .map_err(|e| FoundryError::Auth(e.to_string()))?;
                Ok(format!("Bearer {}", token.token.secret()))
            }
        }
    }

    /// Get an access token for the Cognitive Services scope.
    ///
    /// This is useful when you need the raw token and expiration time,
    /// for example for caching or monitoring token lifetimes.
    ///
    /// # Errors
    ///
    /// Returns an error if this is an API key credential (use `resolve()` instead)
    /// or if token acquisition fails.
    pub async fn get_token(&self) -> FoundryResult<AccessToken> {
        match self {
            Self::ApiKey(_) => Err(FoundryError::Auth(
                "Cannot get token from API key credential. Use resolve() instead.".into(),
            )),
            Self::TokenCredential(credential) => {
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                credential
                    .get_token(scopes, None)
                    .await
                    .map_err(|e| FoundryError::Auth(e.to_string()))
            }
        }
    }

    /// Get an access token with custom options.
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
            Self::ApiKey(_) => Err(FoundryError::Auth(
                "Cannot get token from API key credential.".into(),
            )),
            Self::TokenCredential(credential) => {
                let scopes = &[COGNITIVE_SERVICES_SCOPE];
                credential
                    .get_token(scopes, Some(options))
                    .await
                    .map_err(|e| FoundryError::Auth(e.to_string()))
            }
        }
    }
}

impl Clone for FoundryCredential {
    fn clone(&self) -> Self {
        match self {
            Self::ApiKey(key) => Self::ApiKey(key.clone()),
            Self::TokenCredential(cred) => Self::TokenCredential(Arc::clone(cred)),
        }
    }
}

impl std::fmt::Debug for FoundryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => write!(f, "FoundryCredential::ApiKey(****)"),
            Self::TokenCredential(_) => write!(f, "FoundryCredential::TokenCredential(...)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
        assert!(matches!(cred, FoundryCredential::TokenCredential(_)));
        assert!(matches!(cloned, FoundryCredential::TokenCredential(_)));
    }

    #[test]
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
    fn from_env_with_empty_api_key_falls_back() {
        // Save original value
        let original = std::env::var("AZURE_AI_FOUNDRY_API_KEY").ok();

        // Set empty env var - should fall back to developer tools
        std::env::set_var("AZURE_AI_FOUNDRY_API_KEY", "");

        // This may fail if Azure CLI is not installed, which is expected
        let result = FoundryCredential::from_env();
        // Either succeeds with TokenCredential or fails with auth error
        match result {
            Ok(cred) => assert!(matches!(cred, FoundryCredential::TokenCredential(_))),
            Err(e) => assert!(matches!(e, FoundryError::Auth(_))),
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
        assert!(matches!(cred, FoundryCredential::TokenCredential(_)));
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
        assert!(matches!(err, FoundryError::Auth(_)));
        assert!(err.to_string().contains("Mock credential failure"));
    }

    #[tokio::test]
    async fn get_token_with_api_key_fails() {
        let cred = FoundryCredential::api_key("my-key");
        let result = cred.get_token().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FoundryError::Auth(_)));
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
        assert!(matches!(result.unwrap_err(), FoundryError::Auth(_)));
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
}
