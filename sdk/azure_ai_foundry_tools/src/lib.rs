//! # Azure AI Foundry Tools
//!
//! Vision and Document Intelligence clients for the Azure AI Foundry Rust SDK.
//!
//! This crate provides Rust bindings for two Azure AI Services accessible through
//! AI Foundry:
//!
//! - **Vision** - Image Analysis 4.0 for visual analysis (tags, captions, object
//!   detection, OCR, dense captions, smart crops, people detection).
//! - **Document Intelligence** - Document Intelligence v4.0 for document analysis
//!   (OCR, layout, invoices, receipts, ID documents, business cards).
//!
//! ## Quick Start - Vision
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_tools::vision::{self, ImageAnalysisRequest, VisualFeature};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! let request = ImageAnalysisRequest::builder()
//!     .url("https://example.com/image.jpg")
//!     .features(vec![VisualFeature::Tags, VisualFeature::Caption])
//!     .build()?;
//!
//! let result = vision::analyze(&client, &request).await?;
//! if let Some(caption) = &result.caption_result {
//!     println!("Caption: {} ({:.2}%)", caption.text, caption.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Quick Start - Document Intelligence
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_tools::document_intelligence::{self, DocumentAnalysisRequest, PREBUILT_READ};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! let request = DocumentAnalysisRequest::builder()
//!     .model_id(PREBUILT_READ)
//!     .url_source("https://example.com/document.pdf")
//!     .build()?;
//!
//! let operation = document_intelligence::analyze(&client, &request).await?;
//! let result = document_intelligence::poll_until_complete(
//!     &client,
//!     &operation.operation_location,
//!     std::time::Duration::from_secs(2),
//!     60,
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod document_intelligence;
pub mod models;
pub mod vision;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    use azure_ai_foundry_core::auth::FoundryCredential;
    use azure_ai_foundry_core::client::FoundryClient;
    use wiremock::MockServer;

    /// Test API key (not a real key).
    pub const TEST_API_KEY: &str = "test-api-key";

    /// Create a test client connected to a mock server.
    pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key(TEST_API_KEY))
            .build()
            .expect("should build client")
    }
}
