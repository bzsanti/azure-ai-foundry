use crate::auth::FoundryCredential;
use crate::error::{FoundryError, FoundryResult};
use reqwest::Client as HttpClient;
use url::Url;

/// The base client for interacting with the Azure AI Foundry API.
///
/// This client handles authentication, HTTP transport, and endpoint management.
/// It is used by higher-level crates (`azure_ai_foundry_models`, `azure_ai_foundry_agents`)
/// to make API calls.
#[derive(Debug, Clone)]
pub struct FoundryClient {
    pub(crate) http: HttpClient,
    pub(crate) endpoint: Url,
    pub(crate) credential: FoundryCredential,
    pub(crate) api_version: String,
}

/// Builder for constructing a [`FoundryClient`].
#[derive(Debug)]
pub struct FoundryClientBuilder {
    endpoint: Option<String>,
    credential: Option<FoundryCredential>,
    api_version: Option<String>,
    http_client: Option<HttpClient>,
}

impl FoundryClient {
    /// Create a new builder for configuring a `FoundryClient`.
    pub fn builder() -> FoundryClientBuilder {
        FoundryClientBuilder {
            endpoint: None,
            credential: None,
            api_version: None,
            http_client: None,
        }
    }

    /// Get the base endpoint URL.
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Build a full URL for an API path.
    pub fn url(&self, path: &str) -> FoundryResult<Url> {
        self.endpoint
            .join(path)
            .map_err(|e| FoundryError::InvalidEndpoint(e.to_string()))
    }

    /// Send a GET request to the API.
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
                        message: err_obj
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or(&body)
                            .to_string(),
                    });
                }
            }

            Err(FoundryError::Http {
                status,
                message: body,
            })
        }
    }
}

impl FoundryClientBuilder {
    /// Set the Azure AI Foundry endpoint URL.
    ///
    /// This should be in the format:
    /// `https://<resource-name>.services.ai.azure.com`
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the credential to use for authentication.
    pub fn credential(mut self, credential: FoundryCredential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Set the API version. Defaults to `2025-01-01-preview`.
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Set a custom HTTP client.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the `FoundryClient`.
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
                .unwrap_or_else(|| "2025-01-01-preview".to_string()),
        })
    }
}
