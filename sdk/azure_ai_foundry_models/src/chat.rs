//! Chat completion types and API calls for Azure AI Foundry Models.
//!
//! This module provides both synchronous (single response) and streaming
//! chat completion APIs.
//!
//! # Streaming Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::chat::*;
//! # use futures::StreamExt;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = ChatCompletionRequest::builder()
//!     .model("gpt-4o")
//!     .message(Message::user("Tell me a story"))
//!     .build();
//!
//! let stream = complete_stream(client, &request).await?;
//! let mut stream = std::pin::pin!(stream);
//! while let Some(chunk) = stream.next().await {
//!     let chunk = chunk?;
//!     if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.as_ref()) {
//!         print!("{}", content);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use azure_ai_foundry_core::models::Usage;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A chat completion request.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
}

/// Builder for [`ChatCompletionRequest`].
pub struct ChatCompletionRequestBuilder {
    model: Option<String>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
}

impl ChatCompletionRequest {
    /// Create a new builder.
    pub fn builder() -> ChatCompletionRequestBuilder {
        ChatCompletionRequestBuilder {
            model: None,
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
        }
    }
}

impl ChatCompletionRequestBuilder {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Build the request, returning an error if required fields are missing.
    pub fn try_build(self) -> FoundryResult<ChatCompletionRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;

        Ok(ChatCompletionRequest {
            model,
            messages: self.messages,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            stream: None,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
        })
    }

    /// Build the request. Panics if `model` is not set.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> ChatCompletionRequest {
        self.try_build().expect("builder validation failed")
    }
}

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(content.into()),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
        }
    }
}

/// The role of a message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// A single choice in a chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Streaming response types
// ---------------------------------------------------------------------------

/// A streaming chunk from a chat completion response.
///
/// This represents a single Server-Sent Event (SSE) from the streaming API.
/// Each chunk contains partial content that should be concatenated to form
/// the complete response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion.
    pub id: String,

    /// Object type, always "chat.completion.chunk".
    pub object: String,

    /// Unix timestamp when the chunk was created.
    pub created: u64,

    /// Model used for the completion.
    pub model: String,

    /// List of choices (usually one for non-n requests).
    pub choices: Vec<ChunkChoice>,

    /// Usage statistics (only present in the final chunk when requested).
    pub usage: Option<Usage>,
}

/// A single choice in a streaming chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkChoice {
    /// Index of this choice.
    pub index: u32,

    /// The delta containing new content.
    pub delta: Delta,

    /// Reason the generation stopped (only in final chunk).
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming chunk.
///
/// Contains the incremental content added in this chunk.
/// The first chunk typically contains the role, subsequent chunks contain content.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Delta {
    /// Role of the assistant (only in first chunk).
    pub role: Option<Role>,

    /// Incremental content to append.
    pub content: Option<String>,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Send a chat completion request.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::chat::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = ChatCompletionRequest::builder()
///     .model("gpt-4o")
///     .message(Message::system("You are a helpful assistant."))
///     .message(Message::user("What is Rust?"))
///     .temperature(0.7)
///     .build();
///
/// let response = complete(client, &request).await?;
/// println!("{:?}", response.choices[0].message.content);
/// # Ok(())
/// # }
/// ```
pub async fn complete(
    client: &FoundryClient,
    request: &ChatCompletionRequest,
) -> FoundryResult<ChatCompletionResponse> {
    let response = client.post("/openai/v1/chat/completions", request).await?;

    let body = response.json::<ChatCompletionResponse>().await?;
    Ok(body)
}

/// Send a streaming chat completion request.
///
/// Returns a stream of [`ChatCompletionChunk`]s that can be consumed
/// as they arrive from the server.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::chat::*;
/// # use futures::StreamExt;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = ChatCompletionRequest::builder()
///     .model("gpt-4o")
///     .message(Message::user("Hello!"))
///     .build();
///
/// let stream = complete_stream(client, &request).await?;
/// let mut stream = std::pin::pin!(stream);
/// while let Some(chunk) = stream.next().await {
///     let chunk = chunk?;
///     if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.as_ref()) {
///         print!("{}", content);
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub async fn complete_stream(
    client: &FoundryClient,
    request: &ChatCompletionRequest,
) -> FoundryResult<impl Stream<Item = FoundryResult<ChatCompletionChunk>>> {
    // Create a modified request with stream: true
    let stream_request = StreamingRequest {
        model: &request.model,
        messages: &request.messages,
        temperature: request.temperature,
        top_p: request.top_p,
        max_tokens: request.max_tokens,
        stream: true,
        stop: request.stop.as_deref(),
        presence_penalty: request.presence_penalty,
        frequency_penalty: request.frequency_penalty,
    };

    let response = client
        .post_stream("/openai/v1/chat/completions", &stream_request)
        .await?;

    Ok(parse_sse_stream(response))
}

/// Internal request type for streaming chat completions.
///
/// This is a zero-copy variant of [`ChatCompletionRequest`] that:
/// - Uses references to avoid cloning request data
/// - Always sets `stream: true` for SSE responses
/// - Is used internally by [`complete_stream`]
///
/// Users should construct [`ChatCompletionRequest`] instead of this type directly.
#[derive(Serialize)]
struct StreamingRequest<'a> {
    /// Model ID for the completion.
    model: &'a str,
    /// Conversation messages.
    messages: &'a [Message],
    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Nucleus sampling probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// Always `true` for streaming requests.
    stream: bool,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<&'a [String]>,
    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
}

/// Parse Server-Sent Events (SSE) stream into ChatCompletionChunks.
///
/// Optimized for performance:
/// - Uses `memchr` for fast newline detection
/// - Works with `Vec<u8>` to minimize UTF-8 validation overhead
/// - Minimizes allocations by draining processed bytes
fn parse_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = FoundryResult<ChatCompletionChunk>> {
    let byte_stream = response.bytes_stream();

    // Buffer for incomplete lines across chunks (bytes for efficiency)
    stream::unfold(
        (byte_stream, Vec::<u8>::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                // Fast newline search using memchr
                if let Some(newline_pos) = memchr::memchr(b'\n', &buffer) {
                    // Extract line bytes and drain from buffer
                    let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();

                    // Convert to string only when needed (skip trailing newline)
                    let line = match std::str::from_utf8(&line_bytes[..line_bytes.len() - 1]) {
                        Ok(s) => s,
                        Err(_) => {
                            // Invalid UTF-8, skip this line
                            continue;
                        }
                    };

                    // Parse the line
                    if let Some(chunk) = parse_sse_line(line) {
                        return Some((chunk, (byte_stream, buffer)));
                    }
                    // Continue to next line if this one was skipped
                    continue;
                }

                // Need more data
                match TryStreamExt::try_next(&mut byte_stream).await {
                    Ok(Some(bytes)) => {
                        buffer.extend_from_slice(&bytes);
                    }
                    Ok(None) => {
                        // Stream ended, try remaining buffer
                        if !buffer.is_empty() {
                            if let Ok(line) = std::str::from_utf8(&buffer) {
                                if let Some(chunk) = parse_sse_line(line) {
                                    buffer.clear();
                                    return Some((chunk, (byte_stream, buffer)));
                                }
                            }
                            buffer.clear();
                        }
                        return None;
                    }
                    Err(e) => {
                        return Some((Err(FoundryError::from(e)), (byte_stream, buffer)));
                    }
                }
            }
        },
    )
}

/// Parse a single SSE line, returning None for lines that should be skipped.
fn parse_sse_line(line: &str) -> Option<FoundryResult<ChatCompletionChunk>> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Handle data lines
    if let Some(data) = line.strip_prefix("data: ") {
        let data = data.trim();

        // Skip [DONE] marker
        if data == "[DONE]" {
            return None;
        }

        // Parse JSON
        match serde_json::from_str::<ChatCompletionChunk>(data) {
            Ok(chunk) => Some(Ok(chunk)),
            Err(e) => Some(Err(FoundryError::Stream(format!(
                "Failed to parse chunk: {}",
                e
            )))),
        }
    } else {
        // Skip other SSE fields (event:, id:, retry:)
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Builder tests ---

    #[test]
    fn builder_with_required_fields_only() {
        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hello"))
            .build();

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 1);
        assert!(request.temperature.is_none());
        assert!(request.top_p.is_none());
        assert!(request.max_tokens.is_none());
        assert!(request.stream.is_none());
        assert!(request.stop.is_none());
        assert!(request.presence_penalty.is_none());
        assert!(request.frequency_penalty.is_none());
    }

    #[test]
    fn builder_with_all_fields() {
        let request = ChatCompletionRequest::builder()
            .model("gpt-4o-mini")
            .message(Message::system("You are helpful."))
            .message(Message::user("Hi"))
            .temperature(0.7)
            .top_p(0.9)
            .max_tokens(100)
            .stop(vec!["END".into()])
            .presence_penalty(0.5)
            .frequency_penalty(0.3)
            .build();

        assert_eq!(request.model, "gpt-4o-mini");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.stop, Some(vec!["END".into()]));
        assert_eq!(request.presence_penalty, Some(0.5));
        assert_eq!(request.frequency_penalty, Some(0.3));
    }

    #[test]
    fn builder_messages_method() {
        let messages = vec![
            Message::system("System prompt"),
            Message::user("User message"),
            Message::assistant("Assistant response"),
        ];

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .messages(messages)
            .build();

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::System);
        assert_eq!(request.messages[1].role, Role::User);
        assert_eq!(request.messages[2].role, Role::Assistant);
    }

    #[test]
    #[should_panic(expected = "model is required")]
    fn builder_without_model_panics() {
        ChatCompletionRequest::builder()
            .message(Message::user("Hello"))
            .build();
    }

    #[test]
    fn try_build_returns_error_when_model_missing() {
        let result = ChatCompletionRequest::builder()
            .message(Message::user("Hello"))
            .try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, azure_ai_foundry_core::error::FoundryError::Builder(_)));
        assert!(err.to_string().contains("model"));
    }

    #[test]
    fn try_build_success() {
        let result = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hello"))
            .try_build();

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.model, "gpt-4o");
    }

    // --- Message constructor tests ---

    #[test]
    fn message_system_constructor() {
        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.content, Some("You are a helpful assistant.".into()));
    }

    #[test]
    fn message_user_constructor() {
        let msg = Message::user("What is Rust?");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, Some("What is Rust?".into()));
    }

    #[test]
    fn message_assistant_constructor() {
        let msg = Message::assistant("Rust is a systems programming language.");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, Some("Rust is a systems programming language.".into()));
    }

    // --- Serialization tests ---

    #[test]
    fn role_serialization() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&Role::Assistant).unwrap(), "\"assistant\"");
        assert_eq!(serde_json::to_string(&Role::Tool).unwrap(), "\"tool\"");
    }

    #[test]
    fn role_deserialization() {
        assert_eq!(serde_json::from_str::<Role>("\"system\"").unwrap(), Role::System);
        assert_eq!(serde_json::from_str::<Role>("\"user\"").unwrap(), Role::User);
        assert_eq!(serde_json::from_str::<Role>("\"assistant\"").unwrap(), Role::Assistant);
        assert_eq!(serde_json::from_str::<Role>("\"tool\"").unwrap(), Role::Tool);
    }

    #[test]
    fn request_serialization_skips_none_fields() {
        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hi"))
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert!(json.get("temperature").is_none());
        assert!(json.get("top_p").is_none());
        assert!(json.get("max_tokens").is_none());
        assert!(json.get("stream").is_none());
        assert!(json.get("stop").is_none());
        assert!(json.get("presence_penalty").is_none());
        assert!(json.get("frequency_penalty").is_none());
    }

    #[test]
    fn request_serialization_includes_set_fields() {
        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hi"))
            .temperature(0.5)
            .max_tokens(50)
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_tokens"], 50);
    }

    #[test]
    fn response_deserialization() {
        let json = serde_json::json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 15,
                "total_tokens": 25
            }
        });

        let response: ChatCompletionResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.id, "chatcmpl-abc123");
        assert_eq!(response.object, "chat.completion");
        assert_eq!(response.created, 1700000000);
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].index, 0);
        assert_eq!(response.choices[0].message.role, Role::Assistant);
        assert_eq!(
            response.choices[0].message.content,
            Some("Hello! How can I help you today?".into())
        );
        assert_eq!(response.choices[0].finish_reason, Some("stop".into()));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, Some(15));
        assert_eq!(usage.total_tokens, 25);
    }

    #[test]
    fn response_deserialization_without_usage() {
        let json = serde_json::json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hi!"
                },
                "finish_reason": null
            }]
        });

        let response: ChatCompletionResponse = serde_json::from_value(json).unwrap();

        assert!(response.usage.is_none());
        assert!(response.choices[0].finish_reason.is_none());
    }

    // --- Integration tests with wiremock ---

    use crate::test_utils::setup_mock_client;

    #[tokio::test]
    async fn complete_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Rust is a systems programming language."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 10,
                "total_tokens": 30
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::system("You are helpful."))
            .message(Message::user("What is Rust?"))
            .build();

        let response = complete(&client, &request).await.expect("should succeed");

        assert_eq!(response.id, "chatcmpl-test123");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            Some("Rust is a systems programming language.".into())
        );
    }

    #[tokio::test]
    async fn complete_with_parameters() {
        let server = MockServer::start().await;

        let expected_request = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.5,
            "max_tokens": 100
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .and(body_json(&expected_request))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 1700000000,
                "model": "gpt-4o-mini",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Hi!"},
                    "finish_reason": "stop"
                }]
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o-mini")
            .message(Message::user("Hello"))
            .temperature(0.5)
            .max_tokens(100)
            .build();

        let response = complete(&client, &request).await.expect("should succeed");
        assert_eq!(response.model, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn complete_api_error() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidModel",
                "message": "The model 'nonexistent' does not exist"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("nonexistent")
            .message(Message::user("Hello"))
            .build();

        let result = complete(&client, &request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FoundryError::Api { code, message } => {
                assert_eq!(code, "InvalidModel");
                assert!(message.contains("does not exist"));
            }
            other => panic!("Expected Api error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn complete_rate_limit_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hello"))
            .build();

        let result = complete(&client, &request).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            FoundryError::Http { status, message } => {
                assert_eq!(status, 429);
                assert!(message.contains("Rate limit"));
            }
            other => panic!("Expected Http error, got {:?}", other),
        }
    }

    // --- Streaming types tests ---

    #[test]
    fn chunk_deserialization() {
        let json = serde_json::json!({
            "id": "chatcmpl-chunk123",
            "object": "chat.completion.chunk",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hello"
                },
                "finish_reason": null
            }]
        });

        let chunk: ChatCompletionChunk = serde_json::from_value(json).unwrap();

        assert_eq!(chunk.id, "chatcmpl-chunk123");
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.model, "gpt-4o");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.role, Some(Role::Assistant));
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".into()));
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn chunk_deserialization_content_only() {
        let json = serde_json::json!({
            "id": "chatcmpl-chunk123",
            "object": "chat.completion.chunk",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": " world"
                },
                "finish_reason": null
            }]
        });

        let chunk: ChatCompletionChunk = serde_json::from_value(json).unwrap();

        assert!(chunk.choices[0].delta.role.is_none());
        assert_eq!(chunk.choices[0].delta.content, Some(" world".into()));
    }

    #[test]
    fn chunk_deserialization_final_chunk() {
        let json = serde_json::json!({
            "id": "chatcmpl-chunk123",
            "object": "chat.completion.chunk",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        });

        let chunk: ChatCompletionChunk = serde_json::from_value(json).unwrap();

        assert!(chunk.choices[0].delta.role.is_none());
        assert!(chunk.choices[0].delta.content.is_none());
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".into()));
    }

    #[test]
    fn parse_sse_line_data() {
        let line = "data: {\"id\":\"test\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}";
        let result = super::parse_sse_line(line);

        assert!(result.is_some());
        let chunk = result.unwrap().expect("should parse");
        assert_eq!(chunk.id, "test");
        assert_eq!(chunk.choices[0].delta.content, Some("Hi".into()));
    }

    #[test]
    fn parse_sse_line_done() {
        let line = "data: [DONE]";
        let result = super::parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn parse_sse_line_empty() {
        let result = super::parse_sse_line("");
        assert!(result.is_none());
    }

    #[test]
    fn parse_sse_line_comment() {
        let result = super::parse_sse_line(": keep-alive");
        assert!(result.is_none());
    }

    #[test]
    fn parse_sse_line_invalid_json() {
        let line = "data: {invalid json}";
        let result = super::parse_sse_line(line);

        assert!(result.is_some());
        let err = result.unwrap();
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Failed to parse chunk"));
    }

    // --- Streaming integration tests ---

    #[tokio::test]
    async fn complete_stream_success() {
        use futures::StreamExt;

        let server = MockServer::start().await;

        // SSE response with multiple chunks
        let sse_body = concat!(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_body)
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hello"))
            .build();

        let stream = complete_stream(&client, &request).await.expect("should start stream");
        let chunks: Vec<_> = stream.collect().await;

        assert_eq!(chunks.len(), 3);

        // First chunk has role
        let first = chunks[0].as_ref().expect("chunk 1");
        assert_eq!(first.choices[0].delta.role, Some(Role::Assistant));
        assert_eq!(first.choices[0].delta.content, Some("Hello".into()));

        // Second chunk has content only
        let second = chunks[1].as_ref().expect("chunk 2");
        assert!(second.choices[0].delta.role.is_none());
        assert_eq!(second.choices[0].delta.content, Some(" world".into()));

        // Third chunk has finish_reason
        let third = chunks[2].as_ref().expect("chunk 3");
        assert_eq!(third.choices[0].finish_reason, Some("stop".into()));
    }

    #[tokio::test]
    async fn complete_stream_request_includes_stream_true() {
        use futures::StreamExt;

        let server = MockServer::start().await;

        // Verify the request body includes stream: true
        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi"}],
                "stream": true
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("data: [DONE]\n\n")
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("Hi"))
            .build();

        let stream = complete_stream(&client, &request).await.expect("should start");
        let _: Vec<_> = stream.collect().await;
    }

    #[tokio::test]
    async fn complete_stream_api_error() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidModel",
                "message": "Model not found"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(404).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("nonexistent")
            .message(Message::user("Hi"))
            .build();

        let result = complete_stream(&client, &request).await;

        match result {
            Ok(_) => panic!("Expected error, got Ok"),
            Err(e) => assert!(e.to_string().contains("InvalidModel")),
        }
    }

    #[tokio::test]
    async fn complete_stream_collects_full_content() {
        use futures::StreamExt;

        let server = MockServer::start().await;

        let sse_body = concat!(
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"The \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"answer \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"is 42.\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/openai/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_body)
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = ChatCompletionRequest::builder()
            .model("gpt-4o")
            .message(Message::user("What is the meaning of life?"))
            .build();

        let stream = complete_stream(&client, &request).await.expect("should start");

        // Collect all content
        let mut full_content = String::new();
        let mut stream = std::pin::pin!(stream);
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.as_ref()) {
                    full_content.push_str(content);
                }
            }
        }

        assert_eq!(full_content, "The answer is 42.");
    }
}
