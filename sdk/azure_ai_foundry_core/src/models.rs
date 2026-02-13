//! Common types shared across all Azure AI Foundry crates.

use serde::{Deserialize, Serialize};

/// Usage statistics returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: Option<u32>,
    pub total_tokens: u32,
}
