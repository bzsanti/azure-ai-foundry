# TDD Plan: v0.7.0 Quality Fixes — Azure AI Foundry Rust SDK

## Context

The v0.7.0 release is a breaking-version cycle. A quality review identified 16 findings
(2 critical, 7 recommended, 7 optional) across all four crates. After deduplication (R5=C2,
R6=R2, R7=R3) and skipping O7 (proc macro), there are 12 unique items to address.

This plan structures those 12 items into 6 milestones ordered by dependency. Each milestone
ends in a fully compilable, all-tests-green state.

**Stack detected**: Rust workspace, MSRV 1.88, `tokio` async, `serde`/`serde_json`, `reqwest`,
`wiremock`, `tracing-test`, `thiserror`.

**Conventions observed**:
- `pub(crate) mod test_utils` with `setup_mock_client` and `TEST_API_KEY` in each crate's `lib.rs`
- Builder pattern: `try_build() -> FoundryResult<T>` + `build(self) -> T { self.try_build().expect(...) }`
- `#[tracing::instrument]` on all public async API functions
- `// ---` section separator lines
- TDD cycles written before production code
- Tests inline in source files under `#[cfg(test)] mod tests { ... }`

**Does this affect a hot path?**: No. All changes are in HTTP API wrappers, type definitions,
and doc comments. No throughput benchmarks required.

**Affected files**:
- `sdk/azure_ai_foundry_core/src/client.rs` — C1, C2, O1
- `sdk/azure_ai_foundry_core/Cargo.toml` — C2
- `sdk/azure_ai_foundry_models/src/responses.rs` — R1, O2
- `sdk/azure_ai_foundry_models/src/audio.rs` — R4
- `sdk/azure_ai_foundry_agents/src/run.rs` — R2
- `sdk/azure_ai_foundry_agents/src/vector_store.rs` — O4
- `sdk/azure_ai_foundry_tools/src/vision.rs` — O5
- `sdk/azure_ai_foundry_core/src/lib.rs` (or new file) — O3
- `sdk/azure_ai_foundry_agents/src/models.rs` — R3 (doc only)
- `sdk/azure_ai_foundry_core/src/auth.rs` — O6
- `sdk/azure_ai_foundry_agents/src/run.rs` — O6

---

## Findings Reference Table

| ID | Severity | Description | Milestone |
|----|----------|-------------|-----------|
| C1 | Critical | UTF-8 boundary panic in `sanitize_error_message` | 1 |
| C2 | Critical | `build()` methods undocumented panic, `missing_panics_doc` suppressed globally | 2 |
| R1 | Recommended | `created_at: f64` in `Response` vs `u64` elsewhere | 3 |
| R2/R6 | Recommended | `tool_outputs.to_vec()` unnecessary clone in `SubmitToolOutputsRequest` | 4 |
| R3/R7 | Recommended | Agents API version hardcoded — document the limitation | 5 |
| R4 | Recommended | `build_audio_form` allocates `.to_string()` on every call | 4 |
| O1 | Optional | Expose `is_retriable_status` from error type as `FoundryError::is_retryable()` | 3 |
| O2 | Optional | `stream` field on `CreateResponseRequest` is a footgun — remove it | 3 |
| O3 | Optional | Test utility code duplicated across 3 crates | 6 |
| O4 | Optional | `ExpiresAfter::anchor` should be an enum | 4 |
| O5 | Optional | Validate `features` not empty in `ImageAnalysisRequest::try_build()` | 5 |
| O6 | Optional | Add removal-version notices to deprecated items | 5 |

---

## Milestone 1: Fix UTF-8 boundary panic in `sanitize_error_message`

**Finding C1.** `redact_header_value_case_insensitive` computes byte-level indices on
`lower_result` (the lowercase version) but then uses those same indices to slice and
`replace_range` on `result` (the original). If `result` contains multi-byte characters before
the match position, the byte offsets diverge and a panic on a UTF-8 boundary occurs.

The correct fix: perform all searching on `lower_result`, but skip-whitespace and value-end
scanning must also operate on `lower_result` (not on `result`), since the byte positions must
stay consistent. The replacement is still applied to `result` using the computed ranges, which
is safe because the ranges were computed over `lower_result` whose byte length equals `result`'s
byte length only when both have the same codepoint structure — which is not guaranteed for
characters like `É` (1 byte in original, 2 bytes in lowercase `é`... wait, `.to_lowercase()` in
Rust preserves byte length for ASCII but may change it for multi-byte codepoints).

The robust fix: work entirely on `lower_result` — find ranges there, then apply those same
ranges back to `lower_result` (not `result`), and return the sanitized lowercase string.
Alternatively, refactor to avoid the dual-string approach by using a regex or by iterating
char indices. The simplest correct approach is: sanitize and return `lower_result` (the already
computed lowercase), since callers of `sanitize_error_message` use it for error reporting, not
for preserving the original case.

### Cycle 1.1: Reproduce the panic with a multi-byte input

- RED: Write test `sanitize_error_message_with_multibyte_before_api_key` that verifies the
  function does not panic and correctly redacts when multi-byte characters precede the header.
  Assert: the returned string contains `[REDACTED]` and does not contain the secret value.

  File: `sdk/azure_ai_foundry_core/src/client.rs`, test section (inline `mod tests`)

  ```rust
  #[test]
  fn sanitize_error_message_with_multibyte_before_api_key() {
      // "café" contains a 2-byte UTF-8 codepoint (é = 0xC3 0xA9).
      // Without the fix this panics on a UTF-8 boundary when applying
      // replace_range with indices computed on the lowercase copy.
      let msg = "café api-key: supersecret123 failed";
      let result = FoundryClient::sanitize_error_message(msg);
      assert!(
          !result.contains("supersecret123"),
          "secret should be redacted, got: {result}"
      );
      assert!(
          result.contains("[REDACTED]"),
          "should contain redaction marker, got: {result}"
      );
  }

  #[test]
  fn sanitize_error_message_with_emoji_before_api_key() {
      // Emoji are 4-byte UTF-8 codepoints.
      let msg = "error 🚀 api-key: topsecret456 endpoint";
      let result = FoundryClient::sanitize_error_message(msg);
      assert!(
          !result.contains("topsecret456"),
          "secret should be redacted, got: {result}"
      );
  }

  #[test]
  fn sanitize_error_message_multibyte_in_key_value_itself() {
      // Multi-byte characters in the value region must not cause boundary issues.
      let msg = "api-key: sécrêt123 was rejected";
      let result = FoundryClient::sanitize_error_message(msg);
      assert!(
          !result.contains("sécrêt123"),
          "multi-byte secret should be redacted, got: {result}"
      );
  }
  ```

  Run `cargo test --package azure_ai_foundry_core`: at least the first two tests panic (or
  produce wrong output), confirming the bug.

- GREEN: Refactor `redact_header_value_case_insensitive` so all index computation AND the final
  replacement both operate on `lower_result`. Change the function to mutate `lower_result`
  internally and then overwrite `result` at the end:

  File: `sdk/azure_ai_foundry_core/src/client.rs`

  Replace the entire `redact_header_value_case_insensitive` function body with an
  implementation that:
  1. Clones `result` as `lower = result.to_lowercase()`.
  2. Finds all value ranges in `lower` (same logic as today).
  3. Applies `replace_range` on `lower` in reverse order.
  4. Replaces `*result = lower` at the end.

  This means `sanitize_error_message` now returns a lowercased, redacted string. The doc
  comment must be updated to state: "Returns a lowercase, sanitized copy of the input."
  Because this is `pub(crate)`, there is no public API breakage.

  ```rust
  fn redact_header_value_case_insensitive(result: &mut String, header_lower: &str) {
      let redacted = "[REDACTED]";
      let mut lower = result.to_lowercase();
      let mut ranges: Vec<(usize, usize)> = Vec::new();
      let mut search_start = 0;

      while search_start < lower.len() {
          if let Some(relative_pos) = lower[search_start..].find(header_lower) {
              let key_pos = search_start + relative_pos + header_lower.len();

              let value_start = lower[key_pos..]
                  .find(|c: char| !c.is_whitespace())
                  .map(|pos| key_pos + pos)
                  .unwrap_or(lower.len());

              if value_start >= lower.len() {
                  break;
              }

              let value_end = lower[value_start..]
                  .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                  .map(|pos| value_start + pos)
                  .unwrap_or(lower.len());

              if value_end > value_start {
                  ranges.push((value_start, value_end));
                  search_start = value_end;
              } else {
                  search_start = value_start + 1;
              }
          } else {
              break;
          }
      }

      for (start, end) in ranges.into_iter().rev() {
          lower.replace_range(start..end, redacted);
      }

      *result = lower;
  }
  ```

  Run `cargo test --package azure_ai_foundry_core`: all three new tests pass, all existing
  sanitization tests pass.

- REFACTOR: Update the doc-comment on `sanitize_error_message` to document that the return
  value is lowercase:

  ```rust
  /// Sanitize error messages by removing sensitive data like tokens and API keys.
  ///
  /// Returns a **lowercase** copy of `msg` with credential-like patterns replaced
  /// by `[REDACTED]`. Lowercasing is a side-effect of the case-insensitive header
  /// scanning and is intentional: callers use this for error reporting only.
  ```

  Run `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`:
  zero failures, zero warnings.

---

## Milestone 2: Document panics on all `build()` methods, remove lint suppression

**Finding C2 (= R5).** 19 `build()` methods across 4 crates call
`self.try_build().expect("builder validation failed")` without a `# Panics` doc section.
The workspace `Cargo.toml` suppresses the `missing_panics_doc` Clippy lint globally
(`missing_panics_doc = "allow"` at line 29). Removing that suppression will cause Clippy to
enforce the documentation requirement on all affected `build()` methods.

The affected builders (confirmed by grep):

**azure_ai_foundry_models** (6):
- `TranscriptionRequestBuilder::build()` — `audio.rs`
- `TranslationRequestBuilder::build()` — `audio.rs`
- `SpeechRequestBuilder::build()` — `audio.rs`
- `ChatCompletionRequestBuilder::build()` — `chat.rs`
- `EmbeddingRequestBuilder::build()` — `embeddings.rs`
- `ImageGenerationRequestBuilder::build()` — `images.rs`
- `ImageEditRequestBuilder::build()` — `images.rs`
- `CreateResponseRequestBuilder::build()` — `responses.rs`

**azure_ai_foundry_agents** (5):
- `AgentCreateRequestBuilder::build()` — `agent.rs`
- `AgentUpdateRequestBuilder::build()` — `agent.rs`
- `MessageCreateRequestBuilder::build()` — `message.rs`
- `RunCreateRequestBuilder::build()` — `run.rs`
- `CreateThreadAndRunRequestBuilder::build()` — `run.rs`

**azure_ai_foundry_tools** (2):
- `ImageAnalysisRequestBuilder::build()` — `vision.rs`
- `DocumentAnalysisRequestBuilder::build()` — `document_intelligence.rs`

**azure_ai_foundry_core**: `FoundryClientBuilder::build()` returns `FoundryResult<T>` and does
not call `expect()`, so it is not subject to `missing_panics_doc`.

Builders with infallible `build()` (those that do NOT call `try_build().expect()`):
`VectorStoreCreateRequestBuilder`, `VectorStoreUpdateRequestBuilder`,
`MessageUpdateRequestBuilder`, `ThreadUpdateRequestBuilder` — these return the struct directly
and never panic, so no `# Panics` section is needed for them.

### Cycle 2.1: Enable the lint and verify it fails

- RED: Remove the `missing_panics_doc = "allow"` line from
  `sdk/../Cargo.toml` (workspace `[workspace.lints.clippy]` section, line 29).

  Run `cargo clippy --workspace --all-targets -- -D warnings`: Clippy reports
  `missing_panics_doc` on all the builders listed above. This is the expected RED state.

- GREEN: Add a `# Panics` doc section to every `build()` that calls `try_build().expect()`.
  The canonical form (adapt the trigger condition per builder):

  ```rust
  /// Build the request.
  ///
  /// # Panics
  ///
  /// Panics if required fields are missing. Use [`try_build`](Self::try_build)
  /// for fallible construction that returns a `FoundryResult`.
  pub fn build(self) -> TranscriptionRequest {
      self.try_build().expect("builder validation failed")
  }
  ```

  The condition to name in `# Panics` is builder-specific:
  - Audio builders: "if `model`, `filename`, or `data` is not set"
  - Chat: "if `model` is not set"
  - Embeddings: "if `model` or `input` is not set"
  - Images generation: "if `model` or `prompt` is not set"
  - Images edit: "if `model`, `image`, or `prompt` is not set"
  - Responses: "if `model` or `input` is not set"
  - Agent create: "if `model` is not set"
  - Agent update: "if parameter values are out of range"
  - Message create: "if `role` or `content` is not set"
  - Run create: "if `assistant_id` is not set"
  - Thread+Run: "if `assistant_id` is not set"
  - Vision: "if `image_url` or `features` is not set"
  - Document Intelligence: "if `model_id` or source is not set"

  Apply to all 15 `build()` methods. No production logic changes — doc comments only.

  Run `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings.

- REFACTOR: No structural refactoring needed. Run full test suite to confirm nothing broke:
  `cargo test --workspace`.

### Cycle 2.2: Verify doc-test compilation

- Run `cargo doc --workspace --no-deps`: docs build without errors or warnings.
  This confirms the `# Panics` sections are valid rustdoc syntax.

---

## Milestone 3: Fix `created_at: f64`, remove `stream` footgun, add `is_retryable()`

This milestone groups three independent changes that all touch API surface (breaking changes
acceptable in v0.7.0) and can be done in sequence within one compilable state.

### Cycle 3.1: `Response::created_at` — verify API type and change to `u64`

**Finding R1.** `Response::created_at` is `f64` while all other timestamp fields across the
codebase use `u64` (e.g. `VectorStore::created_at`, `Run::created_at`, `Agent::created_at`).
The OpenAI Responses API returns `created_at` as an integer Unix timestamp. Using `f64` is
inconsistent and loses precision for timestamps beyond 2^53.

- RED: Write test `response_created_at_deserializes_as_integer` that confirms an integer JSON
  value (no decimal point) round-trips correctly:

  File: `sdk/azure_ai_foundry_models/src/responses.rs`, test section

  ```rust
  #[test]
  fn response_created_at_is_u64() {
      // Verify that created_at is u64, not f64.
      // An integer timestamp must deserialize without precision loss.
      let json = serde_json::json!({
          "id": "resp_ts",
          "object": "response",
          "created_at": 1700000001u64,
          "status": "completed",
          "model": "gpt-4o",
          "output": []
      });
      let response: Response = serde_json::from_value(json).unwrap();
      assert_eq!(response.created_at, 1_700_000_001u64);
  }
  ```

  Run `cargo test`: fails to compile because `created_at` is `f64` and the assertion type is
  `u64`.

- GREEN: Change the field type in `Response`:

  File: `sdk/azure_ai_foundry_models/src/responses.rs`, line 381

  ```rust
  // Before:
  pub created_at: f64,

  // After:
  pub created_at: u64,
  ```

  Update the two existing tests that use `1700000000.0`:

  In `test_response_deserialization` (around line 820): change `"created_at": 1700000000.0`
  to `"created_at": 1700000000` and `assert!((response.created_at - 1_700_000_000.0).abs() ...`
  to `assert_eq!(response.created_at, 1_700_000_000u64)`.

  Run `cargo test --package azure_ai_foundry_models`: all tests pass.

- REFACTOR: No further changes needed. Run `cargo clippy --workspace --all-targets -- -D warnings`.

### Cycle 3.2: Remove `stream` field from `CreateResponseRequest`

**Finding O2.** Setting `stream: true` causes a server-side error because streaming is not
implemented. The field is a footgun: it is public, settable via the builder, and silently causes
failures. Remove both the field and the builder method. This is a breaking change; acceptable
in v0.7.0.

- RED: Write test `create_response_request_has_no_stream_field` that fails to compile if the
  `stream` field still exists:

  File: `sdk/azure_ai_foundry_models/src/responses.rs`, test section

  ```rust
  #[test]
  fn create_response_request_builder_has_no_stream_method() {
      // This test documents that the stream footgun was removed.
      // It verifies a request can be built with all legitimate fields
      // and that the serialized form does not contain a "stream" key.
      let request = CreateResponseRequest::builder()
          .model("gpt-4o")
          .input("Hello")
          .temperature(0.7)
          .build();
      let json = serde_json::to_value(&request).unwrap();
      assert!(
          json.get("stream").is_none(),
          "stream field should not be serialized"
      );
  }
  ```

  Note: this test will pass even before the removal if `stream` is `None` and the field has
  `skip_serializing_if = "Option::is_none"`. The stronger verification is to check that
  calling `.stream(true)` does not compile — but that cannot be written as a positive test.
  The removal is enforced by deleting the method and the field; the compiler catches all
  callers.

- GREEN: Remove from `CreateResponseRequest` (struct):
  ```rust
  // Delete this field:
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stream: Option<bool>,
  ```

  Remove from `CreateResponseRequestBuilder` (struct):
  ```rust
  // Delete:
  stream: Option<bool>,
  ```

  Remove the `stream()` builder method entirely.

  Remove the `stream` field from the `try_build()` return expression.

  Run `cargo test --package azure_ai_foundry_models`: all tests pass.

- REFACTOR: Check for any test that calls `.stream(...)` and remove it. Run
  `cargo clippy --workspace --all-targets -- -D warnings`.

### Cycle 3.3: Expose `FoundryError::is_retryable()` helper

**Finding O1.** `is_retriable_status(status: u16) -> bool` is already public (exported from
`azure_ai_foundry_core::client`). However, callers working with `FoundryError` values cannot
easily determine if a given error is retryable without checking the variant and extracting the
status code themselves. Adding an `is_retryable()` method directly on `FoundryError` provides
a clean, stable API for user-space retry logic.

- RED: Write test `foundry_error_is_retryable_for_http_errors`:

  File: `sdk/azure_ai_foundry_core/src/error.rs` (or `client.rs` if `FoundryError` tests live
  there — check existing test structure)

  ```rust
  #[test]
  fn foundry_error_is_retryable_for_rate_limit() {
      let err = FoundryError::http(429, "Too Many Requests".into());
      assert!(err.is_retryable(), "429 should be retryable");
  }

  #[test]
  fn foundry_error_is_retryable_for_server_error() {
      let err = FoundryError::http(503, "Service Unavailable".into());
      assert!(err.is_retryable(), "503 should be retryable");
  }

  #[test]
  fn foundry_error_is_not_retryable_for_client_error() {
      let err = FoundryError::http(400, "Bad Request".into());
      assert!(!err.is_retryable(), "400 should not be retryable");
  }

  #[test]
  fn foundry_error_is_not_retryable_for_auth_error() {
      let err = FoundryError::auth("unauthorized");
      assert!(!err.is_retryable(), "auth errors should not be retryable");
  }

  #[test]
  fn foundry_error_is_not_retryable_for_validation_error() {
      let err = FoundryError::validation("missing field");
      assert!(!err.is_retryable(), "validation errors should not be retryable");
  }
  ```

  Run `cargo test --package azure_ai_foundry_core`: fails (method does not exist).

  First inspect `FoundryError` variants in `error.rs` to confirm which variant carries an HTTP
  status code (likely `Http { status: u16, ... }` or similar) and which constructors exist
  (`FoundryError::http()`, `FoundryError::auth()`, `FoundryError::validation()`).

- GREEN: Add `is_retryable()` to `FoundryError` in
  `sdk/azure_ai_foundry_core/src/error.rs`:

  ```rust
  impl FoundryError {
      /// Returns `true` if this error is likely transient and the request may
      /// succeed on retry.
      ///
      /// Retryable errors are HTTP 429 (rate limit), 500, 502, 503, and 504.
      /// All other error types (validation, auth, client errors) are not retryable.
      pub fn is_retryable(&self) -> bool {
          match self {
              FoundryError::Http { status, .. } => is_retriable_status(*status),
              _ => false,
          }
      }
  }
  ```

  The exact variant name and field must match the actual `FoundryError` definition in `error.rs`.
  Use `is_retriable_status` from `client.rs` (already `pub`).

  Run `cargo test --package azure_ai_foundry_core`: all 5 new tests pass.

- REFACTOR: Add `# Example` to the `is_retryable()` doc comment showing typical usage:

  ```rust
  /// # Example
  ///
  /// ```rust
  /// use azure_ai_foundry_core::error::FoundryError;
  ///
  /// let err = FoundryError::http(429, "Too Many Requests".into());
  /// if err.is_retryable() {
  ///     println!("Will retry after backoff");
  /// }
  /// ```
  ```

  Run `cargo doc --workspace --no-deps`: doc-test compiles successfully.

---

## Milestone 4: Eliminate unnecessary allocations (R2, R4, O4)

Three separate allocation-related fixes that share a milestone because they are all
independent and produce no observable behavioural changes — only allocation reduction.

### Cycle 4.1: Remove `tool_outputs.to_vec()` clone in `SubmitToolOutputsRequest`

**Finding R2 (= R6).** In `run.rs` line 873, `SubmitToolOutputsRequest` is constructed as:

```rust
let request = SubmitToolOutputsRequest {
    tool_outputs: tool_outputs.to_vec(),
};
```

`tool_outputs` is already `&[ToolOutput]`. The `.to_vec()` clones the entire slice into a new
`Vec` just to satisfy the `Vec<ToolOutput>` field. Since `SubmitToolOutputsRequest` is
`pub(crate)` and is serialized immediately after construction (never stored), it can borrow.

- RED: Write test `submit_tool_outputs_request_does_not_clone_unnecessarily`:

  File: `sdk/azure_ai_foundry_agents/src/run.rs`, test section

  ```rust
  #[test]
  fn submit_tool_outputs_request_serializes_from_slice() {
      // Verifies that SubmitToolOutputsRequest can be constructed from a slice
      // reference without requiring an owned Vec (no unnecessary clone).
      let outputs = [
          ToolOutput { tool_call_id: "c1".into(), output: "r1".into() },
          ToolOutput { tool_call_id: "c2".into(), output: "r2".into() },
      ];
      let request = SubmitToolOutputsRequest { tool_outputs: &outputs };
      let json = serde_json::to_value(&request).unwrap();
      assert_eq!(json["tool_outputs"][0]["tool_call_id"], "c1");
      assert_eq!(json["tool_outputs"][1]["output"], "r2");
  }
  ```

  Run `cargo test`: fails to compile because `SubmitToolOutputsRequest.tool_outputs` is
  `Vec<ToolOutput>`, not `&[ToolOutput]`.

- GREEN: Change `SubmitToolOutputsRequest` to use a lifetime and borrow:

  File: `sdk/azure_ai_foundry_agents/src/run.rs`

  ```rust
  // Before:
  pub(crate) struct SubmitToolOutputsRequest {
      pub tool_outputs: Vec<ToolOutput>,
  }

  // After:
  pub(crate) struct SubmitToolOutputsRequest<'a> {
      pub tool_outputs: &'a [ToolOutput],
  }
  ```

  Update the construction site at line 872–874:

  ```rust
  // Before:
  let request = SubmitToolOutputsRequest {
      tool_outputs: tool_outputs.to_vec(),
  };

  // After:
  let request = SubmitToolOutputsRequest { tool_outputs };
  ```

  The existing test `test_submit_tool_outputs_request_serialization` constructed the struct
  with an owned Vec — update it to match the new borrowed form, or remove it if the new test
  supersedes it.

  Run `cargo test --package azure_ai_foundry_agents`: all tests pass.

- REFACTOR: Run `cargo clippy --workspace --all-targets -- -D warnings`. The compiler will
  enforce that the lifetime annotation is correct.

### Cycle 4.2: Reduce allocations in `build_audio_form`

**Finding R4.** In `audio.rs`, `build_audio_form` receives `model: &str` but converts it with
`.to_string()` to pass to `form.text("model", model.to_string())`. The `reqwest` multipart
`text()` method accepts `impl Into<Cow<'static, str>>`. When passing a `String`, no extra
allocation is needed. When passing `&str` the conversion allocates. The function is called once
per request (not in a retry loop), so the impact is minor — but consistent with the codebase's
allocation discipline.

The actual callers already clone strings from the request struct before passing them:
```rust
let model = request.model.clone();  // -> String
...
build_audio_form(&data, &filename, &model, ...)
```

The `model` parameter in `build_audio_form` is `&str`. To avoid the extra `.to_string()` on
`.text("model", model.to_string())`, change the signature to accept `String` and pass the
owned value directly.

- RED: This is a pure refactoring with no behavioural change. The existing tests
  `test_transcribe_audio_success` and `test_translate_audio_success` serve as the regression
  guard. Confirm they pass before changes.

  Write one documentation test to pin the intent:

  ```rust
  #[test]
  fn build_audio_form_accepts_owned_strings() {
      // Smoke test: verify build_audio_form does not panic and returns a form.
      // The test cannot inspect form fields directly, but if the function
      // signature accepts &str-coercible types this will compile.
      let _form = build_audio_form(
          &[0u8, 1, 2],
          "test.wav".to_string(),
          "whisper-1".to_string(),
          None,
          None,
          None,
          None,
      );
  }
  ```

  Run: compile error because `build_audio_form` currently takes `&str` parameters. This is the
  RED compile failure.

- GREEN: Change `build_audio_form` signature from `&str` to `String` for `filename` and
  `model`, and remove the `.to_string()` calls inside the function body:

  ```rust
  fn build_audio_form(
      data: &[u8],
      filename: String,
      model: String,
      language: Option<&str>,
      prompt: Option<&str>,
      response_format: Option<AudioResponseFormat>,
      temperature: Option<f32>,
  ) -> reqwest::multipart::Form {
      let file_part = reqwest::multipart::Part::bytes(data.to_vec()).file_name(filename);
      let mut form = reqwest::multipart::Form::new()
          .part("file", file_part)
          .text("model", model);
      // ... rest unchanged
  }
  ```

  Update callers in `transcribe()` and `translate()` — they already hold `String` clones, so
  pass them directly:

  ```rust
  build_audio_form(
      &data,
      filename,   // was: &filename
      model,      // was: &model
      ...
  )
  ```

  Run `cargo test --package azure_ai_foundry_models`: all tests pass.

- REFACTOR: Run `cargo clippy`. No further changes needed.

### Cycle 4.3: Make `ExpiresAfter::anchor` a typed enum

**Finding O4.** The OpenAI/Azure Agents API only accepts `"last_active_at"` for the `anchor`
field. Using `String` allows callers to pass invalid values at runtime. An enum provides
compile-time safety.

- RED: Write test `expires_after_anchor_enum_serializes_correctly`:

  File: `sdk/azure_ai_foundry_agents/src/vector_store.rs`, test section

  ```rust
  #[test]
  fn expires_after_anchor_serializes_to_last_active_at() {
      let ea = ExpiresAfter {
          anchor: ExpiresAfterAnchor::LastActiveAt,
          days: 7,
      };
      let json = serde_json::to_value(&ea).unwrap();
      assert_eq!(json["anchor"], "last_active_at");
      assert_eq!(json["days"], 7);
  }

  #[test]
  fn expires_after_anchor_deserializes_from_last_active_at() {
      let json = serde_json::json!({"anchor": "last_active_at", "days": 14});
      let ea: ExpiresAfter = serde_json::from_value(json).unwrap();
      assert_eq!(ea.anchor, ExpiresAfterAnchor::LastActiveAt);
      assert_eq!(ea.days, 14);
  }
  ```

  Run `cargo test`: fails to compile — `ExpiresAfterAnchor` does not exist.

- GREEN: Add the enum in `vector_store.rs` (before the `ExpiresAfter` struct definition):

  ```rust
  /// The anchor point for vector store expiration.
  ///
  /// Currently the only accepted value is `"last_active_at"`.
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum ExpiresAfterAnchor {
      /// Expiration is measured from the last time the vector store was accessed.
      LastActiveAt,
  }
  ```

  Change `ExpiresAfter`:

  ```rust
  // Before:
  pub struct ExpiresAfter {
      pub anchor: String,
      pub days: u32,
  }

  // After:
  pub struct ExpiresAfter {
      pub anchor: ExpiresAfterAnchor,
      pub days: u32,
  }
  ```

  Update the existing test in `test_vector_store_deserialization` that asserts
  `ea.anchor == "last_active_at"`:

  ```rust
  assert_eq!(ea.anchor, ExpiresAfterAnchor::LastActiveAt);
  ```

  Update the existing test `test_vector_store_create_request_serialization` that constructs
  `ExpiresAfter { anchor: "last_active_at".into(), days: 30 }`:

  ```rust
  ExpiresAfter {
      anchor: ExpiresAfterAnchor::LastActiveAt,
      days: 30,
  }
  ```

  Run `cargo test --package azure_ai_foundry_agents`: all tests pass.

- REFACTOR: Run `cargo clippy --workspace --all-targets -- -D warnings`. Export
  `ExpiresAfterAnchor` from `lib.rs` if needed (check if `vector_store` types are re-exported).

---

## Milestone 5: Validation, documentation, and Agents API version note (R3, O5, O6)

### Cycle 5.1: Validate `features` not empty in `ImageAnalysisRequest::try_build()`

**Finding O5.** `ImageAnalysisRequest::try_build()` validates `image_url` but does not check
that `features` is non-empty. An empty features list would produce a valid HTTP request that
the Azure Vision API rejects with a 400 error. Failing early with a clear message is better.

- RED: Write test `image_analysis_request_rejects_empty_features`:

  File: `sdk/azure_ai_foundry_tools/src/vision.rs`, test section

  ```rust
  #[test]
  fn image_analysis_request_rejects_empty_features() {
      let result = ImageAnalysisRequest::builder()
          .image_url("https://example.com/photo.jpg")
          // no .features() call — empty by default
          .try_build();
      assert!(result.is_err(), "empty features should be rejected");
      let msg = result.unwrap_err().to_string();
      assert!(
          msg.contains("features"),
          "error should mention 'features', got: {msg}"
      );
  }

  #[test]
  fn image_analysis_request_accepts_non_empty_features() {
      let result = ImageAnalysisRequest::builder()
          .image_url("https://example.com/photo.jpg")
          .features(vec![VisualFeature::Caption])
          .try_build();
      assert!(result.is_ok(), "should succeed with one feature");
  }
  ```

  Run `cargo test --package azure_ai_foundry_tools`: the `rejects_empty_features` test fails
  (no validation yet; `try_build()` currently succeeds with empty features).

- GREEN: Add the guard in `ImageAnalysisRequestBuilder::try_build()`:

  File: `sdk/azure_ai_foundry_tools/src/vision.rs`

  Inside `try_build()`, after the `image_url` validation, add:

  ```rust
  if self.features.is_empty() {
      return Err(FoundryError::validation(
          "features cannot be empty; specify at least one VisualFeature",
      ));
  }
  ```

  Run `cargo test --package azure_ai_foundry_tools`: both new tests pass, existing tests pass.

- REFACTOR: Update the `build()` `# Panics` doc comment (from Milestone 2) to include the
  features condition:

  ```
  /// Panics if `image_url` is not set or `features` is empty.
  ```

  Run `cargo clippy --workspace --all-targets -- -D warnings`.

### Cycle 5.2: Document Agents API version hardcoding

**Finding R3 (= R7).** The Agents API version string `api-version=2025-01-01-preview` is
hardcoded in `sdk/azure_ai_foundry_agents/src/models.rs` (line 6) as a module-level constant.
The `FoundryClientBuilder::api_version()` method only controls the models/core API version.
This is a known limitation with no current workaround.

This cycle is documentation-only (no test cycle needed — there is no behaviour to assert):

- Add a `# Note` to the constant in `models.rs`:

  ```rust
  /// API version used for the Agents Service endpoints.
  ///
  /// # Note
  ///
  /// This version string is hardcoded and is not affected by
  /// [`FoundryClientBuilder::api_version()`](azure_ai_foundry_core::client::FoundryClientBuilder::api_version).
  /// The Agents Service uses a separate versioning scheme from the model inference APIs.
  /// Changing this value requires a crate-level change and will be exposed as a
  /// configuration option in a future release.
  pub(crate) const API_VERSION: &str = "api-version=2025-01-01-preview";
  ```

- Add a corresponding note in `FoundryClientBuilder::api_version()` doc comment (in
  `sdk/azure_ai_foundry_core/src/client.rs`):

  ```
  /// # Note
  ///
  /// This setting controls the API version for model inference endpoints only.
  /// The Agents Service endpoints use a separately hardcoded version string
  /// (`azure_ai_foundry_agents`). See that crate's documentation for details.
  ```

- Run `cargo doc --workspace --no-deps`: builds without errors.
- Run `cargo test --workspace`: all tests pass.

### Cycle 5.3: Add removal-version notices to deprecated items

**Finding O6.** Two sets of deprecated items exist:

1. `RunUsage` (deprecated since v0.7.0) — `sdk/azure_ai_foundry_agents/src/run.rs:525`
2. `get_token()` and `get_token_with_options()` (deprecated since v0.3.0) —
   `sdk/azure_ai_foundry_core/src/auth.rs:309,315`

Both lack a note stating in which version they will be removed. Add the information:

- For `RunUsage` in `run.rs`:

  ```rust
  /// Deprecated: Use [`azure_ai_foundry_core::models::Usage`] instead.
  ///
  /// This type alias will be **removed in v0.8.0**.
  #[deprecated(
      since = "0.7.0",
      note = "Use azure_ai_foundry_core::models::Usage instead. \
              This alias will be removed in v0.8.0."
  )]
  pub type RunUsage = Usage;
  ```

- For `get_token()` in `auth.rs`:

  ```rust
  /// Deprecated: use [`fetch_fresh_token()`](Self::fetch_fresh_token) instead.
  ///
  /// This method will be **removed in v0.8.0**.
  #[deprecated(
      since = "0.3.0",
      note = "Use fetch_fresh_token() instead. \
              This method will be removed in v0.8.0."
  )]
  pub async fn get_token(&self) -> FoundryResult<AccessToken> {
  ```

- For `get_token_with_options()` in `auth.rs`:

  ```rust
  /// Deprecated: use [`fetch_fresh_token_with_options()`](Self::fetch_fresh_token_with_options) instead.
  ///
  /// This method will be **removed in v0.8.0**.
  #[deprecated(
      since = "0.3.0",
      note = "Use fetch_fresh_token_with_options() instead. \
              This method will be removed in v0.8.0."
  )]
  ```

  No tests needed for doc changes. Run `cargo test --workspace` to confirm no regressions.

---

## Milestone 6: Consolidate test utilities into core (O3)

**Finding O3.** Three crates duplicate `setup_mock_client()` and `TEST_API_KEY`:
- `sdk/azure_ai_foundry_models/src/lib.rs` — `pub(crate) mod test_utils`
- `sdk/azure_ai_foundry_agents/src/lib.rs` — `pub(crate) mod test_utils`
- `sdk/azure_ai_foundry_tools/src/lib.rs` — `pub(crate) mod test_utils`

The canonical fix: add a `#[cfg(test)]` test helper module in `azure_ai_foundry_core` that is
re-exported under a `#[doc(hidden)]` feature flag, so the three crates can depend on it without
polluting the public API.

The simpler acceptable fix (matching the existing architecture where all three crates already
depend on `azure_ai_foundry_core`): expose the test utilities from `azure_ai_foundry_core`
under a `cfg(test)` guard and re-export them. Each dependent crate's `test_utils` module becomes
a one-line re-export.

### Cycle 6.1: Add canonical `test_utils` to `azure_ai_foundry_core`

- RED: Write a test in `azure_ai_foundry_core` that uses the to-be-created utility:

  File: `sdk/azure_ai_foundry_core/src/client.rs`, test section

  ```rust
  #[cfg(test)]
  mod test_utils_exist {
      use crate::test_utils::{setup_mock_client, TEST_API_KEY};

      #[test]
      fn test_utils_constants_are_correct() {
          assert_eq!(TEST_API_KEY, "test-api-key");
      }
  }
  ```

  Run `cargo test --package azure_ai_foundry_core`: fails (module does not exist).

- GREEN: Add to `sdk/azure_ai_foundry_core/src/lib.rs`:

  ```rust
  /// Test utilities for integration testing with mock servers.
  ///
  /// Available only in test builds. Intended for use by sibling crates in this workspace.
  #[cfg(test)]
  pub mod test_utils {
      use crate::auth::FoundryCredential;
      use crate::client::FoundryClient;
      use wiremock::MockServer;

      /// Test API key (not a real key).
      pub const TEST_API_KEY: &str = "test-api-key";

      /// Create a test `FoundryClient` connected to a mock server.
      pub async fn setup_mock_client(server: &MockServer) -> FoundryClient {
          FoundryClient::builder()
              .endpoint(server.uri())
              .credential(FoundryCredential::api_key(TEST_API_KEY))
              .build()
              .expect("should build test client")
      }
  }
  ```

  Add `wiremock` as a `[dev-dependencies]` in `azure_ai_foundry_core/Cargo.toml` if not
  already present (check first — it may already be there due to existing tests in `client.rs`).

  Run `cargo test --package azure_ai_foundry_core`: the new test passes.

### Cycle 6.2: Replace duplicated test_utils in models, agents, and tools

- GREEN: For each of the three crates, replace the body of `pub(crate) mod test_utils` with
  a re-export:

  File: `sdk/azure_ai_foundry_models/src/lib.rs`

  ```rust
  #[cfg(test)]
  pub(crate) mod test_utils {
      // Re-export from core to avoid duplication.
      // Model-specific constants kept here.
      pub use azure_ai_foundry_core::test_utils::{setup_mock_client, TEST_API_KEY};

      pub const TEST_CHAT_MODEL: &str = "gpt-4o";
      pub const TEST_EMBEDDING_MODEL: &str = "text-embedding-ada-002";
      pub const TEST_AUDIO_MODEL: &str = "whisper-1";
      pub const TEST_TTS_MODEL: &str = "tts-1";
      pub const TEST_IMAGE_MODEL: &str = "dall-e-3";
      pub const TEST_TIMESTAMP: u64 = 1700000000;
  }
  ```

  File: `sdk/azure_ai_foundry_agents/src/lib.rs`

  ```rust
  #[cfg(test)]
  pub(crate) mod test_utils {
      pub use azure_ai_foundry_core::test_utils::{setup_mock_client, TEST_API_KEY};

      pub const TEST_TIMESTAMP: u64 = 1700000000;
      pub const TEST_MODEL: &str = "gpt-4o";
  }
  ```

  File: `sdk/azure_ai_foundry_tools/src/lib.rs`

  ```rust
  #[cfg(test)]
  pub(crate) mod test_utils {
      pub use azure_ai_foundry_core::test_utils::{setup_mock_client, TEST_API_KEY};
  }
  ```

  Run `cargo test --workspace`: all tests pass. The re-exports are transparent — all test
  modules that import `crate::test_utils::setup_mock_client` continue to work unchanged.

- REFACTOR: Run `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo doc --workspace --no-deps`.

---

## Dependency Order

```
Milestone 1 (C1: UTF-8 panic fix)          — no prerequisites
Milestone 2 (C2: panics doc + lint)        — no prerequisites; run after M1 to batch clippy passes
Milestone 3 (R1, O1, O2: type/API fixes)  — no prerequisites; O2 is breaking, do in one PR
Milestone 4 (R2, R4, O4: allocations)     — no prerequisites
Milestone 5 (R3, O5, O6: docs/validation) — M2 must complete first (O5 Panics doc depends on M2)
Milestone 6 (O3: test utility dedup)      — no prerequisites; completely independent
```

Recommended execution order: **1 → 2 → 3 → 4 → 5 → 6**

All milestones except 2 → 5 are fully independent and can be done in any order. Milestones 1
and 2 should be done first because they touch the most files and establish the lint baseline.

---

## Estimation

| Milestone | Description | Estimate |
|-----------|-------------|----------|
| 1 | C1 — UTF-8 panic fix + 3 tests | 45 min |
| 2 | C2 — 15 `# Panics` docs + remove lint suppression | 60 min |
| 3 | R1 + O1 + O2 — type change, remove field, add method | 45 min |
| 4 | R2 + R4 + O4 — lifetime, string owned, enum | 60 min |
| 5 | R3 + O5 + O6 — validation + doc comments | 30 min |
| 6 | O3 — test utility consolidation | 30 min |
| **Total** | | **~5 h** |

---

## Success Criteria

- [ ] `cargo test --workspace` passes with zero failures
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` reports zero warnings
- [ ] `cargo doc --workspace --no-deps` builds without errors
- [ ] `cargo fmt --all -- --check` reports no changes needed
- [ ] `missing_panics_doc = "allow"` line removed from workspace `Cargo.toml`
- [ ] All `build()` methods that call `expect()` have a `# Panics` section
- [ ] `sanitize_error_message` does not panic on multi-byte or emoji input
- [ ] `Response::created_at` is `u64`
- [ ] `CreateResponseRequest` has no `stream` field
- [ ] `FoundryError::is_retryable()` is public and tested
- [ ] `SubmitToolOutputsRequest` borrows `&[ToolOutput]` instead of owning `Vec<ToolOutput>`
- [ ] `ExpiresAfter::anchor` is `ExpiresAfterAnchor` enum
- [ ] `ImageAnalysisRequest::try_build()` rejects empty `features`
- [ ] `RunUsage`, `get_token()`, `get_token_with_options()` have v0.8.0 removal notices
- [ ] `setup_mock_client` and `TEST_API_KEY` defined once in `azure_ai_foundry_core::test_utils`
