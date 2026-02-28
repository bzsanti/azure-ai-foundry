//! Document Intelligence client for Azure AI Foundry Document Intelligence API v4.0.
//!
//! This module provides functions to analyze documents using the Azure Document
//! Intelligence API, supporting prebuilt models for reading, layout analysis,
//! invoices, receipts, ID documents, and business cards.
//!
//! The Document Intelligence API uses an asynchronous pattern: a submit request
//! returns `202 Accepted` with an `Operation-Location` header, and the client
//! polls that URL until the analysis completes.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_tools::document_intelligence::{self, DocumentAnalysisRequest, PREBUILT_READ};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! let request = DocumentAnalysisRequest::builder()
//!     .model_id(PREBUILT_READ)
//!     .url_source("https://example.com/document.pdf")
//!     .build()?;
//!
//! let operation = document_intelligence::analyze(&client, &request).await?;
//! let result = document_intelligence::poll_until_complete(
//!     &client,
//!     &operation.operation_location,
//!     std::time::Duration::from_secs(2),
//!     60,
//! ).await?;
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::models::DOCUMENT_INTELLIGENCE_API_VERSION;

// ---------------------------------------------------------------------------
// Prebuilt model ID constants
// ---------------------------------------------------------------------------

/// Prebuilt model for general text extraction (OCR).
pub const PREBUILT_READ: &str = "prebuilt-read";

/// Prebuilt model for document layout analysis (tables, figures, sections).
pub const PREBUILT_LAYOUT: &str = "prebuilt-layout";

/// Prebuilt model for invoice data extraction.
pub const PREBUILT_INVOICE: &str = "prebuilt-invoice";

/// Prebuilt model for receipt data extraction.
pub const PREBUILT_RECEIPT: &str = "prebuilt-receipt";

/// Prebuilt model for ID document data extraction (passports, driver licenses).
pub const PREBUILT_ID_DOCUMENT: &str = "prebuilt-idDocument";

/// Prebuilt model for business card data extraction.
pub const PREBUILT_BUSINESS_CARD: &str = "prebuilt-businessCard";

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// An optional analysis feature to enable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DocumentAnalysisFeature {
    /// High-resolution OCR for small or dense text.
    #[serde(rename = "ocrHighResolution")]
    OcrHighResolution,
    /// Detect document languages.
    #[serde(rename = "languages")]
    Languages,
    /// Detect barcodes and QR codes.
    #[serde(rename = "barcodes")]
    Barcodes,
    /// Detect mathematical formulas.
    #[serde(rename = "formulas")]
    Formulas,
    /// Extract key-value pairs.
    #[serde(rename = "keyValuePairs")]
    KeyValuePairs,
    /// Detect font styles.
    #[serde(rename = "styleFont")]
    StyleFont,
    /// Enable query fields extraction.
    #[serde(rename = "queryFields")]
    QueryFields,
}

impl DocumentAnalysisFeature {
    /// Returns the API string representation of this feature.
    fn as_str(&self) -> &'static str {
        match self {
            Self::OcrHighResolution => "ocrHighResolution",
            Self::Languages => "languages",
            Self::Barcodes => "barcodes",
            Self::Formulas => "formulas",
            Self::KeyValuePairs => "keyValuePairs",
            Self::StyleFont => "styleFont",
            Self::QueryFields => "queryFields",
        }
    }
}

/// A request to analyze a document.
///
/// Use the builder pattern to construct requests:
///
/// ```rust
/// use azure_ai_foundry_tools::document_intelligence::{DocumentAnalysisRequest, PREBUILT_READ};
///
/// let request = DocumentAnalysisRequest::builder()
///     .model_id(PREBUILT_READ)
///     .url_source("https://example.com/document.pdf")
///     .build()
///     .expect("valid request");
/// ```
#[derive(Debug, Clone)]
pub struct DocumentAnalysisRequest {
    /// The model ID to use for analysis.
    pub model_id: String,

    /// URL of the document to analyze (mutually exclusive with `base64_source`).
    url_source: Option<String>,

    /// Base64-encoded document content (mutually exclusive with `url_source`).
    base64_source: Option<String>,

    /// Page ranges to analyze (e.g., "1-3,5").
    pages: Option<String>,

    /// Document locale (e.g., "en-US").
    locale: Option<String>,

    /// Optional analysis features to enable.
    features: Option<Vec<DocumentAnalysisFeature>>,
}

/// The JSON body sent to the Document Intelligence analyze endpoint.
#[derive(Debug, Serialize)]
struct DocumentAnalysisBody {
    #[serde(rename = "urlSource", skip_serializing_if = "Option::is_none")]
    url_source: Option<String>,

    #[serde(rename = "base64Source", skip_serializing_if = "Option::is_none")]
    base64_source: Option<String>,
}

impl DocumentAnalysisRequest {
    /// Creates a new builder for a document analysis request.
    pub fn builder() -> DocumentAnalysisRequestBuilder {
        DocumentAnalysisRequestBuilder::default()
    }

    /// Returns the JSON body for the API request.
    fn body(&self) -> DocumentAnalysisBody {
        DocumentAnalysisBody {
            url_source: self.url_source.clone(),
            base64_source: self.base64_source.clone(),
        }
    }

    /// Builds the query string for the API request.
    pub(crate) fn query_string(&self) -> String {
        let mut params = DOCUMENT_INTELLIGENCE_API_VERSION.to_string();

        if let Some(ref pages) = self.pages {
            params.push_str(&format!("&pages={pages}"));
        }
        if let Some(ref locale) = self.locale {
            params.push_str(&format!("&locale={locale}"));
        }
        if let Some(ref features) = self.features {
            if !features.is_empty() {
                let features_str: Vec<&str> = features.iter().map(|f| f.as_str()).collect();
                params.push_str(&format!("&features={}", features_str.join(",")));
            }
        }

        params
    }
}

/// Builder for [`DocumentAnalysisRequest`].
#[derive(Debug, Default)]
pub struct DocumentAnalysisRequestBuilder {
    model_id: Option<String>,
    url_source: Option<String>,
    base64_source: Option<String>,
    pages: Option<String>,
    locale: Option<String>,
    features: Option<Vec<DocumentAnalysisFeature>>,
}

impl DocumentAnalysisRequestBuilder {
    /// Sets the model ID to use for analysis (required).
    pub fn model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Sets the URL of the document to analyze.
    ///
    /// Mutually exclusive with [`base64_source`](Self::base64_source).
    pub fn url_source(mut self, url: impl Into<String>) -> Self {
        self.url_source = Some(url.into());
        self
    }

    /// Sets the base64-encoded document content.
    ///
    /// Mutually exclusive with [`url_source`](Self::url_source).
    pub fn base64_source(mut self, data: impl Into<String>) -> Self {
        self.base64_source = Some(data.into());
        self
    }

    /// Sets the page ranges to analyze (e.g., "1-3,5").
    pub fn pages(mut self, pages: impl Into<String>) -> Self {
        self.pages = Some(pages.into());
        self
    }

    /// Sets the document locale.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Sets optional analysis features.
    pub fn features(mut self, features: Vec<DocumentAnalysisFeature>) -> Self {
        self.features = Some(features);
        self
    }

    /// Builds the request, validating all required fields.
    ///
    /// # Errors
    ///
    /// Returns [`FoundryError::Builder`] if:
    /// - `model_id` is missing or empty
    /// - Neither `url_source` nor `base64_source` is set
    /// - Both `url_source` and `base64_source` are set
    pub fn build(self) -> FoundryResult<DocumentAnalysisRequest> {
        let model_id = self
            .model_id
            .filter(|m| !m.is_empty())
            .ok_or_else(|| FoundryError::Builder("model_id is required".into()))?;

        let url_source = self.url_source.filter(|s| !s.is_empty());
        let base64_source = self.base64_source.filter(|s| !s.is_empty());
        let has_url = url_source.is_some();
        let has_base64 = base64_source.is_some();

        if !has_url && !has_base64 {
            return Err(FoundryError::Builder(
                "source is required: set url_source or base64_source".into(),
            ));
        }

        if has_url && has_base64 {
            return Err(FoundryError::Builder(
                "only one source allowed: set url_source or base64_source, not both".into(),
            ));
        }

        Ok(DocumentAnalysisRequest {
            model_id,
            url_source,
            base64_source,
            pages: self.pages,
            locale: self.locale,
            features: self.features,
        })
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// The status of an asynchronous analyze operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalyzeResultStatus {
    /// The operation has not started.
    NotStarted,
    /// The operation is in progress.
    Running,
    /// The operation completed successfully.
    Succeeded,
    /// The operation failed.
    Failed,
}

impl AnalyzeResultStatus {
    /// Returns `true` if the status is terminal (succeeded or failed).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

impl std::fmt::Display for AnalyzeResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NotStarted => "notStarted",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        };
        f.write_str(s)
    }
}

/// An error returned by the Document Intelligence API when an operation fails.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeOperationError {
    /// The error code.
    pub code: String,
    /// Human-readable error description.
    pub message: String,
}

/// The result returned when polling an analyze operation.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeOperationResult {
    /// Current status of the operation.
    pub status: AnalyzeResultStatus,

    /// Error details, present when status is `Failed`.
    pub error: Option<AnalyzeOperationError>,

    /// The analysis result, present when status is `Succeeded`.
    #[serde(rename = "analyzeResult")]
    pub analyze_result: Option<AnalyzeResult>,
}

/// The full result of a document analysis.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeResult {
    /// API version used for analysis.
    #[serde(rename = "apiVersion")]
    pub api_version: String,

    /// Model ID used for analysis.
    #[serde(rename = "modelId")]
    pub model_id: String,

    /// Full text content extracted from the document.
    pub content: Option<String>,

    /// Pages in the document.
    pub pages: Option<Vec<DocumentPage>>,

    /// Tables found in the document.
    pub tables: Option<Vec<DocumentTable>>,

    /// Key-value pairs extracted from the document.
    #[serde(rename = "keyValuePairs")]
    pub key_value_pairs: Option<Vec<DocumentKeyValuePair>>,

    /// Structured documents extracted.
    pub documents: Option<Vec<DocumentTypeResult>>,
}

/// A page in the analyzed document.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentPage {
    /// 1-based page number.
    #[serde(rename = "pageNumber")]
    pub page_number: u32,

    /// Rotation angle in degrees.
    pub angle: Option<f64>,

    /// Page width in the unit specified by `unit`.
    pub width: Option<f64>,

    /// Page height in the unit specified by `unit`.
    pub height: Option<f64>,

    /// Unit of measurement (e.g., "inch", "pixel").
    pub unit: Option<String>,

    /// Words detected on the page.
    pub words: Option<Vec<DocumentWord>>,

    /// Lines detected on the page.
    pub lines: Option<Vec<DocumentLine>>,
}

/// A word detected in a document.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentWord {
    /// The word text.
    pub content: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// A line of text detected in a document.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentLine {
    /// The line text.
    pub content: String,
}

/// A table detected in a document.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentTable {
    /// Number of rows.
    #[serde(rename = "rowCount")]
    pub row_count: u32,
    /// Number of columns.
    #[serde(rename = "columnCount")]
    pub column_count: u32,
    /// Table cells.
    pub cells: Vec<DocumentTableCell>,
}

/// A cell in a document table.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentTableCell {
    /// 0-based row index.
    #[serde(rename = "rowIndex")]
    pub row_index: u32,
    /// 0-based column index.
    #[serde(rename = "columnIndex")]
    pub column_index: u32,
    /// Cell text content.
    pub content: String,
}

/// A key-value pair extracted from a document.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentKeyValuePair {
    /// The key element.
    pub key: Option<DocumentKeyValueElement>,
    /// The value element.
    pub value: Option<DocumentKeyValueElement>,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// A key or value element in a key-value pair.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentKeyValueElement {
    /// Text content.
    pub content: Option<String>,
}

/// A structured document result.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentTypeResult {
    /// Document type (e.g., "invoice", "receipt").
    #[serde(rename = "docType")]
    pub doc_type: String,

    /// Extracted fields as a JSON value (schema varies by document type).
    pub fields: Option<serde_json::Value>,

    /// Confidence score (0.0 to 1.0).
    pub confidence: Option<f64>,
}

// ---------------------------------------------------------------------------
// Operation status
// ---------------------------------------------------------------------------

/// The result of submitting a document for analysis.
///
/// Contains the `Operation-Location` URL to poll for results.
#[derive(Debug, Clone)]
pub struct OperationStatus {
    /// The URL to poll for the analysis result.
    pub operation_location: String,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Submit a document for analysis.
///
/// Returns an [`OperationStatus`] with the `Operation-Location` URL to poll.
/// The API returns `202 Accepted` on success.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_tools::document_intelligence::{self, DocumentAnalysisRequest, PREBUILT_READ};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = DocumentAnalysisRequest::builder()
///     .model_id(PREBUILT_READ)
///     .url_source("https://example.com/doc.pdf")
///     .build()?;
///
/// let operation = document_intelligence::analyze(client, &request).await?;
/// println!("Poll at: {}", operation.operation_location);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::document_intelligence::analyze` with field `model_id`.
#[tracing::instrument(
    name = "foundry::document_intelligence::analyze",
    skip(client, request),
    fields(model_id = %request.model_id)
)]
pub async fn analyze(
    client: &FoundryClient,
    request: &DocumentAnalysisRequest,
) -> FoundryResult<OperationStatus> {
    tracing::debug!("submitting document for analysis");

    let path = format!(
        "/documentintelligence/documentModels/{}:analyze?{}",
        request.model_id,
        request.query_string(),
    );

    let body = request.body();
    let response = client.post(&path, &body).await?;

    let operation_location = response
        .headers()
        .get("Operation-Location")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| FoundryError::Api {
            code: "MissingHeader".into(),
            message: "Operation-Location header missing from response".into(),
        })?;

    tracing::debug!(operation_location = %operation_location, "document analysis submitted");

    Ok(OperationStatus { operation_location })
}

/// Get the current result of an analyze operation.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_tools::document_intelligence::{self, DocumentAnalysisRequest, PREBUILT_READ};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = DocumentAnalysisRequest::builder()
///     .model_id(PREBUILT_READ)
///     .url_source("https://example.com/doc.pdf")
///     .build()?;
///
/// let operation = document_intelligence::analyze(client, &request).await?;
/// let result = document_intelligence::get_result(client, &operation.operation_location).await?;
/// println!("Status: {}", result.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::document_intelligence::get_result`.
#[tracing::instrument(
    name = "foundry::document_intelligence::get_result",
    skip(client),
    fields(operation_location = %operation_location)
)]
pub async fn get_result(
    client: &FoundryClient,
    operation_location: &str,
) -> FoundryResult<AnalyzeOperationResult> {
    tracing::debug!("fetching analyze result");

    // The Operation-Location is a full URL. Extract the path + query to use
    // with the client's relative path-based API.
    let parsed = url::Url::parse(operation_location).map_err(|e| {
        FoundryError::invalid_endpoint_with_source("failed to parse Operation-Location URL", e)
    })?;

    let relative_path = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };

    let response = client.get(&relative_path).await?;
    let result = response.json::<AnalyzeOperationResult>().await?;

    tracing::debug!(status = ?result.status, "analyze result fetched");
    Ok(result)
}

/// Poll an analyze operation until it reaches a terminal status.
///
/// Returns the final [`AnalyzeOperationResult`] when the status is `Succeeded`
/// or `Failed`. The caller should check the status to determine if the
/// analysis succeeded.
///
/// # Arguments
///
/// * `client` - The Foundry client.
/// * `operation_location` - The URL returned by [`analyze`].
/// * `poll_interval` - How often to check the status.
/// * `max_attempts` - Maximum number of poll attempts before returning an error.
///   Set to `0` to disable the limit (not recommended for production).
///
/// # Errors
///
/// Returns [`FoundryError::Api`] if `max_attempts` is exceeded before
/// the operation reaches a terminal status.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_tools::document_intelligence::{self, AnalyzeResultStatus};
/// # async fn example(client: &FoundryClient, operation_location: &str) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = document_intelligence::poll_until_complete(
///     client,
///     operation_location,
///     std::time::Duration::from_secs(2),
///     60,
/// ).await?;
///
/// match result.status {
///     AnalyzeResultStatus::Succeeded => println!("Analysis complete!"),
///     AnalyzeResultStatus::Failed => println!("Analysis failed"),
///     _ => unreachable!("poll_until_complete returns only terminal statuses"),
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::document_intelligence::poll_until_complete`.
#[tracing::instrument(
    name = "foundry::document_intelligence::poll_until_complete",
    skip(client),
    fields(operation_location = %operation_location)
)]
pub async fn poll_until_complete(
    client: &FoundryClient,
    operation_location: &str,
    poll_interval: Duration,
    max_attempts: u32,
) -> FoundryResult<AnalyzeOperationResult> {
    tracing::debug!("starting to poll for completion");

    let mut attempts = 0u32;

    loop {
        if max_attempts > 0 {
            attempts += 1;
            if attempts > max_attempts {
                return Err(FoundryError::Api {
                    code: "PollTimeout".into(),
                    message: format!(
                        "poll_until_complete timed out after {max_attempts} max_attempts"
                    ),
                });
            }
        }

        let result = get_result(client, operation_location).await?;

        if result.status.is_terminal() {
            tracing::debug!(status = ?result.status, "operation reached terminal status");
            return Ok(result);
        }

        tracing::trace!(
            status = ?result.status,
            attempt = attempts,
            "operation still in progress, waiting",
        );
        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_mock_client;
    use wiremock::matchers::{method, path as match_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -----------------------------------------------------------------------
    // Cycle 13: DocumentAnalysisRequest builder validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_doc_analysis_request_requires_model_id() {
        let result = DocumentAnalysisRequest::builder()
            .url_source("https://example.com/doc.pdf")
            .build();
        let err = result.expect_err("should require model_id");
        assert!(err.to_string().contains("model_id"), "error: {err}");
    }

    #[test]
    fn test_doc_analysis_request_rejects_empty_model_id() {
        let result = DocumentAnalysisRequest::builder()
            .model_id("")
            .url_source("https://example.com/doc.pdf")
            .build();
        let err = result.expect_err("should reject empty model_id");
        assert!(err.to_string().contains("model_id"), "error: {err}");
    }

    #[test]
    fn test_doc_analysis_request_requires_source() {
        let result = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .build();
        let err = result.expect_err("should require source");
        assert!(err.to_string().contains("source"), "error: {err}");
    }

    #[test]
    fn test_doc_analysis_request_rejects_both_sources() {
        let result = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .base64_source("aGVsbG8=")
            .build();
        let err = result.expect_err("should reject both sources");
        assert!(err.to_string().contains("only one"), "error: {err}");
    }

    #[test]
    fn test_doc_analysis_request_rejects_empty_url_source() {
        let result = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("")
            .build();
        let err = result.expect_err("empty url_source should be rejected");
        assert!(
            err.to_string().contains("source"),
            "error should mention source: {err}",
        );
    }

    #[test]
    fn test_doc_analysis_request_rejects_empty_base64_source() {
        let result = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .base64_source("")
            .build();
        let err = result.expect_err("empty base64_source should be rejected");
        assert!(
            err.to_string().contains("source"),
            "error should mention source: {err}",
        );
    }

    #[test]
    fn test_doc_analysis_request_accepts_url_source() {
        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("should accept url_source");
        assert_eq!(request.model_id, PREBUILT_READ);
    }

    #[test]
    fn test_doc_analysis_request_accepts_base64_source() {
        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .base64_source("aGVsbG8=")
            .build()
            .expect("should accept base64_source");
        assert_eq!(request.model_id, PREBUILT_READ);
    }

    // -----------------------------------------------------------------------
    // Cycle 14: Request body serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_doc_analysis_request_url_source_serialization() {
        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let body = request.body();
        let json = serde_json::to_value(&body).expect("should serialize");
        assert_eq!(json["urlSource"], "https://example.com/doc.pdf");
        assert!(json.get("base64Source").is_none());
    }

    #[test]
    fn test_doc_analysis_request_base64_source_serialization() {
        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .base64_source("aGVsbG8=")
            .build()
            .expect("valid request");

        let body = request.body();
        let json = serde_json::to_value(&body).expect("should serialize");
        assert_eq!(json["base64Source"], "aGVsbG8=");
        assert!(json.get("urlSource").is_none());
    }

    #[test]
    fn test_doc_analysis_request_query_string() {
        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .pages("1-3")
            .locale("en-US")
            .features(vec![DocumentAnalysisFeature::OcrHighResolution])
            .build()
            .expect("valid request");

        let qs = request.query_string();
        assert!(qs.contains("api-version=2024-11-30"), "qs: {qs}");
        assert!(qs.contains("pages=1-3"), "qs: {qs}");
        assert!(qs.contains("locale=en-US"), "qs: {qs}");
        assert!(qs.contains("features=ocrHighResolution"), "qs: {qs}");
    }

    // -----------------------------------------------------------------------
    // Cycle 15: AnalyzeResultStatus and AnalyzeOperationResult deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_result_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<AnalyzeResultStatus>(r#""notStarted""#).unwrap(),
            AnalyzeResultStatus::NotStarted,
        );
        assert_eq!(
            serde_json::from_str::<AnalyzeResultStatus>(r#""running""#).unwrap(),
            AnalyzeResultStatus::Running,
        );
        assert_eq!(
            serde_json::from_str::<AnalyzeResultStatus>(r#""succeeded""#).unwrap(),
            AnalyzeResultStatus::Succeeded,
        );
        assert_eq!(
            serde_json::from_str::<AnalyzeResultStatus>(r#""failed""#).unwrap(),
            AnalyzeResultStatus::Failed,
        );
    }

    #[test]
    fn test_analyze_result_status_is_terminal() {
        assert!(!AnalyzeResultStatus::NotStarted.is_terminal());
        assert!(!AnalyzeResultStatus::Running.is_terminal());
        assert!(AnalyzeResultStatus::Succeeded.is_terminal());
        assert!(AnalyzeResultStatus::Failed.is_terminal());
    }

    #[test]
    fn test_analyze_result_status_display() {
        assert_eq!(AnalyzeResultStatus::NotStarted.to_string(), "notStarted");
        assert_eq!(AnalyzeResultStatus::Running.to_string(), "running");
        assert_eq!(AnalyzeResultStatus::Succeeded.to_string(), "succeeded");
        assert_eq!(AnalyzeResultStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_analyze_operation_result_deserialization_succeeded() {
        let json = r#"{
            "status": "succeeded",
            "analyzeResult": {
                "apiVersion": "2024-11-30",
                "modelId": "prebuilt-read",
                "content": "Hello world",
                "pages": [{"pageNumber": 1, "words": [{"content": "Hello", "confidence": 0.99}]}]
            }
        }"#;

        let result: AnalyzeOperationResult =
            serde_json::from_str(json).expect("should deserialize");
        assert_eq!(result.status, AnalyzeResultStatus::Succeeded);
        let ar = result.analyze_result.expect("should have analyze_result");
        assert_eq!(ar.api_version, "2024-11-30");
        assert_eq!(ar.model_id, "prebuilt-read");
        assert_eq!(ar.content.as_deref(), Some("Hello world"));
        let pages = ar.pages.expect("should have pages");
        assert_eq!(pages[0].page_number, 1);
        let words = pages[0].words.as_ref().expect("should have words");
        assert_eq!(words[0].content, "Hello");
    }

    #[test]
    fn test_analyze_operation_result_failed_with_error_details() {
        let json = r#"{
            "status": "failed",
            "error": {
                "code": "InvalidRequest",
                "message": "The document format is not supported."
            }
        }"#;
        let result: AnalyzeOperationResult =
            serde_json::from_str(json).expect("should deserialize");
        assert_eq!(result.status, AnalyzeResultStatus::Failed);

        let err = result.error.expect("should have error details");
        assert_eq!(err.code, "InvalidRequest");
        assert!(err.message.contains("not supported"));
    }

    #[test]
    fn test_analyze_operation_result_deserialization_running() {
        let json = r#"{"status": "running"}"#;
        let result: AnalyzeOperationResult =
            serde_json::from_str(json).expect("should deserialize");
        assert_eq!(result.status, AnalyzeResultStatus::Running);
        assert!(result.analyze_result.is_none());
    }

    #[test]
    fn test_analyze_operation_result_with_tables() {
        let json = r#"{
            "status": "succeeded",
            "analyzeResult": {
                "apiVersion": "2024-11-30",
                "modelId": "prebuilt-layout",
                "tables": [{
                    "rowCount": 2,
                    "columnCount": 3,
                    "cells": [
                        {"rowIndex": 0, "columnIndex": 0, "content": "Header 1"},
                        {"rowIndex": 0, "columnIndex": 1, "content": "Header 2"},
                        {"rowIndex": 0, "columnIndex": 2, "content": "Header 3"}
                    ]
                }]
            }
        }"#;

        let result: AnalyzeOperationResult =
            serde_json::from_str(json).expect("should deserialize");
        let ar = result.analyze_result.expect("should have result");
        let tables = ar.tables.expect("should have tables");
        assert_eq!(tables[0].row_count, 2);
        assert_eq!(tables[0].column_count, 3);
        assert_eq!(tables[0].cells[0].content, "Header 1");
    }

    #[test]
    fn test_analyze_operation_result_with_key_value_pairs() {
        let json = r#"{
            "status": "succeeded",
            "analyzeResult": {
                "apiVersion": "2024-11-30",
                "modelId": "prebuilt-invoice",
                "keyValuePairs": [{
                    "key": {"content": "Invoice Number"},
                    "value": {"content": "INV-001"},
                    "confidence": 0.95
                }]
            }
        }"#;

        let result: AnalyzeOperationResult =
            serde_json::from_str(json).expect("should deserialize");
        let ar = result.analyze_result.expect("should have result");
        let kvps = ar.key_value_pairs.expect("should have key-value pairs");
        assert_eq!(
            kvps[0].key.as_ref().and_then(|k| k.content.as_deref()),
            Some("Invoice Number"),
        );
        assert_eq!(
            kvps[0].value.as_ref().and_then(|v| v.content.as_deref()),
            Some("INV-001"),
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 16: analyze submit success path
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_analyze_document_submit_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/result-id-123",
            server.uri(),
        );

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(
                ResponseTemplate::new(202)
                    .append_header("Operation-Location", op_location.as_str()),
            )
            .expect(1)
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let operation = analyze(&client, &request).await.expect("should succeed");
        assert!(
            operation.operation_location.contains("result-id-123"),
            "got: {}",
            operation.operation_location,
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 17: Missing Operation-Location header
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_analyze_document_missing_operation_location() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(ResponseTemplate::new(202))
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let err = analyze(&client, &request)
            .await
            .expect_err("should fail without Operation-Location");
        assert!(
            err.to_string().contains("Operation-Location"),
            "error: {err}",
        );
    }

    #[tokio::test]
    async fn test_analyze_document_missing_operation_location_returns_api_error() {
        use azure_ai_foundry_core::error::FoundryError;

        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(ResponseTemplate::new(202)) // no Operation-Location header
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let err = analyze(&client, &request)
            .await
            .expect_err("should fail without Operation-Location");

        // Must be Api variant, NOT MissingConfig
        assert!(
            matches!(err, FoundryError::Api { .. }),
            "expected FoundryError::Api, got: {err:?}",
        );
        assert!(
            err.to_string().contains("Operation-Location"),
            "error: {err}",
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 18: get_result polling function
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_analyze_result_succeeded() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/result-id-123",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "succeeded",
                "analyzeResult": {
                    "apiVersion": "2024-11-30",
                    "modelId": "prebuilt-read",
                    "content": "Hello world"
                }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/result-id-123",
            server.uri(),
        );

        let result = get_result(&client, &op_location)
            .await
            .expect("should succeed");
        assert_eq!(result.status, AnalyzeResultStatus::Succeeded);
        let ar = result.analyze_result.expect("should have result");
        assert_eq!(ar.content.as_deref(), Some("Hello world"));
    }

    #[tokio::test]
    async fn test_get_result_with_malformed_url_returns_invalid_endpoint() {
        use azure_ai_foundry_core::error::FoundryError;

        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let err = get_result(&client, "not-a-valid-url")
            .await
            .expect_err("should fail with malformed URL");

        assert!(
            matches!(err, FoundryError::InvalidEndpoint { .. }),
            "expected FoundryError::InvalidEndpoint, got: {err:?}",
        );
        assert!(
            err.to_string().contains("Operation-Location"),
            "error should mention Operation-Location: {err}",
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 20: poll_until_complete
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_poll_until_complete_immediate_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        // First poll: running. Second poll: succeeded.
        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-1",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "running"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "succeeded",
                "analyzeResult": {
                    "apiVersion": "2024-11-30",
                    "modelId": "prebuilt-read",
                    "content": "Done"
                }
            })))
            .mount(&server)
            .await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-1",
            server.uri(),
        );

        let result = poll_until_complete(&client, &op_location, Duration::from_millis(10), 10)
            .await
            .expect("should succeed");
        assert_eq!(result.status, AnalyzeResultStatus::Succeeded);
        let ar = result.analyze_result.expect("should have result");
        assert_eq!(ar.content.as_deref(), Some("Done"));
    }

    #[tokio::test]
    async fn test_poll_until_complete_failed_status() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-fail",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "failed"})),
            )
            .mount(&server)
            .await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-fail",
            server.uri(),
        );

        let result = poll_until_complete(&client, &op_location, Duration::from_millis(10), 10)
            .await
            .expect("should return Ok even on failed status");
        assert_eq!(result.status, AnalyzeResultStatus::Failed);
    }

    #[tokio::test]
    async fn test_poll_until_complete_exceeds_max_attempts() {
        use azure_ai_foundry_core::error::FoundryError;

        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        // Always return "running" — will never terminate naturally
        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/infinite",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "running"})),
            )
            .mount(&server)
            .await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/infinite",
            server.uri(),
        );

        let err = poll_until_complete(&client, &op_location, Duration::from_millis(1), 3)
            .await
            .expect_err("should fail after max_attempts exceeded");

        assert!(
            matches!(err, FoundryError::Api { .. }),
            "expected FoundryError::Api, got: {err:?}",
        );
        assert!(
            err.to_string().contains("max_attempts") || err.to_string().contains("timed out"),
            "error: {err}",
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 21: Prebuilt model ID constants
    // -----------------------------------------------------------------------

    #[test]
    fn test_prebuilt_model_id_constants() {
        assert_eq!(PREBUILT_READ, "prebuilt-read");
        assert_eq!(PREBUILT_LAYOUT, "prebuilt-layout");
        assert_eq!(PREBUILT_INVOICE, "prebuilt-invoice");
        assert_eq!(PREBUILT_RECEIPT, "prebuilt-receipt");
        assert_eq!(PREBUILT_ID_DOCUMENT, "prebuilt-idDocument");
        assert_eq!(PREBUILT_BUSINESS_CARD, "prebuilt-businessCard");
    }

    // -----------------------------------------------------------------------
    // Cycle 22: Error handling
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_analyze_document_unauthorized_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "code": "Unauthorized",
                    "message": "Invalid API key"
                }
            })))
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let err = analyze(&client, &request).await.expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("401") || msg.contains("Unauthorized"),
            "unexpected error: {msg}",
        );
    }

    #[tokio::test]
    async fn test_analyze_document_model_not_found_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/nonexistent:analyze",
            ))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "code": "NotFound",
                    "message": "Model not found"
                }
            })))
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id("nonexistent")
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let err = analyze(&client, &request).await.expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("404") || msg.contains("NotFound"),
            "unexpected error: {msg}",
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 23: Tracing spans
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_document_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-trace",
            server.uri(),
        );

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(
                ResponseTemplate::new(202)
                    .append_header("Operation-Location", op_location.as_str()),
            )
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let _ = analyze(&client, &request).await;
        assert!(logs_contain("foundry::document_intelligence::analyze"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_document_emits_span_with_model_id_field() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-field",
            server.uri(),
        );

        Mock::given(method("POST"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read:analyze",
            ))
            .respond_with(
                ResponseTemplate::new(202)
                    .append_header("Operation-Location", op_location.as_str()),
            )
            .mount(&server)
            .await;

        let request = DocumentAnalysisRequest::builder()
            .model_id(PREBUILT_READ)
            .url_source("https://example.com/doc.pdf")
            .build()
            .expect("valid request");

        let _ = analyze(&client, &request).await;

        // Verify the model_id field value appears in the trace output.
        assert!(logs_contain("prebuilt-read"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_poll_until_complete_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(match_path(
                "/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-span",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "succeeded",
                "analyzeResult": {
                    "apiVersion": "2024-11-30",
                    "modelId": "prebuilt-read"
                }
            })))
            .mount(&server)
            .await;

        let op_location = format!(
            "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-span",
            server.uri(),
        );

        let _ = poll_until_complete(&client, &op_location, Duration::from_millis(10), 10).await;
        assert!(logs_contain(
            "foundry::document_intelligence::poll_until_complete"
        ));
    }

    // -----------------------------------------------------------------------
    // DocumentAnalysisFeature serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_document_analysis_feature_as_str_matches_serde() {
        let variants = [
            (
                DocumentAnalysisFeature::OcrHighResolution,
                "ocrHighResolution",
            ),
            (DocumentAnalysisFeature::Languages, "languages"),
            (DocumentAnalysisFeature::Barcodes, "barcodes"),
            (DocumentAnalysisFeature::Formulas, "formulas"),
            (DocumentAnalysisFeature::KeyValuePairs, "keyValuePairs"),
            (DocumentAnalysisFeature::StyleFont, "styleFont"),
            (DocumentAnalysisFeature::QueryFields, "queryFields"),
        ];

        for (variant, expected) in &variants {
            assert_eq!(
                variant.as_str(),
                *expected,
                "as_str() mismatch for {expected}",
            );
            let serialized = serde_json::to_string(variant).expect("should serialize");
            assert_eq!(
                serialized,
                format!("\"{expected}\""),
                "serde rename mismatch for {expected}",
            );
        }
    }

    #[test]
    fn test_document_analysis_feature_serialization() {
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::OcrHighResolution).unwrap(),
            r#""ocrHighResolution""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::Languages).unwrap(),
            r#""languages""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::Barcodes).unwrap(),
            r#""barcodes""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::Formulas).unwrap(),
            r#""formulas""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::KeyValuePairs).unwrap(),
            r#""keyValuePairs""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::StyleFont).unwrap(),
            r#""styleFont""#,
        );
        assert_eq!(
            serde_json::to_string(&DocumentAnalysisFeature::QueryFields).unwrap(),
            r#""queryFields""#,
        );
    }
}
