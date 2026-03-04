#![doc = include_str!("../README.md")]

pub mod auth;
pub mod client;
pub mod error;
pub mod models;

pub use error::FoundryError;

/// Test utilities for building mock clients in unit tests.
///
/// Available when running tests (`cfg(test)`) or when the `test-support`
/// feature is enabled. Sibling crates in this workspace activate
/// `test-support` in their `[dev-dependencies]` to re-use these helpers
/// instead of duplicating them.
#[cfg(any(test, feature = "test-support"))]
pub mod test_utils {
    use crate::auth::FoundryCredential;
    use crate::client::FoundryClient;
    use wiremock::MockServer;

    /// Test API key (not a real key).
    pub const TEST_API_KEY: &str = "test-api-key";

    /// Create a test [`FoundryClient`] connected to a [`MockServer`].
    ///
    /// # Panics
    ///
    /// Panics if the client builder fails — should never happen with valid
    /// mock-server URIs.
    pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
        FoundryClient::builder()
            .endpoint(server.uri())
            .credential(FoundryCredential::api_key(TEST_API_KEY))
            .build()
            .expect("should build test client")
    }
}
