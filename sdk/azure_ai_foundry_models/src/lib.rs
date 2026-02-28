#![doc = include_str!("../README.md")]

pub mod chat;
pub mod embeddings;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    use azure_ai_foundry_core::auth::FoundryCredential;
    use azure_ai_foundry_core::client::FoundryClient;
    use wiremock::MockServer;

    // --- Test constants ---
    // These constants are available for use in integration tests.
    // Not all are currently used, but they provide consistent values across tests.

    /// Default test model for chat completions.
    #[allow(dead_code)]
    pub const TEST_CHAT_MODEL: &str = "gpt-4o";

    /// Default test model for embeddings.
    #[allow(dead_code)]
    pub const TEST_EMBEDDING_MODEL: &str = "text-embedding-ada-002";

    /// Test API key (not a real key).
    pub const TEST_API_KEY: &str = "test-api-key";

    /// Unix timestamp used in test responses.
    #[allow(dead_code)]
    pub const TEST_TIMESTAMP: u64 = 1700000000;

    /// Create a test client connected to a mock server.
    pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key(TEST_API_KEY))
            .build()
            .expect("should build client")
    }
}
