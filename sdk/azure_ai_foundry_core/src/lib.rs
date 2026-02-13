//! # Azure AI Foundry Core
//!
//! Core types, authentication, and HTTP client for the Azure AI Foundry Rust SDK.
//!
//! This crate provides the foundational building blocks used by all other
//! `azure_ai_foundry_*` crates. You typically won't use this crate directly
//! unless you're building a custom integration.
//!
//! ## Authentication
//!
//! The SDK supports Microsoft Entra ID (Azure AD) authentication via
//! [`azure_identity`]. API key authentication is also supported for
//! the OpenAI-compatible endpoint.
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = FoundryClient::builder()
//!         .endpoint("https://your-resource.services.ai.azure.com")
//!         .credential(FoundryCredential::from_env()?)
//!         .build()?;
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod client;
pub mod error;
pub mod models;

pub use error::FoundryError;
