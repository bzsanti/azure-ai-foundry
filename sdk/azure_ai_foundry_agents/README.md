# azure_ai_foundry_agents

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_agents.svg)](https://crates.io/crates/azure_ai_foundry_agents)
[![docs.rs](https://docs.rs/azure_ai_foundry_agents/badge.svg)](https://docs.rs/azure_ai_foundry_agents)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Agent Service client for the Azure AI Foundry Rust SDK.

## Features

- **Agents** — Create, get, list, and delete AI agents
- **Threads** — Manage conversation threads
- **Messages** — Add and retrieve messages in threads
- **Runs** — Execute agents and poll for completion
- **Tracing** — Full instrumentation with `tracing` spans

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.3"
azure_ai_foundry_agents = "0.3"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Create an Agent

```rust
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
        .build();

    let agent = agent::create(&client, &request).await?;
    println!("Created agent: {}", agent.id);
    Ok(())
}
```

### Run a Conversation

```rust
use azure_ai_foundry_agents::{agent, thread, message, run};
use azure_ai_foundry_agents::message::MessageCreateRequest;
use azure_ai_foundry_agents::run::RunCreateRequest;
use std::time::Duration;

// Create a thread
let thread = thread::create(&client, None).await?;

// Add a message
let msg_request = MessageCreateRequest::builder()
    .content("What is the weather in Paris?")
    .build();
message::create(&client, &thread.id, &msg_request).await?;

// Run the agent
let run_request = RunCreateRequest::builder()
    .assistant_id(&agent.id)
    .build();
let created_run = run::create(&client, &thread.id, &run_request).await?;

// Poll until complete
let completed_run = run::poll_until_complete(
    &client,
    &thread.id,
    &created_run.id,
    Duration::from_secs(1),
).await?;

// Get the response
let messages = message::list(&client, &thread.id).await?;
println!("Response: {:?}", messages.data[0].content);
```

### Create Thread and Run (Shorthand)

```rust
use azure_ai_foundry_agents::run::{self, CreateThreadAndRunRequest};

let request = CreateThreadAndRunRequest::builder()
    .assistant_id(&agent.id)
    .build();

let run = run::create_thread_and_run(&client, &request).await?;
```

## Modules

| Module | Description |
|--------|-------------|
| `agent` | Create, get, list, delete agents |
| `thread` | Create, get, delete conversation threads |
| `message` | Create, list, get messages in threads |
| `run` | Execute agents and poll for completion |

## Tracing Spans

All API calls emit tracing spans for observability:

| Span | Fields |
|------|--------|
| `foundry::agents::create` | model |
| `foundry::agents::get` | agent_id |
| `foundry::agents::list` | - |
| `foundry::agents::delete` | agent_id |
| `foundry::threads::create` | - |
| `foundry::threads::get` | thread_id |
| `foundry::threads::delete` | thread_id |
| `foundry::messages::create` | thread_id |
| `foundry::messages::list` | thread_id |
| `foundry::messages::get` | thread_id, message_id |
| `foundry::runs::create` | thread_id, assistant_id |
| `foundry::runs::get` | thread_id, run_id |
| `foundry::runs::create_thread_and_run` | assistant_id |
| `foundry::runs::poll_until_complete` | thread_id, run_id |

## Related Crates

- [`azure_ai_foundry_core`](../azure_ai_foundry_core) — Core types, authentication, and HTTP client
- [`azure_ai_foundry_models`](../azure_ai_foundry_models) — Chat completions and embeddings

## License

This project is licensed under the [MIT License](../../LICENSE).
