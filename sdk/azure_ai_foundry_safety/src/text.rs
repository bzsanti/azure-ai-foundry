//! Text content analysis for Azure AI Content Safety.
//!
//! Analyzes text for harmful content across four categories: hate, self-harm,
//! sexual, and violence. Supports custom blocklists and configurable severity levels.

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::{CategoryAnalysis, HarmCategory, OutputType, CONTENT_SAFETY_API_VERSION};

/// Maximum text length allowed by the Content Safety API (Unicode code points).
const MAX_TEXT_LENGTH: usize = 10_000;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for the text content analysis endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeTextRequest {
    text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    categories: Option<Vec<HarmCategory>>,

    #[serde(rename = "blocklistNames", skip_serializing_if = "Option::is_none")]
    blocklist_names: Option<Vec<String>>,

    #[serde(rename = "haltOnBlocklistHit", skip_serializing_if = "Option::is_none")]
    halt_on_blocklist_hit: Option<bool>,

    #[serde(rename = "outputType", skip_serializing_if = "Option::is_none")]
    output_type: Option<OutputType>,
}

impl AnalyzeTextRequest {
    /// Creates a new builder for `AnalyzeTextRequest`.
    pub fn builder() -> AnalyzeTextRequestBuilder {
        AnalyzeTextRequestBuilder::default()
    }

    /// Returns the text being analyzed.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Builder for [`AnalyzeTextRequest`].
#[derive(Debug, Default)]
pub struct AnalyzeTextRequestBuilder {
    text: Option<String>,
    categories: Option<Vec<HarmCategory>>,
    blocklist_names: Option<Vec<String>>,
    halt_on_blocklist_hit: Option<bool>,
    output_type: Option<OutputType>,
}

impl AnalyzeTextRequestBuilder {
    /// Sets the text to analyze (required, max 10,000 characters).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Sets the harm categories to analyze. If not set, all categories are analyzed.
    pub fn categories(mut self, categories: Vec<HarmCategory>) -> Self {
        self.categories = Some(categories);
        self
    }

    /// Sets blocklist names to check against.
    pub fn blocklist_names(mut self, names: Vec<String>) -> Self {
        self.blocklist_names = Some(names);
        self
    }

    /// If `true`, stops analysis when a blocklist match is found.
    pub fn halt_on_blocklist_hit(mut self, halt: bool) -> Self {
        self.halt_on_blocklist_hit = Some(halt);
        self
    }

    /// Sets the output type (four or eight severity levels).
    pub fn output_type(mut self, output_type: OutputType) -> Self {
        self.output_type = Some(output_type);
        self
    }

    /// Builds the request, returning an error if validation fails.
    pub fn try_build(self) -> FoundryResult<AnalyzeTextRequest> {
        let text = self
            .text
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| FoundryError::Builder("text is required".into()))?;

        if text.chars().count() > MAX_TEXT_LENGTH {
            return Err(FoundryError::Builder(format!(
                "text exceeds maximum length of {MAX_TEXT_LENGTH} characters"
            )));
        }

        Ok(AnalyzeTextRequest {
            text,
            categories: self.categories,
            blocklist_names: self.blocklist_names,
            halt_on_blocklist_hit: self.halt_on_blocklist_hit,
            output_type: self.output_type,
        })
    }

    /// Builds the request, panicking if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing or invalid. Use [`try_build`](Self::try_build)
    /// for a fallible alternative.
    pub fn build(self) -> AnalyzeTextRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from the text content analysis endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeTextResponse {
    /// Analysis results per harm category.
    #[serde(rename = "categoriesAnalysis")]
    pub categories_analysis: Vec<CategoryAnalysis>,

    /// Blocklist matches, if blocklists were specified in the request.
    #[serde(rename = "blocklistsMatch", default)]
    pub blocklists_match: Option<Vec<BlocklistMatch>>,
}

/// A match against a blocklist item.
#[derive(Debug, Clone, Deserialize)]
pub struct BlocklistMatch {
    /// The name of the blocklist that matched.
    #[serde(rename = "blocklistName")]
    pub blocklist_name: String,

    /// The ID of the blocklist item that matched.
    #[serde(rename = "blocklistItemId")]
    pub blocklist_item_id: String,

    /// The text of the blocklist item that matched.
    #[serde(rename = "blocklistItemText")]
    pub blocklist_item_text: String,
}

// ---------------------------------------------------------------------------
// API function
// ---------------------------------------------------------------------------

/// Analyze text for harmful content.
///
/// Sends the text to the Azure Content Safety API and returns severity scores
/// for each harm category, plus any blocklist matches.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `request` - The analysis request built via [`AnalyzeTextRequest::builder`].
///
/// # Example
///
/// ```rust,no_run
/// use azure_ai_foundry_core::client::FoundryClient;
/// use azure_ai_foundry_core::auth::FoundryCredential;
/// use azure_ai_foundry_safety::text::{self, AnalyzeTextRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = FoundryClient::builder()
///     .endpoint("https://your-resource.cognitiveservices.azure.com")
///     .credential(FoundryCredential::api_key("your-key"))
///     .build()?;
///
/// let request = AnalyzeTextRequest::builder()
///     .text("Content to analyze")
///     .try_build()?;
///
/// let response = text::analyze_text(&client, &request).await?;
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
#[tracing::instrument(
    name = "foundry::safety::analyze_text",
    skip(client, request),
    fields(text_len = request.text.len())
)]
pub async fn analyze_text(
    client: &FoundryClient,
    request: &AnalyzeTextRequest,
) -> FoundryResult<AnalyzeTextResponse> {
    tracing::debug!("analyzing text for harmful content");

    let path = format!("/contentsafety/text:analyze?{CONTENT_SAFETY_API_VERSION}");
    let response = client.post(&path, request).await?;
    let result = response.json::<AnalyzeTextResponse>().await?;

    tracing::debug!("text analysis complete");
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
    fn test_analyze_text_requires_text() {
        let result = AnalyzeTextRequest::builder().try_build();
        let err = result.expect_err("should require text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_analyze_text_rejects_blank_text() {
        let result = AnalyzeTextRequest::builder().text("   ").try_build();
        let err = result.expect_err("should reject blank text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_analyze_text_rejects_text_too_long() {
        let long_text = "a".repeat(MAX_TEXT_LENGTH + 1);
        let result = AnalyzeTextRequest::builder().text(long_text).try_build();
        let err = result.expect_err("should reject text over 10000 chars");
        assert!(err.to_string().contains("maximum length"), "error: {err}");
    }

    #[test]
    fn test_analyze_text_accepts_boundary_length() {
        let boundary_text = "a".repeat(MAX_TEXT_LENGTH);
        let result = AnalyzeTextRequest::builder()
            .text(boundary_text)
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_text_optional_fields_default_absent() {
        let request = AnalyzeTextRequest::builder().text("test content").build();

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["text"], "test content");
        assert!(json.get("categories").is_none());
        assert!(json.get("blocklistNames").is_none());
        assert!(json.get("haltOnBlocklistHit").is_none());
        assert!(json.get("outputType").is_none());
    }

    #[test]
    fn test_analyze_text_serializes_all_fields() {
        let request = AnalyzeTextRequest::builder()
            .text("test")
            .categories(vec![HarmCategory::Hate, HarmCategory::Violence])
            .blocklist_names(vec!["profanity".into()])
            .halt_on_blocklist_hit(true)
            .output_type(OutputType::EightSeverityLevels)
            .build();

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["text"], "test");
        assert_eq!(json["categories"], serde_json::json!(["Hate", "Violence"]));
        assert_eq!(json["blocklistNames"], serde_json::json!(["profanity"]));
        assert_eq!(json["haltOnBlocklistHit"], true);
        assert_eq!(json["outputType"], "EightSeverityLevels");
    }

    // -- Response deserialization --

    #[test]
    fn test_analyze_text_response_deserializes_minimal() {
        let json = r#"{
            "categoriesAnalysis": [
                {"category": "Hate", "severity": 0},
                {"category": "Violence", "severity": 2}
            ]
        }"#;
        let response: AnalyzeTextResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.categories_analysis.len(), 2);
        assert_eq!(response.categories_analysis[0].category, HarmCategory::Hate);
        assert_eq!(response.categories_analysis[0].severity, 0);
        assert_eq!(response.categories_analysis[1].severity, 2);
        assert!(response.blocklists_match.is_none());
    }

    #[test]
    fn test_analyze_text_response_deserializes_with_blocklists() {
        let json = r#"{
            "categoriesAnalysis": [{"category": "Hate", "severity": 4}],
            "blocklistsMatch": [{
                "blocklistName": "profanity",
                "blocklistItemId": "item-123",
                "blocklistItemText": "bad word"
            }]
        }"#;
        let response: AnalyzeTextResponse = serde_json::from_str(json).unwrap();
        let matches = response.blocklists_match.expect("should have matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].blocklist_name, "profanity");
        assert_eq!(matches[0].blocklist_item_id, "item-123");
        assert_eq!(matches[0].blocklist_item_text, "bad word");
    }

    // -- API function --

    #[tokio::test]
    async fn test_analyze_text_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "categoriesAnalysis": [
                    {"category": "Hate", "severity": 0},
                    {"category": "SelfHarm", "severity": 0},
                    {"category": "Sexual", "severity": 0},
                    {"category": "Violence", "severity": 2}
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = AnalyzeTextRequest::builder()
            .text("some content to analyze")
            .build();

        let result = analyze_text(&client, &request)
            .await
            .expect("should succeed");
        assert_eq!(result.categories_analysis.len(), 4);
        assert_eq!(
            result.categories_analysis[3].category,
            HarmCategory::Violence
        );
        assert_eq!(result.categories_analysis[3].severity, 2);
    }

    #[tokio::test]
    async fn test_analyze_text_api_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:analyze"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": "InvalidRequest",
                    "message": "Text is too long"
                }
            })))
            .mount(&server)
            .await;

        let request = AnalyzeTextRequest::builder().text("test content").build();

        let err = analyze_text(&client, &request)
            .await
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("InvalidRequest") || msg.contains("Text is too long"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_text_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path("/contentsafety/text:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "categoriesAnalysis": []
            })))
            .mount(&server)
            .await;

        let request = AnalyzeTextRequest::builder().text("test").build();

        let _ = analyze_text(&client, &request).await;
        assert!(logs_contain("foundry::safety::analyze_text"));
    }
}
