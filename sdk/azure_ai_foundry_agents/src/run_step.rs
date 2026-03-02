//! Run step inspection for Azure AI Foundry Agent Service.
//!
//! Run steps represent the individual actions taken during a run (e.g., message
//! creation, tool calls). Use this module to inspect what an agent did during execution.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::run_step;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // List steps of a completed run
//! let steps = run_step::list(&client, "thread_abc", "run_xyz").await?;
//! for step in steps.data {
//!     println!("Step {}: {:?} ({:?})", step.id, step.step_type, step.status);
//! }
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::FoundryResult;
use serde::{Deserialize, Serialize};

use crate::models::API_VERSION;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The status of a run step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStepStatus {
    /// The step is currently being processed.
    InProgress,
    /// The step was cancelled.
    Cancelled,
    /// The step failed.
    Failed,
    /// The step completed successfully.
    Completed,
    /// The step expired.
    Expired,
}

/// The type of action taken during a run step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// The step created a message.
    MessageCreation,
    /// The step invoked one or more tools.
    ToolCalls,
}

/// The type of tool called within a run step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallType {
    /// A user-defined function.
    Function,
    /// The code interpreter built-in tool.
    CodeInterpreter,
    /// The file search built-in tool.
    FileSearch,
}

/// Details about what a run step did.
#[derive(Debug, Clone, Deserialize)]
pub struct StepDetails {
    /// The type of step detail.
    #[serde(rename = "type")]
    pub detail_type: StepType,

    /// Details about message creation (present when detail_type is "message_creation").
    pub message_creation: Option<MessageCreationDetails>,

    /// Details about tool calls (present when detail_type is "tool_calls").
    pub tool_calls: Option<Vec<ToolCallDetails>>,
}

/// Details about a message creation step.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageCreationDetails {
    /// The ID of the message that was created.
    pub message_id: String,
}

/// Details about a tool call within a step.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallDetails {
    /// The ID of the tool call.
    pub id: String,

    /// The type of tool call.
    #[serde(rename = "type")]
    pub call_type: ToolCallType,

    /// Function call details (present when call_type is "function").
    pub function: Option<FunctionCallDetails>,

    /// Code interpreter details (present when call_type is "code_interpreter").
    pub code_interpreter: Option<serde_json::Value>,

    /// File search details (present when call_type is "file_search").
    pub file_search: Option<serde_json::Value>,
}

/// Details about a function call.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionCallDetails {
    /// The name of the function.
    pub name: String,

    /// The arguments passed to the function (JSON string).
    pub arguments: String,

    /// The output of the function (set after tool output submission).
    pub output: Option<String>,
}

/// Error information for a run step.
#[derive(Debug, Clone, Deserialize)]
pub struct RunStepError {
    /// The error code.
    pub code: String,

    /// The error message.
    pub message: String,
}

/// Usage statistics for a run step.
#[derive(Debug, Clone, Deserialize)]
pub struct RunStepUsage {
    /// Number of prompt tokens used.
    pub prompt_tokens: u32,

    /// Number of completion tokens used.
    pub completion_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

/// A single step within a run.
#[derive(Debug, Clone, Deserialize)]
pub struct RunStep {
    /// Unique identifier for the run step.
    pub id: String,

    /// Object type, always "thread.run.step".
    pub object: String,

    /// Unix timestamp when the step was created.
    pub created_at: u64,

    /// The run ID this step belongs to.
    pub run_id: String,

    /// The assistant ID associated with this step.
    pub assistant_id: String,

    /// The thread ID this step belongs to.
    pub thread_id: String,

    /// The type of step.
    #[serde(rename = "type")]
    pub step_type: StepType,

    /// The current status of the step.
    pub status: RunStepStatus,

    /// Details about what the step did.
    pub step_details: StepDetails,

    /// Error information if the step failed.
    pub last_error: Option<RunStepError>,

    /// Unix timestamp when the step expired.
    pub expired_at: Option<u64>,

    /// Unix timestamp when the step was cancelled.
    pub cancelled_at: Option<u64>,

    /// Unix timestamp when the step failed.
    pub failed_at: Option<u64>,

    /// Unix timestamp when the step completed.
    pub completed_at: Option<u64>,

    /// Usage statistics for the step.
    pub usage: Option<RunStepUsage>,
}

/// Response from listing run steps.
#[derive(Debug, Clone, Deserialize)]
pub struct RunStepList {
    /// Object type, always "list".
    pub object: String,

    /// List of run steps.
    pub data: Vec<RunStep>,

    /// ID of the first step in the list.
    pub first_id: Option<String>,

    /// ID of the last step in the list.
    pub last_id: Option<String>,

    /// Whether there are more steps to fetch.
    pub has_more: bool,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// List the steps of a run.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run_step;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let steps = run_step::list(client, "thread_abc", "run_xyz").await?;
/// for step in steps.data {
///     println!("{}: {:?} ({:?})", step.id, step.step_type, step.status);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::run_steps::list` with fields `thread_id` and `run_id`.
#[tracing::instrument(
    name = "foundry::run_steps::list",
    skip(client),
    fields(thread_id = %thread_id, run_id = %run_id)
)]
pub async fn list(
    client: &FoundryClient,
    thread_id: &str,
    run_id: &str,
) -> FoundryResult<RunStepList> {
    tracing::debug!("listing run steps");
    FoundryClient::validate_resource_id(thread_id)?;
    FoundryClient::validate_resource_id(run_id)?;
    let path = format!(
        "/threads/{}/runs/{}/steps?{}",
        thread_id, run_id, API_VERSION
    );
    let response = client.get(&path).await?;
    let list = response.json::<RunStepList>().await?;

    tracing::debug!(count = list.data.len(), "run steps listed");
    Ok(list)
}

/// Get a specific run step.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::run_step;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let step = run_step::get(client, "thread_abc", "run_xyz", "step_123").await?;
/// println!("Step type: {:?}", step.step_type);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::run_steps::get` with fields `thread_id`, `run_id`, and `step_id`.
#[tracing::instrument(
    name = "foundry::run_steps::get",
    skip(client),
    fields(thread_id = %thread_id, run_id = %run_id, step_id = %step_id)
)]
pub async fn get(
    client: &FoundryClient,
    thread_id: &str,
    run_id: &str,
    step_id: &str,
) -> FoundryResult<RunStep> {
    tracing::debug!("getting run step");
    FoundryClient::validate_resource_id(thread_id)?;
    FoundryClient::validate_resource_id(run_id)?;
    FoundryClient::validate_resource_id(step_id)?;
    let path = format!(
        "/threads/{}/runs/{}/steps/{}?{}",
        thread_id, run_id, step_id, API_VERSION
    );
    let response = client.get(&path).await?;
    let step = response.json::<RunStep>().await?;

    Ok(step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Quality: StepType and ToolCallType enum serde ---

    #[test]
    fn test_step_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<StepType>("\"message_creation\"").unwrap(),
            StepType::MessageCreation
        );
        assert_eq!(
            serde_json::from_str::<StepType>("\"tool_calls\"").unwrap(),
            StepType::ToolCalls
        );
    }

    #[test]
    fn test_tool_call_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<ToolCallType>("\"function\"").unwrap(),
            ToolCallType::Function
        );
        assert_eq!(
            serde_json::from_str::<ToolCallType>("\"code_interpreter\"").unwrap(),
            ToolCallType::CodeInterpreter
        );
        assert_eq!(
            serde_json::from_str::<ToolCallType>("\"file_search\"").unwrap(),
            ToolCallType::FileSearch
        );
    }

    fn sample_message_creation_step() -> serde_json::Value {
        serde_json::json!({
            "id": "step_msg",
            "object": "thread.run.step",
            "created_at": TEST_TIMESTAMP,
            "run_id": "run_abc",
            "assistant_id": "asst_123",
            "thread_id": "thread_xyz",
            "type": "message_creation",
            "status": "completed",
            "step_details": {
                "type": "message_creation",
                "message_creation": {
                    "message_id": "msg_output"
                }
            },
            "completed_at": TEST_TIMESTAMP
        })
    }

    fn sample_tool_calls_step() -> serde_json::Value {
        serde_json::json!({
            "id": "step_tool",
            "object": "thread.run.step",
            "created_at": TEST_TIMESTAMP,
            "run_id": "run_abc",
            "assistant_id": "asst_123",
            "thread_id": "thread_xyz",
            "type": "tool_calls",
            "status": "completed",
            "step_details": {
                "type": "tool_calls",
                "tool_calls": [{
                    "id": "call_abc",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"location\": \"NYC\"}",
                        "output": "Sunny, 72°F"
                    }
                }]
            },
            "completed_at": TEST_TIMESTAMP,
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 25,
                "total_tokens": 75
            }
        })
    }

    // --- Cycle 5.1: RunStep message creation deserialization ---

    #[test]
    fn test_run_step_message_creation_deserialization() {
        let step: RunStep = serde_json::from_value(sample_message_creation_step()).unwrap();

        assert_eq!(step.id, "step_msg");
        assert_eq!(step.object, "thread.run.step");
        assert_eq!(step.run_id, "run_abc");
        assert_eq!(step.step_type, StepType::MessageCreation);
        assert_eq!(step.status, RunStepStatus::Completed);
        assert_eq!(step.step_details.detail_type, StepType::MessageCreation);
        assert_eq!(
            step.step_details
                .message_creation
                .as_ref()
                .unwrap()
                .message_id,
            "msg_output"
        );
        assert!(step.step_details.tool_calls.is_none());
        assert_eq!(step.completed_at, Some(TEST_TIMESTAMP));
        assert!(step.usage.is_none());
    }

    // --- Cycle 5.2: RunStep with tool_calls ---

    #[test]
    fn test_run_step_tool_calls_deserialization() {
        let step: RunStep = serde_json::from_value(sample_tool_calls_step()).unwrap();

        assert_eq!(step.id, "step_tool");
        assert_eq!(step.step_type, StepType::ToolCalls);
        assert_eq!(step.step_details.detail_type, StepType::ToolCalls);

        let tool_calls = step.step_details.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_abc");
        assert_eq!(tool_calls[0].call_type, ToolCallType::Function);

        let func = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(func.name, "get_weather");
        assert_eq!(func.arguments, "{\"location\": \"NYC\"}");
        assert_eq!(func.output, Some("Sunny, 72°F".into()));
    }

    // --- Cycle 5.3: RunStepList ---

    #[test]
    fn test_run_step_list_deserialization() {
        let json = serde_json::json!({
            "object": "list",
            "data": [sample_message_creation_step(), sample_tool_calls_step()],
            "first_id": "step_msg",
            "last_id": "step_tool",
            "has_more": false
        });

        let list: RunStepList = serde_json::from_value(json).unwrap();

        assert_eq!(list.data.len(), 2);
        assert_eq!(list.first_id, Some("step_msg".into()));
        assert_eq!(list.last_id, Some("step_tool".into()));
        assert!(!list.has_more);
    }

    // --- Cycle 5.4: list() API ---

    #[tokio::test]
    async fn test_list_run_steps_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "object": "list",
            "data": [sample_message_creation_step()],
            "first_id": "step_msg",
            "last_id": "step_msg",
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_xyz/runs/run_abc/steps"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let steps = list(&client, "thread_xyz", "run_abc")
            .await
            .expect("should succeed");

        assert_eq!(steps.data.len(), 1);
        assert_eq!(steps.data[0].id, "step_msg");
    }

    // --- Cycle 5.5: get() API ---

    #[tokio::test]
    async fn test_get_run_step_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/threads/thread_xyz/runs/run_abc/steps/step_msg"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_message_creation_step()))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let step = get(&client, "thread_xyz", "run_abc", "step_msg")
            .await
            .expect("should succeed");

        assert_eq!(step.id, "step_msg");
        assert_eq!(step.step_type, StepType::MessageCreation);
    }

    // --- Cycle 5.6: Edge cases ---

    #[test]
    fn test_run_step_status_serde() {
        assert_eq!(
            serde_json::from_str::<RunStepStatus>("\"in_progress\"").unwrap(),
            RunStepStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<RunStepStatus>("\"cancelled\"").unwrap(),
            RunStepStatus::Cancelled
        );
        assert_eq!(
            serde_json::from_str::<RunStepStatus>("\"failed\"").unwrap(),
            RunStepStatus::Failed
        );
        assert_eq!(
            serde_json::from_str::<RunStepStatus>("\"completed\"").unwrap(),
            RunStepStatus::Completed
        );
        assert_eq!(
            serde_json::from_str::<RunStepStatus>("\"expired\"").unwrap(),
            RunStepStatus::Expired
        );
    }

    #[test]
    fn test_run_step_with_usage() {
        let step: RunStep = serde_json::from_value(sample_tool_calls_step()).unwrap();

        let usage = step.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 50);
        assert_eq!(usage.completion_tokens, 25);
        assert_eq!(usage.total_tokens, 75);
    }

    #[test]
    fn test_run_step_minimal() {
        let json = serde_json::json!({
            "id": "step_min",
            "object": "thread.run.step",
            "created_at": TEST_TIMESTAMP,
            "run_id": "run_abc",
            "assistant_id": "asst_123",
            "thread_id": "thread_xyz",
            "type": "message_creation",
            "status": "in_progress",
            "step_details": {
                "type": "message_creation",
                "message_creation": {
                    "message_id": "msg_123"
                }
            }
        });

        let step: RunStep = serde_json::from_value(json).unwrap();

        assert_eq!(step.id, "step_min");
        assert_eq!(step.status, RunStepStatus::InProgress);
        assert!(step.last_error.is_none());
        assert!(step.expired_at.is_none());
        assert!(step.cancelled_at.is_none());
        assert!(step.failed_at.is_none());
        assert!(step.completed_at.is_none());
        assert!(step.usage.is_none());
    }

    #[tokio::test]
    async fn test_list_run_steps_empty() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "object": "list",
            "data": [],
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/threads/thread_xyz/runs/run_abc/steps"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let steps = list(&client, "thread_xyz", "run_abc")
            .await
            .expect("should succeed");

        assert!(steps.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_run_step_rejects_path_traversal() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let result = get(&client, "../evil", "run_123", "step_123").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, azure_ai_foundry_core::error::FoundryError::Validation { .. }),
            "Expected Validation error, got: {:?}",
            err
        );
    }
}
