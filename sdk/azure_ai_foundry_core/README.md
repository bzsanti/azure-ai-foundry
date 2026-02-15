# azure_ai_foundry_core

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_core.svg)](https://crates.io/crates/azure_ai_foundry_core)
[![docs.rs](https://docs.rs/azure_ai_foundry_core/badge.svg)](https://docs.rs/azure_ai_foundry_core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Core types, authentication, and HTTP client for the Azure AI Foundry Rust SDK.

This crate provides the foundational building blocks used by all other `azure_ai_foundry_*` crates.

## Features

- **FoundryClient** — HTTP client with builder pattern for Azure AI Foundry services
- **FoundryCredential** — Authentication via API key or Microsoft Entra ID
- **FoundryError** — Typed error handling with `thiserror`
- **Retry logic** — Automatic retries with exponential backoff for transient errors
- **Security** — Error sanitization to prevent credential leaks in logs

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.2"
tokio = { version = "1", features = ["full"] }
```

## Usage

### API Key Authentication

```rust
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FoundryClient::builder()
        .endpoint("https://your-resource.services.ai.azure.com")
        .credential(FoundryCredential::api_key("your-api-key"))
        .build()?;
    Ok(())
}
```

### Microsoft Entra ID Authentication

```rust
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FoundryClient::builder()
        .endpoint("https://your-resource.services.ai.azure.com")
        .credential(FoundryCredential::entra_id())
        .build()?;
    Ok(())
}
```

### Environment Variables

The SDK automatically checks these environment variables:

```bash
export AZURE_AI_FOUNDRY_ENDPOINT="https://your-resource.services.ai.azure.com"
export AZURE_AI_FOUNDRY_API_KEY="your-key"  # Falls back to Entra ID if not set
```

```rust
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;

let client = FoundryClient::builder()
    .endpoint_from_env()?
    .credential(FoundryCredential::from_env()?)
    .build()?;
```

## Modules

| Module | Description |
|--------|-------------|
| `auth` | `FoundryCredential` for API key and Entra ID authentication |
| `client` | `FoundryClient` builder and HTTP client |
| `error` | `FoundryError` type with typed error variants |
| `models` | Common types shared across crates |

## Related Crates

- [`azure_ai_foundry_models`](../azure_ai_foundry_models) — Chat completions and embeddings

## License

This project is licensed under the [MIT License](../../LICENSE).
