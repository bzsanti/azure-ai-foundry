# azure_ai_foundry_agents

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_agents.svg)](https://crates.io/crates/azure_ai_foundry_agents)
[![docs.rs](https://docs.rs/azure_ai_foundry_agents/badge.svg)](https://docs.rs/azure_ai_foundry_agents)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Agent Service client for the Azure AI Foundry Rust SDK.

## Features

- **Agents** — Create, get, list, update, and delete AI agents
- **Threads** — Manage conversation threads (create, get, update, delete)
- **Messages** — Add, list, get, and update messages in threads
- **Runs** — Execute agents, poll for completion, submit tool outputs
- **Files** — Upload, download, list, and delete files
- **Vector Stores** — CRUD operations for vector stores, files, and file batches
- **Run Steps** — Inspect individual actions taken during a run
- **Tracing** — Full instrumentation with `tracing` spans

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.5"
azure_ai_foundry_agents = "0.5"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Create an Agent

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_agents::agent::{self, AgentCreateRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FoundryClient::builder()
        .endpoint("https://your-resource.services.ai.azure.com")
        .credential(FoundryCredential::api_key("your-key"))
        .build()?;

    let request = AgentCreateRequest::builder()
        .model("gpt-4o")
        .name("My Assistant")
        .instructions("You are a helpful assistant.")
        .build()?;

    let agent = agent::create(&client, &request).await?;
    println!("Created agent: {}", agent.id);
    Ok(())
}
```

### Run a Conversation

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_agents::{thread, message, run};
use azure_ai_foundry_agents::message::MessageCreateRequest;
use azure_ai_foundry_agents::run::RunCreateRequest;
use std::time::Duration;

# async fn example(client: &FoundryClient, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
// Create a thread
let thread = thread::create(client, None).await?;

// Add a message
let msg_request = MessageCreateRequest::builder()
    .content("What is the weather in Paris?")
    .build()?;
message::create(client, &thread.id, &msg_request).await?;

// Run the agent
let run_request = RunCreateRequest::builder()
    .assistant_id(agent_id)
    .build()?;
let created_run = run::create(client, &thread.id, &run_request).await?;

// Poll until complete
let completed_run = run::poll_until_complete(
    client,
    &thread.id,
    &created_run.id,
    Duration::from_secs(1),
).await?;

// Get the response
let messages = message::list(client, &thread.id).await?;
println!("Response: {:?}", messages.data[0].content);
# Ok(())
# }
```

### Upload and Manage Files

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_agents::file::{self, FilePurpose};

# async fn example(client: &FoundryClient) -> Result<(), Box<dyn std::error::Error>> {
// Upload a file
let file = file::upload(client, "data.jsonl", b"content".to_vec(), FilePurpose::Assistants).await?;

// Download file content
let content = file::download(client, &file.id).await?;

// List all files
let files = file::list(client).await?;

// Delete a file
file::delete(client, &file.id).await?;
# Ok(())
# }
```

### Vector Stores (RAG)

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_agents::vector_store::{self, VectorStoreCreateRequest};

# async fn example(client: &FoundryClient) -> Result<(), Box<dyn std::error::Error>> {
// Create a vector store
let request = VectorStoreCreateRequest::builder()
    .name("My Knowledge Base")
    .build();
let store = vector_store::create(client, &request).await?;

// Add files to the vector store
vector_store::add_file(client, &store.id, "file-abc123").await?;

// Create a file batch
let batch = vector_store::create_file_batch(client, &store.id, &["file-1", "file-2"]).await?;
# Ok(())
# }
```

### Submit Tool Outputs

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_agents::run::{self, ToolOutput};
use std::time::Duration;

# async fn example(client: &FoundryClient) -> Result<(), Box<dyn std::error::Error>> {
let outputs = vec![ToolOutput {
    tool_call_id: "call_abc".to_string(),
    output: "Sunny, 72°F".to_string(),
}];

let run = run::submit_tool_outputs_and_poll(
    client, "thread_xyz", "run_abc", &outputs, Duration::from_secs(1),
).await?;
# Ok(())
# }
```

## Modules

| Module | Description |
|--------|-------------|
| `agent` | Create, get, list, update, delete agents |
| `thread` | Create, get, update, delete conversation threads |
| `message` | Create, list, get, update messages in threads |
| `run` | Execute agents, poll, submit tool outputs |
| `file` | Upload, download, list, delete files |
| `vector_store` | Vector stores, files, and file batches |
| `run_step` | Inspect run step details |

## Tracing Spans

All API calls emit tracing spans for observability:

| Span | Fields |
|------|--------|
| `foundry::agents::create` | model |
| `foundry::agents::get` | agent_id |
| `foundry::agents::list` | - |
| `foundry::agents::delete` | agent_id |
| `foundry::agents::update` | agent_id |
| `foundry::threads::create` | - |
| `foundry::threads::get` | thread_id |
| `foundry::threads::delete` | thread_id |
| `foundry::threads::update` | thread_id |
| `foundry::messages::create` | thread_id |
| `foundry::messages::list` | thread_id |
| `foundry::messages::get` | thread_id, message_id |
| `foundry::messages::update` | thread_id, message_id |
| `foundry::runs::create` | thread_id, assistant_id |
| `foundry::runs::get` | thread_id, run_id |
| `foundry::runs::create_thread_and_run` | assistant_id |
| `foundry::runs::poll_until_complete` | thread_id, run_id |
| `foundry::runs::create_and_poll` | assistant_id |
| `foundry::runs::submit_tool_outputs` | thread_id, run_id, output_count |
| `foundry::runs::submit_tool_outputs_and_poll` | thread_id, run_id, output_count |
| `foundry::files::upload` | filename, purpose |
| `foundry::files::get` | file_id |
| `foundry::files::list` | - |
| `foundry::files::delete` | file_id |
| `foundry::files::download` | file_id |
| `foundry::vector_stores::create` | - |
| `foundry::vector_stores::get` | vector_store_id |
| `foundry::vector_stores::list` | - |
| `foundry::vector_stores::update` | vector_store_id |
| `foundry::vector_stores::delete` | vector_store_id |
| `foundry::vector_stores::add_file` | vector_store_id, file_id |
| `foundry::vector_stores::list_files` | vector_store_id |
| `foundry::vector_stores::get_file` | vector_store_id, file_id |
| `foundry::vector_stores::delete_file` | vector_store_id, file_id |
| `foundry::vector_stores::create_file_batch` | vector_store_id |
| `foundry::vector_stores::get_file_batch` | vector_store_id, batch_id |
| `foundry::run_steps::list` | thread_id, run_id |
| `foundry::run_steps::get` | thread_id, run_id, step_id |

## Related Crates

- [`azure_ai_foundry_core`](../azure_ai_foundry_core) — Core types, authentication, and HTTP client
- [`azure_ai_foundry_models`](../azure_ai_foundry_models) — Chat completions and embeddings
- [`azure_ai_foundry_tools`](../azure_ai_foundry_tools) — Vision and Document Intelligence

## License

This project is licensed under the [MIT License](../../LICENSE).
