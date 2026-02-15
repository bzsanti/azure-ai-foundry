# azure_ai_foundry_models

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_models.svg)](https://crates.io/crates/azure_ai_foundry_models)
[![docs.rs](https://docs.rs/azure_ai_foundry_models/badge.svg)](https://docs.rs/azure_ai_foundry_models)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Chat completions and embeddings client for the Azure AI Foundry Rust SDK.

## Features

- **Chat Completions** — Synchronous and streaming responses
- **Embeddings** — Generate vector embeddings for text
- **Streaming** — SSE with optimized parsing and 1MB buffer protection
- **Builder Pattern** — Type-safe request construction with parameter validation

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.2"
azure_ai_foundry_models = "0.2"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Chat Completions

```rust
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::chat::{ChatCompletionRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FoundryClient::builder()
        .endpoint("https://your-resource.services.ai.azure.com")
        .credential(FoundryCredential::api_key("your-key"))
        .build()?;

    let request = ChatCompletionRequest::builder()
        .model("gpt-4o")
        .message(Message::system("You are a helpful assistant."))
        .message(Message::user("What is Rust?"))
        .build();

    let response = azure_ai_foundry_models::chat::complete(&client, &request).await?;
    println!("{}", response.choices[0].message.content.as_deref().unwrap_or_default());
    Ok(())
}
```

### Streaming Chat Completions

```rust
use azure_ai_foundry_models::chat::{ChatCompletionRequest, Message, complete_stream};
use futures::StreamExt;

let request = ChatCompletionRequest::builder()
    .model("gpt-4o")
    .message(Message::user("Tell me a story"))
    .build();

let mut stream = complete_stream(&client, &request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = chunk.choices[0].delta.content.as_deref() {
        print!("{}", content);
    }
}
```

### Embeddings

```rust
use azure_ai_foundry_models::embeddings::{EmbeddingRequest, embed};

let request = EmbeddingRequest::builder()
    .model("text-embedding-ada-002")
    .input("The quick brown fox jumps over the lazy dog")
    .build();

let response = embed(&client, &request).await?;
println!("Embedding dimensions: {}", response.data[0].embedding.len());
```

### Multiple Embeddings

```rust
let request = EmbeddingRequest::builder()
    .model("text-embedding-ada-002")
    .inputs(vec![
        "First document",
        "Second document",
        "Third document",
    ])
    .build();

let response = embed(&client, &request).await?;
for (i, item) in response.data.iter().enumerate() {
    println!("Document {}: {} dimensions", i, item.embedding.len());
}
```

## Modules

| Module | Description |
|--------|-------------|
| `chat` | Chat completions API with sync and streaming support |
| `embeddings` | Vector embeddings generation |

## Related Crates

- [`azure_ai_foundry_core`](../azure_ai_foundry_core) — Core types, authentication, and HTTP client

## License

This project is licensed under the [MIT License](../../LICENSE).
