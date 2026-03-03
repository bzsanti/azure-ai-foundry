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
//!     .try_build()?;
//! let agent = agent::create(&client, &agent_req).await?;
//!
//! // Create a thread with a message
//! let thread = thread::create(&client, None).await?;
//! let msg_req = MessageCreateRequest::builder()
//!     .content("What is 2+2?")
//!     .try_build()?;
//! message::create(&client, &thread.id, &msg_req).await?;
//!
//! // Run the agent on the thread
//! let run_req = RunCreateRequest::builder()
//!     .assistant_id(&agent.id)
//!     .try_build()?;
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

use std::time::Duration;

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use azure_ai_foundry_core::models::Usage;
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
    pub fn try_build(self) -> FoundryResult<RunCreateRequest> {
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

    /// Build the request.
    ///
    /// # Panics
    ///
    /// Panics if `assistant_id` is not set, or if `temperature` or `top_p` is
    /// out of range. Use [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> RunCreateRequest {
        self.try_build().expect("builder validation failed")
    }
}

/// A tool output to submit for a tool call.
///
/// When a run reaches the `requires_action` status, you must submit outputs
/// for each tool call before the run can continue.
#[derive(Debug, Clone, Serialize)]
pub struct ToolOutput {
    /// The ID of the tool call this output is for.
    pub tool_call_id: String,

    /// The output of the tool call.
    pub output: String,
}

/// Request to submit tool outputs for a run.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubmitToolOutputsRequest<'a> {
    /// The tool outputs to submit.
    pub tool_outputs: &'a [ToolOutput],
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
    /// Role of the message.
    pub role: crate::message::MessageRole,
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
            role: crate::message::MessageRole::User,
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

    /// Build the request, returning an error if required fields are missing.
    pub fn try_build(self) -> FoundryResult<CreateThreadAndRunRequest> {
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

    /// Build the request.
    ///
    /// # Panics
    ///
    /// Panics if `assistant_id` is not set.
    /// Use [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> CreateThreadAndRunRequest {
        self.try_build().expect("builder validation failed")
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

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "queued",
            Self::InProgress => "in_progress",
            Self::RequiresAction => "requires_action",
            Self::Cancelling => "cancelling",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Completed => "completed",
            Self::Incomplete => "incomplete",
            Self::Expired => "expired",
        };
        f.write_str(s)
    }
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
    pub usage: Option<Usage>,

    /// Metadata attached to the run.
    pub metadata: Option<serde_json::Value>,
}

/// The type of action required from the client.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredActionType {
    /// The client must submit tool outputs.
    SubmitToolOutputs,
}

/// Action required from the client.
#[derive(Debug, Clone, Deserialize)]
pub struct RequiredAction {
    /// The type of action required.
    #[serde(rename = "type")]
    pub action_type: RequiredActionType,

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
    pub call_type: crate::run_step::ToolCallType,

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

/// Deprecated: Use [`azure_ai_foundry_core::models::Usage`] instead.
///
/// This type alias will be **removed in v0.8.0**.
#[deprecated(
    since = "0.7.0",
    note = "Use azure_ai_foundry_core::models::Usage instead. This alias will be removed in v0.8.0."
)]
pub type RunUsage = Usage;

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
///     .try_build()?;
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
    FoundryClient::validate_resource_id(thread_id)?;
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
    FoundryClient::validate_resource_id(thread_id)?;
    FoundryClient::validate_resource_id(run_id)?;
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
///     .try_build()?;
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
/// * `max_attempts` - Maximum number of poll attempts, or `None` for unlimited.
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
///     Some(60),
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
    max_attempts: Option<u32>,
) -> FoundryResult<Run> {
    let mut attempts: u32 = 0;
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
                attempts += 1;
                if let Some(max) = max_attempts {
                    if attempts >= max {
                        return Err(FoundryError::validation(format!(
                            "run did not complete after {} poll attempts",
                            max
                        )));
                    }
                }
                tracing::trace!(status = ?run.status, attempt = attempts, "run still in progress");
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
///     .try_build()?;
///
/// let (thread, run) = run::create_and_poll(
///     client,
///     &request,
///     Duration::from_secs(1),
///     Some(60),
/// ).await?;
/// println!("Final status: {:?}", run.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::create_and_poll` with field `assistant_id`.
#[tracing::instrument(
    name = "foundry::runs::create_and_poll",
    skip(client, request),
    fields(assistant_id = %request.assistant_id)
)]
pub async fn create_and_poll(
    client: &FoundryClient,
    request: &CreateThreadAndRunRequest,
    poll_interval: std::time::Duration,
    max_attempts: Option<u32>,
) -> FoundryResult<(Thread, Run)> {
    tracing::debug!("creating thread, run, and polling until complete");

    let initial_run = create_thread_and_run(client, request).await?;
    let thread_id = initial_run.thread_id.clone();

    // Get the thread
    let thread = crate::thread::get(client, &thread_id).await?;

    // Poll until complete
    let final_run = poll_until_complete(
        client,
        &thread_id,
        &initial_run.id,
        poll_interval,
        max_attempts,
    )
    .await?;

    Ok((thread, final_run))
}

/// Submit tool outputs for a run that requires action.
///
/// When a run reaches `RunStatus::RequiresAction`, use this function to provide
/// the results of tool calls back to the agent.
///
/// # Arguments
///
/// * `client` - The Foundry client.
/// * `thread_id` - The thread ID.
/// * `run_id` - The run ID.
/// * `tool_outputs` - The tool outputs to submit.
///
/// # Errors
///
/// Returns an error if `tool_outputs` is empty.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, ToolOutput};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let outputs = vec![ToolOutput {
///     tool_call_id: "call_abc".to_string(),
///     output: "Sunny, 72°F".to_string(),
/// }];
///
/// let run = run::submit_tool_outputs(client, "thread_xyz", "run_abc", &outputs).await?;
/// println!("Run status after submission: {:?}", run.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::submit_tool_outputs` with fields `thread_id` and `run_id`.
#[tracing::instrument(
    name = "foundry::runs::submit_tool_outputs",
    skip(client, tool_outputs),
    fields(thread_id = %thread_id, run_id = %run_id, output_count = tool_outputs.len())
)]
pub async fn submit_tool_outputs(
    client: &FoundryClient,
    thread_id: &str,
    run_id: &str,
    tool_outputs: &[ToolOutput],
) -> FoundryResult<Run> {
    if tool_outputs.is_empty() {
        return Err(FoundryError::validation("tool_outputs cannot be empty"));
    }

    for output in tool_outputs {
        if output.tool_call_id.trim().is_empty() {
            return Err(FoundryError::validation_field(
                "tool_call_id",
                "tool_call_id cannot be empty",
            ));
        }
    }

    FoundryClient::validate_resource_id(thread_id)?;
    FoundryClient::validate_resource_id(run_id)?;
    tracing::debug!("submitting tool outputs");

    let path = format!(
        "/threads/{}/runs/{}/submit_tool_outputs?{}",
        thread_id, run_id, API_VERSION
    );
    let request = SubmitToolOutputsRequest { tool_outputs };
    let response = client.post(&path, &request).await?;
    let run = response.json::<Run>().await?;

    tracing::debug!(status = ?run.status, "tool outputs submitted");
    Ok(run)
}

/// Submit tool outputs and poll until the run reaches a terminal state.
///
/// Convenience function that combines [`submit_tool_outputs`] with [`poll_until_complete`].
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run::{self, ToolOutput};
/// # use std::time::Duration;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let outputs = vec![ToolOutput {
///     tool_call_id: "call_abc".to_string(),
///     output: "Result".to_string(),
/// }];
///
/// let run = run::submit_tool_outputs_and_poll(
///     client,
///     "thread_xyz",
///     "run_abc",
///     &outputs,
///     Duration::from_secs(1),
///     Some(60),
/// ).await?;
/// println!("Final status: {:?}", run.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::runs::submit_tool_outputs_and_poll` with fields
/// `thread_id`, `run_id`, and `output_count`.
#[tracing::instrument(
    name = "foundry::runs::submit_tool_outputs_and_poll",
    skip(client, tool_outputs),
    fields(thread_id = %thread_id, run_id = %run_id, output_count = tool_outputs.len())
)]
pub async fn submit_tool_outputs_and_poll(
    client: &FoundryClient,
    thread_id: &str,
    run_id: &str,
    tool_outputs: &[ToolOutput],
    poll_interval: Duration,
    max_attempts: Option<u32>,
) -> FoundryResult<Run> {
    tracing::debug!("submitting tool outputs and polling until complete");

    let run = submit_tool_outputs(client, thread_id, run_id, tool_outputs).await?;

    // If already terminal, return immediately
    match run.status {
        RunStatus::Completed
        | RunStatus::Failed
        | RunStatus::Cancelled
        | RunStatus::Expired
        | RunStatus::Incomplete => return Ok(run),
        _ => {}
    }

    poll_until_complete(client, thread_id, &run.id, poll_interval, max_attempts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Quality: create_and_poll tracing ---

    #[tokio::test]
    async fn test_create_and_poll_success() {
        let server = MockServer::start().await;

        // POST /threads/runs -> queued run
        let run_response = serde_json::json!({
            "id": "run_cp1",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_cp1",
            "assistant_id": "asst_abc",
            "status": "queued"
        });

        Mock::given(method("POST"))
            .and(path("/threads/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&run_response))
            .mount(&server)
            .await;

        // GET /threads/thread_cp1 -> thread
        let thread_response = serde_json::json!({
            "id": "thread_cp1",
            "object": "thread",
            "created_at": TEST_TIMESTAMP,
            "metadata": null
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_cp1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&thread_response))
            .mount(&server)
            .await;

        // GET /threads/thread_cp1/runs/run_cp1 -> completed
        let get_run_response = serde_json::json!({
            "id": "run_cp1",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_cp1",
            "assistant_id": "asst_abc",
            "status": "completed"
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_cp1/runs/run_cp1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&get_run_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = CreateThreadAndRunRequest::builder()
            .assistant_id("asst_abc")
            .message("Hello!")
            .build();

        let (thread, run) = create_and_poll(&client, &request, Duration::from_millis(10), None)
            .await
            .expect("should succeed");

        assert_eq!(thread.id, "thread_cp1");
        assert_eq!(run.status, RunStatus::Completed);
    }

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
        let request = RunCreateRequest::builder().assistant_id("asst_abc").build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["assistant_id"], "asst_abc");
    }

    #[test]
    fn test_run_builder_requires_assistant_id() {
        let result = RunCreateRequest::builder().try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("assistant_id is required"));
    }

    #[test]
    fn test_run_builder_validates_temperature() {
        let result = RunCreateRequest::builder()
            .assistant_id("asst_abc")
            .temperature(3.0)
            .try_build();

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
        let usage = run.usage.as_ref().unwrap();
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, Some(50));
    }

    #[test]
    fn test_run_deserializes_with_core_usage_type() {
        // Verifies that the core Usage type (with Option<u32> completion_tokens)
        // correctly deserializes from run JSON (where completion_tokens is always present).
        let json = serde_json::json!({
            "id": "run_usage_test",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "completed",
            "model": "gpt-4o",
            "usage": {
                "prompt_tokens": 200,
                "completion_tokens": 100,
                "total_tokens": 300
            }
        });

        let run: Run = serde_json::from_value(json).unwrap();
        let usage = run.usage.expect("should have usage");

        // Core Usage type wraps completion_tokens in Option
        assert_eq!(usage.prompt_tokens, 200);
        assert_eq!(usage.completion_tokens, Some(100));
        assert_eq!(usage.total_tokens, 300);
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

        let request = RunCreateRequest::builder().assistant_id("asst_xyz").build();

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
            .build();

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
            .build();

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
        assert_eq!(action.action_type, RequiredActionType::SubmitToolOutputs);

        let tool_calls = action.submit_tool_outputs.unwrap().tool_calls;
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_abc");
        assert_eq!(tool_calls[0].function.as_ref().unwrap().name, "get_weather");
    }

    // --- Phase 6: Submit Tool Outputs ---

    #[test]
    fn test_tool_output_serialization() {
        let output = ToolOutput {
            tool_call_id: "call_abc".to_string(),
            output: "Sunny, 72°F".to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["tool_call_id"], "call_abc");
        assert_eq!(json["output"], "Sunny, 72°F");
    }

    #[test]
    fn test_submit_tool_outputs_request_serialization() {
        let outputs = [
            ToolOutput {
                tool_call_id: "call_1".to_string(),
                output: "result1".to_string(),
            },
            ToolOutput {
                tool_call_id: "call_2".to_string(),
                output: "result2".to_string(),
            },
        ];
        let request = SubmitToolOutputsRequest {
            tool_outputs: &outputs,
        };

        let json = serde_json::to_value(&request).unwrap();

        let outputs = json["tool_outputs"].as_array().unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0]["tool_call_id"], "call_1");
        assert_eq!(outputs[1]["tool_call_id"], "call_2");
    }

    #[tokio::test]
    async fn test_submit_tool_outputs_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "run_abc",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "in_progress"
        });

        Mock::given(method("POST"))
            .and(path("/threads/thread_xyz/runs/run_abc/submit_tool_outputs"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(body_json(serde_json::json!({
                "tool_outputs": [{
                    "tool_call_id": "call_abc",
                    "output": "Sunny, 72°F"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let outputs = vec![ToolOutput {
            tool_call_id: "call_abc".to_string(),
            output: "Sunny, 72°F".to_string(),
        }];

        let run = submit_tool_outputs(&client, "thread_xyz", "run_abc", &outputs)
            .await
            .expect("should succeed");

        assert_eq!(run.id, "run_abc");
        assert_eq!(run.status, RunStatus::InProgress);
    }

    #[tokio::test]
    async fn test_submit_tool_outputs_rejects_empty() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let result = submit_tool_outputs(&client, "thread_xyz", "run_abc", &[]).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("tool_outputs cannot be empty"));
    }

    #[tokio::test]
    async fn test_submit_tool_outputs_validates_empty_tool_call_id() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let outputs = vec![ToolOutput {
            tool_call_id: "  ".to_string(),
            output: "result".to_string(),
        }];

        let result = submit_tool_outputs(&client, "thread_xyz", "run_abc", &outputs).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("tool_call_id cannot be empty"));
    }

    #[test]
    fn test_tool_output_with_json_output() {
        let output = ToolOutput {
            tool_call_id: "call_abc".to_string(),
            output: r#"{"temperature": 72, "condition": "sunny"}"#.to_string(),
        };

        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["tool_call_id"], "call_abc");
        assert_eq!(
            json["output"],
            r#"{"temperature": 72, "condition": "sunny"}"#
        );
    }

    #[tokio::test]
    async fn test_submit_tool_outputs_and_poll_success() {
        let server = MockServer::start().await;

        // First: submit returns in_progress
        let submit_response = serde_json::json!({
            "id": "run_poll",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "in_progress"
        });

        Mock::given(method("POST"))
            .and(path(
                "/threads/thread_xyz/runs/run_poll/submit_tool_outputs",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&submit_response))
            .mount(&server)
            .await;

        // Second: get returns completed
        let get_response = serde_json::json!({
            "id": "run_poll",
            "object": "thread.run",
            "created_at": TEST_TIMESTAMP,
            "thread_id": "thread_xyz",
            "assistant_id": "asst_123",
            "status": "completed"
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_xyz/runs/run_poll"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&get_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let outputs = vec![ToolOutput {
            tool_call_id: "call_abc".to_string(),
            output: "result".to_string(),
        }];

        let run = submit_tool_outputs_and_poll(
            &client,
            "thread_xyz",
            "run_poll",
            &outputs,
            Duration::from_millis(10),
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(run.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn test_poll_until_complete_respects_max_attempts() {
        let server = MockServer::start().await;

        // Always return in_progress
        Mock::given(method("GET"))
            .and(path("/threads/thread_lim/runs/run_lim"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_lim",
                "object": "thread.run",
                "thread_id": "thread_lim",
                "assistant_id": "asst_1",
                "status": "in_progress",
                "created_at": TEST_TIMESTAMP
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = poll_until_complete(
            &client,
            "thread_lim",
            "run_lim",
            Duration::from_millis(1),
            Some(3),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("did not complete after 3 poll attempts"),
            "Expected poll timeout message, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_poll_until_complete_none_unlimited_completes() {
        let server = MockServer::start().await;

        // Return in_progress twice, then completed
        Mock::given(method("GET"))
            .and(path("/threads/thread_u/runs/run_u"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_u",
                "object": "thread.run",
                "thread_id": "thread_u",
                "assistant_id": "asst_1",
                "status": "in_progress",
                "created_at": TEST_TIMESTAMP
            })))
            .up_to_n_times(2)
            .expect(2)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/threads/thread_u/runs/run_u"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_u",
                "object": "thread.run",
                "thread_id": "thread_u",
                "assistant_id": "asst_1",
                "status": "completed",
                "created_at": TEST_TIMESTAMP
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let run = poll_until_complete(&client, "thread_u", "run_u", Duration::from_millis(1), None)
            .await
            .expect("should complete");

        assert_eq!(run.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn test_get_run_rejects_path_traversal() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let result = get(&client, "../evil", "run_123").await;
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

    // =======================================================================
    // M3: Stringly-typed enums (R1 + R2 + R3)
    // =======================================================================

    // --- R1: RequiredActionType enum ---

    #[test]
    fn test_required_action_type_deserialization() {
        let json = serde_json::json!({
            "type": "submit_tool_outputs",
            "submit_tool_outputs": {
                "tool_calls": []
            }
        });
        let action: RequiredAction = serde_json::from_value(json).unwrap();
        assert_eq!(action.action_type, RequiredActionType::SubmitToolOutputs);
    }

    #[test]
    fn test_tool_call_type_in_run_deserialization() {
        let json = serde_json::json!({
            "id": "call_1",
            "type": "function",
            "function": {"name": "foo", "arguments": "{}"}
        });
        let tc: ToolCall = serde_json::from_value(json).unwrap();
        assert_eq!(tc.call_type, crate::run_step::ToolCallType::Function);
    }

    // --- R3: InitialMessage uses MessageRole ---

    #[test]
    fn test_initial_message_serializes_role_as_string() {
        let msg = InitialMessage {
            role: crate::message::MessageRole::User,
            content: "hello".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
    }

    // --- M4 R7: Poll timeout uses Validation, not Api ---

    #[tokio::test]
    async fn test_poll_timeout_returns_validation_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/threads/thread_val/runs/run_val"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_val",
                "object": "thread.run",
                "thread_id": "thread_val",
                "assistant_id": "asst_1",
                "status": "in_progress",
                "created_at": TEST_TIMESTAMP
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = poll_until_complete(
            &client,
            "thread_val",
            "run_val",
            Duration::from_millis(1),
            Some(2),
        )
        .await;

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
        assert!(err
            .to_string()
            .contains("did not complete after 2 poll attempts"));
    }

    // --- Cycle 6.3: Display for RunStatus ---

    #[test]
    fn test_run_status_display_matches_serde() {
        let pairs = [
            (RunStatus::Queued, "queued"),
            (RunStatus::InProgress, "in_progress"),
            (RunStatus::RequiresAction, "requires_action"),
            (RunStatus::Cancelling, "cancelling"),
            (RunStatus::Cancelled, "cancelled"),
            (RunStatus::Failed, "failed"),
            (RunStatus::Completed, "completed"),
            (RunStatus::Incomplete, "incomplete"),
            (RunStatus::Expired, "expired"),
        ];
        for (status, expected) in pairs {
            assert_eq!(
                status.to_string(),
                expected,
                "Display mismatch for {:?}",
                status
            );
        }
    }
}
