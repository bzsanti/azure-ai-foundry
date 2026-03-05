//! Image content analysis for Azure AI Content Safety.
//!
//! Analyzes images for harmful content across four categories: hate, self-harm,
//! sexual, and violence. Images can be provided as base64-encoded content or
//! Azure Blob Storage URLs.

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::{CategoryAnalysis, HarmCategory, ImageOutputType, CONTENT_SAFETY_API_VERSION};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Image source for content analysis — either base64-encoded content or a blob URL.
#[derive(Debug, Clone, Serialize)]
struct ImageSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    #[serde(rename = "blobUrl", skip_serializing_if = "Option::is_none")]
    blob_url: Option<String>,
}

/// Request body for the image content analysis endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeImageRequest {
    image: ImageSource,

    #[serde(skip_serializing_if = "Option::is_none")]
    categories: Option<Vec<HarmCategory>>,

    #[serde(rename = "outputType", skip_serializing_if = "Option::is_none")]
    output_type: Option<ImageOutputType>,
}

impl AnalyzeImageRequest {
    /// Creates a new builder for `AnalyzeImageRequest`.
    pub fn builder() -> AnalyzeImageRequestBuilder {
        AnalyzeImageRequestBuilder::default()
    }
}

/// Builder for [`AnalyzeImageRequest`].
#[derive(Debug, Default)]
pub struct AnalyzeImageRequestBuilder {
    base64_content: Option<String>,
    blob_url: Option<String>,
    categories: Option<Vec<HarmCategory>>,
    output_type: Option<ImageOutputType>,
}

impl AnalyzeImageRequestBuilder {
    /// Sets the image as base64-encoded content.
    pub fn base64_content(mut self, content: impl Into<String>) -> Self {
        self.base64_content = Some(content.into());
        self
    }

    /// Sets the image as an Azure Blob Storage URL.
    pub fn blob_url(mut self, url: impl Into<String>) -> Self {
        self.blob_url = Some(url.into());
        self
    }

    /// Sets the harm categories to analyze. If not set, all categories are analyzed.
    pub fn categories(mut self, categories: Vec<HarmCategory>) -> Self {
        self.categories = Some(categories);
        self
    }

    /// Sets the output type (only `FourSeverityLevels` is supported for images).
    pub fn output_type(mut self, output_type: ImageOutputType) -> Self {
        self.output_type = Some(output_type);
        self
    }

    /// Builds the request, returning an error if validation fails.
    ///
    /// Exactly one of `base64_content` or `blob_url` must be provided.
    pub fn try_build(self) -> FoundryResult<AnalyzeImageRequest> {
        let content = self.base64_content.filter(|s| !s.trim().is_empty());
        let blob_url = self.blob_url.filter(|s| !s.trim().is_empty());

        match (&content, &blob_url) {
            (None, None) => {
                return Err(FoundryError::Builder(
                    "either base64_content or blob_url is required".into(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(FoundryError::Builder(
                    "only one of base64_content or blob_url can be set, not both".into(),
                ));
            }
            _ => {}
        }

        Ok(AnalyzeImageRequest {
            image: ImageSource { content, blob_url },
            categories: self.categories,
            output_type: self.output_type,
        })
    }

    /// Builds the request, panicking if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing or invalid. Use [`try_build`](Self::try_build)
    /// for a fallible alternative.
    pub fn build(self) -> AnalyzeImageRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from the image content analysis endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeImageResponse {
    /// Analysis results per harm category.
    #[serde(rename = "categoriesAnalysis")]
    pub categories_analysis: Vec<CategoryAnalysis>,
}

// ---------------------------------------------------------------------------
// API function
// ---------------------------------------------------------------------------

/// Analyze an image for harmful content.
///
/// Sends the image to the Azure Content Safety API and returns severity scores
/// for each harm category.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `request` - The analysis request built via [`AnalyzeImageRequest::builder`].
///
/// # Example
///
/// ```rust,no_run
/// use azure_ai_foundry_core::client::FoundryClient;
/// use azure_ai_foundry_core::auth::FoundryCredential;
/// use azure_ai_foundry_safety::image::{self, AnalyzeImageRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = FoundryClient::builder()
///     .endpoint("https://your-resource.cognitiveservices.azure.com")
///     .credential(FoundryCredential::api_key("your-key"))
///     .build()?;
///
/// let request = AnalyzeImageRequest::builder()
///     .blob_url("https://myblobstore.blob.core.windows.net/images/photo.jpg")
///     .try_build()?;
///
/// let response = image::analyze_image(&client, &request).await?;
/// for analysis in &response.categories_analysis {
///     println!("{}: severity {}", analysis.category, analysis.severity);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if authentication fails, the request fails, or the API
/// returns an error response.
#[tracing::instrument(name = "foundry::safety::analyze_image", skip(client, request))]
pub async fn analyze_image(
    client: &FoundryClient,
    request: &AnalyzeImageRequest,
) -> FoundryResult<AnalyzeImageResponse> {
    tracing::debug!("analyzing image for harmful content");

    let path = format!("/contentsafety/image:analyze?{CONTENT_SAFETY_API_VERSION}");
    let response = client.post(&path, request).await?;
    let result = response.json::<AnalyzeImageResponse>().await?;

    tracing::debug!("image analysis complete");
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
    fn test_analyze_image_requires_image_source() {
        let result = AnalyzeImageRequest::builder().try_build();
        let err = result.expect_err("should require image source");
        assert!(
            err.to_string().contains("base64_content") || err.to_string().contains("blob_url"),
            "error: {err}"
        );
    }

    #[test]
    fn test_analyze_image_rejects_empty_url() {
        let result = AnalyzeImageRequest::builder().blob_url("  ").try_build();
        let err = result.expect_err("should reject blank url");
        assert!(err.to_string().contains("required"), "error: {err}");
    }

    #[test]
    fn test_analyze_image_rejects_empty_base64() {
        let result = AnalyzeImageRequest::builder()
            .base64_content("  ")
            .try_build();
        let err = result.expect_err("should reject blank base64");
        assert!(err.to_string().contains("required"), "error: {err}");
    }

    #[test]
    fn test_analyze_image_rejects_both_sources() {
        let result = AnalyzeImageRequest::builder()
            .base64_content("abc123")
            .blob_url("https://blob.example.com/image.jpg")
            .try_build();
        let err = result.expect_err("should reject both sources");
        assert!(err.to_string().contains("not both"), "error: {err}");
    }

    #[test]
    fn test_analyze_image_accepts_url_source() {
        let result = AnalyzeImageRequest::builder()
            .blob_url("https://blob.example.com/image.jpg")
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_image_accepts_base64_source() {
        let result = AnalyzeImageRequest::builder()
            .base64_content("aGVsbG8=")
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_image_serializes_blob_url() {
        let request = AnalyzeImageRequest::builder()
            .blob_url("https://blob.example.com/image.jpg")
            .build();

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(
            json["image"]["blobUrl"],
            "https://blob.example.com/image.jpg"
        );
        assert!(json["image"].get("content").is_none());
    }

    #[test]
    fn test_analyze_image_serializes_base64_content() {
        let request = AnalyzeImageRequest::builder()
            .base64_content("aGVsbG8=")
            .categories(vec![HarmCategory::Violence])
            .build();

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["image"]["content"], "aGVsbG8=");
        assert!(json["image"].get("blobUrl").is_none());
        assert_eq!(json["categories"], serde_json::json!(["Violence"]));
    }

    // -- Response deserialization --

    #[test]
    fn test_analyze_image_response_deserialization() {
        let json = r#"{
            "categoriesAnalysis": [
                {"category": "Hate", "severity": 0},
                {"category": "Violence", "severity": 4}
            ]
        }"#;
        let response: AnalyzeImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.categories_analysis.len(), 2);
        assert_eq!(response.categories_analysis[1].severity, 4);
    }

    // -- API function --

    #[tokio::test]
    async fn test_analyze_image_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/image:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "categoriesAnalysis": [
                    {"category": "Hate", "severity": 0},
                    {"category": "Sexual", "severity": 0},
                    {"category": "Violence", "severity": 6}
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = AnalyzeImageRequest::builder()
            .blob_url("https://blob.example.com/image.jpg")
            .build();

        let result = analyze_image(&client, &request)
            .await
            .expect("should succeed");
        assert_eq!(result.categories_analysis.len(), 3);
        assert_eq!(result.categories_analysis[2].severity, 6);
    }

    #[tokio::test]
    async fn test_analyze_image_api_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/image:analyze"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": "InvalidImage",
                    "message": "Image is too small"
                }
            })))
            .mount(&server)
            .await;

        let request = AnalyzeImageRequest::builder()
            .base64_content("aGVsbG8=")
            .build();

        let err = analyze_image(&client, &request)
            .await
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("InvalidImage") || msg.contains("Image is too small"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_image_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/image:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "categoriesAnalysis": []
            })))
            .mount(&server)
            .await;

        let request = AnalyzeImageRequest::builder()
            .blob_url("https://blob.example.com/img.jpg")
            .build();

        let _ = analyze_image(&client, &request).await;
        assert!(logs_contain("foundry::safety::analyze_image"));
    }
}
