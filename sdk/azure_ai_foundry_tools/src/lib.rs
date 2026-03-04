#![doc = include_str!("../README.md")]

pub mod document_intelligence;
pub mod models;
pub mod vision;

/// Percent-encode a query parameter value using `application/x-www-form-urlencoded` rules.
pub(crate) fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::setup_mock_client;
}
