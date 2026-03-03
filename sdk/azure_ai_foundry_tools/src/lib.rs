#![doc = include_str!("../README.md")]

pub mod document_intelligence;
pub mod models;
pub mod vision;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::setup_mock_client;
}
