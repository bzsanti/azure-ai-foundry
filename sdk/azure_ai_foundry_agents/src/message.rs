//! Message management for Azure AI Foundry Agent Service.
//!
//! Messages are the content within threads. Users add messages to threads,
//! and agents respond with assistant messages when runs are executed.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::{thread, message};
//! use azure_ai_foundry_agents::message::MessageCreateRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // Create a thread first
//! let thread = thread::create(&client, None).await?;
//!
//! // Add a user message
//! let request = MessageCreateRequest::builder()
//!     .content("Hello, can you help me?")
//!     .build()?;
//!
//! let msg = message::create(&client, &thread.id, &request).await?;
//! println!("Created message: {}", msg.id);
//!
//! // List messages in thread
//! let messages = message::list(&client, &thread.id).await?;
//! for m in messages.data {
//!     println!("{:?}: {:?}", m.role, m.content);
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::API_VERSION;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A request to create a new message in a thread.
#[derive(Debug, Clone, Serialize)]
pub struct MessageCreateRequest {
    /// The role of the message author.
    pub role: MessageRole,

    /// The content of the message.
    pub content: String,

    /// Optional metadata for the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Builder for [`MessageCreateRequest`].
#[derive(Debug, Default)]
pub struct MessageCreateRequestBuilder {
    content: Option<String>,
    role: Option<MessageRole>,
    metadata: Option<serde_json::Value>,
}

impl MessageCreateRequest {
    /// Create a new builder for `MessageCreateRequest`.
    pub fn builder() -> MessageCreateRequestBuilder {
        MessageCreateRequestBuilder::default()
    }
}

impl MessageCreateRequestBuilder {
    /// Set the content of the message.
    ///
    /// **Required.**
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the role of the message.
    ///
    /// Defaults to [`MessageRole::User`] if not set.
    pub fn role(mut self, role: MessageRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Set metadata for the message.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request.
    ///
    /// # Errors
    ///
    /// Returns an error if `content` is not set.
    pub fn build(self) -> FoundryResult<MessageCreateRequest> {
        let content = self
            .content
            .ok_or_else(|| FoundryError::Builder("content is required".into()))?;

        if content.trim().is_empty() {
            return Err(FoundryError::Builder("content cannot be empty".into()));
        }

        Ok(MessageCreateRequest {
            role: self.role.unwrap_or(MessageRole::User),
            content,
            metadata: self.metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A message from the user.
    User,
    /// A message from the assistant.
    Assistant,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A message in a thread.
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    /// Unique identifier for the message.
    pub id: String,

    /// Object type, always "thread.message".
    pub object: String,

    /// Unix timestamp when the message was created.
    pub created_at: u64,

    /// The thread ID this message belongs to.
    pub thread_id: String,

    /// The role of the message author.
    pub role: MessageRole,

    /// The content of the message.
    pub content: Vec<MessageContent>,

    /// ID of the assistant that authored this message (if role is assistant).
    pub assistant_id: Option<String>,

    /// ID of the run that created this message (if applicable).
    pub run_id: Option<String>,

    /// Metadata attached to the message.
    pub metadata: Option<serde_json::Value>,
}

/// Content of a message.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageContent {
    /// The type of content (e.g., "text").
    #[serde(rename = "type")]
    pub content_type: String,

    /// Text content (if type is "text").
    pub text: Option<TextContent>,
}

/// Text content within a message.
#[derive(Debug, Clone, Deserialize)]
pub struct TextContent {
    /// The text value.
    pub value: String,

    /// Annotations (citations, file references, etc.).
    #[serde(default)]
    pub annotations: Vec<serde_json::Value>,
}

/// Response from listing messages.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageList {
    /// Object type, always "list".
    pub object: String,

    /// List of messages.
    pub data: Vec<Message>,

    /// ID of the first message in the list.
    pub first_id: Option<String>,

    /// ID of the last message in the list.
    pub last_id: Option<String>,

    /// Whether there are more messages to fetch.
    pub has_more: bool,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Create a new message in a thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::message::{self, MessageCreateRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = MessageCreateRequest::builder()
///     .content("What is 2+2?")
///     .build()?;
///
/// let msg = message::create(client, "thread_abc123", &request).await?;
/// println!("Created message: {}", msg.id);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::messages::create` with field `thread_id`.
#[tracing::instrument(
    name = "foundry::messages::create",
    skip(client, request),
    fields(thread_id = %thread_id)
)]
pub async fn create(
    client: &FoundryClient,
    thread_id: &str,
    request: &MessageCreateRequest,
) -> FoundryResult<Message> {
    tracing::debug!("creating message");

    let path = format!("/threads/{}/messages?{}", thread_id, API_VERSION);
    let response = client.post(&path, request).await?;
    let message = response.json::<Message>().await?;

    tracing::debug!(message_id = %message.id, "message created");
    Ok(message)
}

/// List messages in a thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::message;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let messages = message::list(client, "thread_abc123").await?;
/// for msg in messages.data {
///     println!("{:?}: {:?}", msg.role, msg.content);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::messages::list` with field `thread_id`.
#[tracing::instrument(
    name = "foundry::messages::list",
    skip(client),
    fields(thread_id = %thread_id)
)]
pub async fn list(client: &FoundryClient, thread_id: &str) -> FoundryResult<MessageList> {
    tracing::debug!("listing messages");

    let path = format!("/threads/{}/messages?{}", thread_id, API_VERSION);
    let response = client.get(&path).await?;
    let list = response.json::<MessageList>().await?;

    tracing::debug!(count = list.data.len(), "messages listed");
    Ok(list)
}

/// Get a specific message from a thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::message;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let msg = message::get(client, "thread_abc123", "msg_xyz789").await?;
/// println!("Message role: {:?}", msg.role);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::messages::get` with fields `thread_id` and `message_id`.
#[tracing::instrument(
    name = "foundry::messages::get",
    skip(client),
    fields(thread_id = %thread_id, message_id = %message_id)
)]
pub async fn get(
    client: &FoundryClient,
    thread_id: &str,
    message_id: &str,
) -> FoundryResult<Message> {
    tracing::debug!("getting message");

    let path = format!(
        "/threads/{}/messages/{}?{}",
        thread_id, message_id, API_VERSION
    );
    let response = client.get(&path).await?;
    let message = response.json::<Message>().await?;

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Cycle 13: Message types tests ---

    #[test]
    fn test_message_role_serialization() {
        assert_eq!(
            serde_json::to_string(&MessageRole::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&MessageRole::Assistant).unwrap(),
            "\"assistant\""
        );
    }

    #[test]
    fn test_message_role_deserialization() {
        assert_eq!(
            serde_json::from_str::<MessageRole>("\"user\"").unwrap(),
            MessageRole::User
        );
        assert_eq!(
            serde_json::from_str::<MessageRole>("\"assistant\"").unwrap(),
            MessageRole::Assistant
        );
    }

    #[test]
    fn test_message_request_serialization() {
        let request = MessageCreateRequest::builder()
            .content("Hello!")
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello!");
    }

    #[test]
    fn test_message_builder_requires_content() {
        let result = MessageCreateRequest::builder().build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("content is required"));
    }

    #[test]
    fn test_message_builder_rejects_empty_content() {
        let result = MessageCreateRequest::builder().content("   ").build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("content cannot be empty"));
    }

    #[test]
    fn test_message_response_deserialization() {
        let json = serde_json::json!({
            "id": "msg_abc123",
            "object": "thread.message",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "role": "user",
            "content": [{
                "type": "text",
                "text": {
                    "value": "Hello!",
                    "annotations": []
                }
            }]
        });

        let message: Message = serde_json::from_value(json).unwrap();

        assert_eq!(message.id, "msg_abc123");
        assert_eq!(message.thread_id, "thread_xyz");
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content.len(), 1);
        assert_eq!(message.content[0].content_type, "text");
        assert_eq!(message.content[0].text.as_ref().unwrap().value, "Hello!");
    }

    // --- Cycle 14: Create message API tests ---

    #[tokio::test]
    async fn test_create_message_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "msg_test123",
            "object": "thread.message",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_abc",
            "role": "user",
            "content": [{
                "type": "text",
                "text": {
                    "value": "What is 2+2?",
                    "annotations": []
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/threads/thread_abc/messages"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(body_json(serde_json::json!({
                "role": "user",
                "content": "What is 2+2?"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = MessageCreateRequest::builder()
            .content("What is 2+2?")
            .build()
            .expect("valid request");

        let message = create(&client, "thread_abc", &request)
            .await
            .expect("should succeed");

        assert_eq!(message.id, "msg_test123");
        assert_eq!(message.thread_id, "thread_abc");
    }

    // --- Cycle 15: List messages API tests ---

    #[tokio::test]
    async fn test_list_messages_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "msg_1",
                    "object": "thread.message",
                    "created_at": TEST_TIMESTAMP,
                    "thread_id": "thread_abc",
                    "role": "user",
                    "content": [{"type": "text", "text": {"value": "Hi", "annotations": []}}]
                },
                {
                    "id": "msg_2",
                    "object": "thread.message",
                    "created_at": TEST_TIMESTAMP,
                    "thread_id": "thread_abc",
                    "role": "assistant",
                    "content": [{"type": "text", "text": {"value": "Hello!", "annotations": []}}]
                }
            ],
            "first_id": "msg_1",
            "last_id": "msg_2",
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_abc/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let list = list(&client, "thread_abc").await.expect("should succeed");

        assert_eq!(list.data.len(), 2);
        assert_eq!(list.data[0].role, MessageRole::User);
        assert_eq!(list.data[1].role, MessageRole::Assistant);
        assert!(!list.has_more);
    }

    #[tokio::test]
    async fn test_get_message_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "msg_xyz",
            "object": "thread.message",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_abc",
            "role": "user",
            "content": [{"type": "text", "text": {"value": "Test", "annotations": []}}]
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_abc/messages/msg_xyz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let message = get(&client, "thread_abc", "msg_xyz")
            .await
            .expect("should succeed");

        assert_eq!(message.id, "msg_xyz");
    }
}
