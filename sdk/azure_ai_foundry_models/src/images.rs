//! Image generation and editing types and functions for Azure AI Foundry Models.
//!
//! This module provides image generation and image editing APIs.
//!
//! # Image Generation Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::images::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = ImageGenerationRequest::builder()
//!     .model("dall-e-3")
//!     .prompt("A sunset over mountains")
//!     .size(ImageSize::S1024x1024)
//!     .build();
//!
//! let response = generate(client, &request).await?;
//! if let Some(url) = &response.data[0].url {
//!     println!("Image URL: {}", url);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Image Editing Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::images::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let image_data = std::fs::read("input.png").unwrap();
//! let request = ImageEditRequest::builder()
//!     .model("dall-e-2")
//!     .image(image_data, "input.png")
//!     .prompt("Add a rainbow in the sky")
//!     .build();
//!
//! let response = edit(client, &request).await?;
//! println!("Edited {} images", response.data.len());
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Image size for generation and editing requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSize {
    /// 256x256 pixels.
    #[serde(rename = "256x256")]
    S256x256,
    /// 512x512 pixels.
    #[serde(rename = "512x512")]
    S512x512,
    /// 1024x1024 pixels.
    #[serde(rename = "1024x1024")]
    S1024x1024,
    /// 1536x1024 pixels (landscape).
    #[serde(rename = "1536x1024")]
    S1536x1024,
    /// 1024x1536 pixels (portrait).
    #[serde(rename = "1024x1536")]
    S1024x1536,
    /// Automatic size selection.
    #[serde(rename = "auto")]
    Auto,
}

impl ImageSize {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::S256x256 => "256x256",
            Self::S512x512 => "512x512",
            Self::S1024x1024 => "1024x1024",
            Self::S1536x1024 => "1536x1024",
            Self::S1024x1536 => "1024x1536",
            Self::Auto => "auto",
        }
    }
}

/// Image quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageQuality {
    /// Standard quality.
    Standard,
    /// High definition quality.
    Hd,
    /// Low quality.
    Low,
    /// Medium quality.
    Medium,
    /// High quality.
    High,
    /// Automatic quality selection.
    Auto,
}

/// Response format for image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResponseFormat {
    /// Return a URL to the generated image.
    #[serde(rename = "url")]
    Url,
    /// Return the image as a base64-encoded JSON string.
    #[serde(rename = "b64_json")]
    B64Json,
}

impl ImageQuality {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Hd => "hd",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Auto => "auto",
        }
    }
}

impl ImageResponseFormat {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::B64Json => "b64_json",
        }
    }
}

/// Output format for the generated image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageOutputFormat {
    /// PNG image format.
    Png,
    /// JPEG image format.
    Jpeg,
    /// WebP image format.
    Webp,
}

// ---------------------------------------------------------------------------
// Image generation request
// ---------------------------------------------------------------------------

/// A request to generate images from a text prompt.
#[derive(Debug, Clone, Serialize)]
pub struct ImageGenerationRequest {
    /// The model to use for generation.
    pub model: String,
    /// The text prompt describing the desired image.
    pub prompt: String,

    /// The number of images to generate (1-10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// The size of the generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    /// The quality of the generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<ImageQuality>,

    /// The format in which the generated images are returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ImageResponseFormat>,

    /// The output image format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<ImageOutputFormat>,
}

impl ImageGenerationRequest {
    /// Create a new builder.
    pub fn builder() -> ImageGenerationRequestBuilder {
        ImageGenerationRequestBuilder {
            model: None,
            prompt: None,
            n: None,
            size: None,
            quality: None,
            response_format: None,
            output_format: None,
        }
    }
}

/// Builder for [`ImageGenerationRequest`].
pub struct ImageGenerationRequestBuilder {
    model: Option<String>,
    prompt: Option<String>,
    n: Option<u32>,
    size: Option<ImageSize>,
    quality: Option<ImageQuality>,
    response_format: Option<ImageResponseFormat>,
    output_format: Option<ImageOutputFormat>,
}

impl ImageGenerationRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the text prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the number of images to generate (1-10).
    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    /// Set the image size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the image quality.
    pub fn quality(mut self, quality: ImageQuality) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ImageResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the output image format.
    pub fn output_format(mut self, format: ImageOutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<ImageGenerationRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let prompt = self
            .prompt
            .ok_or_else(|| FoundryError::Builder("prompt is required".into()))?;
        if prompt.is_empty() {
            return Err(FoundryError::Builder("prompt cannot be empty".into()));
        }

        if let Some(n) = self.n {
            if !(1..=10).contains(&n) {
                return Err(FoundryError::Builder("n must be between 1 and 10".into()));
            }
        }

        Ok(ImageGenerationRequest {
            model,
            prompt,
            n: self.n,
            size: self.size,
            quality: self.quality,
            response_format: self.response_format,
            output_format: self.output_format,
        })
    }

    /// Build the request. Panics if required fields are missing.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> ImageGenerationRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Image edit request
// ---------------------------------------------------------------------------

/// A request to edit an existing image using a text prompt.
#[derive(Debug, Clone)]
pub struct ImageEditRequest {
    /// The model to use for editing.
    pub model: String,
    /// The image to edit as raw bytes.
    pub image: Vec<u8>,
    /// The filename of the image.
    pub image_filename: String,
    /// The text prompt describing the desired edit.
    pub prompt: String,
    /// An optional mask image indicating which areas to edit.
    pub mask: Option<Vec<u8>>,
    /// The filename of the mask image.
    pub mask_filename: Option<String>,
    /// The number of images to generate (1-10).
    pub n: Option<u32>,
    /// The size of the generated images.
    pub size: Option<ImageSize>,
    /// The quality of the generated images.
    pub quality: Option<ImageQuality>,
    /// The format in which the images are returned.
    pub response_format: Option<ImageResponseFormat>,
}

impl ImageEditRequest {
    /// Create a new builder.
    pub fn builder() -> ImageEditRequestBuilder {
        ImageEditRequestBuilder {
            model: None,
            image: None,
            image_filename: None,
            prompt: None,
            mask: None,
            mask_filename: None,
            n: None,
            size: None,
            quality: None,
            response_format: None,
        }
    }
}

/// Builder for [`ImageEditRequest`].
pub struct ImageEditRequestBuilder {
    model: Option<String>,
    image: Option<Vec<u8>>,
    image_filename: Option<String>,
    prompt: Option<String>,
    mask: Option<Vec<u8>>,
    mask_filename: Option<String>,
    n: Option<u32>,
    size: Option<ImageSize>,
    quality: Option<ImageQuality>,
    response_format: Option<ImageResponseFormat>,
}

impl ImageEditRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the image data and filename.
    pub fn image(mut self, data: Vec<u8>, filename: impl Into<String>) -> Self {
        self.image = Some(data);
        self.image_filename = Some(filename.into());
        self
    }

    /// Set the text prompt describing the edit.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the mask image data and filename.
    pub fn mask(mut self, data: Vec<u8>, filename: impl Into<String>) -> Self {
        self.mask = Some(data);
        self.mask_filename = Some(filename.into());
        self
    }

    /// Set the number of images to generate (1-10).
    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    /// Set the image size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the image quality.
    pub fn quality(mut self, quality: ImageQuality) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ImageResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<ImageEditRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let image = self
            .image
            .ok_or_else(|| FoundryError::Builder("image is required".into()))?;
        if image.is_empty() {
            return Err(FoundryError::Builder("image data cannot be empty".into()));
        }

        let image_filename = self
            .image_filename
            .ok_or_else(|| FoundryError::Builder("image filename is required".into()))?;
        if image_filename.is_empty() {
            return Err(FoundryError::Builder(
                "image filename cannot be empty".into(),
            ));
        }

        let prompt = self
            .prompt
            .ok_or_else(|| FoundryError::Builder("prompt is required".into()))?;
        if prompt.is_empty() {
            return Err(FoundryError::Builder("prompt cannot be empty".into()));
        }

        if let Some(n) = self.n {
            if !(1..=10).contains(&n) {
                return Err(FoundryError::Builder("n must be between 1 and 10".into()));
            }
        }

        Ok(ImageEditRequest {
            model,
            image,
            image_filename,
            prompt,
            mask: self.mask,
            mask_filename: self.mask_filename,
            n: self.n,
            size: self.size,
            quality: self.quality,
            response_format: self.response_format,
        })
    }

    /// Build the request. Panics if required fields are missing.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> ImageEditRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from an image generation or editing request.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageResponse {
    /// Unix timestamp when the images were created.
    pub created: u64,
    /// The generated or edited images.
    pub data: Vec<ImageData>,
}

/// A single image in the response.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageData {
    /// The URL of the generated image (when response_format is `url`).
    pub url: Option<String>,
    /// The base64-encoded image data (when response_format is `b64_json`).
    pub b64_json: Option<String>,
    /// The prompt that was used to generate the image (may be revised by the model).
    pub revised_prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Generate images from a text prompt.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::images::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = ImageGenerationRequest::builder()
///     .model("dall-e-3")
///     .prompt("A futuristic city at night")
///     .build();
///
/// let response = generate(client, &request).await?;
/// println!("Generated {} images", response.data.len());
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::images::generate` with field `model`.
#[tracing::instrument(
    name = "foundry::images::generate",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn generate(
    client: &FoundryClient,
    request: &ImageGenerationRequest,
) -> FoundryResult<ImageResponse> {
    tracing::debug!("sending image generation request");

    let response = client
        .post("/openai/v1/images/generations", request)
        .await?;
    let body = response.json::<ImageResponse>().await?;
    Ok(body)
}

/// Edit an existing image using a text prompt.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::images::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let image_data = std::fs::read("photo.png").unwrap();
/// let request = ImageEditRequest::builder()
///     .model("dall-e-2")
///     .image(image_data, "photo.png")
///     .prompt("Add a hat to the person")
///     .build();
///
/// let response = edit(client, &request).await?;
/// println!("Edited {} images", response.data.len());
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::images::edit` with field `model`.
#[tracing::instrument(
    name = "foundry::images::edit",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn edit(
    client: &FoundryClient,
    request: &ImageEditRequest,
) -> FoundryResult<ImageResponse> {
    tracing::debug!("sending image edit request");

    let image_data = request.image.clone();
    let image_filename = request.image_filename.clone();
    let model = request.model.clone();
    let prompt = request.prompt.clone();
    let mask_data = request.mask.clone();
    let mask_filename = request.mask_filename.clone();
    let n = request.n;
    let size = request.size;
    let quality = request.quality;
    let response_format = request.response_format;

    let response = client
        .post_multipart("/openai/v1/images/edits", move || {
            let image_part = reqwest::multipart::Part::bytes(image_data.clone())
                .file_name(image_filename.clone());
            let mut form = reqwest::multipart::Form::new()
                .part("image", image_part)
                .text("model", model.clone())
                .text("prompt", prompt.clone());

            if let Some(ref mask) = mask_data {
                let mask_part = reqwest::multipart::Part::bytes(mask.clone())
                    .file_name(mask_filename.clone().unwrap_or_else(|| "mask.png".into()));
                form = form.part("mask", mask_part);
            }

            if let Some(n) = n {
                form = form.text("n", n.to_string());
            }
            if let Some(size) = size {
                form = form.text("size", size.as_str());
            }
            if let Some(quality) = quality {
                form = form.text("quality", quality.as_str());
            }
            if let Some(fmt) = response_format {
                form = form.text("response_format", fmt.as_str());
            }

            form
        })
        .await?;

    let body = response.json::<ImageResponse>().await?;
    Ok(body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_API_KEY, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // =======================================================================
    // Phase 3: Image Generation
    // =======================================================================

    // --- Cycle 3.1: ImageGenerationRequest builder ---

    #[test]
    fn test_image_generation_request_builder() {
        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A sunset")
            .build();

        assert_eq!(request.model, "dall-e-3");
        assert_eq!(request.prompt, "A sunset");
        assert!(request.n.is_none());
        assert!(request.size.is_none());
        assert!(request.quality.is_none());
        assert!(request.response_format.is_none());
        assert!(request.output_format.is_none());
    }

    #[test]
    fn test_image_generation_request_builder_all_fields() {
        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A sunset")
            .n(2)
            .size(ImageSize::S1024x1024)
            .quality(ImageQuality::Hd)
            .response_format(ImageResponseFormat::Url)
            .output_format(ImageOutputFormat::Png)
            .build();

        assert_eq!(request.n, Some(2));
        assert_eq!(request.size, Some(ImageSize::S1024x1024));
        assert_eq!(request.quality, Some(ImageQuality::Hd));
        assert_eq!(request.response_format, Some(ImageResponseFormat::Url));
        assert_eq!(request.output_format, Some(ImageOutputFormat::Png));
    }

    // --- Cycle 3.2: ImageSize serde ---

    #[test]
    fn test_image_size_serialization() {
        assert_eq!(
            serde_json::to_string(&ImageSize::S256x256).unwrap(),
            "\"256x256\""
        );
        assert_eq!(
            serde_json::to_string(&ImageSize::S512x512).unwrap(),
            "\"512x512\""
        );
        assert_eq!(
            serde_json::to_string(&ImageSize::S1024x1024).unwrap(),
            "\"1024x1024\""
        );
        assert_eq!(
            serde_json::to_string(&ImageSize::S1536x1024).unwrap(),
            "\"1536x1024\""
        );
        assert_eq!(
            serde_json::to_string(&ImageSize::S1024x1536).unwrap(),
            "\"1024x1536\""
        );
        assert_eq!(serde_json::to_string(&ImageSize::Auto).unwrap(), "\"auto\"");
    }

    // --- Cycle 3.3: ImageQuality and ImageResponseFormat serde ---

    #[test]
    fn test_image_quality_serialization() {
        assert_eq!(
            serde_json::to_string(&ImageQuality::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(serde_json::to_string(&ImageQuality::Hd).unwrap(), "\"hd\"");
        assert_eq!(
            serde_json::to_string(&ImageQuality::Low).unwrap(),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&ImageQuality::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&ImageQuality::High).unwrap(),
            "\"high\""
        );
        assert_eq!(
            serde_json::to_string(&ImageQuality::Auto).unwrap(),
            "\"auto\""
        );
    }

    #[test]
    fn test_image_response_format_serialization() {
        assert_eq!(
            serde_json::to_string(&ImageResponseFormat::Url).unwrap(),
            "\"url\""
        );
        assert_eq!(
            serde_json::to_string(&ImageResponseFormat::B64Json).unwrap(),
            "\"b64_json\""
        );
    }

    #[test]
    fn test_image_output_format_serialization() {
        assert_eq!(
            serde_json::to_string(&ImageOutputFormat::Png).unwrap(),
            "\"png\""
        );
        assert_eq!(
            serde_json::to_string(&ImageOutputFormat::Jpeg).unwrap(),
            "\"jpeg\""
        );
        assert_eq!(
            serde_json::to_string(&ImageOutputFormat::Webp).unwrap(),
            "\"webp\""
        );
    }

    // --- Cycle 3.4: ImageGenerationRequest serialization ---

    #[test]
    fn test_image_generation_request_serialization() {
        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A sunset")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "dall-e-3");
        assert_eq!(json["prompt"], "A sunset");
        assert!(json.get("n").is_none());
        assert!(json.get("size").is_none());
    }

    #[test]
    fn test_image_generation_request_serialization_all_fields() {
        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A sunset")
            .n(2)
            .size(ImageSize::S1024x1024)
            .quality(ImageQuality::Hd)
            .response_format(ImageResponseFormat::B64Json)
            .output_format(ImageOutputFormat::Webp)
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["n"], 2);
        assert_eq!(json["size"], "1024x1024");
        assert_eq!(json["quality"], "hd");
        assert_eq!(json["response_format"], "b64_json");
        assert_eq!(json["output_format"], "webp");
    }

    // --- Cycle 3.5: Builder validation ---

    #[test]
    fn test_image_generation_rejects_empty_prompt() {
        let result = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("prompt cannot be empty"));
    }

    #[test]
    fn test_image_generation_rejects_empty_model() {
        let result = ImageGenerationRequest::builder()
            .model("")
            .prompt("A sunset")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model cannot be empty"));
    }

    #[test]
    fn test_image_generation_validates_n() {
        let zero = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("test")
            .n(0)
            .try_build();
        assert!(zero.is_err());
        assert!(zero
            .unwrap_err()
            .to_string()
            .contains("n must be between 1 and 10"));

        let eleven = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("test")
            .n(11)
            .try_build();
        assert!(eleven.is_err());

        let valid = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("test")
            .n(5)
            .try_build();
        assert!(valid.is_ok());
    }

    // --- Cycle 3.6: Response types ---

    #[test]
    fn test_image_response_deserialization() {
        let json = serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{
                "url": "https://example.com/image.png",
                "revised_prompt": "A beautiful sunset over mountains"
            }]
        });

        let response: ImageResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.created, TEST_TIMESTAMP);
        assert_eq!(response.data.len(), 1);
        assert_eq!(
            response.data[0].url,
            Some("https://example.com/image.png".into())
        );
        assert_eq!(
            response.data[0].revised_prompt,
            Some("A beautiful sunset over mountains".into())
        );
        assert!(response.data[0].b64_json.is_none());
    }

    #[test]
    fn test_image_response_deserialization_b64() {
        let json = serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{
                "b64_json": "iVBORw0KGgo=",
                "revised_prompt": "An image"
            }]
        });

        let response: ImageResponse = serde_json::from_value(json).unwrap();

        assert!(response.data[0].url.is_none());
        assert_eq!(response.data[0].b64_json, Some("iVBORw0KGgo=".into()));
    }

    // --- Cycle 3.7: generate() API function ---

    #[tokio::test]
    async fn test_generate_image_success() {
        let server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "model": "dall-e-3",
            "prompt": "A sunset"
        });

        let response_body = serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{
                "url": "https://example.com/image.png",
                "revised_prompt": "A sunset over mountains"
            }]
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/generations"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .and(header("content-type", "application/json"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A sunset")
            .build();

        let response = generate(&client, &request).await.expect("should succeed");

        assert_eq!(response.created, TEST_TIMESTAMP);
        assert_eq!(response.data.len(), 1);
        assert!(response.data[0].url.is_some());
    }

    // --- Cycle 3.8: generate() error handling ---

    #[tokio::test]
    async fn test_generate_image_returns_error_on_400() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidRequest",
                "message": "Invalid prompt"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/generations"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("test")
            .build();

        let result = generate(&client, &request).await;

        assert!(result.is_err());
    }

    // --- Cycle 3.9: ImageEditRequest builder ---

    #[test]
    fn test_image_edit_request_builder() {
        let request = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![1, 2, 3], "input.png")
            .prompt("Add a rainbow")
            .build();

        assert_eq!(request.model, "dall-e-2");
        assert_eq!(request.image, vec![1, 2, 3]);
        assert_eq!(request.image_filename, "input.png");
        assert_eq!(request.prompt, "Add a rainbow");
        assert!(request.mask.is_none());
        assert!(request.mask_filename.is_none());
        assert!(request.n.is_none());
    }

    #[test]
    fn test_image_edit_request_builder_with_mask() {
        let request = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![1, 2, 3], "input.png")
            .prompt("Edit")
            .mask(vec![4, 5, 6], "mask.png")
            .n(2)
            .size(ImageSize::S512x512)
            .build();

        assert_eq!(request.mask, Some(vec![4, 5, 6]));
        assert_eq!(request.mask_filename, Some("mask.png".into()));
        assert_eq!(request.n, Some(2));
        assert_eq!(request.size, Some(ImageSize::S512x512));
    }

    #[test]
    fn test_image_edit_request_rejects_empty_image() {
        let result = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![], "input.png")
            .prompt("Edit")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("image data cannot be empty"));
    }

    #[test]
    fn test_image_edit_request_rejects_empty_prompt() {
        let result = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![1], "input.png")
            .prompt("")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("prompt cannot be empty"));
    }

    // --- Cycle 3.10: edit() API function ---

    #[tokio::test]
    async fn test_edit_image_success() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{
                "url": "https://example.com/edited.png",
                "revised_prompt": "An edited image"
            }]
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/edits"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![0u8; 100], "input.png")
            .prompt("Add a rainbow")
            .build();

        let response = edit(&client, &request).await.expect("should succeed");

        assert_eq!(response.created, TEST_TIMESTAMP);
        assert_eq!(response.data.len(), 1);
        assert!(response.data[0].url.is_some());
    }

    #[tokio::test]
    async fn test_edit_image_returns_error_on_400() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidRequest",
                "message": "Image format not supported"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/edits"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![0u8; 100], "input.png")
            .prompt("Add a rainbow")
            .build();

        let result = edit(&client, &request).await;

        assert!(result.is_err());
    }

    // --- Quality fixes: as_str() tests ---

    #[test]
    fn test_image_size_as_str() {
        assert_eq!(ImageSize::S256x256.as_str(), "256x256");
        assert_eq!(ImageSize::S512x512.as_str(), "512x512");
        assert_eq!(ImageSize::S1024x1024.as_str(), "1024x1024");
        assert_eq!(ImageSize::S1536x1024.as_str(), "1536x1024");
        assert_eq!(ImageSize::S1024x1536.as_str(), "1024x1536");
        assert_eq!(ImageSize::Auto.as_str(), "auto");
    }

    #[test]
    fn test_image_quality_as_str() {
        assert_eq!(ImageQuality::Standard.as_str(), "standard");
        assert_eq!(ImageQuality::Hd.as_str(), "hd");
        assert_eq!(ImageQuality::Low.as_str(), "low");
        assert_eq!(ImageQuality::Medium.as_str(), "medium");
        assert_eq!(ImageQuality::High.as_str(), "high");
        assert_eq!(ImageQuality::Auto.as_str(), "auto");
    }

    #[test]
    fn test_image_response_format_as_str() {
        assert_eq!(ImageResponseFormat::Url.as_str(), "url");
        assert_eq!(ImageResponseFormat::B64Json.as_str(), "b64_json");
    }

    // --- Quality fixes: image_filename validation ---

    #[test]
    fn test_image_edit_request_rejects_empty_image_filename() {
        let result = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![1, 2, 3], "")
            .prompt("Edit this image")
            .try_build();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("image filename cannot be empty"),
            "error should mention empty filename"
        );
    }

    // --- Quality fixes: tracing span tests ---

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_generate_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": TEST_TIMESTAMP,
                "data": [{"url": "https://example.com/img.png"}]
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageGenerationRequest::builder()
            .model("dall-e-3")
            .prompt("A mountain")
            .build();

        let _ = generate(&client, &request).await;

        assert!(logs_contain("foundry::images::generate"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_edit_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/images/edits"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": TEST_TIMESTAMP,
                "data": [{"url": "https://example.com/edited.png"}]
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ImageEditRequest::builder()
            .model("dall-e-2")
            .image(vec![0u8; 10], "img.png")
            .prompt("Add a hat")
            .build();

        let _ = edit(&client, &request).await;

        assert!(logs_contain("foundry::images::edit"));
    }
}
