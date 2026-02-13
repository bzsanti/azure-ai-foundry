use crate::error::{FoundryError, FoundryResult};
use secrecy::{ExposeSecret, SecretString};

/// Credential types supported by the Azure AI Foundry SDK.
#[derive(Clone)]
pub enum FoundryCredential {
    /// API key authentication (for OpenAI-compatible endpoints).
    ApiKey(SecretString),

    /// Microsoft Entra ID (Azure AD) token-based authentication.
    /// Uses `azure_identity::DefaultAzureCredential` under the hood.
    EntraId,
}

impl FoundryCredential {
    /// Create a credential from the `AZURE_AI_FOUNDRY_API_KEY` environment variable.
    /// Falls back to Entra ID if the variable is not set.
    pub fn from_env() -> FoundryResult<Self> {
        match std::env::var("AZURE_AI_FOUNDRY_API_KEY") {
            Ok(key) if !key.is_empty() => Ok(Self::ApiKey(SecretString::from(key))),
            _ => Ok(Self::EntraId),
        }
    }

    /// Create an API key credential.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(SecretString::from(key.into()))
    }

    /// Create an Entra ID credential.
    pub fn entra_id() -> Self {
        Self::EntraId
    }

    /// Resolve the credential to an authorization header value.
    pub async fn resolve(&self) -> FoundryResult<String> {
        match self {
            Self::ApiKey(key) => Ok(format!("Bearer {}", key.expose_secret())),
            Self::EntraId => {
                // TODO: Implement azure_identity::DefaultAzureCredential token acquisition
                // For now, check for a pre-set token in env
                let token = std::env::var("AZURE_AI_FOUNDRY_TOKEN").map_err(|_| {
                    FoundryError::Auth(
                        "Entra ID authentication not yet fully implemented. \
                         Set AZURE_AI_FOUNDRY_API_KEY or AZURE_AI_FOUNDRY_TOKEN."
                            .into(),
                    )
                })?;
                Ok(format!("Bearer {}", token))
            }
        }
    }
}

impl std::fmt::Debug for FoundryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => write!(f, "FoundryCredential::ApiKey(****)"),
            Self::EntraId => write!(f, "FoundryCredential::EntraId"),
        }
    }
}
