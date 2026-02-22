//! Integration tests for azure_ai_foundry_agents.
//!
//! These tests require a live Azure AI Foundry endpoint.
//! Run with: `cargo test --features integration-tests`
//!
//! Required environment variables:
//! - `AZURE_AI_FOUNDRY_ENDPOINT`: The Azure AI Foundry endpoint URL
//! - `AZURE_AI_FOUNDRY_API_KEY`: The API key for authentication

#![cfg(feature = "integration-tests")]

use azure_ai_foundry_agents::agent::AgentCreateRequest;
use azure_ai_foundry_agents::message::MessageCreateRequest;
use azure_ai_foundry_agents::run::{CreateThreadAndRunRequest, RunCreateRequest, RunStatus};
use azure_ai_foundry_agents::{agent, message, run, thread};
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_core::client::FoundryClient;
use std::time::Duration;

fn get_client() -> FoundryClient {
    let endpoint =
        std::env::var("AZURE_AI_FOUNDRY_ENDPOINT").expect("AZURE_AI_FOUNDRY_ENDPOINT not set");
    let api_key =
        std::env::var("AZURE_AI_FOUNDRY_API_KEY").expect("AZURE_AI_FOUNDRY_API_KEY not set");

    FoundryClient::builder()
        .endpoint(endpoint)
        .credential(FoundryCredential::api_key(api_key))
        .build()
        .expect("Failed to build client")
}

fn get_model() -> String {
    std::env::var("AZURE_AI_FOUNDRY_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())
}

#[tokio::test]
async fn test_agent_lifecycle() {
    let client = get_client();
    let model = get_model();

    // Create an agent
    let request = AgentCreateRequest::builder()
        .model(&model)
        .name("Integration Test Agent")
        .instructions("You are a helpful test assistant.")
        .build()
        .expect("valid request");

    let created_agent = agent::create(&client, &request)
        .await
        .expect("create agent");
    assert!(!created_agent.id.is_empty());
    assert_eq!(created_agent.model, model);

    // Get the agent
    let fetched_agent = agent::get(&client, &created_agent.id)
        .await
        .expect("get agent");
    assert_eq!(fetched_agent.id, created_agent.id);

    // List agents (should include our agent)
    let agents = agent::list(&client).await.expect("list agents");
    assert!(agents.data.iter().any(|a| a.id == created_agent.id));

    // Delete the agent
    let deletion = agent::delete(&client, &created_agent.id)
        .await
        .expect("delete agent");
    assert!(deletion.deleted);
}

#[tokio::test]
async fn test_thread_lifecycle() {
    let client = get_client();

    // Create a thread
    let created_thread = thread::create(&client, None).await.expect("create thread");
    assert!(!created_thread.id.is_empty());

    // Get the thread
    let fetched_thread = thread::get(&client, &created_thread.id)
        .await
        .expect("get thread");
    assert_eq!(fetched_thread.id, created_thread.id);

    // Delete the thread
    let deletion = thread::delete(&client, &created_thread.id)
        .await
        .expect("delete thread");
    assert!(deletion.deleted);
}

#[tokio::test]
async fn test_message_lifecycle() {
    let client = get_client();

    // Create a thread
    let created_thread = thread::create(&client, None).await.expect("create thread");

    // Create a message
    let msg_request = MessageCreateRequest::builder()
        .content("Hello, this is a test message.")
        .build()
        .expect("valid request");

    let created_msg = message::create(&client, &created_thread.id, &msg_request)
        .await
        .expect("create message");
    assert!(!created_msg.id.is_empty());
    assert_eq!(created_msg.thread_id, created_thread.id);

    // List messages
    let messages = message::list(&client, &created_thread.id)
        .await
        .expect("list messages");
    assert!(messages.data.iter().any(|m| m.id == created_msg.id));

    // Get specific message
    let fetched_msg = message::get(&client, &created_thread.id, &created_msg.id)
        .await
        .expect("get message");
    assert_eq!(fetched_msg.id, created_msg.id);

    // Cleanup
    thread::delete(&client, &created_thread.id)
        .await
        .expect("delete thread");
}

#[tokio::test]
async fn test_run_lifecycle() {
    let client = get_client();
    let model = get_model();

    // Create an agent
    let agent_request = AgentCreateRequest::builder()
        .model(&model)
        .name("Run Test Agent")
        .instructions("You are a helpful assistant. Respond briefly.")
        .build()
        .expect("valid request");

    let created_agent = agent::create(&client, &agent_request)
        .await
        .expect("create agent");

    // Create a thread
    let created_thread = thread::create(&client, None).await.expect("create thread");

    // Add a message
    let msg_request = MessageCreateRequest::builder()
        .content("What is 2+2? Answer with just the number.")
        .build()
        .expect("valid request");

    message::create(&client, &created_thread.id, &msg_request)
        .await
        .expect("create message");

    // Create and poll run
    let run_request = RunCreateRequest::builder()
        .assistant_id(&created_agent.id)
        .build()
        .expect("valid request");

    let created_run = run::create(&client, &created_thread.id, &run_request)
        .await
        .expect("create run");
    assert!(!created_run.id.is_empty());

    // Poll until complete
    let final_run = run::poll_until_complete(
        &client,
        &created_thread.id,
        &created_run.id,
        Duration::from_secs(1),
    )
    .await
    .expect("poll run");

    assert!(matches!(
        final_run.status,
        RunStatus::Completed | RunStatus::Failed
    ));

    // List messages to see the response
    let messages = message::list(&client, &created_thread.id)
        .await
        .expect("list messages");

    // Should have at least 2 messages (user + assistant)
    assert!(messages.data.len() >= 2);

    // Cleanup
    thread::delete(&client, &created_thread.id)
        .await
        .expect("delete thread");
    agent::delete(&client, &created_agent.id)
        .await
        .expect("delete agent");
}

#[tokio::test]
async fn test_create_thread_and_run() {
    let client = get_client();
    let model = get_model();

    // Create an agent
    let agent_request = AgentCreateRequest::builder()
        .model(&model)
        .name("Thread+Run Test Agent")
        .instructions("You are helpful. Be brief.")
        .build()
        .expect("valid request");

    let created_agent = agent::create(&client, &agent_request)
        .await
        .expect("create agent");

    // Create thread and run in one call
    let request = CreateThreadAndRunRequest::builder()
        .assistant_id(&created_agent.id)
        .message("Say hello in one word.")
        .build()
        .expect("valid request");

    let created_run = run::create_thread_and_run(&client, &request)
        .await
        .expect("create thread and run");

    assert!(!created_run.id.is_empty());
    assert!(!created_run.thread_id.is_empty());

    // Poll until complete
    let final_run = run::poll_until_complete(
        &client,
        &created_run.thread_id,
        &created_run.id,
        Duration::from_secs(1),
    )
    .await
    .expect("poll run");

    assert!(matches!(
        final_run.status,
        RunStatus::Completed | RunStatus::Failed
    ));

    // Cleanup
    thread::delete(&client, &created_run.thread_id)
        .await
        .expect("delete thread");
    agent::delete(&client, &created_agent.id)
        .await
        .expect("delete agent");
}
