//! Run execution for Azure AI Foundry Agent Service.
//!
//! A run executes an agent on a thread to generate responses. Runs go through
//! various statuses (queued, in_progress, completed, failed, etc.) and may
//! require action if the agent needs to use tools.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::{agent, thread, message, run};
//! use azure_ai_foundry_agents::agent::AgentCreateRequest;
//! use azure_ai_foundry_agents::message::MessageCreateRequest;
//! use azure_ai_foundry_agents::run::{RunCreateRequest, RunStatus};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // Create an agent
//! let agent_req = AgentCreateRequest::builder()
//!     .model("gpt-4o")
//!     .instructions("You are helpful.")
//!     .build()?;
//! let agent = agent::create(&client, &agent_req).await?;
//!
//! // Create a thread with a message
//! let thread = thread::create(&client, None).await?;
//! let msg_req = MessageCreateRequest::builder()
//!     .content("What is 2+2?")
//!     .build()?;
//! message::create(&client, &thread.id, &msg_req).await?;
//!
//! // Run the agent on the thread
//! let run_req = RunCreateRequest::builder()
//!     .assistant_id(&agent.id)
//!     .build()?;
//! let mut run_result = run::create(&client, &thread.id, &run_req).await?;
//!
//! // Poll until complete
//! while !matches!(run_result.status, RunStatus::Completed | RunStatus::Failed) {
//!     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//!     run_result = run::get(&client, &thread.id, &run_result.id).await?;
//! }
//!
//! println!("Run completed with status: {:?}", run_result.status);
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::API_VERSION;
use crate::thread::Thread;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A request to create a run on a thread.
#[derive(Debug, Clone, Serialize)]
pub struct RunCreateRequest {
    /// The ID of the assistant to run.
    pub assistant_id: String,

    /// Optional override instructions for this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional additional instructions appended to the agent's instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_instructions: Option<String>,

    /// Optional metadata for the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional temperature override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Optional top_p override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Optional maximum number of prompt tokens to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_prompt_tokens: Option<u32>,

    /// Optional maximum number of completion tokens to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
}

/// Builder for [`RunCreateRequest`].
#[derive(Debug, Default)]
pub struct RunCreateRequestBuilder {
    assistant_id: Option<String>,
    instructions: Option<String>,
    additional_instructions: Option<String>,
    metadata: Option<serde_json::Value>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_prompt_tokens: Option<u32>,
    max_completion_tokens: Option<u32>,
}

impl RunCreateRequest {
    /// Create a new builder for `RunCreateRequest`.
    pub fn builder() -> RunCreateRequestBuilder {
        RunCreateRequestBuilder::default()
    }
}

impl RunCreateRequestBuilder {
    /// Set the assistant ID to run.
    ///
    /// **Required.**
    pub fn assistant_id(mut self, assistant_id: impl Into<String>) -> Self {
        self.assistant_id = Some(assistant_id.into());
        self
    }

    /// Override the instructions for this run.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Append additional instructions to the agent's instructions.
    pub fn additional_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.additional_instructions = Some(instructions.into());
        self
    }

    /// Set metadata for the run.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Override the temperature for this run.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Override the top_p for this run.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set maximum prompt tokens.
    pub fn max_prompt_tokens(mut self, tokens: u32) -> Self {
        self.max_prompt_tokens = Some(tokens);
        self
    }

    /// Set maximum completion tokens.
    pub fn max_completion_tokens(mut self, tokens: u32) -> Self {
        self.max_completion_tokens = Some(tokens);
        self
    }

    /// Build the request.
    ///
    /// # Errors
    ///
    /// Returns an error if `assistant_id` is not set.
    pub fn build(self) -> FoundryResult<RunCreateRequest> {
        let assistant_id = self
            .assistant_id
            .ok_or_else(|| FoundryError::Builder("assistant_id is required".into()))?;

        if assistant_id.trim().is_empty() {
            return Err(FoundryError::Builder("assistant_id cannot be empty".into()));
        }

        // Validate temperature if provided
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(FoundryError::Builder(
                    "temperature must be between 0.0 and 2.0".into(),
                ));
            }
        }

        // Validate top_p if provided
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(FoundryError::Builder(
                    "top_p must be between 0.0 and 1.0".into(),
                ));
            }
        }

        Ok(RunCreateRequest {
            assistant_id,
            instructions: self.instructions,
            additional_instructions: self.additional_instructions,
            metadata: self.metadata,
            temperature: self.temperature,
            top_p: self.top_p,
            max_prompt_tokens: self.max_prompt_tokens,
            max_completion_tokens: self.max_completion_tokens,
        })
    }
}

/// Request to create a thread and run in a single call.
#[derive(Debug, Clone, Serialize)]
pub struct CreateThreadAndRunRequest {
    /// The ID of the assistant to run.
    pub assistant_id: String,

    /// Optional thread configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<ThreadCreateConfig>,

    /// Optional override instructions for this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional metadata for the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Configuration for creating a thread as part of a run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ThreadCreateConfig {
    /// Initial messages to add to the thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<InitialMessage>>,

    /// Optional metadata for the thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// An initial message to add when creating a thread.
#[derive(Debug, Clone, Serialize)]
pub struct InitialMessage {
    /// Role of the message (typically "user").
    pub role: String,
    /// Content of the message.
    pub content: String,
}

/// Builder for [`CreateThreadAndRunRequest`].
#[derive(Debug, Default)]
pub struct CreateThreadAndRunRequestBuilder {
    assistant_id: Option<String>,
    messages: Vec<InitialMessage>,
    thread_metadata: Option<serde_json::Value>,
    instructions: Option<String>,
    run_metadata: Option<serde_json::Value>,
}

impl CreateThreadAndRunRequest {
    /// Create a new builder.
    pub fn builder() -> CreateThreadAndRunRequestBuilder {
        CreateThreadAndRunRequestBuilder::default()
    }
}

impl CreateThreadAndRunRequestBuilder {
    /// Set the assistant ID.
    pub fn assistant_id(mut self, id: impl Into<String>) -> Self {
        self.assistant_id = Some(id.into());
        self
    }

    /// Add an initial user message.
    pub fn message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(InitialMessage {
            role: "user".into(),
            content: content.into(),
        });
        self
    }

    /// Set thread metadata.
    pub fn thread_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.thread_metadata = Some(metadata);
        self
    }

    /// Override instructions for the run.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Set run metadata.
    pub fn run_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.run_metadata = Some(metadata);
        self
    }

    /// Build the request.
    pub fn build(self) -> FoundryResult<CreateThreadAndRunRequest> {
        let assistant_id = self
            .assistant_id
            .ok_or_else(|| FoundryError::Builder("assistant_id is required".into()))?;

        let thread = if !self.messages.is_empty() || self.thread_metadata.is_some() {
            Some(ThreadCreateConfig {
                messages: if self.messages.is_empty() {
                    None
                } else {
                    Some(self.messages)
                },
                metadata: self.thread_metadata,
            })
        } else {
            None
        };

        Ok(CreateThreadAndRunRequest {
            assistant_id,
            thread,
            instructions: self.instructions,
            metadata: self.run_metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// The status of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run is waiting to be processed.
    Queued,
    /// The run is currently being processed.
    InProgress,
    /// The run requires action (tool call outputs).
    RequiresAction,
    /// The run is being cancelled.
    Cancelling,
    /// The run was cancelled.
    Cancelled,
    /// The run failed.
    Failed,
    /// The run completed successfully.
    Completed,
    /// The run is incomplete (hit token limit).
    Incomplete,
    /// The run expired.
    Expired,
}

/// A run on a thread.
#[derive(Debug, Clone, Deserialize)]
pub struct Run {
    /// Unique identifier for the run.
    pub id: String,

    /// Object type, always "thread.run".
    pub object: String,

    /// Unix timestamp when the run was created.
    pub created_at: u64,

    /// The thread ID this run is on.
    pub thread_id: String,

    /// The assistant ID used for this run.
    pub assistant_id: String,

    /// The current status of the run.
    pub status: RunStatus,

    /// Required action if status is `requires_action`.
    pub required_action: Option<RequiredAction>,

    /// The last error that occurred, if any.
    pub last_error: Option<RunError>,

    /// Unix timestamp when the run started.
    pub started_at: Option<u64>,

    /// Unix timestamp when the run will expire.
    pub expires_at: Option<u64>,

    /// Unix timestamp when the run was cancelled.
    pub cancelled_at: Option<u64>,

    /// Unix timestamp when the run failed.
    pub failed_at: Option<u64>,

    /// Unix timestamp when the run completed.
    pub completed_at: Option<u64>,

    /// The model used for this run.
    pub model: Option<String>,

    /// Instructions used for this run.
    pub instructions: Option<String>,

    /// Usage statistics for the run.
    pub usage: Option<RunUsage>,

    /// Metadata attached to the run.
    pub metadata: Option<serde_json::Value>,
}

/// Action required from the client.
#[derive(Debug, Clone, Deserialize)]
pub struct RequiredAction {
    /// The type of action required.
    #[serde(rename = "type")]
    pub action_type: String,

    /// Tool calls that need outputs submitted.
    pub submit_tool_outputs: Option<SubmitToolOutputs>,
}

/// Tool outputs that need to be submitted.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitToolOutputs {
    /// The tool calls that need outputs.
    pub tool_calls: Vec<ToolCall>,
}

/// A tool call that needs an output.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,

    /// The type of tool call.
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function that was called.
    pub function: Option<FunctionCall>,
}

/// A function call within a tool call.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionCall {
    /// The name of the function.
    pub name: String,

    /// The arguments passed to the function (JSON string).
    pub arguments: String,
}

/// Error information for a failed run.
#[derive(Debug, Clone, Deserialize)]
pub struct RunError {
    /// The error code.
    pub code: String,

    /// The error message.
    pub message: String,
}

/// Usage statistics for a run.
#[derive(Debug, Clone, Deserialize)]
pub struct RunUsage {
    /// Number of prompt tokens used.
    pub prompt_tokens: u32,

    /// Number of completion tokens used.
    pub completion_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Create and start a run on a thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, RunCreateRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = RunCreateRequest::builder()
///     .assistant_id("asst_abc123")
///     .build()?;
///
/// let run = run::create(client, "thread_xyz", &request).await?;
/// println!("Run started: {} (status: {:?})", run.id, run.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::create` with fields `thread_id` and `assistant_id`.
#[tracing::instrument(
    name = "foundry::runs::create",
    skip(client, request),
    fields(thread_id = %thread_id, assistant_id = %request.assistant_id)
)]
pub async fn create(
    client: &FoundryClient,
    thread_id: &str,
    request: &RunCreateRequest,
) -> FoundryResult<Run> {
    tracing::debug!("creating run");

    let path = format!("/threads/{}/runs?{}", thread_id, API_VERSION);
    let response = client.post(&path, request).await?;
    let run = response.json::<Run>().await?;

    tracing::debug!(run_id = %run.id, status = ?run.status, "run created");
    Ok(run)
}

/// Get the current state of a run.
///
/// Use this to poll for run completion.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, RunStatus};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let mut run = run::get(client, "thread_xyz", "run_abc").await?;
///
/// while !matches!(run.status, RunStatus::Completed | RunStatus::Failed) {
///     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///     run = run::get(client, "thread_xyz", "run_abc").await?;
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::get` with fields `thread_id` and `run_id`.
#[tracing::instrument(
    name = "foundry::runs::get",
    skip(client),
    fields(thread_id = %thread_id, run_id = %run_id)
)]
pub async fn get(client: &FoundryClient, thread_id: &str, run_id: &str) -> FoundryResult<Run> {
    tracing::debug!("getting run");

    let path = format!("/threads/{}/runs/{}?{}", thread_id, run_id, API_VERSION);
    let response = client.get(&path).await?;
    let run = response.json::<Run>().await?;

    tracing::debug!(status = ?run.status, "run retrieved");
    Ok(run)
}

/// Create a thread and run in a single request.
///
/// This is useful for one-off conversations where you don't need to reuse the thread.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, CreateThreadAndRunRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = CreateThreadAndRunRequest::builder()
///     .assistant_id("asst_abc123")
///     .message("What is the weather like?")
///     .build()?;
///
/// let run = run::create_thread_and_run(client, &request).await?;
/// println!("Thread: {}, Run: {}", run.thread_id, run.id);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::create_thread_and_run` with field `assistant_id`.
#[tracing::instrument(
    name = "foundry::runs::create_thread_and_run",
    skip(client, request),
    fields(assistant_id = %request.assistant_id)
)]
pub async fn create_thread_and_run(
    client: &FoundryClient,
    request: &CreateThreadAndRunRequest,
) -> FoundryResult<Run> {
    tracing::debug!("creating thread and run");

    let path = format!("/threads/runs?{}", API_VERSION);
    let response = client.post(&path, request).await?;
    let run = response.json::<Run>().await?;

    tracing::debug!(
        thread_id = %run.thread_id,
        run_id = %run.id,
        status = ?run.status,
        "thread and run created"
    );
    Ok(run)
}

/// Poll a run until it reaches a terminal state.
///
/// Returns the final run state when it completes, fails, or is cancelled.
///
/// # Arguments
///
/// * `client` - The Foundry client.
/// * `thread_id` - The thread ID.
/// * `run_id` - The run ID.
/// * `poll_interval` - How often to check the run status.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run;
/// # use std::time::Duration;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let final_run = run::poll_until_complete(
///     client,
///     "thread_xyz",
///     "run_abc",
///     Duration::from_secs(1),
/// ).await?;
///
/// println!("Run finished with status: {:?}", final_run.status);
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(
    name = "foundry::runs::poll_until_complete",
    skip(client),
    fields(thread_id = %thread_id, run_id = %run_id)
)]
pub async fn poll_until_complete(
    client: &FoundryClient,
    thread_id: &str,
    run_id: &str,
    poll_interval: std::time::Duration,
) -> FoundryResult<Run> {
    loop {
        let run = get(client, thread_id, run_id).await?;

        match run.status {
            RunStatus::Completed
            | RunStatus::Failed
            | RunStatus::Cancelled
            | RunStatus::Expired
            | RunStatus::Incomplete => {
                tracing::debug!(status = ?run.status, "run reached terminal state");
                return Ok(run);
            }
            RunStatus::RequiresAction => {
                tracing::warn!("run requires action - returning for tool output submission");
                return Ok(run);
            }
            _ => {
                tracing::trace!(status = ?run.status, "run still in progress");
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
}

/// Create a thread and run, then poll until complete.
///
/// Convenience function that combines [`create_thread_and_run`] with [`poll_until_complete`].
/// Returns both the thread and the final run state.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, CreateThreadAndRunRequest};
/// # use std::time::Duration;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = CreateThreadAndRunRequest::builder()
///     .assistant_id("asst_abc123")
///     .message("Hello!")
///     .build()?;
///
/// let (thread, run) = run::create_and_poll(client, &request, Duration::from_secs(1)).await?;
/// println!("Final status: {:?}", run.status);
/// # Ok(())
/// # }
/// ```
pub async fn create_and_poll(
    client: &FoundryClient,
    request: &CreateThreadAndRunRequest,
    poll_interval: std::time::Duration,
) -> FoundryResult<(Thread, Run)> {
    let initial_run = create_thread_and_run(client, request).await?;
    let thread_id = initial_run.thread_id.clone();

    // Get the thread
    let thread = crate::thread::get(client, &thread_id).await?;

    // Poll until complete
    let final_run = poll_until_complete(client, &thread_id, &initial_run.id, poll_interval).await?;

    Ok((thread, final_run))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Cycle 16: Run types tests ---

    #[test]
    fn test_run_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<RunStatus>("\"queued\"").unwrap(),
            RunStatus::Queued
        );
        assert_eq!(
            serde_json::from_str::<RunStatus>("\"in_progress\"").unwrap(),
            RunStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<RunStatus>("\"completed\"").unwrap(),
            RunStatus::Completed
        );
        assert_eq!(
            serde_json::from_str::<RunStatus>("\"failed\"").unwrap(),
            RunStatus::Failed
        );
        assert_eq!(
            serde_json::from_str::<RunStatus>("\"requires_action\"").unwrap(),
            RunStatus::RequiresAction
        );
    }

    #[test]
    fn test_run_request_serialization() {
        let request = RunCreateRequest::builder()
            .assistant_id("asst_abc")
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["assistant_id"], "asst_abc");
    }

    #[test]
    fn test_run_builder_requires_assistant_id() {
        let result = RunCreateRequest::builder().build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("assistant_id is required"));
    }

    #[test]
    fn test_run_builder_validates_temperature() {
        let result = RunCreateRequest::builder()
            .assistant_id("asst_abc")
            .temperature(3.0)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_run_response_deserialization() {
        let json = serde_json::json!({
            "id": "run_abc123",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "completed",
            "model": "gpt-4o",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            }
        });

        let run: Run = serde_json::from_value(json).unwrap();

        assert_eq!(run.id, "run_abc123");
        assert_eq!(run.status, RunStatus::Completed);
        assert!(run.usage.is_some());
        assert_eq!(run.usage.as_ref().unwrap().total_tokens, 150);
    }

    // --- Cycle 17: Create run API tests ---

    #[tokio::test]
    async fn test_create_run_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "run_test123",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_abc",
            "assistant_id": "asst_xyz",
            "status": "queued"
        });

        Mock::given(method("POST"))
            .and(path("/threads/thread_abc/runs"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(body_json(serde_json::json!({
                "assistant_id": "asst_xyz"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = RunCreateRequest::builder()
            .assistant_id("asst_xyz")
            .build()
            .expect("valid request");

        let run = create(&client, "thread_abc", &request)
            .await
            .expect("should succeed");

        assert_eq!(run.id, "run_test123");
        assert_eq!(run.status, RunStatus::Queued);
    }

    // --- Cycle 18: Get run API tests ---

    #[tokio::test]
    async fn test_get_run_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "run_abc",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "in_progress"
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_xyz/runs/run_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let run = get(&client, "thread_xyz", "run_abc")
            .await
            .expect("should succeed");

        assert_eq!(run.id, "run_abc");
        assert_eq!(run.status, RunStatus::InProgress);
    }

    // --- Cycle 19: Create thread and run tests ---

    #[test]
    fn test_create_thread_and_run_request_serialization() {
        let request = CreateThreadAndRunRequest::builder()
            .assistant_id("asst_abc")
            .message("Hello!")
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["assistant_id"], "asst_abc");
        assert!(json["thread"]["messages"].is_array());
        assert_eq!(json["thread"]["messages"][0]["content"], "Hello!");
    }

    #[tokio::test]
    async fn test_create_thread_and_run_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "run_new123",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_new456",
            "assistant_id": "asst_abc",
            "status": "queued"
        });

        Mock::given(method("POST"))
            .and(path("/threads/runs"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = CreateThreadAndRunRequest::builder()
            .assistant_id("asst_abc")
            .message("Hi there!")
            .build()
            .expect("valid request");

        let run = create_thread_and_run(&client, &request)
            .await
            .expect("should succeed");

        assert_eq!(run.id, "run_new123");
        assert_eq!(run.thread_id, "thread_new456");
    }

    // --- Run with required action tests ---

    #[test]
    fn test_run_with_required_action_deserialization() {
        let json = serde_json::json!({
            "id": "run_action",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "requires_action",
            "required_action": {
                "type": "submit_tool_outputs",
                "submit_tool_outputs": {
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"NYC\"}"
                        }
                    }]
                }
            }
        });

        let run: Run = serde_json::from_value(json).unwrap();

        assert_eq!(run.status, RunStatus::RequiresAction);
        assert!(run.required_action.is_some());

        let action = run.required_action.unwrap();
        assert_eq!(action.action_type, "submit_tool_outputs");

        let tool_calls = action.submit_tool_outputs.unwrap().tool_calls;
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_abc");
        assert_eq!(tool_calls[0].function.as_ref().unwrap().name, "get_weather");
    }
}
