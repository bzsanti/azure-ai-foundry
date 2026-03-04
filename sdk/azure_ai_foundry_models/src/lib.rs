#![doc = include_str!("../README.md")]

pub mod audio;
pub mod chat;
pub mod embeddings;
pub mod images;
pub mod responses;

/// Test utilities shared across modules.
#[cfg(test)]
pub(crate) mod test_utils {
    pub use azure_ai_foundry_core::test_utils::{setup_mock_client, TEST_API_KEY};

    /// Default test model for chat completions.
    #[allow(dead_code)]
    pub const TEST_CHAT_MODEL: &str = "gpt-4o";

    /// Default test model for embeddings.
    #[allow(dead_code)]
    pub const TEST_EMBEDDING_MODEL: &str = "text-embedding-ada-002";

    /// Default test model for audio transcription/translation.
    #[allow(dead_code)]
    pub const TEST_AUDIO_MODEL: &str = "whisper-1";

    /// Default test model for text-to-speech.
    #[allow(dead_code)]
    pub const TEST_TTS_MODEL: &str = "tts-1";

    /// Default test model for image generation.
    #[allow(dead_code)]
    pub const TEST_IMAGE_MODEL: &str = "dall-e-3";

    /// Unix timestamp used in test responses.
    #[allow(dead_code)]
    pub const TEST_TIMESTAMP: u64 = 1700000000;
}
