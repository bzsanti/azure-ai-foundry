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
| [`azure_ai_foundry_core`](./sdk/azure_ai_foundry_core) | Auth, HTTP client, shared types | ✅ Released |
| [`azure_ai_foundry_models`](./sdk/azure_ai_foundry_models) | Chat completions, embeddings, audio, images | ✅ Released |
| [`azure_ai_foundry_agents`](./sdk/azure_ai_foundry_agents) | Agent Service (threads, runs, vector stores) | ✅ Released |
| [`azure_ai_foundry_tools`](./sdk/azure_ai_foundry_tools) | Vision, Document Intelligence | ✅ Released |
| `azure_ai_foundry_safety` | Content Safety, Prompt Shields | 📋 Planned (v0.8.0) |
| `azure_ai_foundry_language` | Text analytics, translation, PII | 📋 Planned (v0.9.0) |
| `azure_ai_foundry_speech` | Speech-to-text, text-to-speech | 📋 Planned (v0.10.0) |
| `azure_ai_foundry_realtime` | Realtime voice (WebSocket/WebRTC) | 📋 Planned (v0.13.0) |

## Quick Start

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
azure_ai_foundry_core = "0.7"
azure_ai_foundry_models = "0.7"
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
let credential = FoundryCredential::developer_tools()?;
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

### v0.3.0 ✅
- [x] Tracing instrumentation across all API calls
- [x] Agent Service crate (agents, threads, messages, runs)
- [x] Security: HTTPS validation, token refresh hardening
- [x] 270 tests passing

### v0.4.0 ✅
- [x] Foundry Tools crate (Vision API, Document Intelligence API)
- [x] docs.rs documentation with `include_str!` pattern
- [x] Quality review + 14 TDD-driven fixes
- [x] 347 tests passing

### v0.5.0 ✅
- [x] File upload/download/list/delete (`/files` API)
- [x] Vector stores CRUD (`/vector_stores` API)
- [x] Vector store files and file batches
- [x] Run steps — list and get (`/runs/{id}/steps`)
- [x] Submit tool outputs (`/runs/{id}/submit_tool_outputs`)
- [x] Agent, thread, and message update operations
- [x] `post_multipart()` and `get_bytes()` on FoundryClient
- [x] 451 tests passing

### v0.6.0 ✅
- [x] Audio transcription (Whisper STT)
- [x] Audio translation
- [x] Text-to-speech synthesis
- [x] Image generation (DALL-E, gpt-image-1)
- [x] Image editing
- [x] Responses API (`POST /responses`, `GET /responses/{id}`)
- [x] 581 tests passing

### v0.7.0 ✅
- [x] Quality refactor — 4 rounds, 52 findings resolved
- [x] Critical fix: UTF-8 boundary panic in `truncate_message`
- [x] Typed enums replacing stringly-typed fields across all crates
- [x] `bytes::Bytes` migration for zero-copy in audio/image/file requests
- [x] Percent-encoding, validation hardening, `Display` impls
- [x] 705 tests passing

### v0.8.0 — Content Safety
New crate: `azure_ai_foundry_safety`
- [ ] Text content analysis (hate, violence, sexual, self-harm)
- [ ] Image content analysis
- [ ] Prompt Shields (jailbreak detection)
- [ ] Groundedness detection
- [ ] Protected material detection (text + code)
- [ ] Blocklist management (CRUD + items)
- [ ] Custom categories (standard + rapid)

### v0.9.0 — Language & Translation
New crate: `azure_ai_foundry_language`
- [ ] Text analytics (sentiment, NER, key phrases, language detection)
- [ ] PII detection and redaction
- [ ] Text summarization (extractive + abstractive)
- [ ] Question answering
- [ ] Conversational language understanding (CLU)
- [ ] Text translation (translate, transliterate, detect, dictionary)
- [ ] Document batch translation

### v0.10.0 — Speech
New crate: `azure_ai_foundry_speech`
- [ ] Real-time speech-to-text
- [ ] Fast transcription (synchronous)
- [ ] Batch transcription (async)
- [ ] Text-to-speech (SSML)
- [ ] Voice listing

### v0.11.0 — Advanced Models & Batch
- [ ] Batch API (create, list, get, cancel)
- [ ] Fine-tuning jobs (create, list, get, cancel, pause, resume, events, checkpoints)
- [ ] Model listing and retrieval
- [ ] Files API for OpenAI endpoint (upload, list, get, delete, content)
- [ ] Evaluations API (evals, runs, output items)

### v0.12.0 — Content Understanding & Extended Tools
- [ ] Content Understanding API (multimodal: documents, images, audio, video)
- [ ] Document Intelligence: custom model build/compose/copy/delete
- [ ] Document Intelligence: classifiers
- [ ] Vision: image/text vectorization endpoints
- [ ] Face API (detect, verify, identify, person groups)

### v0.13.0 — Realtime
New crate: `azure_ai_foundry_realtime`
- [ ] Realtime API via WebSocket (voice conversations)
- [ ] Voice Live API via WebSocket (agent voice)
- [ ] WebRTC session token management

### v1.0.0 — Production Ready
- [ ] Full API parity with Azure AI Foundry platform
- [ ] Comprehensive integration test suite
- [ ] Performance benchmarks
- [ ] Migration guide from Python/C# SDKs
- [ ] Stable public API (semver guarantee)

> **Note:** An agent orchestration framework (agent loop, workflows, MCP, A2A, multi-agent) is planned as a **separate project** that uses this SDK as one of its providers. See [forja](https://github.com/bzsanti/forja) (coming soon).

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the [MIT License](LICENSE).

## Disclaimer

This is an unofficial community SDK. "Microsoft", "Azure", and "Foundry" are trademarks of Microsoft Corporation. This project is not affiliated with, endorsed by, or sponsored by Microsoft.
