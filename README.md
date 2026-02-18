# Azure AI Foundry SDK for Rust 🦀

[![CI](https://github.com/bzsanti/azure-ai-foundry/actions/workflows/ci.yml/badge.svg)](https://github.com/bzsanti/azure-ai-foundry/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_core.svg)](https://crates.io/crates/azure_ai_foundry_core)
[![docs.rs](https://docs.rs/azure_ai_foundry_core/badge.svg)](https://docs.rs/azure_ai_foundry_core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

An **unofficial** Rust SDK for [Microsoft Foundry](https://azure.microsoft.com/en-us/products/ai-foundry) (formerly Azure AI Foundry). Type-safe, async-first, and built on top of the official [Azure SDK for Rust](https://github.com/Azure/azure-sdk-for-rust).

> ⚠️ **This is a community project** and is not affiliated with or endorsed by Microsoft.

## Features

- **Chat Completions** — Call Azure OpenAI and Foundry direct models
- **Embeddings** — Generate vector embeddings for your data
- **Microsoft Entra ID** — First-class authentication via `azure_identity`
- **Streaming** — Server-sent events (SSE) for real-time responses
- **Type-safe** — Fully typed request/response models with serde
- **Async** — Built on `tokio` and `reqwest`

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| [`azure_ai_foundry_core`](./sdk/azure_ai_foundry_core) | Auth, HTTP client, shared types | ✅ Released (v0.2.0) |
| [`azure_ai_foundry_models`](./sdk/azure_ai_foundry_models) | Chat completions, embeddings | ✅ Released (v0.2.0) |
| `azure_ai_foundry_agents` | Agent Service (threads, tool calling) | 📋 Planned (v0.3.0) |
| `azure_ai_foundry_tools` | Vision, Document Intelligence, Translation | 📋 Planned (v0.3.0) |

## Quick Start

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
azure_ai_foundry_core = "0.2"
azure_ai_foundry_models = "0.2"
tokio = { version = "1", features = ["full"] }
```

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

## Authentication

The SDK supports two authentication methods:

### API Key
```rust
let credential = FoundryCredential::api_key("your-api-key");
```

### Microsoft Entra ID (Azure AD)
```rust
let credential = FoundryCredential::entra_id();
```

### Environment Variables
```bash
# The SDK checks these automatically:
export AZURE_AI_FOUNDRY_ENDPOINT="https://your-resource.services.ai.azure.com"
export AZURE_AI_FOUNDRY_API_KEY="your-key"  # Falls back to Entra ID if not set
```

## Roadmap

### v0.1.0 ✅
- [x] Project structure and CI
- [x] Chat completions (sync)
- [x] Chat completions (streaming)
- [x] Entra ID token acquisition via `azure_identity`
- [x] Embeddings

### v0.2.0 ✅
- [x] Real authentication with `azure_identity` (`Arc<dyn TokenCredential>`)
- [x] SSE parsing optimized with `memchr`
- [x] Quality improvements (buffer limits, error sanitization, streaming timeouts, retry logic)
- [x] Builder validations
- [x] High concurrency tests
- [x] Trusted Publishing to crates.io

### v0.3.0 (Current)
- [ ] Tracing instrumentation
- [ ] Agent Service (threads, tool calling)
- [ ] Foundry Tools (Vision, Document Intelligence, Translation)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the [MIT License](LICENSE).

## Disclaimer

This is an unofficial community SDK. "Microsoft", "Azure", and "Foundry" are trademarks of Microsoft Corporation. This project is not affiliated with, endorsed by, or sponsored by Microsoft.
