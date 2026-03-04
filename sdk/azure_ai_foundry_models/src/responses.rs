//! Responses API types and functions for Azure AI Foundry Models.
//!
//! The Responses API is a unified interface for model interactions that
//! supports chaining, built-in tools, and structured outputs.
//!
//! # Create a Response
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::responses::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = CreateResponseRequest::builder()
//!     .model("gpt-4o")
//!     .input("What is Rust?")
//!     .build();
//!
//! let response = create(client, &request).await?;
//! if let Some(text) = response.output_text() {
//!     println!("{}", text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Get a Previous Response
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::responses;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let response = responses::get(client, "resp_abc123").await?;
//! println!("Status: {:?}", response.status);
//! # Ok(())
//! # }
//! ```
//!
//! # Delete a Response
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::responses;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let result = responses::delete(client, "resp_abc123").await?;
//! if result.deleted {
//!     println!("Response deleted");
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chat::Role;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Content type identifier for text output blocks in a Response.
///
/// Retained for backward compatibility. Prefer matching against
/// [`ResponseContentType::OutputText`] for exhaustive pattern matching.
pub const OUTPUT_TEXT_TYPE: &str = "output_text";

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input for a response request.
///
/// Can be a simple text string or a list of structured messages.
#[derive(Debug, Clone)]
pub enum ResponseInput {
    /// A simple text prompt.
    Text(String),
    /// A list of structured messages.
    Messages(Vec<ResponseMessage>),
}

impl Serialize for ResponseInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Text(s) => s.serialize(serializer),
            Self::Messages(msgs) => msgs.serialize(serializer),
        }
    }
}

/// A message in a response input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    /// The role of the message author.
    pub role: Role,
    /// The text content of the message.
    ///
    /// # Limitation
    ///
    /// This field is currently a plain `String` and does not support multimodal content
    /// (e.g., image URLs, tool call results). Full multimodal input support is planned
    /// for a future release.
    pub content: String,
}

impl ResponseMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A request to create a response.
#[derive(Debug, Clone, Serialize)]
pub struct CreateResponseRequest {
    /// The model to use.
    pub model: String,
    /// The input for the response.
    pub input: ResponseInput,

    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Maximum number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// The ID of a previous response to continue from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
}

impl CreateResponseRequest {
    /// Create a new builder.
    pub fn builder() -> CreateResponseRequestBuilder {
        CreateResponseRequestBuilder {
            model: None,
            input: None,
            temperature: None,
            top_p: None,
            max_output_tokens: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            previous_response_id: None,
        }
    }
}

/// Builder for [`CreateResponseRequest`].
#[derive(Debug)]
pub struct CreateResponseRequestBuilder {
    model: Option<String>,
    input: Option<ResponseInput>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_output_tokens: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<Vec<String>>,
    previous_response_id: Option<String>,
}

impl CreateResponseRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set a simple text input.
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(ResponseInput::Text(input.into()));
        self
    }

    /// Set structured message input.
    pub fn messages(mut self, messages: Vec<ResponseMessage>) -> Self {
        self.input = Some(ResponseInput::Messages(messages));
        self
    }

    /// Set the sampling temperature (0.0 to 2.0).
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the nucleus sampling parameter (0.0 to 1.0).
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the maximum number of output tokens.
    pub fn max_output_tokens(mut self, max_tokens: u32) -> Self {
        self.max_output_tokens = Some(max_tokens);
        self
    }

    /// Set the frequency penalty (-2.0 to 2.0).
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty (-2.0 to 2.0).
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences.
    ///
    /// Accepts any iterable of string-like values, including `Vec<String>`,
    /// `&[&str]`, arrays, or any iterator yielding `impl Into<String>`.
    pub fn stop(mut self, stop: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stop = Some(stop.into_iter().map(Into::into).collect());
        self
    }

    /// Set the ID of a previous response to continue from.
    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.previous_response_id = Some(id.into());
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<CreateResponseRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.trim().is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let input = self
            .input
            .ok_or_else(|| FoundryError::Builder("input is required".into()))?;

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(FoundryError::Builder(
                    "temperature must be between 0.0 and 2.0".into(),
                ));
            }
        }

        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(FoundryError::Builder(
                    "top_p must be between 0.0 and 1.0".into(),
                ));
            }
        }

        if let Some(fp) = self.frequency_penalty {
            if !(-2.0..=2.0).contains(&fp) {
                return Err(FoundryError::Builder(
                    "frequency_penalty must be between -2.0 and 2.0".into(),
                ));
            }
        }

        if let Some(pp) = self.presence_penalty {
            if !(-2.0..=2.0).contains(&pp) {
                return Err(FoundryError::Builder(
                    "presence_penalty must be between -2.0 and 2.0".into(),
                ));
            }
        }

        if let Some(ref prev_id) = self.previous_response_id {
            if prev_id.trim().is_empty() {
                return Err(FoundryError::Builder(
                    "previous_response_id cannot be empty or whitespace".into(),
                ));
            }
        }

        Ok(CreateResponseRequest {
            model,
            input,
            temperature: self.temperature,
            top_p: self.top_p,
            max_output_tokens: self.max_output_tokens,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            stop: self.stop,
            previous_response_id: self.previous_response_id,
        })
    }

    /// Build the request.
    ///
    /// # Panics
    ///
    /// Panics if `model` or `input` is not set, or if parameter values
    /// (`temperature`, `top_p`, `frequency_penalty`, `presence_penalty`) are out
    /// of range. Use [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> CreateResponseRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// The status of a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    /// The response is complete.
    Completed,
    /// The response failed.
    Failed,
    /// The response is still being generated.
    InProgress,
    /// The response was cancelled.
    Cancelled,
}

/// A response from the Responses API.
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    /// Unique identifier for the response.
    pub id: String,
    /// Object type, always "response".
    pub object: String,
    /// Unix timestamp when the response was created.
    pub created_at: u64,
    /// The status of the response.
    pub status: ResponseStatus,
    /// The model used to generate the response.
    pub model: String,
    /// The output items of the response.
    pub output: Vec<ResponseOutput>,
    /// Token usage statistics.
    pub usage: Option<ResponseUsage>,
    /// Metadata associated with the response.
    pub metadata: Option<HashMap<String, String>>,
}

impl Response {
    /// Extract the first text output from the response.
    ///
    /// Convenience method that searches through the output items
    /// for the first text content block and returns its text.
    pub fn output_text(&self) -> Option<&str> {
        for output in &self.output {
            if let Some(ref content) = output.content {
                for c in content {
                    if c.content_type == ResponseContentType::OutputText {
                        if let Some(ref text) = c.text {
                            return Some(text.as_str());
                        }
                    }
                }
            }
        }
        None
    }
}

/// The type of a response output item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseOutputType {
    /// A message output containing text or other content blocks.
    Message,
    /// An unknown output type returned by the API (forward-compatibility).
    #[serde(other)]
    Other,
}

/// An output item in a response.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseOutput {
    /// Unique identifier for the output item.
    pub id: String,
    /// The type of output item.
    #[serde(rename = "type")]
    pub output_type: ResponseOutputType,
    /// The role of the output (e.g., `Role::Assistant`).
    pub role: Option<crate::chat::Role>,
    /// The content blocks of the output.
    pub content: Option<Vec<ResponseContent>>,
}

/// The type of a content block within a response output item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseContentType {
    /// Plain text output.
    OutputText,
    /// An unknown content type returned by the API (forward-compatibility).
    #[serde(other)]
    Other,
}

/// A content block within a response output.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseContent {
    /// The type of content.
    #[serde(rename = "type")]
    pub content_type: ResponseContentType,
    /// The text content, if this is a text block.
    pub text: Option<String>,
}

/// Token usage statistics for a response.
///
/// This type is intentionally separate from [`azure_ai_foundry_core::models::Usage`]
/// because the Responses API uses different field names at the wire level:
/// `input_tokens`/`output_tokens` instead of `prompt_tokens`/`completion_tokens`.
/// Unifying the two types would require a custom deserializer or breaking field names.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseUsage {
    /// Number of tokens in the input.
    pub input_tokens: u32,
    /// Number of tokens in the output.
    pub output_tokens: u32,
    /// Total number of tokens used.
    pub total_tokens: u32,
}

/// Response from deleting a response.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseDeletionResponse {
    /// ID of the deleted response.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Whether the deletion was successful.
    pub deleted: bool,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Create a new response.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::responses::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = CreateResponseRequest::builder()
///     .model("gpt-4o")
///     .input("Tell me a joke")
///     .build();
///
/// let response = create(client, &request).await?;
/// println!("Response: {:?}", response.output_text());
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::responses::create` with field `model`.
#[tracing::instrument(
    name = "foundry::responses::create",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn create(
    client: &FoundryClient,
    request: &CreateResponseRequest,
) -> FoundryResult<Response> {
    tracing::debug!("sending create response request");

    let response = client.post("/openai/v1/responses", request).await?;
    let body = response.json::<Response>().await?;
    Ok(body)
}

/// Get a previously created response by ID.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::responses;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let response = responses::get(client, "resp_abc123").await?;
/// println!("Model: {}", response.model);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::responses::get` with field `response_id`.
#[tracing::instrument(
    name = "foundry::responses::get",
    skip(client),
    fields(response_id = %response_id)
)]
pub async fn get(client: &FoundryClient, response_id: &str) -> FoundryResult<Response> {
    tracing::debug!("getting response");
    FoundryClient::validate_resource_id(response_id)?;

    let path = format!("/openai/v1/responses/{}", response_id);
    let response = client.get(&path).await?;
    let body = response.json::<Response>().await?;
    Ok(body)
}

/// Delete a response by ID.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::responses;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = responses::delete(client, "resp_abc123").await?;
/// if result.deleted {
///     println!("Deleted");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::responses::delete` with field `response_id`.
#[tracing::instrument(
    name = "foundry::responses::delete",
    skip(client),
    fields(response_id = %response_id)
)]
pub async fn delete(
    client: &FoundryClient,
    response_id: &str,
) -> FoundryResult<ResponseDeletionResponse> {
    tracing::debug!("deleting response");
    FoundryClient::validate_resource_id(response_id)?;

    let path = format!("/openai/v1/responses/{}", response_id);
    let response = client.delete(&path).await?;
    let body = response.json::<ResponseDeletionResponse>().await?;
    Ok(body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_API_KEY};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // =======================================================================
    // Phase 4: Responses API
    // =======================================================================

    // --- Cycle 4.1: ResponseInput enum ---

    #[test]
    fn test_response_input_text_serialization() {
        let input = ResponseInput::Text("Hello".into());
        let json = serde_json::to_value(&input).unwrap();

        assert_eq!(json, "Hello");
    }

    #[test]
    fn test_response_input_messages_serialization() {
        let input = ResponseInput::Messages(vec![
            ResponseMessage::system("You are helpful"),
            ResponseMessage::user("Hi"),
        ]);
        let json = serde_json::to_value(&input).unwrap();

        assert!(json.is_array());
        assert_eq!(json[0]["role"], "system");
        assert_eq!(json[0]["content"], "You are helpful");
        assert_eq!(json[1]["role"], "user");
        assert_eq!(json[1]["content"], "Hi");
    }

    // --- Cycle 4.2: CreateResponseRequest builder ---

    #[test]
    fn test_create_response_request_builder() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .build();

        assert_eq!(request.model, "gpt-4o");
        assert!(request.temperature.is_none());
        assert!(request.top_p.is_none());
        assert!(request.max_output_tokens.is_none());
        assert!(request.frequency_penalty.is_none());
        assert!(request.presence_penalty.is_none());
        assert!(request.stop.is_none());
        assert!(request.previous_response_id.is_none());
    }

    #[test]
    fn test_create_response_request_builder_with_messages() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .messages(vec![
                ResponseMessage::system("Be concise"),
                ResponseMessage::user("Hello"),
            ])
            .build();

        match &request.input {
            ResponseInput::Messages(msgs) => {
                assert_eq!(msgs.len(), 2);
                assert_eq!(msgs[0].role, crate::chat::Role::System);
                assert_eq!(msgs[1].role, crate::chat::Role::User);
            }
            ResponseInput::Text(_) => panic!("Expected Messages, got Text"),
        }
    }

    #[test]
    fn test_create_response_request_builder_all_fields() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .temperature(0.7)
            .top_p(0.9)
            .max_output_tokens(1000)
            .frequency_penalty(0.5)
            .presence_penalty(-0.5)
            .stop(["END"])
            .previous_response_id("resp_prev123")
            .build();

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.max_output_tokens, Some(1000));
        assert_eq!(request.frequency_penalty, Some(0.5));
        assert_eq!(request.presence_penalty, Some(-0.5));
        assert_eq!(request.stop, Some(vec!["END".to_string()]));
        assert_eq!(request.previous_response_id, Some("resp_prev123".into()));
    }

    // --- Cycle 4.3: CreateResponseRequest serialization ---

    #[test]
    fn test_create_response_request_serialization() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["input"], "Hello");
        assert!(json.get("temperature").is_none());
        assert!(json.get("top_p").is_none());
        assert!(json.get("max_output_tokens").is_none());
    }

    #[test]
    fn test_create_response_request_serialization_all_fields() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .temperature(0.5)
            .top_p(0.5)
            .max_output_tokens(500)
            .frequency_penalty(0.25)
            .presence_penalty(-0.25)
            .stop(["END"])
            .previous_response_id("resp_prev")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["top_p"], 0.5);
        assert_eq!(json["max_output_tokens"], 500);
        assert_eq!(json["frequency_penalty"], 0.25);
        assert_eq!(json["presence_penalty"], -0.25);
        assert_eq!(json["stop"], serde_json::json!(["END"]));
        assert!(
            json.get("stream").is_none(),
            "stream field should not exist"
        );
        assert_eq!(json["previous_response_id"], "resp_prev");
    }

    // --- Cycle 4.4: Builder validation ---

    #[test]
    fn test_create_response_rejects_empty_model() {
        let result = CreateResponseRequest::builder()
            .model("")
            .input("Hello")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model cannot be empty"));
    }

    #[test]
    fn test_create_response_rejects_missing_input() {
        let result = CreateResponseRequest::builder().model("gpt-4o").try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("input is required"));
    }

    #[test]
    fn test_create_response_validates_temperature() {
        let too_high = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .temperature(2.5)
            .try_build();

        assert!(too_high.is_err());
        assert!(too_high
            .unwrap_err()
            .to_string()
            .contains("temperature must be between 0.0 and 2.0"));

        let negative = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .temperature(-0.1)
            .try_build();

        assert!(negative.is_err());
    }

    #[test]
    fn test_create_response_validates_top_p() {
        let result = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .top_p(1.5)
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("top_p must be between 0.0 and 1.0"));
    }

    #[test]
    fn test_create_response_validates_penalties() {
        let fp = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .frequency_penalty(3.0)
            .try_build();

        assert!(fp.is_err());
        assert!(fp
            .unwrap_err()
            .to_string()
            .contains("frequency_penalty must be between -2.0 and 2.0"));

        let pp = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .presence_penalty(-3.0)
            .try_build();

        assert!(pp.is_err());
        assert!(pp
            .unwrap_err()
            .to_string()
            .contains("presence_penalty must be between -2.0 and 2.0"));
    }

    // --- Cycle 4.5: Response type deserialization ---

    fn sample_response_json() -> serde_json::Value {
        serde_json::json!({
            "id": "resp_abc123",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4o-2024-08-06",
            "output": [{
                "id": "msg_001",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "Hello, how can I help?"
                }]
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20,
                "total_tokens": 30
            },
            "metadata": {
                "session": "test-123"
            }
        })
    }

    #[test]
    fn test_response_deserialization() {
        let json = sample_response_json();
        let response: Response = serde_json::from_value(json).unwrap();

        assert_eq!(response.id, "resp_abc123");
        assert_eq!(response.object, "response");
        assert_eq!(response.created_at, 1_700_000_000u64);
        assert_eq!(response.status, ResponseStatus::Completed);
        assert_eq!(response.model, "gpt-4o-2024-08-06");
        assert_eq!(response.output.len(), 1);
        assert_eq!(response.output[0].id, "msg_001");
        assert_eq!(response.output[0].output_type, ResponseOutputType::Message);
        assert_eq!(response.output[0].role, Some(crate::chat::Role::Assistant));

        let content = response.output[0].content.as_ref().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0].content_type, ResponseContentType::OutputText);
        assert_eq!(content[0].text, Some("Hello, how can I help?".into()));

        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 30);

        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.get("session"), Some(&"test-123".into()));
    }

    // --- Cycle 4.6: Response minimal deserialization ---

    #[test]
    fn test_response_deserialization_minimal() {
        let json = serde_json::json!({
            "id": "resp_xyz",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4o",
            "output": []
        });

        let response: Response = serde_json::from_value(json).unwrap();

        assert_eq!(response.id, "resp_xyz");
        assert!(response.output.is_empty());
        assert!(response.usage.is_none());
        assert!(response.metadata.is_none());
    }

    #[test]
    fn response_created_at_is_u64() {
        let json = serde_json::json!({
            "id": "resp_ts",
            "object": "response",
            "created_at": 1700000001u64,
            "status": "completed",
            "model": "gpt-4o",
            "output": []
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.created_at, 1_700_000_001u64);
    }

    // --- Cycle 4.7: ResponseStatus serde ---

    #[test]
    fn test_response_status_serde() {
        assert_eq!(
            serde_json::to_string(&ResponseStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&ResponseStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&ResponseStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&ResponseStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );

        // Round-trip
        for status in [
            ResponseStatus::Completed,
            ResponseStatus::Failed,
            ResponseStatus::InProgress,
            ResponseStatus::Cancelled,
        ] {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: ResponseStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    // --- Cycle 4.8: create() API function ---

    #[tokio::test]
    async fn test_create_response_success() {
        let server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "model": "gpt-4o",
            "input": "Hello"
        });

        let response_body = sample_response_json();

        Mock::given(method("POST"))
            .and(path("/openai/v1/responses"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .and(header("content-type", "application/json"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .build();

        let response = create(&client, &request).await.expect("should succeed");

        assert_eq!(response.id, "resp_abc123");
        assert_eq!(response.status, ResponseStatus::Completed);
        assert_eq!(response.output_text(), Some("Hello, how can I help?"));
    }

    // --- Cycle 4.9: create() error handling ---

    #[tokio::test]
    async fn test_create_response_returns_error_on_400() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidRequest",
                "message": "Invalid parameters"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/responses"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .build();

        let result = create(&client, &request).await;

        assert!(result.is_err());
    }

    // --- Cycle 4.10: get() API function ---

    #[tokio::test]
    async fn test_get_response_success() {
        let server = MockServer::start().await;

        let response_body = sample_response_json();

        Mock::given(method("GET"))
            .and(path("/openai/v1/responses/resp_abc123"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let response = get(&client, "resp_abc123").await.expect("should succeed");

        assert_eq!(response.id, "resp_abc123");
        assert_eq!(response.status, ResponseStatus::Completed);
    }

    // --- Cycle 4.11: delete() API function ---

    #[tokio::test]
    async fn test_delete_response_success() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "id": "resp_abc123",
            "object": "response.deleted",
            "deleted": true
        });

        Mock::given(method("DELETE"))
            .and(path("/openai/v1/responses/resp_abc123"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete(&client, "resp_abc123")
            .await
            .expect("should succeed");

        assert_eq!(result.id, "resp_abc123");
        assert!(result.deleted);
    }

    // --- Cycle 4.12: Edge cases ---

    #[test]
    fn test_response_with_previous_response_id() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Continue")
            .previous_response_id("resp_prev123")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["previous_response_id"], "resp_prev123");
    }

    #[test]
    fn test_response_input_text_convenience() {
        // builder.input(string) should create Text variant
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Simple text")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["input"], "Simple text");
    }

    #[tokio::test]
    async fn test_get_response_not_found() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "NotFound",
                "message": "Response not found"
            }
        });

        Mock::given(method("GET"))
            .and(path("/openai/v1/responses/resp_nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = get(&client, "resp_nonexistent").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_response_not_found() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "NotFound",
                "message": "Response not found"
            }
        });

        Mock::given(method("DELETE"))
            .and(path("/openai/v1/responses/resp_nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete(&client, "resp_nonexistent").await;

        assert!(result.is_err());
    }

    // --- Response::output_text() tests ---

    #[test]
    fn test_response_output_text_returns_first_text() {
        let json = sample_response_json();
        let response: Response = serde_json::from_value(json).unwrap();

        assert_eq!(response.output_text(), Some("Hello, how can I help?"));
    }

    #[test]
    fn test_response_output_text_returns_none_for_empty_output() {
        let json = serde_json::json!({
            "id": "resp_xyz",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4o",
            "output": []
        });

        let response: Response = serde_json::from_value(json).unwrap();

        assert_eq!(response.output_text(), None);
    }

    // --- ResponseMessage constructors ---

    #[test]
    fn test_response_message_constructors() {
        let user = ResponseMessage::user("Hello");
        assert_eq!(user.role, crate::chat::Role::User);
        assert_eq!(user.content, "Hello");

        let system = ResponseMessage::system("Be helpful");
        assert_eq!(system.role, crate::chat::Role::System);
        assert_eq!(system.content, "Be helpful");

        let assistant = ResponseMessage::assistant("Hi there");
        assert_eq!(assistant.role, crate::chat::Role::Assistant);
        assert_eq!(assistant.content, "Hi there");
    }

    // --- ResponseDeletionResponse deserialization ---

    #[test]
    fn test_response_deletion_response_deserialization() {
        let json = serde_json::json!({
            "id": "resp_abc",
            "object": "response.deleted",
            "deleted": true
        });

        let result: ResponseDeletionResponse = serde_json::from_value(json).unwrap();

        assert_eq!(result.id, "resp_abc");
        assert_eq!(result.object, "response.deleted");
        assert!(result.deleted);
    }

    // =======================================================================
    // Quality fixes
    // =======================================================================

    // --- OUTPUT_TEXT_TYPE constant test ---

    #[test]
    fn test_output_text_type_constant() {
        assert_eq!(OUTPUT_TEXT_TYPE, "output_text");
    }

    // --- ResponseOutputType / ResponseContentType enum tests ---

    #[test]
    fn test_response_output_type_deserializes_message() {
        let json = r#"{"type": "message"}"#;
        #[derive(Deserialize)]
        struct W {
            #[serde(rename = "type")]
            t: ResponseOutputType,
        }
        let w: W = serde_json::from_str(json).unwrap();
        assert_eq!(w.t, ResponseOutputType::Message);
    }

    #[test]
    fn test_response_output_type_deserializes_unknown() {
        let json = r#"{"type": "function_call"}"#;
        #[derive(Deserialize)]
        struct W {
            #[serde(rename = "type")]
            t: ResponseOutputType,
        }
        let w: W = serde_json::from_str(json).unwrap();
        assert_eq!(w.t, ResponseOutputType::Other);
    }

    #[test]
    fn test_response_content_type_deserializes_output_text() {
        let json = r#"{"type": "output_text", "text": "hello"}"#;
        let c: ResponseContent = serde_json::from_str(json).unwrap();
        assert_eq!(c.content_type, ResponseContentType::OutputText);
    }

    #[test]
    fn test_response_content_type_deserializes_unknown() {
        let json = r#"{"type": "refusal"}"#;
        let c: ResponseContent = serde_json::from_str(json).unwrap();
        assert_eq!(c.content_type, ResponseContentType::Other);
    }

    #[test]
    fn test_output_text_uses_typed_content_type() {
        let response_json = r#"{
            "id": "resp_1",
            "object": "response",
            "created_at": 1700000000,
            "model": "gpt-4o",
            "status": "completed",
            "output": [{
                "id": "out_1",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "hello from the model"
                }]
            }]
        }"#;
        let response: Response = serde_json::from_str(response_json).unwrap();
        assert_eq!(response.output_text(), Some("hello from the model"));
    }

    // --- Tracing span tests ---

    fn sample_response_json_for_tracing() -> serde_json::Value {
        serde_json::json!({
            "id": "resp_trace",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4o",
            "output": []
        })
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_create_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(sample_response_json_for_tracing()),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .build();

        let _ = create(&client, &request).await;

        assert!(logs_contain("foundry::responses::create"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_get_response_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/openai/v1/responses/resp_trace"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(sample_response_json_for_tracing()),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let _ = get(&client, "resp_trace").await;

        assert!(logs_contain("foundry::responses::get"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_delete_response_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/openai/v1/responses/resp_trace"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_trace",
                "object": "response.deleted",
                "deleted": true
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let _ = delete(&client, "resp_trace").await;

        assert!(logs_contain("foundry::responses::delete"));
    }

    #[tokio::test]
    async fn test_get_response_rejects_path_traversal() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let result = get(&client, "../etc/passwd").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                azure_ai_foundry_core::error::FoundryError::Validation { .. }
            ),
            "Expected Validation error, got: {:?}",
            err
        );
    }

    // --- Cycle 6.1: stop() accepts impl IntoIterator ---

    #[test]
    fn test_stop_accepts_str_slice_responses() {
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .stop(["stop1", "stop2"])
            .build();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["stop"], serde_json::json!(["stop1", "stop2"]));
    }

    #[test]
    fn test_stop_accepts_iterator_responses() {
        let stops = vec!["a".to_string(), "b".to_string()];
        let request = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("Hello")
            .stop(stops)
            .build();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["stop"], serde_json::json!(["a", "b"]));
    }

    // --- Cycle 6.4: ResponseMessage typed role ---

    #[test]
    fn test_response_message_typed_role_serializes() {
        let msg = ResponseMessage::user("Hello");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
    }

    #[test]
    fn test_response_message_typed_role_deserializes() {
        let json = serde_json::json!({"role": "assistant", "content": "Hi"});
        let msg: ResponseMessage = serde_json::from_value(json).unwrap();
        assert_eq!(msg.role, crate::chat::Role::Assistant);
    }

    // --- M3 R4: ResponseOutput::role typed as Role ---

    #[test]
    fn test_response_output_role_is_typed() {
        let json = serde_json::json!({
            "id": "out_1",
            "type": "message",
            "role": "assistant",
            "content": []
        });
        let output: ResponseOutput = serde_json::from_value(json).unwrap();
        assert_eq!(output.role, Some(crate::chat::Role::Assistant));
    }

    #[test]
    fn test_response_output_role_none_when_absent() {
        let json = serde_json::json!({
            "id": "out_2",
            "type": "web_search_call"
        });
        let output: ResponseOutput = serde_json::from_value(json).unwrap();
        assert_eq!(output.role, None);
    }

    #[test]
    fn test_response_builder_implements_debug() {
        let builder = CreateResponseRequest::builder().model("gpt-4o");
        let debug = format!("{:?}", builder);
        assert!(debug.contains("CreateResponseRequestBuilder"));
        assert!(debug.contains("gpt-4o"));
    }

    #[test]
    fn test_response_rejects_whitespace_only_model() {
        let result = CreateResponseRequest::builder()
            .model("  ")
            .input("Hello")
            .try_build();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model cannot be empty"));
    }

    // --- previous_response_id validation tests ---

    #[test]
    fn test_try_build_rejects_empty_previous_response_id() {
        let result = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("hello")
            .previous_response_id("")
            .try_build();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("previous_response_id"), "got: {err}");
    }

    #[test]
    fn test_try_build_rejects_whitespace_previous_response_id() {
        let result = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("hello")
            .previous_response_id("   ")
            .try_build();
        assert!(result.is_err());
    }

    #[test]
    fn test_try_build_accepts_valid_previous_response_id() {
        let result = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("hello")
            .previous_response_id("resp_abc123")
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_build_without_previous_response_id_is_ok() {
        let result = CreateResponseRequest::builder()
            .model("gpt-4o")
            .input("hello")
            .try_build();
        assert!(result.is_ok());
    }
}
