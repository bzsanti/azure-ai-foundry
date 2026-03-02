# TDD Plan: Quality Fixes for azure_ai_foundry_models v0.6.0

## Context

The v0.6.0 release of `azure_ai_foundry_models` introduced three new modules (`audio`, `images`,
`responses`) and extended the `embeddings` module. A code review identified 13 quality findings
ranging from latent panics in production paths to missing test coverage and missing type aliases.
This plan addresses all 13 findings in dependency order using strict TDD cycles.

**Stack detected**: Rust workspace, `tokio` async runtime, `serde`/`serde_json`, `reqwest`,
`wiremock` for mocking, `tracing-test` for span assertions, `thiserror` for error types.

**Conventions observed**:
- `as_str() -> &'static str` pattern for enum-to-form-field conversion (`file.rs:69`)
- `#[traced_test]` + `assert!(logs_contain("foundry::..."))` for tracing assertions (`embeddings.rs:831`)
- `// ---------------------------------------------------------------------------` section headers
- `FilePurpose::as_str()` is the canonical reference implementation for finding #1
- `Arc<Vec<u8>>` is used in `file.rs::upload()` but that usage is justified (closure called multiple
  times by the retry loop); in `audio.rs::transcribe()` and `images.rs::edit()` the closure is
  called exactly once, making the Arc allocation unnecessary

**Affected files**:
- `sdk/azure_ai_foundry_models/src/audio.rs`
- `sdk/azure_ai_foundry_models/src/images.rs`
- `sdk/azure_ai_foundry_models/src/responses.rs`

**Does this affect a hot path?**: No. These are HTTP API call wrappers. No throughput benchmarks
are required.

---

## Grouping of the 13 Findings

| # | Finding | Severity | Cycle |
|---|---------|----------|-------|
| 1 | Panics in production closures (`serde_json::to_value().expect().as_str().expect()`) | Critical | 1 |
| 2 | `translate()` returns `TranscriptionResponse` instead of `TranslationResponse` | Critical | 2 |
| 3 | Unnecessary `Arc<Vec<u8>>` clone in `transcribe()`, `translate()`, `edit()` | Critical | 3 |
| 12 | `Arc` import removable (resolved as part of #3) | Optional | 3 |
| 6 | `"output_text"` hardcoded without a named constant | Recommended | 4 |
| 9 | `ResponseMessage::content` has no multimodal limitation doc-comment | Recommended | 4 |
| 13 | `stream: Option<bool>` has no streaming-not-supported warning doc | Optional | 4 |
| 11 | `speak()` has no doc-comment noting limited retry on `response.bytes()` | Optional | 4 |
| 5 | `image_filename` empty string not validated in `ImageEditRequestBuilder::try_build()` | Recommended | 5 |
| 7 | Missing `test_edit_image_returns_error_on_400` test for `edit()` | Recommended | 6 |
| 10 | Missing tests for optional multipart fields in audio (`language`, `prompt`, `temperature`) | Optional | 6 |
| 4 | Missing `#[traced_test]` span emission tests in all 3 new modules | Recommended | 7 |
| 8 | Missing doc-comments on `EmbeddingRequest` fields (preexisting debt, OUT OF SCOPE) | - | - |

---

## Plan de Ejecucion

### Fase 1: as_str() implementations — eliminate panics in multipart closures

**Finding #1**

The panics live in `build_audio_form()` (`audio.rs:585-590`) and inside the `edit()` closure
(`images.rs:562-583`). The root cause is the pattern
`serde_json::to_value(x).expect(...).as_str().expect(...)` used to convert enum variants to their
API string representations inside closures. This can only fail due to a programmer error (a future
enum variant without a serde rename), but using `.expect()` in library code is still incorrect
practice. The fix is to implement `as_str() -> &'static str` on each affected enum, mirroring the
`FilePurpose::as_str()` pattern from `file.rs:65-76`.

Affected enums: `AudioResponseFormat` (`audio.rs:62`), `ImageSize` (`images.rs:55`),
`ImageQuality` (`images.rs:77`), `ImageResponseFormat` (`images.rs:96`).

#### Cycle 1.1 — RED: `AudioResponseFormat::as_str()`

- File: `sdk/azure_ai_foundry_models/src/audio.rs` (test section)
- Write test in the existing `tests` module:

```rust
#[test]
fn test_audio_response_format_as_str() {
    assert_eq!(AudioResponseFormat::Json.as_str(), "json");
    assert_eq!(AudioResponseFormat::Text.as_str(), "text");
    assert_eq!(AudioResponseFormat::Srt.as_str(), "srt");
    assert_eq!(AudioResponseFormat::Vtt.as_str(), "vtt");
    assert_eq!(AudioResponseFormat::VerboseJson.as_str(), "verbose_json");
}
```

- Run `cargo test`: test fails to compile (method does not exist).

#### Cycle 1.1 — GREEN: implement `AudioResponseFormat::as_str()`

- File: `sdk/azure_ai_foundry_models/src/audio.rs`
- Add after the enum definition (after line 73):

```rust
impl AudioResponseFormat {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json        => "json",
            Self::Text        => "text",
            Self::Srt         => "srt",
            Self::Vtt         => "vtt",
            Self::VerboseJson => "verbose_json",
        }
    }
}
```

- Run `cargo test`: test passes.

#### Cycle 1.1 — REFACTOR: replace panic pattern in `build_audio_form()`

- File: `sdk/azure_ai_foundry_models/src/audio.rs`, lines 584-590
- Replace:

```rust
    if let Some(fmt) = response_format {
        let fmt_str = serde_json::to_value(fmt)
            .expect("AudioResponseFormat should serialize")
            .as_str()
            .expect("AudioResponseFormat should serialize to string")
            .to_string();
        form = form.text("response_format", fmt_str);
    }
```

with:

```rust
    if let Some(fmt) = response_format {
        form = form.text("response_format", fmt.as_str());
    }
```

- Run `cargo test --workspace`: all tests pass, zero warnings.

---

#### Cycle 1.2 — RED: `ImageSize::as_str()`

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section)

```rust
#[test]
fn test_image_size_as_str() {
    assert_eq!(ImageSize::S256x256.as_str(), "256x256");
    assert_eq!(ImageSize::S512x512.as_str(), "512x512");
    assert_eq!(ImageSize::S1024x1024.as_str(), "1024x1024");
    assert_eq!(ImageSize::S1536x1024.as_str(), "1536x1024");
    assert_eq!(ImageSize::S1024x1536.as_str(), "1024x1536");
    assert_eq!(ImageSize::Auto.as_str(), "auto");
}
```

#### Cycle 1.2 — GREEN: implement `ImageSize::as_str()`

- File: `sdk/azure_ai_foundry_models/src/images.rs`, after the `ImageSize` enum (after line 74):

```rust
impl ImageSize {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::S256x256   => "256x256",
            Self::S512x512   => "512x512",
            Self::S1024x1024 => "1024x1024",
            Self::S1536x1024 => "1536x1024",
            Self::S1024x1536 => "1024x1536",
            Self::Auto       => "auto",
        }
    }
}
```

---

#### Cycle 1.3 — RED: `ImageQuality::as_str()`

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section)

```rust
#[test]
fn test_image_quality_as_str() {
    assert_eq!(ImageQuality::Standard.as_str(), "standard");
    assert_eq!(ImageQuality::Hd.as_str(), "hd");
    assert_eq!(ImageQuality::Low.as_str(), "low");
    assert_eq!(ImageQuality::Medium.as_str(), "medium");
    assert_eq!(ImageQuality::High.as_str(), "high");
    assert_eq!(ImageQuality::Auto.as_str(), "auto");
}
```

#### Cycle 1.3 — GREEN: implement `ImageQuality::as_str()`

```rust
impl ImageQuality {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Hd       => "hd",
            Self::Low      => "low",
            Self::Medium   => "medium",
            Self::High     => "high",
            Self::Auto     => "auto",
        }
    }
}
```

---

#### Cycle 1.4 — RED: `ImageResponseFormat::as_str()`

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section)

```rust
#[test]
fn test_image_response_format_as_str() {
    assert_eq!(ImageResponseFormat::Url.as_str(), "url");
    assert_eq!(ImageResponseFormat::B64Json.as_str(), "b64_json");
}
```

#### Cycle 1.4 — GREEN: implement `ImageResponseFormat::as_str()`

```rust
impl ImageResponseFormat {
    /// Return the string representation used by the API.
    ///
    /// This matches the serialized form used in multipart form fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Url     => "url",
            Self::B64Json => "b64_json",
        }
    }
}
```

#### Cycle 1.4 — REFACTOR: replace panic patterns in `edit()` closure

- File: `sdk/azure_ai_foundry_models/src/images.rs`, lines 562-583
- Replace all three `serde_json::to_value(...).expect(...).as_str().expect(...)` blocks:

```rust
            if let Some(size) = size {
                form = form.text("size", size.as_str());
            }
            if let Some(quality) = quality {
                form = form.text("quality", quality.as_str());
            }
            if let Some(fmt) = response_format {
                form = form.text("response_format", fmt.as_str());
            }
```

- Run `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: all pass.

**Consistency check**: Also verify that `as_str()` return values match `serde_json::to_string()`
for each variant. The existing serialization tests (`test_image_size_serialization`, etc.) already
cover this: the serde rename attributes and the `as_str()` strings must be identical. No additional
tests needed.

**Estimacion**: 30 min implementation + 10 min verification.

---

### Fase 2: TranslationResponse type alias

**Finding #2**

`translate()` in `audio.rs:697` returns `FoundryResult<TranscriptionResponse>`. The API
distinction matters for callers: a translation always produces English text, and callers should be
able to store the return type using a semantically correct name.

#### Cycle 2.1 — RED: type alias existence

- File: `sdk/azure_ai_foundry_models/src/audio.rs` (test section)

```rust
#[test]
fn test_translation_response_is_transcription_response() {
    // TranslationResponse must be a type alias for TranscriptionResponse.
    // This test asserts structural identity by constructing one and using it as the other.
    let r = TranslationResponse {
        task: None,
        language: None,
        duration: None,
        text: "Hello".into(),
        segments: None,
        words: None,
    };
    let text: &str = &r.text;
    assert_eq!(text, "Hello");
}
```

#### Cycle 2.1 — GREEN: add type alias and update `translate()` signature

- File: `sdk/azure_ai_foundry_models/src/audio.rs`
- After the `TranscriptionResponse` struct definition, add:

```rust
/// Response from an audio translation request.
///
/// Translation always produces English text regardless of the input language.
/// Structurally identical to [`TranscriptionResponse`]; defined as a distinct
/// type alias for semantic clarity at call sites.
pub type TranslationResponse = TranscriptionResponse;
```

- Update the `translate()` function signature from:

```rust
) -> FoundryResult<TranscriptionResponse> {
```

to:

```rust
) -> FoundryResult<TranslationResponse> {
```

- Update the doc-comment example in `translate()` to use `TranslationResponse` if it references the
  return type explicitly (currently it does not, so no change needed there).
- Run `cargo test --workspace && cargo doc --workspace --no-deps`: both pass.

**Estimacion**: 10 min.

---

### Fase 3: Remove unnecessary Arc allocations

**Finding #3 and #12**

In `audio.rs::transcribe()` (lines 637-638) and `audio.rs::translate()` (lines 700-701), and in
`images.rs::edit()` (line 532), the pattern `Arc::new(request.data.clone())` is used before the
`post_multipart` closure. The `Arc` is unnecessary because `post_multipart` calls the closure
exactly once (it is a retry helper but the closure itself only borrows for one invocation at a
time). The minimum correct fix is to remove the intermediate `Arc` and clone the `Vec<u8>`
directly inside the closure, eliminating one allocation per call.

As a consequence, the `use std::sync::Arc;` import in `audio.rs` (line 46) and `images.rs`
(line 47) becomes unused and must be removed (finding #12).

Note: `file.rs::upload()` intentionally keeps `Arc` because its closure is called multiple times
by the retry loop in `FoundryClient::post_multipart`. The difference is that `audio.rs` and
`images.rs` do not use retry — the closure captures by move and runs once.

#### Cycle 3.1 — RED: no new behavioral test needed

The existing integration-style tests (`test_transcribe_audio_success`, `test_translate_audio_success`,
`test_edit_image_success`) already cover the observable behaviour. The correctness of removing `Arc`
is verified by the compiler: if the code compiles and existing tests pass, the refactoring is correct.

Write one unit test that documents intent:

- File: `sdk/azure_ai_foundry_models/src/audio.rs` (test section)

```rust
#[test]
fn test_transcription_request_data_is_cloneable_without_arc() {
    // Verify that TranscriptionRequest data can be cloned directly,
    // confirming no Arc wrapper is needed at the call site.
    let req = TranscriptionRequest::builder()
        .model("whisper-1")
        .filename("a.wav")
        .data(vec![1u8, 2, 3])
        .build();
    let cloned: Vec<u8> = req.data.clone();
    assert_eq!(cloned, vec![1u8, 2, 3]);
}
```

#### Cycle 3.1 — GREEN: refactor `transcribe()` in `audio.rs`

- File: `sdk/azure_ai_foundry_models/src/audio.rs`, lines 637-638
- Replace:

```rust
    let data = Arc::new(request.data.clone());
    let filename = request.filename.clone();
```

with:

```rust
    let data = request.data.clone();
    let filename = request.filename.clone();
```

- Inside the closure, change `build_audio_form(&data, ...)` — the function currently takes
  `&Arc<Vec<u8>>`. Update `build_audio_form`'s signature to accept `&[u8]` instead:

```rust
fn build_audio_form(
    data: &[u8],
    filename: &str,
    ...
```

- Inside `build_audio_form`, change:

```rust
    let file_part =
        reqwest::multipart::Part::bytes((**data).clone()).file_name(filename.to_string());
```

to:

```rust
    let file_part =
        reqwest::multipart::Part::bytes(data.to_vec()).file_name(filename.to_string());
```

- Update `transcribe()` closure call to pass `&data` (now `&Vec<u8>` which coerces to `&[u8]`):

```rust
        .post_multipart("/openai/v1/audio/transcriptions", move || {
            build_audio_form(
                &data,
                ...
```

No change needed here — `&data` already worked; now it's `&[u8]` coercion instead of
deref-of-Arc.

#### Cycle 3.2 — GREEN: refactor `translate()` in `audio.rs`

Same pattern as `transcribe()`. Replace `Arc::new(request.data.clone())` with
`request.data.clone()` at line 700. The call to `build_audio_form` already passes `&data`.

#### Cycle 3.3 — GREEN: refactor `edit()` in `images.rs`

- File: `sdk/azure_ai_foundry_models/src/images.rs`, line 532-536
- Replace:

```rust
    let image_data = Arc::new(request.image.clone());
    ...
    let mask_data = request.mask.as_ref().map(|m| Arc::new(m.clone()));
```

with:

```rust
    let image_data = request.image.clone();
    ...
    let mask_data = request.mask.clone();
```

- Inside the closure, change:

```rust
            let image_part = reqwest::multipart::Part::bytes((*image_data).clone())
```

to:

```rust
            let image_part = reqwest::multipart::Part::bytes(image_data.clone())
```

- For the mask part, change:

```rust
            if let Some(ref mask) = mask_data {
                let mask_part = reqwest::multipart::Part::bytes((**mask).clone())
```

to:

```rust
            if let Some(ref mask) = mask_data {
                let mask_part = reqwest::multipart::Part::bytes(mask.clone())
```

#### Cycle 3.4 — REFACTOR: remove Arc imports

- File: `sdk/azure_ai_foundry_models/src/audio.rs`, line 46
  - Remove: `use std::sync::Arc;`
- File: `sdk/azure_ai_foundry_models/src/images.rs`, line 47
  - Remove: `use std::sync::Arc;`

- Run `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings.
- Run `cargo test --workspace`: all tests pass.

**Estimacion**: 30 min.

---

### Fase 4: Documentation-only fixes (doc-comments and constants)

**Findings #6, #9, #11, #13** — no test cycles required (pure documentation), but each change
must not break compilation or doc tests.

#### Cycle 4.1 — Named constant for `"output_text"` (Finding #6)

- File: `sdk/azure_ai_foundry_models/src/responses.rs`
- Add a private constant in the constants section (before the `ResponseInput` type, consistent
  with `audio.rs` `MAX_SPEECH_INPUT_LENGTH` pattern):

```rust
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Content type string used to identify text output blocks in a Response.
const OUTPUT_TEXT_TYPE: &str = "output_text";
```

- Update `Response::output_text()` method (line 377):

Replace:

```rust
                    if c.content_type == "output_text" {
```

with:

```rust
                    if c.content_type == OUTPUT_TEXT_TYPE {
```

- Write a test confirming the constant value (ensures the constant is not accidentally changed):

```rust
#[test]
fn test_output_text_type_constant() {
    assert_eq!(OUTPUT_TEXT_TYPE, "output_text");
}
```

**Note**: `OUTPUT_TEXT_TYPE` is `const` (not `pub const`) because it is an implementation detail.
No public API surface change.

#### Cycle 4.2 — Multimodal limitation doc-comment on `ResponseMessage::content` (Finding #9)

- File: `sdk/azure_ai_foundry_models/src/responses.rs`, struct `ResponseMessage`, field `content`
  (line 89)
- Change:

```rust
    /// The content of the message.
    pub content: String,
```

to:

```rust
    /// The text content of the message.
    ///
    /// # Limitation
    ///
    /// This field is currently a plain `String` and does not support multimodal content
    /// (e.g., image URLs, tool call results). Full multimodal input support is planned
    /// for a future release (v0.7.0).
    pub content: String,
```

#### Cycle 4.3 — Streaming not supported warning doc on `stream` field (Finding #13)

- File: `sdk/azure_ai_foundry_models/src/responses.rs`, struct `CreateResponseRequest`, field
  `stream` (line 153-155)
- Change:

```rust
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
```

to:

```rust
    /// Whether to stream the response.
    ///
    /// # Warning
    ///
    /// Streaming is **not supported** by [`create()`]. Setting this field to `true`
    /// will result in a server-side error or an incomplete response. This field is
    /// preserved in the API surface for future streaming support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
```

#### Cycle 4.4 — `speak()` retry limitation doc-comment (Finding #11)

- File: `sdk/azure_ai_foundry_models/src/audio.rs`
- In the `speak()` function doc-comment (after the `# Tracing` section), add:

```
/// # Limitations
///
/// The `response.bytes()` call that reads the audio body is **not retried**
/// if the connection drops mid-stream. On transient network errors after the
/// HTTP 200 response headers are received, the caller must retry the full
/// `speak()` call.
```

- Run `cargo doc --workspace --no-deps`: builds without errors.
- Run `cargo test --workspace`: all tests still pass.

**Estimacion**: 20 min for all four doc changes.

---

### Fase 5: image_filename empty string validation

**Finding #5**

`ImageEditRequestBuilder::try_build()` validates that `image_filename` is present (it returns an
error if the `Option` is `None`) but does not validate that the string is non-empty. An empty
filename would produce a malformed multipart request.

#### Cycle 5.1 — RED: test that empty filename is rejected

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section)

```rust
#[test]
fn test_image_edit_request_rejects_empty_image_filename() {
    let result = ImageEditRequest::builder()
        .model("dall-e-2")
        .image(vec![1, 2, 3], "")       // empty filename
        .prompt("Edit this image")
        .try_build();

    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("image filename cannot be empty"),
        "error should mention empty filename"
    );
}
```

- Run `cargo test test_image_edit_request_rejects_empty_image_filename`: fails (no validation yet).

#### Cycle 5.1 — GREEN: add empty-filename guard in `try_build()`

- File: `sdk/azure_ai_foundry_models/src/images.rs`, `ImageEditRequestBuilder::try_build()`,
  after line 392 where `image_filename` is unwrapped:

```rust
        let image_filename = self
            .image_filename
            .ok_or_else(|| FoundryError::Builder("image filename is required".into()))?;
        if image_filename.is_empty() {
            return Err(FoundryError::Builder("image filename cannot be empty".into()));
        }
```

- Run `cargo test`: new test passes, existing tests unaffected.

**Estimacion**: 15 min.

---

### Fase 6: Missing error-path and optional-param tests

**Finding #7 — `test_edit_image_returns_error_on_400`**

#### Cycle 6.1 — RED: write the missing error test for `edit()`

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section, after `test_edit_image_success`)

```rust
#[tokio::test]
async fn test_edit_image_returns_error_on_400() {
    let server = MockServer::start().await;

    let error_response = serde_json::json!({
        "error": {
            "code": "InvalidRequest",
            "message": "Image format not supported"
        }
    });

    Mock::given(method("POST"))
        .and(path("/openai/v1/images/edits"))
        .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let request = ImageEditRequest::builder()
        .model("dall-e-2")
        .image(vec![0u8; 100], "input.png")
        .prompt("Add a rainbow")
        .build();

    let result = edit(&client, &request).await;

    assert!(result.is_err());
}
```

- Run `cargo test test_edit_image_returns_error_on_400`: the test must fail at this point only if
  `edit()` somehow returns `Ok` on 400 — in practice it should already fail. The value of writing
  this test is to lock in the behaviour and prevent future regressions. Confirm it passes.

**Finding #10 — optional multipart fields in audio**

#### Cycle 6.2 — RED: tests for optional fields in transcription form

- File: `sdk/azure_ai_foundry_models/src/audio.rs` (test section)

The `wiremock` form-body matcher is not straightforward for multipart. Use a custom wiremock
matcher or verify via the request log. The recommended approach for this codebase (matching the
pattern in `test_upload_file_success`) is to use `wiremock::matchers::method` and `path` only
and verify the response is processed correctly. Actual form field verification requires a custom
matcher. Given that complexity, the test should verify that the optional parameters are accepted
by the builder and the function runs successfully end-to-end when a mock server is configured.

```rust
#[tokio::test]
async fn test_transcribe_with_optional_params_succeeds() {
    let server = MockServer::start().await;

    let response_body = serde_json::json!({
        "text": "Bonjour le monde",
        "task": "transcribe",
        "language": "fr",
        "duration": 2.5
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

    // All optional fields are set; verify the request reaches the server and
    // the response is correctly parsed.
    let response = transcribe(&client, &request).await.expect("should succeed");
    assert_eq!(response.text, "Bonjour le monde");
    assert_eq!(response.language, Some("fr".into()));
}

#[tokio::test]
async fn test_translate_with_optional_params_succeeds() {
    let server = MockServer::start().await;

    let response_body = serde_json::json!({
        "text": "Hello world",
        "task": "translate"
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
```

**Estimacion**: 25 min.

---

### Fase 7: Tracing span tests for the three new modules

**Finding #4**

The pattern is established in `embeddings.rs:831-868`: import `tracing_test::traced_test`,
annotate the test `#[tokio::test] #[traced_test]`, call the function, then assert
`logs_contain("foundry::module::function_name")`.

Each test requires a `MockServer` to avoid actual HTTP calls. The test only asserts that a span
is emitted, not its field values (that level of detail is not tested even in the embeddings
reference).

Verify that `tracing-test` is already in `azure_ai_foundry_models/Cargo.toml` as a dev
dependency before implementing. If it is missing, add it.

#### Cycle 7.1 — RED: tracing test for `audio::transcribe`

- File: `sdk/azure_ai_foundry_models/src/audio.rs` (test section)

```rust
#[tokio::test]
#[traced_test]
async fn test_transcribe_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello"
        })))
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
```

#### Cycle 7.2 — RED: tracing test for `audio::translate`

```rust
#[tokio::test]
#[traced_test]
async fn test_translate_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/translations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello"
        })))
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
```

#### Cycle 7.3 — RED: tracing test for `audio::speak`

```rust
#[tokio::test]
#[traced_test]
async fn test_speak_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
```

#### Cycle 7.4 — RED: tracing test for `images::generate`

- File: `sdk/azure_ai_foundry_models/src/images.rs` (test section)
- Add `use tracing_test::traced_test;` to the test module imports.

```rust
#[tokio::test]
#[traced_test]
async fn test_generate_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{"url": "https://example.com/img.png"}]
        })))
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let request = ImageGenerationRequest::builder()
        .model("dall-e-3")
        .prompt("A mountain")
        .build();

    let _ = generate(&client, &request).await;

    assert!(logs_contain("foundry::images::generate"));
}
```

#### Cycle 7.5 — RED: tracing test for `images::edit`

```rust
#[tokio::test]
#[traced_test]
async fn test_edit_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/v1/images/edits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": TEST_TIMESTAMP,
            "data": [{"url": "https://example.com/edited.png"}]
        })))
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let request = ImageEditRequest::builder()
        .model("dall-e-2")
        .image(vec![0u8; 10], "img.png")
        .prompt("Add a hat")
        .build();

    let _ = edit(&client, &request).await;

    assert!(logs_contain("foundry::images::edit"));
}
```

#### Cycle 7.6 — RED: tracing tests for `responses::create`, `::get`, `::delete`

- File: `sdk/azure_ai_foundry_models/src/responses.rs` (test section)
- Add `use tracing_test::traced_test;` to the test module imports.

```rust
fn sample_response_json_for_tracing() -> serde_json::Value {
    serde_json::json!({
        "id": "resp_trace",
        "object": "response",
        "created_at": 1700000000.0,
        "status": "completed",
        "model": "gpt-4o",
        "output": []
    })
}

#[tokio::test]
#[traced_test]
async fn test_create_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/openai/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(&sample_response_json_for_tracing()),
        )
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let request = CreateResponseRequest::builder()
        .model("gpt-4o")
        .input("Hello")
        .build();

    let _ = create(&client, &request).await;

    assert!(logs_contain("foundry::responses::create"));
}

#[tokio::test]
#[traced_test]
async fn test_get_response_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/openai/v1/responses/resp_trace"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(&sample_response_json_for_tracing()),
        )
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let _ = get(&client, "resp_trace").await;

    assert!(logs_contain("foundry::responses::get"));
}

#[tokio::test]
#[traced_test]
async fn test_delete_response_emits_tracing_span() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/openai/v1/responses/resp_trace"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "resp_trace",
            "object": "response.deleted",
            "deleted": true
        })))
        .mount(&server)
        .await;

    let client = setup_mock_client(&server).await;

    let _ = delete(&client, "resp_trace").await;

    assert!(logs_contain("foundry::responses::delete"));
}
```

#### Cycle 7.x — GREEN for all tracing tests

All functions already have `#[tracing::instrument]` annotations. The tests will pass once
`tracing-test` is confirmed as a dev dependency. No production code changes required.

- Verify `tracing-test` is in `sdk/azure_ai_foundry_models/Cargo.toml` under `[dev-dependencies]`.
- Add `use tracing_test::traced_test;` at the top of each test module that lacks it (check
  `audio.rs` and `images.rs` test sections; `responses.rs` test section).

**Estimacion**: 30 min (writing tests) + 5 min (dependency check).

---

## Dependency Graph

```
Fase 1 (as_str impls)
  |
  +-- Fase 3 (remove Arc) — depends on Fase 1 completing the as_str refactor in edit()
  |
  +-- Fase 2 (TranslationResponse) — independent of Fase 1
  |
  +-- Fase 4 (doc-comments) — independent of everything
  |
  +-- Fase 5 (filename validation) — independent
  |
  +-- Fase 6 (error/optional tests) — can run after Fase 1 and 3 pass (uses edit())
  |
  +-- Fase 7 (tracing tests) — fully independent, can run in any order
```

Recommended execution order: **1 → 3 → 2 → 5 → 6 → 4 → 7**

The critical fixes (1, 2, 3) must be done first. Fases 4, 5, 6, 7 are independent of each other
once the critical fixes are in place.

---

## Estimacion Total

| Fase | Descripcion | Estimacion |
|------|-------------|------------|
| 1 | as_str() for 4 enums + refactor of panic patterns | 40 min |
| 2 | TranslationResponse type alias | 10 min |
| 3 | Remove Arc allocations + import cleanup | 30 min |
| 4 | Doc-comment additions (4 changes) | 20 min |
| 5 | image_filename empty validation | 15 min |
| 6 | 3 missing tests (edit error, audio optional params) | 25 min |
| 7 | 8 tracing span tests across 3 modules | 35 min |
| **Total** | | **~3 h** |

---

## Criterios de Exito

- [ ] `cargo test --workspace` passes with zero failures
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` reports zero warnings
- [ ] `cargo doc --workspace --no-deps` builds without errors
- [ ] `cargo fmt --all -- --check` reports no formatting changes needed
- [ ] No `.expect()` calls remain inside `post_multipart` closures in `audio.rs` or `images.rs`
- [ ] `translate()` return type is `FoundryResult<TranslationResponse>`
- [ ] `use std::sync::Arc;` is removed from `audio.rs` and `images.rs`
- [ ] `ImageEditRequestBuilder::try_build()` rejects empty `image_filename`
- [ ] `test_edit_image_returns_error_on_400` exists and passes
- [ ] At least one `#[traced_test]` test per public API function in `audio`, `images`, `responses`
- [ ] `OUTPUT_TEXT_TYPE` constant is used in `Response::output_text()` instead of a string literal
