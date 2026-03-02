//! Audio API types and functions for Azure AI Foundry Models.
//!
//! This module provides transcription (speech-to-text), translation, and
//! text-to-speech (TTS) APIs.
//!
//! # Transcription Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::audio::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let audio_data = std::fs::read("recording.wav").unwrap();
//! let request = TranscriptionRequest::builder()
//!     .model("whisper-1")
//!     .filename("recording.wav")
//!     .data(audio_data)
//!     .build();
//!
//! let response = transcribe(client, &request).await?;
//! println!("Transcription: {}", response.text);
//! # Ok(())
//! # }
//! ```
//!
//! # Text-to-Speech Example
//!
//! ```rust,no_run
//! # use azure_ai_foundry_core::client::FoundryClient;
//! # use azure_ai_foundry_models::audio::*;
//! # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
//! let request = SpeechRequest::builder()
//!     .model("tts-1")
//!     .input("Hello, world!")
//!     .voice("alloy")
//!     .build();
//!
//! let audio_bytes = speak(client, &request).await?;
//! std::fs::write("output.mp3", &audio_bytes).unwrap();
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum input length for text-to-speech requests (4096 characters).
pub const MAX_SPEECH_INPUT_LENGTH: usize = 4096;

// ---------------------------------------------------------------------------
// Audio response format
// ---------------------------------------------------------------------------

/// Output format for transcription and translation responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioResponseFormat {
    /// JSON response with just the text.
    Json,
    /// Plain text response.
    Text,
    /// SubRip subtitle format.
    Srt,
    /// WebVTT subtitle format.
    Vtt,
    /// Verbose JSON with timestamps, segments, and metadata.
    VerboseJson,
}

impl AudioResponseFormat {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Text => "text",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::VerboseJson => "verbose_json",
        }
    }
}

// ---------------------------------------------------------------------------
// Speech format
// ---------------------------------------------------------------------------

/// Output format for text-to-speech audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechFormat {
    /// MP3 audio format.
    Mp3,
    /// Opus audio format.
    Opus,
    /// AAC audio format.
    Aac,
    /// FLAC audio format.
    Flac,
    /// WAV audio format.
    Wav,
    /// PCM 16-bit audio format.
    #[serde(rename = "pcm16")]
    Pcm16,
}

// ---------------------------------------------------------------------------
// Transcription request
// ---------------------------------------------------------------------------

/// A request to transcribe audio to text.
#[derive(Debug)]
pub struct TranscriptionRequest {
    /// The model to use for transcription.
    pub model: String,
    /// The audio file name.
    pub filename: String,
    /// The raw audio file data.
    pub data: Vec<u8>,
    /// The language of the input audio in ISO-639-1 format.
    pub language: Option<String>,
    /// An optional text to guide the model's style or continue a previous segment.
    pub prompt: Option<String>,
    /// The format of the transcript output.
    pub response_format: Option<AudioResponseFormat>,
    /// The sampling temperature, between 0.0 and 1.0.
    pub temperature: Option<f32>,
}

impl TranscriptionRequest {
    /// Create a new builder.
    pub fn builder() -> TranscriptionRequestBuilder {
        TranscriptionRequestBuilder {
            model: None,
            filename: None,
            data: None,
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
        }
    }
}

/// Builder for [`TranscriptionRequest`].
pub struct TranscriptionRequestBuilder {
    model: Option<String>,
    filename: Option<String>,
    data: Option<Vec<u8>>,
    language: Option<String>,
    prompt: Option<String>,
    response_format: Option<AudioResponseFormat>,
    temperature: Option<f32>,
}

impl TranscriptionRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the audio filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Set the audio file data.
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the language of the input audio (ISO-639-1).
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set a prompt to guide the model.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: AudioResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the sampling temperature (0.0 to 1.0).
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<TranscriptionRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let filename = self
            .filename
            .ok_or_else(|| FoundryError::Builder("filename is required".into()))?;
        if filename.is_empty() {
            return Err(FoundryError::Builder("filename cannot be empty".into()));
        }

        let data = self
            .data
            .ok_or_else(|| FoundryError::Builder("data is required".into()))?;
        if data.is_empty() {
            return Err(FoundryError::Builder("data cannot be empty".into()));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=1.0).contains(&temp) {
                return Err(FoundryError::Builder(
                    "temperature must be between 0.0 and 1.0".into(),
                ));
            }
        }

        Ok(TranscriptionRequest {
            model,
            filename,
            data,
            language: self.language,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
        })
    }

    /// Build the request. Panics if required fields are missing.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> TranscriptionRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Translation request
// ---------------------------------------------------------------------------

/// A request to translate audio to English text.
///
/// Translation always outputs English. For same-language transcription,
/// use [`TranscriptionRequest`] instead.
#[derive(Debug)]
pub struct TranslationRequest {
    /// The model to use for translation.
    pub model: String,
    /// The audio file name.
    pub filename: String,
    /// The raw audio file data.
    pub data: Vec<u8>,
    /// An optional text to guide the model's style or continue a previous segment.
    pub prompt: Option<String>,
    /// The format of the transcript output.
    pub response_format: Option<AudioResponseFormat>,
    /// The sampling temperature, between 0.0 and 1.0.
    pub temperature: Option<f32>,
}

impl TranslationRequest {
    /// Create a new builder.
    pub fn builder() -> TranslationRequestBuilder {
        TranslationRequestBuilder {
            model: None,
            filename: None,
            data: None,
            prompt: None,
            response_format: None,
            temperature: None,
        }
    }
}

/// Builder for [`TranslationRequest`].
pub struct TranslationRequestBuilder {
    model: Option<String>,
    filename: Option<String>,
    data: Option<Vec<u8>>,
    prompt: Option<String>,
    response_format: Option<AudioResponseFormat>,
    temperature: Option<f32>,
}

impl TranslationRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the audio filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Set the audio file data.
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Set a prompt to guide the model.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: AudioResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the sampling temperature (0.0 to 1.0).
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<TranslationRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let filename = self
            .filename
            .ok_or_else(|| FoundryError::Builder("filename is required".into()))?;
        if filename.is_empty() {
            return Err(FoundryError::Builder("filename cannot be empty".into()));
        }

        let data = self
            .data
            .ok_or_else(|| FoundryError::Builder("data is required".into()))?;
        if data.is_empty() {
            return Err(FoundryError::Builder("data cannot be empty".into()));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=1.0).contains(&temp) {
                return Err(FoundryError::Builder(
                    "temperature must be between 0.0 and 1.0".into(),
                ));
            }
        }

        Ok(TranslationRequest {
            model,
            filename,
            data,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
        })
    }

    /// Build the request. Panics if required fields are missing.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> TranslationRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Speech request
// ---------------------------------------------------------------------------

/// A request to generate speech from text.
#[derive(Debug, Clone, Serialize)]
pub struct SpeechRequest {
    /// The model to use for speech generation.
    pub model: String,
    /// The text to generate audio for.
    pub input: String,
    /// The voice to use for generation.
    pub voice: String,

    /// The format of the audio output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SpeechFormat>,

    /// The speed of the generated audio (0.25 to 4.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

impl SpeechRequest {
    /// Create a new builder.
    pub fn builder() -> SpeechRequestBuilder {
        SpeechRequestBuilder {
            model: None,
            input: None,
            voice: None,
            response_format: None,
            speed: None,
        }
    }
}

/// Builder for [`SpeechRequest`].
pub struct SpeechRequestBuilder {
    model: Option<String>,
    input: Option<String>,
    voice: Option<String>,
    response_format: Option<SpeechFormat>,
    speed: Option<f32>,
}

impl SpeechRequestBuilder {
    /// Set the model ID.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the text input to generate speech for.
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Set the voice to use for generation.
    pub fn voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = Some(voice.into());
        self
    }

    /// Set the audio output format.
    pub fn response_format(mut self, format: SpeechFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the speed of the generated audio (0.25 to 4.0).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Build the request, returning an error if required fields are missing
    /// or parameter values are invalid.
    pub fn try_build(self) -> FoundryResult<SpeechRequest> {
        let model = self
            .model
            .ok_or_else(|| FoundryError::Builder("model is required".into()))?;
        if model.is_empty() {
            return Err(FoundryError::Builder("model cannot be empty".into()));
        }

        let input = self
            .input
            .ok_or_else(|| FoundryError::Builder("input is required".into()))?;
        if input.is_empty() {
            return Err(FoundryError::Builder("input cannot be empty".into()));
        }
        if input.len() > MAX_SPEECH_INPUT_LENGTH {
            return Err(FoundryError::Builder(format!(
                "input exceeds maximum length of {} characters",
                MAX_SPEECH_INPUT_LENGTH
            )));
        }

        let voice = self
            .voice
            .ok_or_else(|| FoundryError::Builder("voice is required".into()))?;
        if voice.is_empty() {
            return Err(FoundryError::Builder("voice cannot be empty".into()));
        }

        if let Some(speed) = self.speed {
            if !(0.25..=4.0).contains(&speed) {
                return Err(FoundryError::Builder(
                    "speed must be between 0.25 and 4.0".into(),
                ));
            }
        }

        Ok(SpeechRequest {
            model,
            input,
            voice,
            response_format: self.response_format,
            speed: self.speed,
        })
    }

    /// Build the request. Panics if required fields are missing.
    ///
    /// Consider using [`try_build`](Self::try_build) for fallible construction.
    pub fn build(self) -> SpeechRequest {
        self.try_build().expect("builder validation failed")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from a transcription or translation request.
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionResponse {
    /// The transcribed or translated text.
    pub text: String,
}

/// Response from an audio translation request.
///
/// Translation always produces English text regardless of the input language.
/// Structurally identical to [`TranscriptionResponse`]; defined as a distinct
/// type alias for semantic clarity at call sites.
pub type TranslationResponse = TranscriptionResponse;

/// Verbose response from a transcription request with timestamps and segments.
#[derive(Debug, Clone, Deserialize)]
pub struct VerboseTranscriptionResponse {
    /// The task that was performed (e.g., "transcribe").
    pub task: String,
    /// The detected or specified language.
    pub language: String,
    /// The duration of the audio in seconds.
    pub duration: f64,
    /// The transcribed text.
    pub text: String,
    /// The segments of the transcription.
    pub segments: Option<Vec<TranscriptionSegment>>,
}

/// A segment of a verbose transcription response.
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionSegment {
    /// Segment index.
    pub id: u32,
    /// Seek offset in the audio.
    pub seek: u32,
    /// Start time of the segment in seconds.
    pub start: f64,
    /// End time of the segment in seconds.
    pub end: f64,
    /// The transcribed text of the segment.
    pub text: String,
    /// Token IDs for the segment.
    pub tokens: Vec<u32>,
    /// Temperature used for this segment.
    pub temperature: f64,
    /// Average log probability.
    pub avg_logprob: f64,
    /// Compression ratio of the segment.
    pub compression_ratio: f64,
    /// Probability that the segment contains no speech.
    pub no_speech_prob: f64,
}

// ---------------------------------------------------------------------------
// Helper: build multipart form for audio
// ---------------------------------------------------------------------------

/// Build a multipart form for audio transcription/translation.
fn build_audio_form(
    data: &[u8],
    filename: &str,
    model: &str,
    language: Option<&str>,
    prompt: Option<&str>,
    response_format: Option<AudioResponseFormat>,
    temperature: Option<f32>,
) -> reqwest::multipart::Form {
    let file_part = reqwest::multipart::Part::bytes(data.to_vec()).file_name(filename.to_string());
    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }
    if let Some(p) = prompt {
        form = form.text("prompt", p.to_string());
    }
    if let Some(fmt) = response_format {
        form = form.text("response_format", fmt.as_str());
    }
    if let Some(temp) = temperature {
        form = form.text("temperature", temp.to_string());
    }

    form
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Transcribe audio to text.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::audio::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = TranscriptionRequest::builder()
///     .model("whisper-1")
///     .filename("audio.wav")
///     .data(vec![0u8; 100])
///     .build();
///
/// let response = transcribe(client, &request).await?;
/// println!("{}", response.text);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::audio::transcribe` with field `model`.
#[tracing::instrument(
    name = "foundry::audio::transcribe",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn transcribe(
    client: &FoundryClient,
    request: &TranscriptionRequest,
) -> FoundryResult<TranscriptionResponse> {
    tracing::debug!("sending transcription request");

    let data = request.data.clone();
    let filename = request.filename.clone();
    let model = request.model.clone();
    let language = request.language.clone();
    let prompt = request.prompt.clone();
    let response_format = request.response_format;
    let temperature = request.temperature;

    let response = client
        .post_multipart("/openai/v1/audio/transcriptions", move || {
            build_audio_form(
                &data,
                &filename,
                &model,
                language.as_deref(),
                prompt.as_deref(),
                response_format,
                temperature,
            )
        })
        .await?;

    let body = response.json::<TranscriptionResponse>().await?;
    Ok(body)
}

/// Translate audio to English text.
///
/// Translation always outputs English regardless of the input language.
/// For same-language transcription, use [`transcribe`] instead.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::audio::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = TranslationRequest::builder()
///     .model("whisper-1")
///     .filename("audio_fr.wav")
///     .data(vec![0u8; 100])
///     .build();
///
/// let response = translate(client, &request).await?;
/// println!("English: {}", response.text);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::audio::translate` with field `model`.
#[tracing::instrument(
    name = "foundry::audio::translate",
    skip(client, request),
    fields(model = %request.model)
)]
pub async fn translate(
    client: &FoundryClient,
    request: &TranslationRequest,
) -> FoundryResult<TranslationResponse> {
    tracing::debug!("sending translation request");

    let data = request.data.clone();
    let filename = request.filename.clone();
    let model = request.model.clone();
    let prompt = request.prompt.clone();
    let response_format = request.response_format;
    let temperature = request.temperature;

    let response = client
        .post_multipart("/openai/v1/audio/translations", move || {
            build_audio_form(
                &data,
                &filename,
                &model,
                None, // translation has no language parameter
                prompt.as_deref(),
                response_format,
                temperature,
            )
        })
        .await?;

    let body = response.json::<TranscriptionResponse>().await?;
    Ok(body)
}

/// Generate speech audio from text.
///
/// Returns the raw audio bytes in the requested format (defaults to MP3).
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_models::audio::*;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = SpeechRequest::builder()
///     .model("tts-1")
///     .input("Hello, world!")
///     .voice("alloy")
///     .build();
///
/// let audio = speak(client, &request).await?;
/// std::fs::write("speech.mp3", &audio).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// # Limitations
///
/// The `response.bytes()` call that reads the audio body is **not retried**
/// if the connection drops mid-stream. On transient network errors after the
/// HTTP 200 response headers are received, the caller must retry the full
/// `speak()` call.
///
/// # Tracing
///
/// Emits a span named `foundry::audio::speak` with fields `model` and `voice`.
#[tracing::instrument(
    name = "foundry::audio::speak",
    skip(client, request),
    fields(model = %request.model, voice = %request.voice)
)]
pub async fn speak(client: &FoundryClient, request: &SpeechRequest) -> FoundryResult<bytes::Bytes> {
    tracing::debug!("sending speech request");

    let response = client.post("/openai/v1/audio/speech", request).await?;
    let body = response.bytes().await?;
    Ok(body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_API_KEY};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // =======================================================================
    // Phase 1: Audio Transcription
    // =======================================================================

    // --- Cycle 1.1: TranscriptionRequest builder ---

    #[test]
    fn test_transcription_request_builder() {
        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![1, 2, 3])
            .build();

        assert_eq!(request.model, "whisper-1");
        assert_eq!(request.filename, "test.wav");
        assert_eq!(request.data, vec![1, 2, 3]);
        assert!(request.language.is_none());
        assert!(request.prompt.is_none());
        assert!(request.response_format.is_none());
        assert!(request.temperature.is_none());
    }

    #[test]
    fn test_transcription_request_builder_all_fields() {
        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![1, 2, 3])
            .language("en")
            .prompt("Some context")
            .response_format(AudioResponseFormat::VerboseJson)
            .temperature(0.5)
            .build();

        assert_eq!(request.language, Some("en".into()));
        assert_eq!(request.prompt, Some("Some context".into()));
        assert_eq!(
            request.response_format,
            Some(AudioResponseFormat::VerboseJson)
        );
        assert_eq!(request.temperature, Some(0.5));
    }

    // --- Cycle 1.2: AudioResponseFormat serde ---

    #[test]
    fn test_audio_response_format_serialization() {
        assert_eq!(
            serde_json::to_string(&AudioResponseFormat::Json).unwrap(),
            "\"json\""
        );
        assert_eq!(
            serde_json::to_string(&AudioResponseFormat::Text).unwrap(),
            "\"text\""
        );
        assert_eq!(
            serde_json::to_string(&AudioResponseFormat::Srt).unwrap(),
            "\"srt\""
        );
        assert_eq!(
            serde_json::to_string(&AudioResponseFormat::Vtt).unwrap(),
            "\"vtt\""
        );
        assert_eq!(
            serde_json::to_string(&AudioResponseFormat::VerboseJson).unwrap(),
            "\"verbose_json\""
        );
    }

    #[test]
    fn test_audio_response_format_deserialization() {
        assert_eq!(
            serde_json::from_str::<AudioResponseFormat>("\"json\"").unwrap(),
            AudioResponseFormat::Json
        );
        assert_eq!(
            serde_json::from_str::<AudioResponseFormat>("\"verbose_json\"").unwrap(),
            AudioResponseFormat::VerboseJson
        );
    }

    // --- Cycle 1.3: Builder validation ---

    #[test]
    fn test_transcription_request_rejects_empty_model() {
        let result = TranscriptionRequest::builder()
            .model("")
            .filename("test.wav")
            .data(vec![1])
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model cannot be empty"));
    }

    #[test]
    fn test_transcription_request_rejects_empty_data() {
        let result = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![])
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("data cannot be empty"));
    }

    #[test]
    fn test_transcription_request_rejects_empty_filename() {
        let result = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("")
            .data(vec![1])
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("filename cannot be empty"));
    }

    #[test]
    fn test_transcription_request_validates_temperature() {
        let result = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![1])
            .temperature(1.5)
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("temperature must be between 0.0 and 1.0"));
    }

    #[test]
    fn test_transcription_request_accepts_valid_temperature() {
        let result = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![1])
            .temperature(0.0)
            .try_build();

        assert!(result.is_ok());
    }

    // --- Cycle 1.4: TranscriptionResponse deserialization ---

    #[test]
    fn test_transcription_response_deserialization() {
        let json = serde_json::json!({"text": "Hello world"});

        let response: TranscriptionResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.text, "Hello world");
    }

    // --- Cycle 1.5: VerboseTranscriptionResponse deserialization ---

    #[test]
    fn test_verbose_transcription_response_deserialization() {
        let json = serde_json::json!({
            "task": "transcribe",
            "language": "en",
            "duration": 2.5,
            "text": "Hello",
            "segments": [{
                "id": 0,
                "seek": 0,
                "start": 0.0,
                "end": 2.5,
                "text": "Hello",
                "tokens": [50364, 2425, 50489],
                "temperature": 0.0,
                "avg_logprob": -0.5,
                "compression_ratio": 1.0,
                "no_speech_prob": 0.01
            }]
        });

        let response: VerboseTranscriptionResponse = serde_json::from_value(json).unwrap();

        assert_eq!(response.task, "transcribe");
        assert_eq!(response.language, "en");
        assert!((response.duration - 2.5).abs() < f64::EPSILON);
        assert_eq!(response.text, "Hello");

        let segments = response.segments.unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].id, 0);
        assert_eq!(segments[0].text, "Hello");
        assert_eq!(segments[0].tokens, vec![50364, 2425, 50489]);
    }

    #[test]
    fn test_verbose_transcription_response_without_segments() {
        let json = serde_json::json!({
            "task": "transcribe",
            "language": "en",
            "duration": 1.0,
            "text": "Hi"
        });

        let response: VerboseTranscriptionResponse = serde_json::from_value(json).unwrap();

        assert!(response.segments.is_none());
    }

    // --- Cycle 1.6: transcribe() API function ---

    #[tokio::test]
    async fn test_transcribe_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({"text": "Hello world"});

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/transcriptions"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![0u8; 100])
            .build();

        let response = transcribe(&client, &request).await.expect("should succeed");

        assert_eq!(response.text, "Hello world");
    }

    // --- Cycle 1.7: transcribe() error handling ---

    #[tokio::test]
    async fn test_transcribe_returns_error_on_400() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidRequest",
                "message": "Invalid audio format"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("bad.txt")
            .data(vec![1, 2, 3])
            .build();

        let result = transcribe(&client, &request).await;

        assert!(result.is_err());
    }

    // --- Cycle 1.8: TranslationRequest builder ---

    #[test]
    fn test_translation_request_builder() {
        let request = TranslationRequest::builder()
            .model("whisper-1")
            .filename("audio_fr.wav")
            .data(vec![1, 2, 3])
            .build();

        assert_eq!(request.model, "whisper-1");
        assert_eq!(request.filename, "audio_fr.wav");
        assert_eq!(request.data, vec![1, 2, 3]);
        assert!(request.prompt.is_none());
        assert!(request.response_format.is_none());
        assert!(request.temperature.is_none());
    }

    #[test]
    fn test_translation_request_builder_all_fields() {
        let request = TranslationRequest::builder()
            .model("whisper-1")
            .filename("audio.wav")
            .data(vec![1])
            .prompt("Context")
            .response_format(AudioResponseFormat::Text)
            .temperature(0.3)
            .build();

        assert_eq!(request.prompt, Some("Context".into()));
        assert_eq!(request.response_format, Some(AudioResponseFormat::Text));
        assert_eq!(request.temperature, Some(0.3));
    }

    // --- Cycle 1.9: translate() API function ---

    #[tokio::test]
    async fn test_translate_success() {
        let server = MockServer::start().await;

        let expected_response = serde_json::json!({"text": "Hello in English"});

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/translations"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranslationRequest::builder()
            .model("whisper-1")
            .filename("audio_fr.wav")
            .data(vec![0u8; 100])
            .build();

        let response = translate(&client, &request).await.expect("should succeed");

        assert_eq!(response.text, "Hello in English");
    }

    // --- Cycle 1.10: Translation validation ---

    #[test]
    fn test_translation_request_rejects_empty_model() {
        let result = TranslationRequest::builder()
            .model("")
            .filename("test.wav")
            .data(vec![1])
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model cannot be empty"));
    }

    #[test]
    fn test_translation_request_rejects_empty_data() {
        let result = TranslationRequest::builder()
            .model("whisper-1")
            .filename("test.wav")
            .data(vec![])
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("data cannot be empty"));
    }

    // =======================================================================
    // Phase 2: Text-to-Speech
    // =======================================================================

    // --- Cycle 2.1: SpeechRequest builder ---

    #[test]
    fn test_speech_request_builder() {
        let request = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .build();

        assert_eq!(request.model, "tts-1");
        assert_eq!(request.input, "Hello");
        assert_eq!(request.voice, "alloy");
        assert!(request.response_format.is_none());
        assert!(request.speed.is_none());
    }

    // --- Cycle 2.2: SpeechRequest serialization ---

    #[test]
    fn test_speech_request_serialization() {
        let request = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "tts-1");
        assert_eq!(json["input"], "Hello");
        assert_eq!(json["voice"], "alloy");
        assert!(json.get("response_format").is_none());
        assert!(json.get("speed").is_none());
    }

    #[test]
    fn test_speech_request_serialization_all_fields() {
        let request = SpeechRequest::builder()
            .model("tts-1-hd")
            .input("Hello world")
            .voice("nova")
            .response_format(SpeechFormat::Opus)
            .speed(1.5)
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "tts-1-hd");
        assert_eq!(json["input"], "Hello world");
        assert_eq!(json["voice"], "nova");
        assert_eq!(json["response_format"], "opus");
        assert_eq!(json["speed"], 1.5);
    }

    // --- Cycle 2.3: SpeechRequest validation ---

    #[test]
    fn test_speech_request_rejects_empty_input() {
        let result = SpeechRequest::builder()
            .model("tts-1")
            .input("")
            .voice("alloy")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("input cannot be empty"));
    }

    #[test]
    fn test_speech_request_rejects_empty_voice() {
        let result = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("voice cannot be empty"));
    }

    #[test]
    fn test_speech_request_validates_speed_range() {
        let too_slow = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .speed(0.1)
            .try_build();

        assert!(too_slow.is_err());
        assert!(too_slow
            .unwrap_err()
            .to_string()
            .contains("speed must be between 0.25 and 4.0"));

        let too_fast = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .speed(5.0)
            .try_build();

        assert!(too_fast.is_err());
    }

    #[test]
    fn test_speech_request_accepts_valid_speed() {
        let result = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .speed(2.0)
            .try_build();

        assert!(result.is_ok());
    }

    // --- Cycle 2.4: SpeechFormat serde ---

    #[test]
    fn test_speech_format_serialization() {
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Mp3).unwrap(),
            "\"mp3\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Opus).unwrap(),
            "\"opus\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Aac).unwrap(),
            "\"aac\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Flac).unwrap(),
            "\"flac\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Wav).unwrap(),
            "\"wav\""
        );
        assert_eq!(
            serde_json::to_string(&SpeechFormat::Pcm16).unwrap(),
            "\"pcm16\""
        );
    }

    // --- Cycle 2.5: speak() API function ---

    #[tokio::test]
    async fn test_speak_success() {
        let server = MockServer::start().await;

        let audio_bytes = vec![0xFF, 0xFB, 0x90, 0x00]; // fake MP3 header

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/speech"))
            .and(header("Authorization", format!("Bearer {}", TEST_API_KEY)))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(audio_bytes.clone()))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello world")
            .voice("alloy")
            .build();

        let result = speak(&client, &request).await.expect("should succeed");

        assert_eq!(result.as_ref(), &audio_bytes[..]);
    }

    // --- Cycle 2.6: speak() error handling ---

    #[tokio::test]
    async fn test_speak_returns_error_on_400() {
        let server = MockServer::start().await;

        let error_response = serde_json::json!({
            "error": {
                "code": "InvalidRequest",
                "message": "Voice not supported"
            }
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/speech"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("invalid")
            .build();

        let result = speak(&client, &request).await;

        assert!(result.is_err());
    }

    // --- Cycle 2.7: MAX_SPEECH_INPUT_LENGTH ---

    #[test]
    fn test_speech_rejects_oversized_input() {
        let long_input = "a".repeat(MAX_SPEECH_INPUT_LENGTH + 1);

        let result = SpeechRequest::builder()
            .model("tts-1")
            .input(long_input)
            .voice("alloy")
            .try_build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum length"));
    }

    #[test]
    fn test_speech_accepts_max_length_input() {
        let max_input = "a".repeat(MAX_SPEECH_INPUT_LENGTH);

        let result = SpeechRequest::builder()
            .model("tts-1")
            .input(max_input)
            .voice("alloy")
            .try_build();

        assert!(result.is_ok());
    }

    // --- Cycle 2.8: MAX_SPEECH_INPUT_LENGTH constant ---

    #[test]
    fn test_max_speech_input_length_constant() {
        assert_eq!(MAX_SPEECH_INPUT_LENGTH, 4096);
    }

    // =======================================================================
    // Quality fixes
    // =======================================================================

    // --- as_str() tests ---

    #[test]
    fn test_audio_response_format_as_str() {
        assert_eq!(AudioResponseFormat::Json.as_str(), "json");
        assert_eq!(AudioResponseFormat::Text.as_str(), "text");
        assert_eq!(AudioResponseFormat::Srt.as_str(), "srt");
        assert_eq!(AudioResponseFormat::Vtt.as_str(), "vtt");
        assert_eq!(AudioResponseFormat::VerboseJson.as_str(), "verbose_json");
    }

    // --- TranslationResponse type alias ---

    #[test]
    fn test_translation_response_is_transcription_response() {
        let r = TranslationResponse {
            text: "Hello".into(),
        };
        assert_eq!(r.text, "Hello");
    }

    // --- Data field access ---

    #[test]
    fn test_transcription_request_data_field_accessible() {
        let req = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("a.wav")
            .data(vec![1u8, 2, 3])
            .build();
        assert_eq!(req.data, vec![1u8, 2, 3]);
    }

    // --- Optional params end-to-end tests ---

    #[tokio::test]
    async fn test_transcribe_with_optional_params_succeeds() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "text": "Bonjour le monde"
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("audio.wav")
            .data(vec![0u8; 100])
            .language("fr")
            .prompt("Context hint")
            .response_format(AudioResponseFormat::VerboseJson)
            .temperature(0.2)
            .build();

        let response = transcribe(&client, &request).await.expect("should succeed");
        assert_eq!(response.text, "Bonjour le monde");
    }

    #[tokio::test]
    async fn test_translate_with_optional_params_succeeds() {
        let server = MockServer::start().await;

        let response_body = serde_json::json!({
            "text": "Hello world"
        });

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/translations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranslationRequest::builder()
            .model("whisper-1")
            .filename("audio_fr.wav")
            .data(vec![0u8; 100])
            .prompt("Translate carefully")
            .response_format(AudioResponseFormat::Json)
            .temperature(0.0)
            .build();

        let response = translate(&client, &request).await.expect("should succeed");
        assert_eq!(response.text, "Hello world");
    }

    // --- Tracing span tests ---

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_transcribe_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/transcriptions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"text": "hello"})),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranscriptionRequest::builder()
            .model("whisper-1")
            .filename("a.wav")
            .data(vec![0u8; 10])
            .build();

        let _ = transcribe(&client, &request).await;

        assert!(logs_contain("foundry::audio::transcribe"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_translate_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/translations"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"text": "hello"})),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = TranslationRequest::builder()
            .model("whisper-1")
            .filename("b.wav")
            .data(vec![0u8; 10])
            .build();

        let _ = translate(&client, &request).await;

        assert!(logs_contain("foundry::audio::translate"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_speak_emits_tracing_span() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/v1/audio/speech"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"fake-mp3-data".to_vec())
                    .append_header("content-type", "audio/mpeg"),
            )
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = SpeechRequest::builder()
            .model("tts-1")
            .input("Hello")
            .voice("alloy")
            .build();

        let _ = speak(&client, &request).await;

        assert!(logs_contain("foundry::audio::speak"));
    }
}
