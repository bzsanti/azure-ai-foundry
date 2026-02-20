//! # Azure AI Foundry Agents
//!
//! AI Agent Service client for the Azure AI Foundry Rust SDK.
//!
//! This crate provides Rust bindings for creating, managing, and running AI agents
//! with threads, messages, and runs. The Agent Service enables cloud-hosted AI
//! workflows that pair large language models with tools to execute complex tasks.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::agent::{self, AgentCreateRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = FoundryClient::builder()
//!         .endpoint("https://your-resource.services.ai.azure.com")
//!         .credential(FoundryCredential::api_key("your-key"))
//!         .build()?;
//!
//!     // Create an agent
//!     let request = AgentCreateRequest::builder()
//!         .model("gpt-4o")
//!         .name("My Assistant")
//!         .instructions("You are a helpful assistant.")
//!         .build()?;
//!
//!     let agent = agent::create(&client, &request).await?;
//!     println!("Created agent: {}", agent.id);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Core Concepts
//!
//! - **Agent**: An AI assistant configured with a model, instructions, and optional tools.
//! - **Thread**: A conversation session that maintains message history.
//! - **Message**: A single message within a thread (user or assistant).
//! - **Run**: An execution of an agent on a thread to generate responses.
//!
//! ## Modules
//!
//! - [`agent`] - Create, retrieve, list, and delete agents
//! - [`thread`] - Manage conversation threads
//! - [`message`] - Add and retrieve messages in threads
//! - [`run`] - Execute agents on threads and monitor progress

pub mod agent;
pub mod message;
pub mod models;
pub mod run;
pub mod thread;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    use azure_ai_foundry_core::auth::FoundryCredential;
    use azure_ai_foundry_core::client::FoundryClient;
    use wiremock::MockServer;

    /// Test API key (not a real key).
    pub const TEST_API_KEY: &str = "test-api-key";

    /// Unix timestamp used in test responses.
    pub const TEST_TIMESTAMP: u64 = 1700000000;

    /// Default test model for agents.
    pub const TEST_MODEL: &str = "gpt-4o";

    /// Create a test client connected to a mock server.
    pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key(TEST_API_KEY))
            .build()
            .expect("should build client")
    }
}
