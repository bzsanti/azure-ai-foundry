//! # Azure AI Foundry Models
//!
//! Chat completions and embeddings client for the Azure AI Foundry Rust SDK.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_models::chat::{ChatCompletionRequest, Message, Role};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = FoundryClient::builder()
//!         .endpoint("https://your-resource.services.ai.azure.com")
//!         .credential(FoundryCredential::api_key("your-key"))
//!         .build()?;
//!
//!     let request = ChatCompletionRequest::builder()
//!         .model("gpt-4o")
//!         .message(Message::user("Hello, world!"))
//!         .build();
//!
//!     let response = azure_ai_foundry_models::chat::complete(&client, &request).await?;
//!     println!("{}", response.choices[0].message.content.as_deref().unwrap_or_default());
//!     Ok(())
//! }
//! ```

pub mod chat;
pub mod embeddings;
