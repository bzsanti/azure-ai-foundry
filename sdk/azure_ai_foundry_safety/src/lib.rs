#![doc = include_str!("../README.md")]

pub mod blocklist;
pub mod image;
pub mod models;
pub mod prompt_shields;
pub mod protected_material;
pub mod text;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::setup_mock_client;
}
