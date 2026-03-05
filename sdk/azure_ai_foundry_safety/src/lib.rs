#![doc = include_str!("../README.md")]

pub mod blocklist;
pub mod image;
pub mod models;
pub mod prompt_shields;
pub mod protected_material;
pub mod text;

/// Percent-encode a query parameter value using `application/x-www-form-urlencoded` rules.
#[allow(dead_code)]
pub(crate) fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::setup_mock_client;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_query_value_encodes_spaces() {
        assert_eq!(encode_query_value("hello world"), "hello+world");
    }
}
