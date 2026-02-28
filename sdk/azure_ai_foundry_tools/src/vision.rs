//! Image Analysis client for Azure AI Foundry Vision API 4.0.
//!
//! This module provides functions to analyze images using the Azure Computer Vision
//! Image Analysis API, supporting features like tags, captions, object detection,
//! OCR, dense captions, smart crops, and people detection.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_tools::vision::{self, ImageAnalysisRequest, VisualFeature};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! let request = ImageAnalysisRequest::builder()
//!     .url("https://example.com/image.jpg")
//!     .features(vec![VisualFeature::Tags, VisualFeature::Caption])
//!     .build()?;
//!
//! let result = vision::analyze(&client, &request).await?;
//! if let Some(caption) = &result.caption_result {
//!     println!("Caption: {} ({:.2}%)", caption.text, caption.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::{BoundingBox, ImageMetadata, ImagePoint, VISION_API_VERSION};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A visual feature to extract from the image.
///
/// Each feature enables a specific type of analysis. Multiple features can be
/// requested in a single call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VisualFeature {
    /// Detect content tags describing the image.
    #[serde(rename = "tags")]
    Tags,
    /// Generate a single natural-language caption.
    #[serde(rename = "caption")]
    Caption,
    /// Generate multiple captions for different image regions.
    #[serde(rename = "denseCaptions")]
    DenseCaptions,
    /// Detect and locate objects in the image.
    #[serde(rename = "objects")]
    Objects,
    /// Extract printed and handwritten text (OCR).
    #[serde(rename = "read")]
    Read,
    /// Suggest smart crop regions for different aspect ratios.
    #[serde(rename = "smartCrops")]
    SmartCrops,
    /// Detect and locate people in the image.
    #[serde(rename = "people")]
    People,
}

impl VisualFeature {
    /// Returns the API string representation of this feature.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Tags => "tags",
            Self::Caption => "caption",
            Self::DenseCaptions => "denseCaptions",
            Self::Objects => "objects",
            Self::Read => "read",
            Self::SmartCrops => "smartCrops",
            Self::People => "people",
        }
    }
}

/// A request to analyze an image.
///
/// Use the builder pattern to construct requests:
///
/// ```rust
/// use azure_ai_foundry_tools::vision::{ImageAnalysisRequest, VisualFeature};
///
/// let request = ImageAnalysisRequest::builder()
///     .url("https://example.com/image.jpg")
///     .features(vec![VisualFeature::Tags, VisualFeature::Caption])
///     .build()
///     .expect("valid request");
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ImageAnalysisRequest {
    /// URL of the image to analyze.
    url: String,

    /// Visual features to extract (not serialized in body — sent as query param).
    #[serde(skip)]
    features: Vec<VisualFeature>,

    /// Language for text output (e.g., "en", "es").
    #[serde(skip)]
    language: Option<String>,

    /// Model version to use.
    #[serde(skip)]
    model_version: Option<String>,

    /// Aspect ratios for smart crop suggestions (0.75 to 1.80).
    #[serde(skip)]
    smartcrops_aspect_ratios: Option<Vec<f64>>,

    /// Whether to generate gender-neutral captions.
    #[serde(skip)]
    gender_neutral_caption: Option<bool>,
}

impl ImageAnalysisRequest {
    /// Creates a new builder for an image analysis request.
    pub fn builder() -> ImageAnalysisRequestBuilder {
        ImageAnalysisRequestBuilder::default()
    }

    /// Returns the image URL set on this request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the features as a comma-separated query parameter value.
    pub(crate) fn features_query_param(&self) -> String {
        self.features
            .iter()
            .map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Builds the full query string for the API request.
    pub(crate) fn query_string(&self) -> String {
        let mut params = format!(
            "features={}&{}",
            self.features_query_param(),
            VISION_API_VERSION,
        );

        if let Some(ref lang) = self.language {
            params.push_str(&format!("&language={lang}"));
        }
        if let Some(ref mv) = self.model_version {
            params.push_str(&format!("&model-version={mv}"));
        }
        if let Some(ref ratios) = self.smartcrops_aspect_ratios {
            let ratios_str: Vec<String> = ratios.iter().map(|r| r.to_string()).collect();
            params.push_str(&format!(
                "&smartcrops-aspect-ratios={}",
                ratios_str.join(",")
            ));
        }
        if let Some(gnc) = self.gender_neutral_caption {
            params.push_str(&format!("&gender-neutral-caption={gnc}"));
        }

        params
    }
}

/// Builder for [`ImageAnalysisRequest`].
#[derive(Debug, Default)]
pub struct ImageAnalysisRequestBuilder {
    url: Option<String>,
    features: Option<Vec<VisualFeature>>,
    language: Option<String>,
    model_version: Option<String>,
    smartcrops_aspect_ratios: Option<Vec<f64>>,
    gender_neutral_caption: Option<bool>,
}

impl ImageAnalysisRequestBuilder {
    /// Sets the URL of the image to analyze (required).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the visual features to extract (required, at least one).
    pub fn features(mut self, features: Vec<VisualFeature>) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets the language for text output.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Sets the model version.
    pub fn model_version(mut self, version: impl Into<String>) -> Self {
        self.model_version = Some(version.into());
        self
    }

    /// Sets the aspect ratios for smart crop suggestions.
    ///
    /// Each ratio must be between 0.75 and 1.80 inclusive.
    pub fn smartcrops_aspect_ratios(mut self, ratios: Vec<f64>) -> Self {
        self.smartcrops_aspect_ratios = Some(ratios);
        self
    }

    /// Sets whether to generate gender-neutral captions.
    pub fn gender_neutral_caption(mut self, value: bool) -> Self {
        self.gender_neutral_caption = Some(value);
        self
    }

    /// Builds the request, validating all required fields.
    ///
    /// # Errors
    ///
    /// Returns [`FoundryError::Builder`] if:
    /// - `url` is missing or empty
    /// - `features` is missing or empty
    /// - Any smart crop aspect ratio is outside the valid range (0.75..=1.80)
    pub fn build(self) -> FoundryResult<ImageAnalysisRequest> {
        let url = self
            .url
            .filter(|u| !u.is_empty())
            .ok_or_else(|| FoundryError::Builder("url is required".into()))?;

        let features = self
            .features
            .filter(|f| !f.is_empty())
            .ok_or_else(|| FoundryError::Builder("features is required (at least one)".into()))?;

        if let Some(ref ratios) = self.smartcrops_aspect_ratios {
            for ratio in ratios {
                if !(ratio.is_finite() && *ratio >= 0.75 && *ratio <= 1.80) {
                    return Err(FoundryError::Builder(format!(
                        "smartcrops aspect ratio {ratio} is outside valid range (0.75..=1.80)"
                    )));
                }
            }
        }

        Ok(ImageAnalysisRequest {
            url,
            features,
            language: self.language,
            model_version: self.model_version,
            smartcrops_aspect_ratios: self.smartcrops_aspect_ratios,
            gender_neutral_caption: self.gender_neutral_caption,
        })
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// The result of an image analysis request.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageAnalysisResult {
    /// The model version used for analysis.
    #[serde(rename = "modelVersion")]
    pub model_version: String,

    /// Image dimensions.
    pub metadata: ImageMetadata,

    /// Single caption describing the image.
    #[serde(rename = "captionResult")]
    pub caption_result: Option<CaptionResult>,

    /// Content tags describing the image.
    #[serde(rename = "tagsResult")]
    pub tags_result: Option<TagsResult>,

    /// Detected objects with locations.
    #[serde(rename = "objectsResult")]
    pub objects_result: Option<ObjectsResult>,

    /// Extracted text (OCR).
    #[serde(rename = "readResult")]
    pub read_result: Option<ReadResult>,

    /// Multiple captions for different image regions.
    #[serde(rename = "denseCaptionsResult")]
    pub dense_captions_result: Option<DenseCaptionsResult>,

    /// Suggested crop regions.
    #[serde(rename = "smartCropsResult")]
    pub smart_crops_result: Option<SmartCropsResult>,

    /// Detected people with locations.
    #[serde(rename = "peopleResult")]
    pub people_result: Option<PeopleResult>,
}

/// A natural-language caption for the image.
#[derive(Debug, Clone, Deserialize)]
pub struct CaptionResult {
    /// The caption text.
    pub text: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// A content tag assigned to the image.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentTag {
    /// Tag name.
    pub name: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// Collection of content tags.
#[derive(Debug, Clone, Deserialize)]
pub struct TagsResult {
    /// The detected tags.
    pub values: Vec<ContentTag>,
}

/// A detected object in the image.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedObject {
    /// Object identifier.
    pub id: String,
    /// Location of the object.
    #[serde(rename = "boundingBox")]
    pub bounding_box: BoundingBox,
    /// Tags describing the object.
    pub tags: Vec<ContentTag>,
}

/// Collection of detected objects.
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectsResult {
    /// The detected objects.
    pub values: Vec<DetectedObject>,
}

/// A detected word in the image text.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedTextWord {
    /// The word text.
    pub text: String,
    /// Polygon coordinates enclosing the word.
    #[serde(rename = "boundingPolygon")]
    pub bounding_polygon: Vec<ImagePoint>,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// A detected line of text.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedTextLine {
    /// The line text.
    pub text: String,
    /// Polygon coordinates enclosing the line.
    #[serde(rename = "boundingPolygon")]
    pub bounding_polygon: Vec<ImagePoint>,
    /// Individual words in the line.
    pub words: Vec<DetectedTextWord>,
}

/// A block of detected text.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedTextBlock {
    /// Lines of text in this block.
    pub lines: Vec<DetectedTextLine>,
}

/// OCR read result.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadResult {
    /// Text blocks found in the image.
    pub blocks: Vec<DetectedTextBlock>,
}

/// A dense caption for a specific image region.
#[derive(Debug, Clone, Deserialize)]
pub struct DenseCaption {
    /// The caption text.
    pub text: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Region this caption describes.
    #[serde(rename = "boundingBox")]
    pub bounding_box: BoundingBox,
}

/// Collection of dense captions.
#[derive(Debug, Clone, Deserialize)]
pub struct DenseCaptionsResult {
    /// The dense captions.
    pub values: Vec<DenseCaption>,
}

/// A suggested crop region.
#[derive(Debug, Clone, Deserialize)]
pub struct CropRegion {
    /// The aspect ratio of this crop.
    #[serde(rename = "aspectRatio")]
    pub aspect_ratio: f64,
    /// The crop bounding box.
    #[serde(rename = "boundingBox")]
    pub bounding_box: BoundingBox,
}

/// Collection of smart crop suggestions.
#[derive(Debug, Clone, Deserialize)]
pub struct SmartCropsResult {
    /// The crop regions.
    pub values: Vec<CropRegion>,
}

/// A detected person in the image.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedPerson {
    /// Location of the detected person.
    #[serde(rename = "boundingBox")]
    pub bounding_box: BoundingBox,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// Collection of detected people.
#[derive(Debug, Clone, Deserialize)]
pub struct PeopleResult {
    /// The detected people.
    pub values: Vec<DetectedPerson>,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Analyze an image using the Vision Image Analysis 4.0 API.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_tools::vision::{self, ImageAnalysisRequest, VisualFeature};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = ImageAnalysisRequest::builder()
///     .url("https://example.com/image.jpg")
///     .features(vec![VisualFeature::Tags, VisualFeature::Caption])
///     .build()?;
///
/// let result = vision::analyze(client, &request).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vision::analyze` with field `features`.
#[tracing::instrument(
    name = "foundry::vision::analyze",
    skip(client, request),
    fields(features = %request.features_query_param())
)]
pub async fn analyze(
    client: &FoundryClient,
    request: &ImageAnalysisRequest,
) -> FoundryResult<ImageAnalysisResult> {
    tracing::debug!("analyzing image");

    let path = format!(
        "/computervision/imageanalysis:analyze?{}",
        request.query_string(),
    );

    // The body only contains the URL; features go in the query string.
    let body = serde_json::json!({ "url": request.url() });
    let response = client.post(&path, &body).await?;
    let result = response.json::<ImageAnalysisResult>().await?;

    tracing::debug!("image analysis complete");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_mock_client;
    use wiremock::matchers::{method, path as match_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -----------------------------------------------------------------------
    // Cycle 6: VisualFeature serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_visual_feature_as_str_matches_serde() {
        let variants = [
            (VisualFeature::Tags, "tags"),
            (VisualFeature::Caption, "caption"),
            (VisualFeature::DenseCaptions, "denseCaptions"),
            (VisualFeature::Objects, "objects"),
            (VisualFeature::Read, "read"),
            (VisualFeature::SmartCrops, "smartCrops"),
            (VisualFeature::People, "people"),
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
    fn test_visual_feature_serialization() {
        assert_eq!(
            serde_json::to_string(&VisualFeature::Tags).unwrap(),
            r#""tags""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::Caption).unwrap(),
            r#""caption""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::DenseCaptions).unwrap(),
            r#""denseCaptions""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::Objects).unwrap(),
            r#""objects""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::Read).unwrap(),
            r#""read""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::SmartCrops).unwrap(),
            r#""smartCrops""#,
        );
        assert_eq!(
            serde_json::to_string(&VisualFeature::People).unwrap(),
            r#""people""#,
        );
    }

    // -----------------------------------------------------------------------
    // Cycle 7: ImageAnalysisRequest builder validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_image_analysis_request_requires_url() {
        let result = ImageAnalysisRequest::builder()
            .features(vec![VisualFeature::Tags])
            .build();
        let err = result.expect_err("should require url");
        assert!(err.to_string().contains("url"), "error: {err}");
    }

    #[test]
    fn test_image_analysis_request_rejects_empty_url() {
        let result = ImageAnalysisRequest::builder()
            .url("")
            .features(vec![VisualFeature::Tags])
            .build();
        let err = result.expect_err("should reject empty url");
        assert!(err.to_string().contains("url"), "error: {err}");
    }

    #[test]
    fn test_image_analysis_request_requires_features() {
        let result = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .build();
        let err = result.expect_err("should require features");
        assert!(err.to_string().contains("features"), "error: {err}");
    }

    #[test]
    fn test_image_analysis_request_rejects_empty_features() {
        let result = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![])
            .build();
        let err = result.expect_err("should reject empty features");
        assert!(err.to_string().contains("features"), "error: {err}");
    }

    #[test]
    fn test_image_analysis_request_rejects_nan_aspect_ratio() {
        let result = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::SmartCrops])
            .smartcrops_aspect_ratios(vec![f64::NAN])
            .build();
        let err = result.expect_err("NaN should be rejected");
        assert!(err.to_string().contains("aspect ratio"), "error: {err}",);
    }

    #[test]
    fn test_image_analysis_request_rejects_infinity_aspect_ratio() {
        let result = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::SmartCrops])
            .smartcrops_aspect_ratios(vec![f64::INFINITY])
            .build();
        let err = result.expect_err("Infinity should be rejected");
        assert!(err.to_string().contains("aspect ratio"), "error: {err}",);
    }

    #[test]
    fn test_image_analysis_request_rejects_invalid_aspect_ratio() {
        let result = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::SmartCrops])
            .smartcrops_aspect_ratios(vec![0.5]) // below 0.75
            .build();
        let err = result.expect_err("should reject invalid ratio");
        assert!(err.to_string().contains("aspect ratio"), "error: {err}");
    }

    #[test]
    fn test_image_analysis_request_url_getter() {
        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/image.jpg")
            .features(vec![VisualFeature::Tags])
            .build()
            .expect("valid request");
        assert_eq!(request.url(), "https://example.com/image.jpg");
    }

    // -----------------------------------------------------------------------
    // Cycle 8: Request serialization and query string
    // -----------------------------------------------------------------------

    #[test]
    fn test_image_analysis_request_body_only_contains_url() {
        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::Tags, VisualFeature::Caption])
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(json["url"], "https://example.com/img.png");
        assert!(
            json.get("features").is_none(),
            "features should not be in body"
        );
    }

    #[test]
    fn test_image_analysis_request_features_to_query_string() {
        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::Tags, VisualFeature::Caption])
            .build()
            .expect("valid request");

        assert_eq!(request.features_query_param(), "tags,caption");
    }

    #[test]
    fn test_image_analysis_request_full_query_string() {
        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.png")
            .features(vec![VisualFeature::Tags])
            .language("en")
            .gender_neutral_caption(true)
            .build()
            .expect("valid request");

        let qs = request.query_string();
        assert!(qs.contains("features=tags"), "qs: {qs}");
        assert!(qs.contains("api-version=2024-02-01"), "qs: {qs}");
        assert!(qs.contains("language=en"), "qs: {qs}");
        assert!(qs.contains("gender-neutral-caption=true"), "qs: {qs}");
    }

    // -----------------------------------------------------------------------
    // Cycle 9: Response types deserialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_image_analysis_result_deserialization_minimal() {
        let json = r#"{
            "modelVersion": "2024-02-01",
            "metadata": {"width": 800, "height": 600}
        }"#;
        let result: ImageAnalysisResult = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(result.model_version, "2024-02-01");
        assert_eq!(result.metadata.width, 800);
        assert_eq!(result.metadata.height, 600);
        assert!(result.caption_result.is_none());
        assert!(result.tags_result.is_none());
    }

    #[test]
    fn test_image_analysis_result_deserialization_full() {
        let json = r#"{
            "modelVersion": "2024-02-01",
            "metadata": {"width": 800, "height": 600},
            "captionResult": {"text": "a cat sitting on a table", "confidence": 0.95},
            "tagsResult": {"values": [{"name": "cat", "confidence": 0.99}]},
            "objectsResult": {
                "values": [{
                    "id": "obj-1",
                    "boundingBox": {"x": 10, "y": 20, "w": 100, "h": 50},
                    "tags": [{"name": "cat", "confidence": 0.98}]
                }]
            },
            "readResult": {
                "blocks": [{
                    "lines": [{
                        "text": "Hello World",
                        "boundingPolygon": [{"x": 0, "y": 0}, {"x": 100, "y": 0}],
                        "words": [{
                            "text": "Hello",
                            "boundingPolygon": [{"x": 0, "y": 0}],
                            "confidence": 0.99
                        }]
                    }]
                }]
            },
            "denseCaptionsResult": {
                "values": [{
                    "text": "a cat",
                    "confidence": 0.90,
                    "boundingBox": {"x": 5, "y": 10, "w": 200, "h": 150}
                }]
            },
            "smartCropsResult": {
                "values": [{"aspectRatio": 1.0, "boundingBox": {"x": 0, "y": 0, "w": 800, "h": 600}}]
            },
            "peopleResult": {
                "values": [{"boundingBox": {"x": 300, "y": 100, "w": 200, "h": 400}, "confidence": 0.85}]
            }
        }"#;

        let result: ImageAnalysisResult = serde_json::from_str(json).expect("should deserialize");

        // Caption
        let caption = result.caption_result.as_ref().expect("should have caption");
        assert_eq!(caption.text, "a cat sitting on a table");
        assert!((caption.confidence - 0.95).abs() < f64::EPSILON);

        // Tags
        let tags = result.tags_result.as_ref().expect("should have tags");
        assert_eq!(tags.values[0].name, "cat");

        // Objects
        let objects = result.objects_result.as_ref().expect("should have objects");
        assert_eq!(objects.values[0].id, "obj-1");
        assert_eq!(objects.values[0].bounding_box.x, 10);

        // Read (OCR)
        let read = result
            .read_result
            .as_ref()
            .expect("should have read result");
        assert_eq!(read.blocks[0].lines[0].text, "Hello World");
        assert_eq!(read.blocks[0].lines[0].words[0].text, "Hello");

        // Dense captions
        let dense = result
            .dense_captions_result
            .as_ref()
            .expect("should have dense captions");
        assert_eq!(dense.values[0].text, "a cat");

        // Smart crops
        let crops = result
            .smart_crops_result
            .as_ref()
            .expect("should have smart crops");
        assert!((crops.values[0].aspect_ratio - 1.0).abs() < f64::EPSILON);

        // People
        let people = result.people_result.as_ref().expect("should have people");
        assert!((people.values[0].confidence - 0.85).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Cycle 10: vision::analyze success path
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_analyze_image_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let response_body = serde_json::json!({
            "modelVersion": "2024-02-01",
            "metadata": {"width": 1024, "height": 768},
            "captionResult": {"text": "a dog in a park", "confidence": 0.92}
        });

        Mock::given(method("POST"))
            .and(match_path("/computervision/imageanalysis:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&server)
            .await;

        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/dog.jpg")
            .features(vec![VisualFeature::Caption])
            .build()
            .expect("valid request");

        let result = analyze(&client, &request).await.expect("should succeed");
        assert_eq!(result.model_version, "2024-02-01");
        assert_eq!(result.metadata.width, 1024);
        let caption = result.caption_result.expect("should have caption");
        assert_eq!(caption.text, "a dog in a park");
    }

    // -----------------------------------------------------------------------
    // Cycle 11: vision::analyze error handling
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_analyze_image_api_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path("/computervision/imageanalysis:analyze"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": "InvalidImageUrl",
                    "message": "URL is not accessible"
                }
            })))
            .mount(&server)
            .await;

        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/invalid.jpg")
            .features(vec![VisualFeature::Tags])
            .build()
            .expect("valid request");

        let err = analyze(&client, &request).await.expect_err("should fail");
        // FoundryClient maps non-success to FoundryError::Http or FoundryError::Api
        let msg = err.to_string();
        assert!(
            msg.contains("InvalidImageUrl") || msg.contains("400"),
            "unexpected error: {msg}",
        );
    }

    #[tokio::test]
    async fn test_analyze_image_http_error() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path("/computervision/imageanalysis:analyze"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.jpg")
            .features(vec![VisualFeature::Tags])
            .build()
            .expect("valid request");

        let err = analyze(&client, &request).await.expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("500"), "unexpected error: {msg}");
    }

    // -----------------------------------------------------------------------
    // Cycle 12: Tracing span emission
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_emits_span_with_features_field() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path("/computervision/imageanalysis:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "modelVersion": "2024-02-01",
                "metadata": {"width": 100, "height": 100}
            })))
            .mount(&server)
            .await;

        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.jpg")
            .features(vec![VisualFeature::Tags, VisualFeature::Caption])
            .build()
            .expect("valid request");

        let _ = analyze(&client, &request).await;

        // Verify the features field value appears in the trace output.
        assert!(logs_contain("tags,caption"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_analyze_emits_vision_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(match_path("/computervision/imageanalysis:analyze"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "modelVersion": "2024-02-01",
                "metadata": {"width": 100, "height": 100}
            })))
            .mount(&server)
            .await;

        let request = ImageAnalysisRequest::builder()
            .url("https://example.com/img.jpg")
            .features(vec![VisualFeature::Tags])
            .build()
            .expect("valid request");

        let _ = analyze(&client, &request).await;
        assert!(logs_contain("foundry::vision::analyze"));
    }
}
