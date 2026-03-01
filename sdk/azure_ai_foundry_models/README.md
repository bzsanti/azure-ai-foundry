# azure_ai_foundry_models

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_models.svg)](https://crates.io/crates/azure_ai_foundry_models)
[![docs.rs](https://docs.rs/azure_ai_foundry_models/badge.svg)](https://docs.rs/azure_ai_foundry_models)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Model inference client for the Azure AI Foundry Rust SDK — chat completions, embeddings, audio, images, and the Responses API.

## Features

- **Chat Completions** — Synchronous and streaming responses
- **Embeddings** — Generate vector embeddings for text
- **Audio** — Transcription (STT), translation, and text-to-speech (TTS)
- **Images** — Image generation and editing
- **Responses** — Unified Responses API (create, get, delete)
- **Streaming** — SSE with optimized parsing and 1MB buffer protection
- **Builder Pattern** — Type-safe request construction with parameter validation
- **Tracing** — Full instrumentation with `tracing` spans

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.6"
azure_ai_foundry_models = "0.6"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Chat Completions

```rust,no_run
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

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::chat::{ChatCompletionRequest, Message, complete_stream};
use futures::StreamExt;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let request = ChatCompletionRequest::builder()
    .model("gpt-4o")
    .message(Message::user("Tell me a story"))
    .build();

let stream = complete_stream(&client, &request).await?;
let mut stream = std::pin::pin!(stream);

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = chunk.choices[0].delta.content.as_deref() {
        print!("{}", content);
    }
}
# Ok(())
# }
```

### Embeddings

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::embeddings::{EmbeddingRequest, embed};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let request = EmbeddingRequest::builder()
    .model("text-embedding-ada-002")
    .input("The quick brown fox jumps over the lazy dog")
    .build();

let response = embed(&client, &request).await?;
println!("Embedding dimensions: {}", response.data[0].embedding.len());
# Ok(())
# }
```

### Multiple Embeddings

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::embeddings::{EmbeddingRequest, embed};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
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
# Ok(())
# }
```

### Audio Transcription

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::audio::{TranscriptionRequest, transcribe};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let audio_data = std::fs::read("recording.wav")?;
let request = TranscriptionRequest::builder()
    .model("whisper-1")
    .filename("recording.wav")
    .data(audio_data)
    .language("en")
    .build();

let response = transcribe(&client, &request).await?;
println!("Transcription: {}", response.text);
# Ok(())
# }
```

### Text-to-Speech

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::audio::{SpeechRequest, speak};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let request = SpeechRequest::builder()
    .model("tts-1")
    .input("Hello, world!")
    .voice("alloy")
    .build();

let audio = speak(&client, &request).await?;
std::fs::write("output.mp3", &audio)?;
# Ok(())
# }
```

### Image Generation

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::images::{ImageGenerationRequest, ImageSize, generate};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let request = ImageGenerationRequest::builder()
    .model("dall-e-3")
    .prompt("A futuristic city at sunset")
    .size(ImageSize::S1024x1024)
    .build();

let response = generate(&client, &request).await?;
if let Some(url) = &response.data[0].url {
    println!("Image: {}", url);
}
# Ok(())
# }
```

### Responses API

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_models::responses::{CreateResponseRequest, create};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let client = FoundryClient::builder()
#     .endpoint("https://your-resource.services.ai.azure.com")
#     .credential(FoundryCredential::api_key("your-key"))
#     .build()?;
let request = CreateResponseRequest::builder()
    .model("gpt-4o")
    .input("What is Rust?")
    .build();

let response = create(&client, &request).await?;
if let Some(text) = response.output_text() {
    println!("{}", text);
}
# Ok(())
# }
```

## Modules

| Module | Description |
|--------|-------------|
| `chat` | Chat completions API with sync and streaming support |
| `embeddings` | Vector embeddings generation |
| `audio` | Transcription, translation, and text-to-speech |
| `images` | Image generation and editing |
| `responses` | Unified Responses API (create, get, delete) |

## Related Crates

- [`azure_ai_foundry_core`](../azure_ai_foundry_core) — Core types, authentication, and HTTP client
- [`azure_ai_foundry_agents`](../azure_ai_foundry_agents) — Agent Service (agents, threads, runs, files, vector stores)
- [`azure_ai_foundry_tools`](../azure_ai_foundry_tools) — Vision and Document Intelligence

## License

This project is licensed under the [MIT License](../../LICENSE).
