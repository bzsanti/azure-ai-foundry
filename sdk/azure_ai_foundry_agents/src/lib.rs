#![doc = include_str!("../README.md")]

pub mod agent;
pub mod file;
pub mod message;
pub mod models;
pub mod run;
pub mod run_step;
pub mod thread;
pub mod vector_store;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::setup_mock_client;

    /// Unix timestamp used in test responses.
    pub const TEST_TIMESTAMP: u64 = 1700000000;

    /// Default test model for agents.
    pub const TEST_MODEL: &str = "gpt-4o";
}
