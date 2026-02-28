# azure_ai_foundry_tools

[![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_tools.svg)](https://crates.io/crates/azure_ai_foundry_tools)
[![docs.rs](https://docs.rs/azure_ai_foundry_tools/badge.svg)](https://docs.rs/azure_ai_foundry_tools)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Vision and Document Intelligence clients for the Azure AI Foundry Rust SDK.

## Features

- **Vision** — Image Analysis 4.0: tags, captions, object detection, OCR, smart crops, people
- **Document Intelligence** — Document Intelligence v4.0: OCR, layout, invoices, receipts,
  ID documents, business cards
- **Tracing** — Full instrumentation with `tracing` spans

## Installation

```toml
[dependencies]
azure_ai_foundry_core = "0.3"
azure_ai_foundry_tools = "0.3"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Analyze an Image

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_tools::vision::{self, ImageAnalysisRequest, VisualFeature};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = FoundryClient::builder()
    .endpoint("https://your-resource.services.ai.azure.com")
    .credential(FoundryCredential::api_key("your-key"))
    .build()?;

let request = ImageAnalysisRequest::builder()
    .url("https://example.com/image.jpg")
    .features(vec![VisualFeature::Tags, VisualFeature::Caption])
    .build()?;

let result = vision::analyze(&client, &request).await?;
if let Some(caption) = &result.caption_result {
    println!("Caption: {} ({:.1}%)", caption.text, caption.confidence * 100.0);
}
# Ok(())
# }
```

### Analyze a Document

```rust,no_run
use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::auth::FoundryCredential;
use azure_ai_foundry_tools::document_intelligence::{
    self, DocumentAnalysisRequest, PREBUILT_READ,
};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = FoundryClient::builder()
    .endpoint("https://your-resource.services.ai.azure.com")
    .credential(FoundryCredential::api_key("your-key"))
    .build()?;

let request = DocumentAnalysisRequest::builder()
    .model_id(PREBUILT_READ)
    .url_source("https://example.com/document.pdf")
    .build()?;

let operation = document_intelligence::analyze(&client, &request).await?;
let result = document_intelligence::poll_until_complete(
    &client,
    &operation.operation_location,
    std::time::Duration::from_secs(2),
    60,
).await?;

if let Some(ar) = &result.analyze_result {
    println!("Extracted text: {:?}", ar.content);
}
# Ok(())
# }
```

## Supported Models (Document Intelligence)

| Constant | Model ID | Purpose |
|---|---|---|
| `PREBUILT_READ` | `prebuilt-read` | General OCR |
| `PREBUILT_LAYOUT` | `prebuilt-layout` | Layout with tables |
| `PREBUILT_INVOICE` | `prebuilt-invoice` | Invoice extraction |
| `PREBUILT_RECEIPT` | `prebuilt-receipt` | Receipt extraction |
| `PREBUILT_ID_DOCUMENT` | `prebuilt-idDocument` | Passport / ID |
| `PREBUILT_BUSINESS_CARD` | `prebuilt-businessCard` | Business card |

## Tracing Spans

| Span | Fields |
|------|--------|
| `foundry::vision::analyze` | features |
| `foundry::document_intelligence::analyze` | model_id |
| `foundry::document_intelligence::get_result` | operation_location |
| `foundry::document_intelligence::poll_until_complete` | operation_location |

## Related Crates

- [`azure_ai_foundry_core`](../azure_ai_foundry_core) — Core types, authentication, and HTTP client
- [`azure_ai_foundry_models`](../azure_ai_foundry_models) — Chat completions and embeddings
- [`azure_ai_foundry_agents`](../azure_ai_foundry_agents) — Agent Service client

## License

This project is licensed under the [MIT License](../../LICENSE).
