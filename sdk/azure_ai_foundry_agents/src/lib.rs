#![doc = include_str!("../README.md")]

pub mod agent;
pub mod message;
pub mod models;
pub mod run;
pub mod thread;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    use azure_ai_foundry_core::auth::FoundryCredential;
    use azure_ai_foundry_core::client::FoundryClient;
    use wiremock::MockServer;

    /// Test API key (not a real key).
    pub const TEST_API_KEY: &str = "test-api-key";

    /// Unix timestamp used in test responses.
    pub const TEST_TIMESTAMP: u64 = 1700000000;

    /// Default test model for agents.
    pub const TEST_MODEL: &str = "gpt-4o";

    /// Create a test client connected to a mock server.
    pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key(TEST_API_KEY))
            .build()
            .expect("should build client")
    }
}
