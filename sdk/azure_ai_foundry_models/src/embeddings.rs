//! Embeddings types and API calls for Azure AI Foundry Models.
//!
//! This module provides the embeddings API for converting text into
//! vector representations for semantic search and similarity comparisons.
//!
//! # Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::embeddings::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = EmbeddingRequest::builder()
//!     .model("text-embedding-ada-002")
//!     .input("Hello, world!")
//!     .build();
//!
//! let response = embed(client, &request).await?;
//! println!("Embedding has {} dimensions", response.data[0].embedding.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Batch Processing
//!
//! For processing multiple texts at once, use the `inputs` method:
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::embeddings::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = EmbeddingRequest::builder()
//!     .model("text-embedding-3-small")
//!     .inputs(vec!["First text", "Second text", "Third text"])
//!     .dimensions(512)
//!     .build();
//!
//! let response = embed(client, &request).await?;
//! for embedding in &response.data {
//!     println!("Index {}: {} dimensions", embedding.index, embedding.embedding.len());
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// An embedding request.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: EmbeddingInput,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EncodingFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Input for an embedding request.
///
/// Can be a single string or multiple strings for batch processing.
#[derive(Debug, Clone)]
pub enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

impl Serialize for EmbeddingInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Single(s) => s.serialize(serializer),
            Self::Multiple(v) => v.serialize(serializer),
        }
    }
}

/// Encoding format for embeddings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EncodingFormat {
    Float,
    Base64,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// An embedding response.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub model: String,
    pub data: Vec<EmbeddingData>,
    pub usage: EmbeddingUsage,
}

/// A single embedding in the response.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    pub index: u32,
    pub embedding: Vec<f32>,
}

/// Usage statistics for an embedding request.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Send an embedding request.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::embeddings::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = EmbeddingRequest::builder()
///     .model("text-embedding-ada-002")
///     .input("The quick brown fox jumps over the lazy dog")
///     .build();
///
/// let response = embed(client, &request).await?;
/// let embedding = &response.data[0].embedding;
/// println!("Generated {} dimensional embedding", embedding.len());
/// # Ok(())
/// # }
/// ```
pub async fn embed(
    client: &FoundryClient,
    request: &EmbeddingRequest,
) -> FoundryResult<EmbeddingResponse> {
    let response = client.post("/openai/v1/embeddings", request).await?;
    let body = response.json::<EmbeddingResponse>().await?;
    Ok(body)
}

/// Builder for [`EmbeddingRequest`].
pub struct EmbeddingRequestBuilder {
    model: Option<String>,
    input: Option<EmbeddingInput>,
    dimensions: Option<u32>,
    encoding_format: Option<EncodingFormat>,
    user: Option<String>,
}

impl EmbeddingRequest {
    /// Create a new builder.
    pub fn builder() -> EmbeddingRequestBuilder {
        EmbeddingRequestBuilder {
            model: None,
            input: None,
            dimensions: None,
            encoding_format: None,
            user: None,
        }
    }
}

impl EmbeddingRequestBuilder {
    /// Set the model ID to use for embedding generation.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set a single text input to embed.
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(EmbeddingInput::Single(input.into()));
        self
    }

    /// Set multiple text inputs for batch embedding.
    ///
    /// Accepts any type that implements `IntoIterator`, including `Vec`, arrays, and slices.
    pub fn inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.input = Some(EmbeddingInput::Multiple(
            inputs.into_iter().map(Into::into).collect(),
        ));
        self
    }

    /// Set the number of dimensions for the output embeddings.
    ///
    /// Only supported by some models (e.g., `text-embedding-3-small`).
    pub fn dimensions(mut self, dimensions: u32) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Set the encoding format for the embeddings.
    ///
    /// Defaults to `Float` if not specified.
    pub fn encoding_format(mut self, format: EncodingFormat) -> Self {
        self.encoding_format = Some(format);
        self
    }

    /// Set a unique identifier for the end-user.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Build the request, returning an error if required fields are missing.
    pub fn try_build(self) -> FoundryResult<EmbeddingRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        let input = self
            .input
            .ok_or_else(|| FoundryError::Builder("input is required".into()))?;

        Ok(EmbeddingRequest {
            model,
            input,
            dimensions: self.dimensions,
            encoding_format: self.encoding_format,
            user: self.user,
        })
    }

    /// Build the request. Panics if `model` or `input` is not set.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> EmbeddingRequest {
        self.try_build().expect("builder validation failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Ciclo 1: Builder with required fields only ---

    #[test]
    fn test_builder_with_required_fields_only() {
        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("Hello, world!")
            .build();

        assert_eq!(request.model, "text-embedding-ada-002");
        // Input should be Single variant
        match &request.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello, world!"),
            EmbeddingInput::Multiple(_) => panic!("Expected Single, got Multiple"),
        }
        assert!(request.dimensions.is_none());
        assert!(request.encoding_format.is_none());
        assert!(request.user.is_none());
    }

    // --- Ciclo 2: Multiple inputs support ---

    #[test]
    fn test_builder_with_multiple_inputs() {
        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .inputs(vec!["Hello", "World", "Rust"])
            .build();

        assert_eq!(request.model, "text-embedding-ada-002");
        match &request.input {
            EmbeddingInput::Multiple(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], "Hello");
                assert_eq!(v[1], "World");
                assert_eq!(v[2], "Rust");
            }
            EmbeddingInput::Single(_) => panic!("Expected Multiple, got Single"),
        }
    }

    // --- Ciclo 3: Optional parameters ---

    #[test]
    fn test_builder_with_all_optional_fields() {
        let request = EmbeddingRequest::builder()
            .model("text-embedding-3-small")
            .input("Test")
            .dimensions(1536)
            .encoding_format(EncodingFormat::Float)
            .user("test-user-123")
            .build();

        assert_eq!(request.dimensions, Some(1536));
        assert_eq!(request.encoding_format, Some(EncodingFormat::Float));
        assert_eq!(request.user, Some("test-user-123".into()));
    }

    // --- Ciclo 4: Request serialization skips None fields ---

    #[test]
    fn test_request_serialization_skips_none_fields() {
        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("Hello")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "text-embedding-ada-002");
        assert_eq!(json["input"], "Hello");
        assert!(json.get("dimensions").is_none());
        assert!(json.get("encoding_format").is_none());
        assert!(json.get("user").is_none());
    }

    // --- Ciclo 5: Request serialization includes set fields ---

    #[test]
    fn test_request_serialization_includes_set_fields() {
        let request = EmbeddingRequest::builder()
            .model("text-embedding-3-small")
            .inputs(vec!["Hello", "World"])
            .dimensions(512)
            .encoding_format(EncodingFormat::Base64)
            .user("user-456")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "text-embedding-3-small");
        assert_eq!(json["input"], serde_json::json!(["Hello", "World"]));
        assert_eq!(json["dimensions"], 512);
        assert_eq!(json["encoding_format"], "base64");
        assert_eq!(json["user"], "user-456");
    }

    // --- Ciclo 6: Response deserialization (single input) ---

    #[test]
    fn test_response_deserialization_single_input() {
        let json = serde_json::json!({
            "object": "list",
            "model": "text-embedding-ada-002",
            "data": [{
                "index": 0,
                "embedding": [0.1, 0.2, 0.3, 0.4, 0.5],
                "object": "embedding"
            }],
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        });

        let response: EmbeddingResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.model, "text-embedding-ada-002");
        assert_eq!(response.object, "list");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].index, 0);
        assert_eq!(response.data[0].embedding.len(), 5);
        assert!((response.data[0].embedding[0] - 0.1).abs() < f32::EPSILON);
        assert_eq!(response.usage.prompt_tokens, 5);
        assert_eq!(response.usage.total_tokens, 5);
    }

    // --- Ciclo 7: Response deserialization (multiple inputs) ---

    #[test]
    fn test_response_deserialization_multiple_inputs() {
        let json = serde_json::json!({
            "object": "list",
            "model": "text-embedding-3-small",
            "data": [
                {"index": 0, "embedding": [0.1, 0.2], "object": "embedding"},
                {"index": 1, "embedding": [0.3, 0.4], "object": "embedding"},
                {"index": 2, "embedding": [0.5, 0.6], "object": "embedding"}
            ],
            "usage": {
                "prompt_tokens": 15,
                "total_tokens": 15
            }
        });

        let response: EmbeddingResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.data.len(), 3);
        assert_eq!(response.data[0].index, 0);
        assert_eq!(response.data[1].index, 1);
        assert_eq!(response.data[2].index, 2);
        assert!(!response.data[0].embedding.is_empty());
        assert!(!response.data[1].embedding.is_empty());
        assert!(!response.data[2].embedding.is_empty());
    }

    // --- Ciclo 8: Basic embed() function ---

    use crate::test_utils::setup_mock_client;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_embed_single_input_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "object": "list",
            "model": "text-embedding-ada-002",
            "data": [{
                "index": 0,
                "embedding": [0.1, 0.2, 0.3],
                "object": "embedding"
            }],
            "usage": {
                "prompt_tokens": 3,
                "total_tokens": 3
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/embeddings"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("Hello, world!")
            .build();

        let response = embed(&client, &request).await.expect("should succeed");

        assert_eq!(response.model, "text-embedding-ada-002");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding.len(), 3);
    }

    // --- Ciclo 9: Error handling for invalid model ---

    #[tokio::test]
    async fn test_embed_invalid_model_error() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "ModelNotFound",
                "message": "The model 'nonexistent' does not exist"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/embeddings"))
            .respond_with(ResponseTemplate::new(404).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = EmbeddingRequest::builder()
            .model("nonexistent")
            .input("Hello")
            .build();

        let result = embed(&client, &request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FoundryError::Api { code, message } => {
                assert_eq!(code, "ModelNotFound");
                assert!(message.contains("does not exist"));
            }
            other => panic!("Expected Api error, got {:?}", other),
        }
    }

    // --- Ciclo 10: Error handling for rate limiting ---

    #[tokio::test]
    async fn test_embed_rate_limit_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/embeddings"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("Hello")
            .build();

        let result = embed(&client, &request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FoundryError::Http { status, message } => {
                assert_eq!(status, 429);
                assert!(message.contains("Rate limit"));
            }
            other => panic!("Expected Http error, got {:?}", other),
        }
    }

    // --- Ciclo 11: Test with all request parameters ---

    #[tokio::test]
    async fn test_embed_with_all_parameters() {
        let server = MockServer::start().await;

        let expected_request = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": ["Hello", "World"],
            "dimensions": 512,
            "encoding_format": "float",
            "user": "user-123"
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/embeddings"))
            .and(body_json(&expected_request))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "model": "text-embedding-3-small",
                "data": [
                    {"index": 0, "embedding": [0.1], "object": "embedding"},
                    {"index": 1, "embedding": [0.2], "object": "embedding"}
                ],
                "usage": {"prompt_tokens": 2, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = EmbeddingRequest::builder()
            .model("text-embedding-3-small")
            .inputs(vec!["Hello", "World"])
            .dimensions(512)
            .encoding_format(EncodingFormat::Float)
            .user("user-123")
            .build();

        let response = embed(&client, &request).await.expect("should succeed");
        assert_eq!(response.data.len(), 2);
    }

    // --- Ciclo 12: EncodingFormat serialization ---

    #[test]
    fn test_encoding_format_serialization() {
        assert_eq!(serde_json::to_string(&EncodingFormat::Float).unwrap(), "\"float\"");
        assert_eq!(serde_json::to_string(&EncodingFormat::Base64).unwrap(), "\"base64\"");
    }

    // --- Ciclo 13: EmbeddingInput single serialization ---

    #[test]
    fn test_embedding_input_single_serialization() {
        let input = EmbeddingInput::Single("test".into());
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json, "test");
    }

    // --- Ciclo 14: EmbeddingInput multiple serialization ---

    #[test]
    fn test_embedding_input_multiple_serialization() {
        let input = EmbeddingInput::Multiple(vec!["a".into(), "b".into()]);
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json, serde_json::json!(["a", "b"]));
    }

    // --- Ciclo 15: Builder panic when model missing ---

    #[test]
    #[should_panic(expected = "model is required")]
    fn test_builder_without_model_panics() {
        EmbeddingRequest::builder()
            .input("Hello")
            .build();
    }

    // --- Ciclo 16: Builder panic when input missing ---

    #[test]
    #[should_panic(expected = "input is required")]
    fn test_builder_without_input_panics() {
        EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .build();
    }

    // --- try_build tests ---

    #[test]
    fn test_try_build_returns_error_when_model_missing() {
        let result = EmbeddingRequest::builder()
            .input("Hello")
            .try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, azure_ai_foundry_core::error::FoundryError::Builder(_)));
        assert!(err.to_string().contains("model"));
    }

    #[test]
    fn test_try_build_returns_error_when_input_missing() {
        let result = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, azure_ai_foundry_core::error::FoundryError::Builder(_)));
        assert!(err.to_string().contains("input"));
    }

    #[test]
    fn test_try_build_success() {
        let result = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("Hello")
            .try_build();

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.model, "text-embedding-ada-002");
    }

    // --- API consistency: inputs() accepts IntoIterator ---

    #[test]
    fn test_inputs_accepts_iterator() {
        // Should accept any IntoIterator, not just Vec
        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .inputs(["Hello", "World"])
            .build();

        match &request.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected Multiple"),
        }
    }

    #[test]
    fn test_inputs_accepts_slice() {
        let texts = ["a", "b", "c"];

        let request = EmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .inputs(texts)
            .build();

        match &request.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 3),
            _ => panic!("Expected Multiple"),
        }
    }
}
