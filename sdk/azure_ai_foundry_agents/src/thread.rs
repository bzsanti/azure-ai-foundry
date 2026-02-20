//! Thread management for Azure AI Foundry Agent Service.
//!
//! Threads represent conversation sessions that maintain message history.
//! Each thread can contain multiple messages and can be used with different agents.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::thread;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // Create a new thread
//! let thread = thread::create(&client, None).await?;
//! println!("Created thread: {}", thread.id);
//!
//! // Get thread by ID
//! let fetched = thread::get(&client, &thread.id).await?;
//! println!("Thread created at: {}", fetched.created_at);
//!
//! // Delete thread when done
//! thread::delete(&client, &thread.id).await?;
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::FoundryResult;
use serde::{Deserialize, Serialize};

use crate::models::API_VERSION;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A request to create a new thread.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ThreadCreateRequest {
    /// Optional metadata for the thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A conversation thread.
#[derive(Debug, Clone, Deserialize)]
pub struct Thread {
    /// Unique identifier for the thread.
    pub id: String,

    /// Object type, always "thread".
    pub object: String,

    /// Unix timestamp when the thread was created.
    pub created_at: u64,

    /// Metadata attached to the thread.
    pub metadata: Option<serde_json::Value>,
}

/// Response from deleting a thread.
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadDeletionResponse {
    /// ID of the deleted thread.
    pub id: String,

    /// Object type, always "thread.deleted".
    pub object: String,

    /// Whether the deletion was successful.
    pub deleted: bool,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Create a new thread.
///
/// # Arguments
///
/// * `client` - The Foundry client.
/// * `metadata` - Optional metadata to attach to the thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::thread;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// // Create thread without metadata
/// let thread = thread::create(client, None).await?;
///
/// // Create thread with metadata
/// let metadata = serde_json::json!({"user_id": "123"});
/// let thread_with_meta = thread::create(client, Some(metadata)).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::threads::create`.
#[tracing::instrument(name = "foundry::threads::create", skip(client, metadata))]
pub async fn create(
    client: &FoundryClient,
    metadata: Option<serde_json::Value>,
) -> FoundryResult<Thread> {
    tracing::debug!("creating thread");

    let request = ThreadCreateRequest { metadata };
    let path = format!("/threads?{}", API_VERSION);
    let response = client.post(&path, &request).await?;
    let thread = response.json::<Thread>().await?;

    tracing::debug!(thread_id = %thread.id, "thread created");
    Ok(thread)
}

/// Get a thread by ID.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::thread;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let thread = thread::get(client, "thread_abc123").await?;
/// println!("Thread created at: {}", thread.created_at);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::threads::get` with field `thread_id`.
#[tracing::instrument(
    name = "foundry::threads::get",
    skip(client),
    fields(thread_id = %thread_id)
)]
pub async fn get(client: &FoundryClient, thread_id: &str) -> FoundryResult<Thread> {
    tracing::debug!("getting thread");

    let path = format!("/threads/{}?{}", thread_id, API_VERSION);
    let response = client.get(&path).await?;
    let thread = response.json::<Thread>().await?;

    Ok(thread)
}

/// Delete a thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::thread;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = thread::delete(client, "thread_abc123").await?;
/// if result.deleted {
///     println!("Thread deleted successfully");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::threads::delete` with field `thread_id`.
#[tracing::instrument(
    name = "foundry::threads::delete",
    skip(client),
    fields(thread_id = %thread_id)
)]
pub async fn delete(client: &FoundryClient, thread_id: &str) -> FoundryResult<ThreadDeletionResponse> {
    tracing::debug!("deleting thread");

    let path = format!("/threads/{}?{}", thread_id, API_VERSION);
    let response = client.delete(&path).await?;
    let result = response.json::<ThreadDeletionResponse>().await?;

    tracing::debug!(deleted = result.deleted, "thread deletion complete");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Cycle 10: Thread creation tests ---

    #[test]
    fn test_thread_response_deserialization() {
        let json = serde_json::json!({
            "id": "thread_abc123",
            "object": "thread",
            "created_at": TEST_TIMESTAMP,
            "metadata": {"key": "value"}
        });

        let thread: Thread = serde_json::from_value(json).unwrap();

        assert_eq!(thread.id, "thread_abc123");
        assert_eq!(thread.object, "thread");
        assert_eq!(thread.created_at, TEST_TIMESTAMP);
        assert!(thread.metadata.is_some());
    }

    #[test]
    fn test_thread_response_minimal() {
        let json = serde_json::json!({
            "id": "thread_abc123",
            "object": "thread",
            "created_at": TEST_TIMESTAMP
        });

        let thread: Thread = serde_json::from_value(json).unwrap();

        assert_eq!(thread.id, "thread_abc123");
        assert!(thread.metadata.is_none());
    }

    #[tokio::test]
    async fn test_create_thread_minimal() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "thread_test123",
            "object": "thread",
            "created_at": TEST_TIMESTAMP
        });

        Mock::given(method("POST"))
            .and(path("/threads"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let thread = create(&client, None).await.expect("should succeed");

        assert_eq!(thread.id, "thread_test123");
        assert_eq!(thread.object, "thread");
    }

    // --- Cycle 11: Thread with metadata tests ---

    #[tokio::test]
    async fn test_create_thread_with_metadata() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "thread_meta123",
            "object": "thread",
            "created_at": TEST_TIMESTAMP,
            "metadata": {"user_id": "user123"}
        });

        Mock::given(method("POST"))
            .and(path("/threads"))
            .and(body_json(serde_json::json!({
                "metadata": {"user_id": "user123"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let metadata = serde_json::json!({"user_id": "user123"});

        let thread = create(&client, Some(metadata)).await.expect("should succeed");

        assert_eq!(thread.id, "thread_meta123");
        assert!(thread.metadata.is_some());
    }

    // --- Cycle 12: Get and delete thread tests ---

    #[tokio::test]
    async fn test_get_thread_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "thread_abc123",
            "object": "thread",
            "created_at": TEST_TIMESTAMP
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_abc123"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let thread = get(&client, "thread_abc123").await.expect("should succeed");

        assert_eq!(thread.id, "thread_abc123");
    }

    #[tokio::test]
    async fn test_delete_thread_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "thread_abc123",
            "object": "thread.deleted",
            "deleted": true
        });

        Mock::given(method("DELETE"))
            .and(path("/threads/thread_abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete(&client, "thread_abc123").await.expect("should succeed");

        assert_eq!(result.id, "thread_abc123");
        assert!(result.deleted);
    }
}
