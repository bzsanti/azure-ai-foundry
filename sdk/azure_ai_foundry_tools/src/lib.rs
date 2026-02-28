#![doc = include_str!("../README.md")]

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
