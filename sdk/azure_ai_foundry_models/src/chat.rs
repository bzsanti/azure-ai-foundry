//! Chat completion types and API calls for Azure AI Foundry Models.

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::FoundryResult;
use azure_ai_foundry_core::models::Usage;
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

    /// Build the request. Panics if `model` is not set.
    pub fn build(self) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: self.model.expect("model is required"),
            messages: self.messages,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            stream: None,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use azure_ai_foundry_core::auth::FoundryCredential;
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

    async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key("test-api-key"))
            .build()
            .expect("should build client")
    }

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
        let err = result.unwrap_err();
        assert!(err.to_string().contains("InvalidModel"));
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
        let err = result.unwrap_err();
        assert!(err.to_string().contains("429"));
    }
}
