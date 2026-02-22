//! Agent management for Azure AI Foundry Agent Service.
//!
//! This module provides functions to create, retrieve, list, update, and delete
//! AI agents. An agent is configured with a model, instructions, and optional tools.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::agent::{self, AgentCreateRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // Create an agent
//! let request = AgentCreateRequest::builder()
//!     .model("gpt-4o")
//!     .name("My Assistant")
//!     .instructions("You are a helpful assistant.")
//!     .build()?;
//!
//! let agent = agent::create(&client, &request).await?;
//! println!("Created agent: {}", agent.id);
//!
//! // List all agents
//! let agents = agent::list(&client).await?;
//! for a in agents.data {
//!     println!("Agent: {} ({})", a.name.unwrap_or_default(), a.id);
//! }
//!
//! // Delete the agent
//! agent::delete(&client, &agent.id).await?;
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

/// A request to create a new agent.
///
/// Use the builder pattern to construct requests:
///
/// ```rust
/// use azure_ai_foundry_agents::agent::AgentCreateRequest;
///
/// let request = AgentCreateRequest::builder()
///     .model("gpt-4o")
///     .name("My Assistant")
///     .instructions("You are a helpful assistant.")
///     .build()
///     .expect("valid request");
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct AgentCreateRequest {
    /// The model ID to use for this agent (e.g., "gpt-4o").
    pub model: String,

    /// Optional name for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional system instructions for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional description of the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional tools available to the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Optional metadata as key-value pairs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional temperature for sampling (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Optional top_p for nucleus sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Builder for [`AgentCreateRequest`].
#[derive(Debug, Default)]
pub struct AgentCreateRequestBuilder {
    model: Option<String>,
    name: Option<String>,
    instructions: Option<String>,
    description: Option<String>,
    tools: Option<Vec<Tool>>,
    metadata: Option<serde_json::Value>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

impl AgentCreateRequest {
    /// Create a new builder for `AgentCreateRequest`.
    pub fn builder() -> AgentCreateRequestBuilder {
        AgentCreateRequestBuilder::default()
    }
}

impl AgentCreateRequestBuilder {
    /// Set the model ID to use for this agent.
    ///
    /// **Required.** Example values: `"gpt-4o"`, `"gpt-4o-mini"`.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the name for this agent.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the system instructions for this agent.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Set a description for this agent.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the tools available to this agent.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set metadata for this agent.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
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

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are out of range.
    pub fn build(self) -> FoundryResult<AgentCreateRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;

        // Validate model is not empty
        if model.trim().is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        // Validate temperature (0.0 - 2.0)
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(FoundryError::Builder(
                    "temperature must be between 0.0 and 2.0".into(),
                ));
            }
        }

        // Validate top_p (0.0 - 1.0)
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(FoundryError::Builder(
                    "top_p must be between 0.0 and 1.0".into(),
                ));
            }
        }

        Ok(AgentCreateRequest {
            model,
            name: self.name,
            instructions: self.instructions,
            description: self.description,
            tools: self.tools,
            metadata: self.metadata,
            temperature: self.temperature,
            top_p: self.top_p,
        })
    }
}

/// A tool that can be used by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (e.g., "code_interpreter", "file_search", "function").
    #[serde(rename = "type")]
    pub tool_type: String,

    /// Function definition (only for function tools).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDefinition>,
}

impl Tool {
    /// Create a code interpreter tool.
    pub fn code_interpreter() -> Self {
        Self {
            tool_type: "code_interpreter".into(),
            function: None,
        }
    }

    /// Create a file search tool.
    pub fn file_search() -> Self {
        Self {
            tool_type: "file_search".into(),
            function: None,
        }
    }

    /// Create a function tool with the given definition.
    pub fn function(definition: FunctionDefinition) -> Self {
        Self {
            tool_type: "function".into(),
            function: Some(definition),
        }
    }
}

/// Definition of a function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function.
    pub name: String,

    /// Description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// An AI agent.
#[derive(Debug, Clone, Deserialize)]
pub struct Agent {
    /// Unique identifier for the agent.
    pub id: String,

    /// Object type, always "assistant".
    pub object: String,

    /// Unix timestamp when the agent was created.
    pub created_at: u64,

    /// The model ID used by this agent.
    pub model: String,

    /// Name of the agent.
    pub name: Option<String>,

    /// Description of the agent.
    pub description: Option<String>,

    /// System instructions for the agent.
    pub instructions: Option<String>,

    /// Tools available to the agent.
    pub tools: Option<Vec<Tool>>,

    /// Metadata attached to the agent.
    pub metadata: Option<serde_json::Value>,

    /// Sampling temperature.
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter.
    pub top_p: Option<f32>,
}

/// Response from listing agents.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentList {
    /// Object type, always "list".
    pub object: String,

    /// List of agents.
    pub data: Vec<Agent>,

    /// ID of the first agent in the list.
    pub first_id: Option<String>,

    /// ID of the last agent in the list.
    pub last_id: Option<String>,

    /// Whether there are more agents to fetch.
    pub has_more: bool,
}

/// Response from deleting an agent.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentDeletionResponse {
    /// ID of the deleted agent.
    pub id: String,

    /// Object type, always "assistant.deleted".
    pub object: String,

    /// Whether the deletion was successful.
    pub deleted: bool,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Create a new agent.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::agent::{self, AgentCreateRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = AgentCreateRequest::builder()
///     .model("gpt-4o")
///     .name("My Assistant")
///     .instructions("You are helpful.")
///     .build()?;
///
/// let agent = agent::create(client, &request).await?;
/// println!("Created agent: {}", agent.id);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::agents::create` with field `model`.
#[tracing::instrument(
    name = "foundry::agents::create",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn create(client: &FoundryClient, request: &AgentCreateRequest) -> FoundryResult<Agent> {
    tracing::debug!("creating agent");

    let path = format!("/assistants?{}", API_VERSION);
    let response = client.post(&path, request).await?;
    let agent = response.json::<Agent>().await?;

    tracing::debug!(agent_id = %agent.id, "agent created");
    Ok(agent)
}

/// Get an agent by ID.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::agent;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let agent = agent::get(client, "asst_abc123").await?;
/// println!("Agent model: {}", agent.model);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::agents::get` with field `agent_id`.
#[tracing::instrument(
    name = "foundry::agents::get",
    skip(client),
    fields(agent_id = %agent_id)
)]
pub async fn get(client: &FoundryClient, agent_id: &str) -> FoundryResult<Agent> {
    tracing::debug!("getting agent");

    let path = format!("/assistants/{}?{}", agent_id, API_VERSION);
    let response = client.get(&path).await?;
    let agent = response.json::<Agent>().await?;

    Ok(agent)
}

/// List all agents.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::agent;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let agents = agent::list(client).await?;
/// for a in agents.data {
///     println!("Agent: {} - {}", a.id, a.name.unwrap_or_default());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::agents::list`.
#[tracing::instrument(name = "foundry::agents::list", skip(client))]
pub async fn list(client: &FoundryClient) -> FoundryResult<AgentList> {
    tracing::debug!("listing agents");

    let path = format!("/assistants?{}", API_VERSION);
    let response = client.get(&path).await?;
    let list = response.json::<AgentList>().await?;

    tracing::debug!(count = list.data.len(), "agents listed");
    Ok(list)
}

/// Delete an agent.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::agent;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = agent::delete(client, "asst_abc123").await?;
/// if result.deleted {
///     println!("Agent deleted successfully");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::agents::delete` with field `agent_id`.
#[tracing::instrument(
    name = "foundry::agents::delete",
    skip(client),
    fields(agent_id = %agent_id)
)]
pub async fn delete(
    client: &FoundryClient,
    agent_id: &str,
) -> FoundryResult<AgentDeletionResponse> {
    tracing::debug!("deleting agent");

    let path = format!("/assistants/{}?{}", agent_id, API_VERSION);
    let response = client.delete(&path).await?;
    let result = response.json::<AgentDeletionResponse>().await?;

    tracing::debug!(deleted = result.deleted, "agent deletion complete");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_MODEL, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // --- Cycle 3: AgentCreateRequest serialization tests ---

    #[test]
    fn test_agent_request_serialization_minimal() {
        let request = AgentCreateRequest::builder()
            .model("gpt-4o")
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert!(json.get("name").is_none());
        assert!(json.get("instructions").is_none());
        assert!(json.get("description").is_none());
        assert!(json.get("tools").is_none());
        assert!(json.get("metadata").is_none());
        assert!(json.get("temperature").is_none());
        assert!(json.get("top_p").is_none());
    }

    #[test]
    fn test_agent_request_serialization_full() {
        let request = AgentCreateRequest::builder()
            .model("gpt-4o")
            .name("Test Agent")
            .instructions("You are helpful.")
            .description("A test agent")
            .temperature(0.7)
            .top_p(0.9)
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["name"], "Test Agent");
        assert_eq!(json["instructions"], "You are helpful.");
        assert_eq!(json["description"], "A test agent");
        // Use approximate comparison for floating point
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001, "temperature should be ~0.7");
        let top_p = json["top_p"].as_f64().unwrap();
        assert!((top_p - 0.9).abs() < 0.001, "top_p should be ~0.9");
    }

    #[test]
    fn test_agent_request_with_tools() {
        let request = AgentCreateRequest::builder()
            .model("gpt-4o")
            .tools(vec![Tool::code_interpreter(), Tool::file_search()])
            .build()
            .expect("valid request");

        let json = serde_json::to_value(&request).unwrap();

        let tools = json["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["type"], "code_interpreter");
        assert_eq!(tools[1]["type"], "file_search");
    }

    // --- Cycle 4: Builder validation tests ---

    #[test]
    fn test_agent_builder_requires_model() {
        let result = AgentCreateRequest::builder().build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("model is required"));
    }

    #[test]
    fn test_agent_builder_rejects_empty_model() {
        let result = AgentCreateRequest::builder().model("   ").build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("model cannot be empty"));
    }

    #[test]
    fn test_agent_builder_validates_temperature() {
        let result = AgentCreateRequest::builder()
            .model("gpt-4o")
            .temperature(3.0)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_agent_builder_validates_top_p() {
        let result = AgentCreateRequest::builder()
            .model("gpt-4o")
            .top_p(1.5)
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("top_p"));
    }

    #[test]
    fn test_agent_builder_accepts_valid_params() {
        let result = AgentCreateRequest::builder()
            .model("gpt-4o")
            .temperature(0.0)
            .top_p(1.0)
            .build();

        assert!(result.is_ok());
    }

    // --- Cycle 5: Agent response deserialization tests ---

    #[test]
    fn test_agent_response_deserialization() {
        let json = serde_json::json!({
            "id": "asst_abc123",
            "object": "assistant",
            "created_at": TEST_TIMESTAMP,
            "model": TEST_MODEL,
            "name": "My Assistant",
            "description": "A helpful assistant",
            "instructions": "You are helpful.",
            "tools": [{"type": "code_interpreter"}],
            "metadata": {"key": "value"},
            "temperature": 0.7,
            "top_p": 0.9
        });

        let agent: Agent = serde_json::from_value(json).unwrap();

        assert_eq!(agent.id, "asst_abc123");
        assert_eq!(agent.object, "assistant");
        assert_eq!(agent.created_at, TEST_TIMESTAMP);
        assert_eq!(agent.model, TEST_MODEL);
        assert_eq!(agent.name, Some("My Assistant".into()));
        assert_eq!(agent.description, Some("A helpful assistant".into()));
        assert_eq!(agent.instructions, Some("You are helpful.".into()));
        assert!(agent.tools.is_some());
        assert_eq!(agent.tools.as_ref().unwrap().len(), 1);
        assert_eq!(agent.temperature, Some(0.7));
        assert_eq!(agent.top_p, Some(0.9));
    }

    #[test]
    fn test_agent_response_minimal() {
        let json = serde_json::json!({
            "id": "asst_abc123",
            "object": "assistant",
            "created_at": TEST_TIMESTAMP,
            "model": TEST_MODEL
        });

        let agent: Agent = serde_json::from_value(json).unwrap();

        assert_eq!(agent.id, "asst_abc123");
        assert!(agent.name.is_none());
        assert!(agent.instructions.is_none());
        assert!(agent.tools.is_none());
    }

    // --- Cycle 6: Create agent API tests ---

    #[tokio::test]
    async fn test_create_agent_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "asst_test123",
            "object": "assistant",
            "created_at": TEST_TIMESTAMP,
            "model": TEST_MODEL,
            "name": "Test Agent",
            "instructions": "You are helpful."
        });

        Mock::given(method("POST"))
            .and(path("/assistants"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(header("content-type", "application/json"))
            .and(body_json(serde_json::json!({
                "model": TEST_MODEL,
                "name": "Test Agent",
                "instructions": "You are helpful."
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = AgentCreateRequest::builder()
            .model(TEST_MODEL)
            .name("Test Agent")
            .instructions("You are helpful.")
            .build()
            .expect("valid request");

        let agent = create(&client, &request).await.expect("should succeed");

        assert_eq!(agent.id, "asst_test123");
        assert_eq!(agent.model, TEST_MODEL);
        assert_eq!(agent.name, Some("Test Agent".into()));
    }

    // --- Cycle 7: Get agent API tests ---

    #[tokio::test]
    async fn test_get_agent_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "asst_abc123",
            "object": "assistant",
            "created_at": TEST_TIMESTAMP,
            "model": TEST_MODEL,
            "name": "Retrieved Agent"
        });

        Mock::given(method("GET"))
            .and(path("/assistants/asst_abc123"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let agent = get(&client, "asst_abc123").await.expect("should succeed");

        assert_eq!(agent.id, "asst_abc123");
        assert_eq!(agent.name, Some("Retrieved Agent".into()));
    }

    // --- Cycle 8: List agents API tests ---

    #[tokio::test]
    async fn test_list_agents_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "asst_1",
                    "object": "assistant",
                    "created_at": TEST_TIMESTAMP,
                    "model": TEST_MODEL,
                    "name": "Agent 1"
                },
                {
                    "id": "asst_2",
                    "object": "assistant",
                    "created_at": TEST_TIMESTAMP,
                    "model": TEST_MODEL,
                    "name": "Agent 2"
                }
            ],
            "first_id": "asst_1",
            "last_id": "asst_2",
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/assistants"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let list = list(&client).await.expect("should succeed");

        assert_eq!(list.object, "list");
        assert_eq!(list.data.len(), 2);
        assert_eq!(list.data[0].id, "asst_1");
        assert_eq!(list.data[1].id, "asst_2");
        assert!(!list.has_more);
    }

    // --- Cycle 9: Delete agent API tests ---

    #[tokio::test]
    async fn test_delete_agent_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({
            "id": "asst_abc123",
            "object": "assistant.deleted",
            "deleted": true
        });

        Mock::given(method("DELETE"))
            .and(path("/assistants/asst_abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete(&client, "asst_abc123")
            .await
            .expect("should succeed");

        assert_eq!(result.id, "asst_abc123");
        assert!(result.deleted);
    }

    // --- Tool tests ---

    #[test]
    fn test_tool_code_interpreter() {
        let tool = Tool::code_interpreter();

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["type"], "code_interpreter");
        assert!(json.get("function").is_none());
    }

    #[test]
    fn test_tool_file_search() {
        let tool = Tool::file_search();

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["type"], "file_search");
    }

    #[test]
    fn test_tool_function() {
        let func = FunctionDefinition {
            name: "get_weather".into(),
            description: Some("Get current weather".into()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            })),
        };

        let tool = Tool::function(func);

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "get_weather");
    }
}
