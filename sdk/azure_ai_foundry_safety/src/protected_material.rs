//! Protected material detection for Azure AI Content Safety.
//!
//! Detects whether text contains copyrighted or protected material
//! (e.g., song lyrics, news articles, code from known repositories).

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::CONTENT_SAFETY_API_VERSION;

/// Maximum text length allowed by the API (Unicode code points).
const MAX_TEXT_LENGTH: usize = 10_000;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for the protected material detection endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct ProtectedMaterialRequest {
    text: String,
}

impl ProtectedMaterialRequest {
    /// Creates a new builder for `ProtectedMaterialRequest`.
    pub fn builder() -> ProtectedMaterialRequestBuilder {
        ProtectedMaterialRequestBuilder::default()
    }

    /// Returns the text being analyzed.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Builder for [`ProtectedMaterialRequest`].
#[derive(Debug, Default)]
pub struct ProtectedMaterialRequestBuilder {
    text: Option<String>,
}

impl ProtectedMaterialRequestBuilder {
    /// Sets the text to analyze for protected material (required, max 10,000 characters).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Builds the request, returning an error if validation fails.
    pub fn try_build(self) -> FoundryResult<ProtectedMaterialRequest> {
        let text = self
            .text
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| FoundryError::Builder("text is required".into()))?;

        if text.chars().count() > MAX_TEXT_LENGTH {
            return Err(FoundryError::Builder(format!(
                "text exceeds maximum length of {MAX_TEXT_LENGTH} characters"
            )));
        }

        Ok(ProtectedMaterialRequest { text })
    }

    /// Builds the request, panicking if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing or invalid. Use [`try_build`](Self::try_build)
    /// for a fallible alternative.
    pub fn build(self) -> ProtectedMaterialRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Details of the protected material analysis.
#[derive(Debug, Clone, Deserialize)]
pub struct ProtectedMaterialAnalysis {
    /// Whether protected material was detected in the text.
    pub detected: bool,
}

/// Response from the protected material detection endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProtectedMaterialResponse {
    /// The analysis result.
    #[serde(rename = "protectedMaterialAnalysis")]
    pub protected_material_analysis: ProtectedMaterialAnalysis,
}

// ---------------------------------------------------------------------------
// API function
// ---------------------------------------------------------------------------

/// Detect protected material (copyrighted content) in text.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `request` - The detection request built via [`ProtectedMaterialRequest::builder`].
///
/// # Example
///
/// ```rust,no_run
/// use azure_ai_foundry_core::client::FoundryClient;
/// use azure_ai_foundry_core::auth::FoundryCredential;
/// use azure_ai_foundry_safety::protected_material::{self, ProtectedMaterialRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = FoundryClient::builder()
///     .endpoint("https://your-resource.cognitiveservices.azure.com")
///     .credential(FoundryCredential::api_key("your-key"))
///     .build()?;
///
/// let request = ProtectedMaterialRequest::builder()
///     .text("Some model-generated text to check")
///     .try_build()?;
///
/// let response = protected_material::detect_protected_material(&client, &request).await?;
/// if response.protected_material_analysis.detected {
///     println!("Protected material detected!");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if authentication fails, the request fails, or the API
/// returns an error response.
#[tracing::instrument(
    name = "foundry::safety::detect_protected_material",
    skip(client, request),
    fields(text_len = request.text.len())
)]
pub async fn detect_protected_material(
    client: &FoundryClient,
    request: &ProtectedMaterialRequest,
) -> FoundryResult<ProtectedMaterialResponse> {
    tracing::debug!("detecting protected material");

    let path = format!("/contentsafety/text:detectProtectedMaterial?{CONTENT_SAFETY_API_VERSION}");
    let response = client.post(&path, request).await?;
    let result = response.json::<ProtectedMaterialResponse>().await?;

    tracing::debug!("protected material detection complete");
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_mock_client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -- Builder validation --

    #[test]
    fn test_protected_material_requires_text() {
        let result = ProtectedMaterialRequest::builder().try_build();
        let err = result.expect_err("should require text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_protected_material_rejects_blank_text() {
        let result = ProtectedMaterialRequest::builder().text("   ").try_build();
        let err = result.expect_err("should reject blank text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_protected_material_rejects_text_too_long() {
        let long_text = "a".repeat(MAX_TEXT_LENGTH + 1);
        let result = ProtectedMaterialRequest::builder()
            .text(long_text)
            .try_build();
        let err = result.expect_err("should reject long text");
        assert!(err.to_string().contains("maximum length"), "error: {err}");
    }

    #[test]
    fn test_protected_material_accepts_valid_text() {
        let result = ProtectedMaterialRequest::builder()
            .text("Some model output to check")
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_protected_material_serialization() {
        let request = ProtectedMaterialRequest::builder()
            .text("test content")
            .build();

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["text"], "test content");
    }

    // -- Response deserialization --

    #[test]
    fn test_protected_material_response_detected_true() {
        let json = r#"{"protectedMaterialAnalysis": {"detected": true}}"#;
        let response: ProtectedMaterialResponse = serde_json::from_str(json).unwrap();
        assert!(response.protected_material_analysis.detected);
    }

    #[test]
    fn test_protected_material_response_detected_false() {
        let json = r#"{"protectedMaterialAnalysis": {"detected": false}}"#;
        let response: ProtectedMaterialResponse = serde_json::from_str(json).unwrap();
        assert!(!response.protected_material_analysis.detected);
    }

    // -- API function --

    #[tokio::test]
    async fn test_detect_protected_material_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:detectProtectedMaterial"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "protectedMaterialAnalysis": {"detected": false}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = ProtectedMaterialRequest::builder()
            .text("Original content here")
            .build();

        let result = detect_protected_material(&client, &request)
            .await
            .expect("should succeed");
        assert!(!result.protected_material_analysis.detected);
    }

    #[tokio::test]
    async fn test_detect_protected_material_api_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:detectProtectedMaterial"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": {
                    "code": "TooManyRequests",
                    "message": "Rate limit exceeded"
                }
            })))
            .mount(&server)
            .await;

        let request = ProtectedMaterialRequest::builder().text("test").build();

        let err = detect_protected_material(&client, &request)
            .await
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("TooManyRequests") || msg.contains("Rate limit"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_detect_protected_material_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:detectProtectedMaterial"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "protectedMaterialAnalysis": {"detected": false}
            })))
            .mount(&server)
            .await;

        let request = ProtectedMaterialRequest::builder().text("test").build();

        let _ = detect_protected_material(&client, &request).await;
        assert!(logs_contain("foundry::safety::detect_protected_material"));
    }
}
