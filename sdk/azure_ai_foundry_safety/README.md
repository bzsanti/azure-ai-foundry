# azure_ai_foundry_safety

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_safety.svg)](https://crates.io/crates/azure_ai_foundry_safety)
[![docs.rs](https://docs.rs/azure_ai_foundry_safety/badge.svg)](https://docs.rs/azure_ai_foundry_safety)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Content Safety client for the [Azure AI Foundry](https://azure.microsoft.com/en-us/products/ai-foundry) Rust SDK.

Part of the [`azure_ai_foundry`](https://github.com/bzsanti/azure-ai-foundry) workspace.

## Features

- **Text Analysis** — Detect hate, violence, sexual, and self-harm content
- **Image Analysis** — Moderate images for harmful content
- **Prompt Shields** — Detect jailbreak and prompt injection attacks
- **Protected Material** — Detect copyrighted text in model outputs
- **Blocklists** — Create and manage custom blocklists with CRUD operations
- **Tracing** — Full instrumentation with `tracing` spans

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.8"
azure_ai_foundry_safety = "0.8"
```

## Usage

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_safety::text::{self, AnalyzeTextRequest};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = FoundryClient::builder()
    .endpoint("https://your-resource.cognitiveservices.azure.com")
    .credential(FoundryCredential::api_key("your-key"))
    .build()?;

let request = AnalyzeTextRequest::builder()
    .text("Content to analyze")
    .try_build()?;

let response = text::analyze_text(&client, &request).await?;
for analysis in &response.categories_analysis {
    println!("{}: severity {}", analysis.category, analysis.severity);
}
# Ok(())
# }
```

## Tracing Spans

| Span | Fields |
|------|--------|
| `foundry::safety::analyze_text` | `text_len` |
| `foundry::safety::analyze_image` | — |
| `foundry::safety::shield_prompt` | — |
| `foundry::safety::detect_protected_material` | `text_len` |
| `foundry::safety::create_or_update_blocklist` | `blocklist_name` |
| `foundry::safety::get_blocklist` | `blocklist_name` |
| `foundry::safety::delete_blocklist` | `blocklist_name` |
| `foundry::safety::list_blocklists` | — |
| `foundry::safety::add_or_update_blocklist_items` | `blocklist_name` |
| `foundry::safety::get_blocklist_item` | `blocklist_name`, `item_id` |
| `foundry::safety::list_blocklist_items` | `blocklist_name` |
| `foundry::safety::remove_blocklist_items` | `blocklist_name` |

## Related Crates

- [`azure_ai_foundry_core`](https://crates.io/crates/azure_ai_foundry_core) — Auth, HTTP client, shared types
- [`azure_ai_foundry_models`](https://crates.io/crates/azure_ai_foundry_models) — Chat, embeddings, audio, images
- [`azure_ai_foundry_agents`](https://crates.io/crates/azure_ai_foundry_agents) — Agent Service
- [`azure_ai_foundry_tools`](https://crates.io/crates/azure_ai_foundry_tools) — Vision, Document Intelligence

## License

[MIT](../../LICENSE)
