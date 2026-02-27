# TDD Plan: Quality Fixes for `azure_ai_foundry_tools` Crate

## Context

A code review of `azure_ai_foundry_tools` (v0.3.0, branch `develop/v0.3.0`) identified 14 quality
findings across `vision.rs`, `document_intelligence.rs`, and related types. This plan addresses all
of them as TDD cycles ordered from highest-severity (critical bugs, security) to lowest (nice to have).

**Stack detected:** Rust 1.88, async-first with `tokio`, `serde`/`serde_json`, `wiremock` for tests.
**Conventions observed:** Builder pattern, `#[serde(rename = "...")]` on enum variants, `thiserror`
errors, `tracing::instrument` on public async functions, unit tests in `#[cfg(test)]` blocks inside
each source file.
**Affects hot path:** No — this crate wraps Azure REST calls. No compute-intensive loops.

## Decisions Resolved Before This Plan

All findings are clear defects or unambiguous improvements. No architecture decisions are needed.

---

## Plan de Ejecucion

### Phase 1: Critical Fixes — Vision

---

#### Cycle 1: NaN/Infinity validation in `smartcrops_aspect_ratios`

**Finding 2 — Critical**

The current guard `!(*ratio >= 0.75 && *ratio <= 1.80)` silently accepts `f64::NAN` because
`NAN >= 0.75` evaluates to `false`, making the whole condition `!(false && ...)` = `true`, so NaN
passes. An `f64::INFINITY` greater than 1.80 is correctly rejected, but `-INFINITY` passes. The fix
is to add `ratio.is_finite()` before the range check.

- [ ] **RED** — Write the failing test.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_image_analysis_request_rejects_nan_aspect_ratio`
  - Test name: `test_image_analysis_request_rejects_infinity_aspect_ratio`

  ```rust
  #[test]
  fn test_image_analysis_request_rejects_nan_aspect_ratio() {
      let result = ImageAnalysisRequest::builder()
          .url("https://example.com/img.png")
          .features(vec![VisualFeature::SmartCrops])
          .smartcrops_aspect_ratios(vec![f64::NAN])
          .build();
      let err = result.expect_err("NaN should be rejected");
      assert!(
          err.to_string().contains("aspect ratio"),
          "error: {err}",
      );
  }

  #[test]
  fn test_image_analysis_request_rejects_infinity_aspect_ratio() {
      let result = ImageAnalysisRequest::builder()
          .url("https://example.com/img.png")
          .features(vec![VisualFeature::SmartCrops])
          .smartcrops_aspect_ratios(vec![f64::INFINITY])
          .build();
      let err = result.expect_err("Infinity should be rejected");
      assert!(
          err.to_string().contains("aspect ratio"),
          "error: {err}",
      );
  }
  ```

  Run `cargo test -p azure_ai_foundry_tools test_image_analysis_request_rejects_nan_aspect_ratio` — it MUST FAIL.

- [ ] **GREEN** — Fix the validation.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, `ImageAnalysisRequestBuilder::build()`
  - Change lines:

  ```rust
  // BEFORE (current):
  if !(*ratio >= 0.75 && *ratio <= 1.80) {

  // AFTER (fixed):
  if !ratio.is_finite() || !(*ratio >= 0.75 && *ratio <= 1.80) {
  ```

  Run tests — both must PASS.

- [ ] **REFACTOR** — None needed. The one-liner fix is clear.

---

#### Cycle 2: Add `url()` getter on `ImageAnalysisRequest`

**Finding 4 — Critical**

`url` is private but accessed directly in `analyze()` via `request.url` because the access is
in the same module. This works now but is fragile: if `analyze()` were ever moved to a separate
module or the field name changed, the compiler would not guide callers. A public getter makes the
API contract explicit and allows future field renaming without breakage.

- [ ] **RED** — Write the failing test.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_image_analysis_request_url_getter`

  ```rust
  #[test]
  fn test_image_analysis_request_url_getter() {
      let request = ImageAnalysisRequest::builder()
          .url("https://example.com/image.jpg")
          .features(vec![VisualFeature::Tags])
          .build()
          .expect("valid request");
      // This line will fail to compile until the getter is added.
      assert_eq!(request.url(), "https://example.com/image.jpg");
  }
  ```

  Run `cargo build -p azure_ai_foundry_tools` — it MUST FAIL (compile error: no method `url`).

- [ ] **GREEN** — Add the getter method.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, inside `impl ImageAnalysisRequest`
  - Add after `features_query_param()`:

  ```rust
  /// Returns the image URL set on this request.
  pub fn url(&self) -> &str {
      &self.url
  }
  ```

  Also update the `analyze()` function to use the getter instead of direct field access, to
  enforce the API boundary:

  ```rust
  // BEFORE:
  let body = serde_json::json!({ "url": request.url });

  // AFTER:
  let body = serde_json::json!({ "url": request.url() });
  ```

  Run tests — test must PASS and compilation must succeed.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 3: Make `features_query_param()` `pub(crate)`

**Finding 12 — Optional**

`features_query_param()` is `pub` but only called internally (by `analyze()` and within tests in
the same crate). Exposing it as public API creates a maintenance surface. Reduce to `pub(crate)`.

- [ ] **RED** — Verify the method is currently `pub` by confirming `cargo clippy` has no warning.
  Check that there is no test outside the crate importing it. There is no RED step for a
  visibility change — this is a compile-time contract tightening. The test that validates it
  is a negative compile test, which is impractical; instead validate that the tracing
  `#[instrument]` attribute still compiles correctly after the change.

  Run `cargo test -p azure_ai_foundry_tools -- --list` to confirm existing tests use it only
  inside the module.

- [ ] **GREEN** — Change visibility.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`

  ```rust
  // BEFORE:
  pub fn features_query_param(&self) -> String {

  // AFTER:
  pub(crate) fn features_query_param(&self) -> String {
  ```

  Run `cargo build --workspace` — MUST compile with zero errors.
  Run `cargo test -p azure_ai_foundry_tools` — ALL tests must pass.

- [ ] **REFACTOR** — None needed.

---

### Phase 2: Critical Fixes — Document Intelligence

---

#### Cycle 4: Empty-string validation for `url_source` and `base64_source`

**Finding 3 — Critical**

`DocumentAnalysisRequestBuilder::build()` only checks that at least one source is set
(`self.url_source.is_some()`), but an empty string `""` passes the `is_some()` check.
`ImageAnalysisRequestBuilder` uses `.filter(|u| !u.is_empty())` — the `DocumentAnalysis` builder
should be consistent.

- [ ] **RED** — Write the failing tests.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test names: `test_doc_analysis_request_rejects_empty_url_source`
                 `test_doc_analysis_request_rejects_empty_base64_source`

  ```rust
  #[test]
  fn test_doc_analysis_request_rejects_empty_url_source() {
      let result = DocumentAnalysisRequest::builder()
          .model_id(PREBUILT_READ)
          .url_source("")
          .build();
      let err = result.expect_err("empty url_source should be rejected");
      assert!(
          err.to_string().contains("source"),
          "error should mention source: {err}",
      );
  }

  #[test]
  fn test_doc_analysis_request_rejects_empty_base64_source() {
      let result = DocumentAnalysisRequest::builder()
          .model_id(PREBUILT_READ)
          .base64_source("")
          .build();
      let err = result.expect_err("empty base64_source should be rejected");
      assert!(
          err.to_string().contains("source"),
          "error should mention source: {err}",
      );
  }
  ```

  Run tests — MUST FAIL (empty strings currently accepted).

- [ ] **GREEN** — Add empty-string filtering in `build()`.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`,
    `DocumentAnalysisRequestBuilder::build()`

  ```rust
  // BEFORE:
  let has_url = self.url_source.is_some();
  let has_base64 = self.base64_source.is_some();

  // AFTER:
  let url_source = self.url_source.filter(|s| !s.is_empty());
  let base64_source = self.base64_source.filter(|s| !s.is_empty());
  let has_url = url_source.is_some();
  let has_base64 = base64_source.is_some();
  ```

  Also update the struct construction at the bottom of `build()`:

  ```rust
  Ok(DocumentAnalysisRequest {
      model_id,
      url_source,       // use filtered version
      base64_source,    // use filtered version
      pages: self.pages,
      locale: self.locale,
      features: self.features,
  })
  ```

  Run tests — MUST PASS.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 5: Fix `FoundryError::MissingConfig` used for missing `Operation-Location` header

**Finding 8 — Recommended**

In `analyze()`, when the `Operation-Location` header is absent, the error is:
`FoundryError::MissingConfig("Operation-Location header missing from response")`.
`MissingConfig` is semantically for missing SDK configuration (endpoint, credentials). A missing
HTTP response header from the server is an API-layer error — `FoundryError::Api` is correct.

- [ ] **RED** — Write the failing test that asserts the correct error variant.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_analyze_document_missing_operation_location_returns_api_error`

  The existing test `test_analyze_document_missing_operation_location` only checks the error
  message string. This new test checks the variant:

  ```rust
  #[tokio::test]
  async fn test_analyze_document_missing_operation_location_returns_api_error() {
      use azure_ai_foundry_core::error::FoundryError;

      let server = MockServer::start().await;
      let client = setup_mock_client(&server).await;

      Mock::given(method("POST"))
          .and(match_path(
              "/documentintelligence/documentModels/prebuilt-read:analyze",
          ))
          .respond_with(ResponseTemplate::new(202)) // no Operation-Location header
          .mount(&server)
          .await;

      let request = DocumentAnalysisRequest::builder()
          .model_id(PREBUILT_READ)
          .url_source("https://example.com/doc.pdf")
          .build()
          .expect("valid request");

      let err = analyze(&client, &request)
          .await
          .expect_err("should fail without Operation-Location");

      // Must be Api variant, NOT MissingConfig
      assert!(
          matches!(err, FoundryError::Api { .. }),
          "expected FoundryError::Api, got: {err:?}",
      );
      assert!(
          err.to_string().contains("Operation-Location"),
          "error: {err}",
      );
  }
  ```

  Run test — MUST FAIL (currently returns `MissingConfig`).

- [ ] **GREEN** — Change the error variant in `analyze()`.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, `analyze()` function

  ```rust
  // BEFORE:
  .ok_or_else(|| {
      FoundryError::MissingConfig("Operation-Location header missing from response".into())
  })?;

  // AFTER:
  .ok_or_else(|| FoundryError::Api {
      code: "MissingHeader".into(),
      message: "Operation-Location header missing from response".into(),
  })?;
  ```

  Run tests — MUST PASS, including existing `test_analyze_document_missing_operation_location`.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 6: Add `error` field to `AnalyzeOperationResult` for `status == "failed"`

**Finding 10 — Recommended**

When Document Intelligence returns `status: "failed"`, the Azure API includes an `error` object
alongside the status. The current `AnalyzeOperationResult` struct has no `error` field, so that
information is silently discarded. Callers cannot tell why the analysis failed.

- [ ] **RED** — Write the failing test.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_analyze_operation_result_failed_with_error_details`

  ```rust
  #[test]
  fn test_analyze_operation_result_failed_with_error_details() {
      let json = r#"{
          "status": "failed",
          "error": {
              "code": "InvalidRequest",
              "message": "The document format is not supported."
          }
      }"#;
      let result: AnalyzeOperationResult =
          serde_json::from_str(json).expect("should deserialize");
      assert_eq!(result.status, AnalyzeResultStatus::Failed);

      // This line will fail to compile until `error` field is added.
      let err = result.error.expect("should have error details");
      assert_eq!(err.code, "InvalidRequest");
      assert!(err.message.contains("not supported"));
  }
  ```

  Run `cargo build -p azure_ai_foundry_tools` — MUST FAIL (no `error` field on struct).

- [ ] **GREEN** — Add the `error` field and a new type.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`

  First, add the error type after `OperationStatus`:

  ```rust
  /// An error returned by the Document Intelligence API when an operation fails.
  #[derive(Debug, Clone, Deserialize)]
  pub struct AnalyzeOperationError {
      /// The error code.
      pub code: String,
      /// Human-readable error description.
      pub message: String,
  }
  ```

  Then add the field to `AnalyzeOperationResult`:

  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct AnalyzeOperationResult {
      /// Current status of the operation.
      pub status: AnalyzeResultStatus,

      /// Error details, present when status is `Failed`.
      #[serde(rename = "error")]
      pub error: Option<AnalyzeOperationError>,

      /// The analysis result, present when status is `Succeeded`.
      #[serde(rename = "analyzeResult")]
      pub analyze_result: Option<AnalyzeResult>,
  }
  ```

  Run tests — MUST PASS.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 7: Add `Display` impl for `AnalyzeResultStatus`

**Finding 13 — Optional**

`AnalyzeResultStatus` derives `Debug` but not `Display`. Callers logging the status must use
`{:?}`, producing `Succeeded` / `Failed` in debug format. A `Display` impl produces clean output
like `"succeeded"` (matching API terminology) and allows `format!("{status}")` in error messages.

- [ ] **RED** — Write the failing test.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_analyze_result_status_display`

  ```rust
  #[test]
  fn test_analyze_result_status_display() {
      // These lines fail to compile until Display is implemented.
      assert_eq!(AnalyzeResultStatus::NotStarted.to_string(), "notStarted");
      assert_eq!(AnalyzeResultStatus::Running.to_string(), "running");
      assert_eq!(AnalyzeResultStatus::Succeeded.to_string(), "succeeded");
      assert_eq!(AnalyzeResultStatus::Failed.to_string(), "failed");
  }
  ```

  Run `cargo build -p azure_ai_foundry_tools` — MUST FAIL (no `Display` impl).

- [ ] **GREEN** — Implement `Display`.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, after `impl AnalyzeResultStatus`

  ```rust
  impl std::fmt::Display for AnalyzeResultStatus {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          // Use the same camelCase strings as the API to be consistent with serde.
          let s = match self {
              Self::NotStarted => "notStarted",
              Self::Running => "running",
              Self::Succeeded => "succeeded",
              Self::Failed => "failed",
          };
          f.write_str(s)
      }
  }
  ```

  Run tests — MUST PASS.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 8: Add `Display` consistency test for `DocumentAnalysisFeature::as_str()` and `serde`

**Finding 7 — Recommended**

`DocumentAnalysisFeature` has both `fn as_str()` (used for query param building) and
`#[serde(rename = "...")]` (used for JSON serialization). If either is updated without the other,
the query string and the body will disagree. A test that cross-validates both representations
catches this divergence at compile time if a variant is added without updating `as_str()`.

Same pattern applies to `VisualFeature`.

- [ ] **RED** — Write the synchronization tests.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_document_analysis_feature_as_str_matches_serde`

  ```rust
  #[test]
  fn test_document_analysis_feature_as_str_matches_serde() {
      let variants = [
          (DocumentAnalysisFeature::OcrHighResolution, "ocrHighResolution"),
          (DocumentAnalysisFeature::Languages, "languages"),
          (DocumentAnalysisFeature::Barcodes, "barcodes"),
          (DocumentAnalysisFeature::Formulas, "formulas"),
          (DocumentAnalysisFeature::KeyValuePairs, "keyValuePairs"),
          (DocumentAnalysisFeature::StyleFont, "styleFont"),
          (DocumentAnalysisFeature::QueryFields, "queryFields"),
      ];

      for (variant, expected) in &variants {
          // as_str() must match
          assert_eq!(
              variant.as_str(),
              *expected,
              "as_str() mismatch for {expected}",
          );
          // serde rename must match
          let serialized = serde_json::to_string(variant).expect("should serialize");
          assert_eq!(
              serialized,
              format!("\"{expected}\""),
              "serde rename mismatch for {expected}",
          );
      }
  }
  ```

  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_visual_feature_as_str_matches_serde`

  ```rust
  #[test]
  fn test_visual_feature_as_str_matches_serde() {
      let variants = [
          (VisualFeature::Tags, "tags"),
          (VisualFeature::Caption, "caption"),
          (VisualFeature::DenseCaptions, "denseCaptions"),
          (VisualFeature::Objects, "objects"),
          (VisualFeature::Read, "read"),
          (VisualFeature::SmartCrops, "smartCrops"),
          (VisualFeature::People, "people"),
      ];

      for (variant, expected) in &variants {
          assert_eq!(
              variant.as_str(),
              *expected,
              "as_str() mismatch for {expected}",
          );
          let serialized = serde_json::to_string(variant).expect("should serialize");
          assert_eq!(
              serialized,
              format!("\"{expected}\""),
              "serde rename mismatch for {expected}",
          );
      }
  }
  ```

  Run tests — these PASS immediately because the current code IS consistent. Their purpose is
  to FAIL in the future when a variant is added without updating `as_str()`. This is a
  regression-guard test, which is still a valid RED→GREEN→REFACTOR cycle: the RED phase is
  "the test does not exist yet" (no protection), GREEN is "test exists" (protected).

- [ ] **GREEN** — Tests pass as-is. No production code changes needed.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 9: Reduce `DocumentAnalysisBody` visibility from `pub(crate)` to `pub(super)`

**Finding 9 — Recommended**

`DocumentAnalysisBody` is `pub(crate)` but is only ever instantiated in `document_intelligence.rs`
and used nowhere else in the crate. `pub(super)` would be even more restrictive, but since it is in
a module file directly under `src/`, `pub(super)` = `pub(crate)` in this case. The correct fix is
to make it fully `pub(self)` (equivalent to private), since it is an implementation detail of the
`analyze()` function only.

- [ ] **RED** — Verify no external access: `cargo grep`-equivalent.

  Run `cargo build --workspace` before the change to confirm baseline. Then change visibility and
  verify nothing breaks.

- [ ] **GREEN** — Change visibility.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`

  ```rust
  // BEFORE:
  pub(crate) struct DocumentAnalysisBody {

  // AFTER:
  struct DocumentAnalysisBody {
  ```

  Also change the `body()` method on `DocumentAnalysisRequest`:

  ```rust
  // BEFORE:
  pub(crate) fn body(&self) -> DocumentAnalysisBody {

  // AFTER (also private, since only used by analyze() in the same file):
  fn body(&self) -> DocumentAnalysisBody {
  ```

  Run `cargo build --workspace` — MUST compile.
  Run `cargo test -p azure_ai_foundry_tools` — ALL tests must pass.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 10: Add `poll_until_complete` iteration limit (`max_attempts`)

**Finding 1 — Critical**

Both `document_intelligence::poll_until_complete` and `agents::run::poll_until_complete` run an
unbounded `loop`. If the Azure service hangs, the caller's future never resolves. Adding a
`max_attempts: u32` parameter lets the caller set an upper bound; exceeding it returns an
`FoundryError::Api` with a clear timeout message.

NOTE: The agents crate `run::poll_until_complete` has the same bug. That crate is not the focus of
this plan, but both functions must be fixed. The plan covers both to maintain consistency.

**Sub-cycle 10a: Document Intelligence `poll_until_complete`**

- [ ] **RED** — Write the test.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_poll_until_complete_exceeds_max_attempts`

  ```rust
  #[tokio::test]
  async fn test_poll_until_complete_exceeds_max_attempts() {
      use azure_ai_foundry_core::error::FoundryError;

      let server = MockServer::start().await;
      let client = setup_mock_client(&server).await;

      // Always return "running" — will never terminate naturally
      Mock::given(method("GET"))
          .and(match_path(
              "/documentintelligence/documentModels/prebuilt-read/analyzeResults/infinite",
          ))
          .respond_with(
              ResponseTemplate::new(200)
                  .set_body_json(serde_json::json!({"status": "running"})),
          )
          .mount(&server)
          .await;

      let op_location = format!(
          "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/infinite",
          server.uri(),
      );

      // This call must fail to compile until max_attempts is added to the signature.
      let err = poll_until_complete(&client, &op_location, Duration::from_millis(1), 3)
          .await
          .expect_err("should fail after max_attempts exceeded");

      assert!(
          matches!(err, FoundryError::Api { .. }),
          "expected FoundryError::Api, got: {err:?}",
      );
      assert!(
          err.to_string().contains("max_attempts") || err.to_string().contains("timed out"),
          "error: {err}",
      );
  }
  ```

  Run `cargo build -p azure_ai_foundry_tools` — MUST FAIL (signature mismatch).

- [ ] **GREEN** — Add `max_attempts: u32` parameter and loop counter.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`

  Update the public doc example in the module-level rustdoc and the function signature:

  ```rust
  /// Poll an analyze operation until it reaches a terminal status.
  ///
  /// Returns the final [`AnalyzeOperationResult`] when the status is `Succeeded`
  /// or `Failed`. The caller should check the status to determine if the
  /// analysis succeeded.
  ///
  /// # Arguments
  ///
  /// * `client` - The Foundry client.
  /// * `operation_location` - The URL returned by [`analyze`].
  /// * `poll_interval` - How often to check the status.
  /// * `max_attempts` - Maximum number of poll attempts before returning an error.
  ///   Set to `0` to disable the limit (not recommended for production).
  ///
  /// # Errors
  ///
  /// Returns [`FoundryError::Api`] if `max_attempts` is exceeded before
  /// the operation reaches a terminal status.
  #[tracing::instrument(
      name = "foundry::document_intelligence::poll_until_complete",
      skip(client),
      fields(operation_location = %operation_location)
  )]
  pub async fn poll_until_complete(
      client: &FoundryClient,
      operation_location: &str,
      poll_interval: Duration,
      max_attempts: u32,
  ) -> FoundryResult<AnalyzeOperationResult> {
      tracing::debug!("starting to poll for completion");

      let mut attempts = 0u32;

      loop {
          if max_attempts > 0 {
              attempts += 1;
              if attempts > max_attempts {
                  return Err(FoundryError::Api {
                      code: "PollTimeout".into(),
                      message: format!(
                          "poll_until_complete timed out after {max_attempts} attempts"
                      ),
                  });
              }
          }

          let result = get_result(client, operation_location).await?;

          if result.status.is_terminal() {
              tracing::debug!(status = ?result.status, "operation reached terminal status");
              return Ok(result);
          }

          tracing::trace!(
              status = ?result.status,
              attempt = attempts,
              "operation still in progress, waiting",
          );
          tokio::time::sleep(poll_interval).await;
      }
  }
  ```

  Update the module-level doc example and the top-of-file `//! ## Example` to pass `max_attempts`:

  ```rust
  // In lib.rs and module doc:
  let result = document_intelligence::poll_until_complete(
      &client,
      &operation.operation_location,
      std::time::Duration::from_secs(2),
      60, // max 60 attempts = 2 minutes
  ).await?;
  ```

  Update existing tests that call `poll_until_complete` to pass a `max_attempts` value:
  - `test_poll_until_complete_immediate_success`: pass `10`
  - `test_poll_until_complete_failed_status`: pass `10`
  - `test_poll_until_complete_emits_span`: pass `10`

  Run all tests — MUST PASS.

- [ ] **REFACTOR** — Ensure doc examples in `lib.rs` and module-level rustdoc are updated to pass
  the new parameter. Run `cargo doc --workspace --no-deps` to verify.

**Sub-cycle 10b: Update module-level example in `document_intelligence.rs`**

  The `//! ## Example` block at the top of the file calls `poll_until_complete` without
  `max_attempts`. Update it:

  ```rust
  //! let result = document_intelligence::poll_until_complete(
  //!     &client,
  //!     &operation.operation_location,
  //!     std::time::Duration::from_secs(2),
  //!     60,
  //! ).await?;
  ```

  Run `cargo test --doc -p azure_ai_foundry_tools` — doc tests must pass.

---

### Phase 3: Recommended Fixes — Tests

---

#### Cycle 11: Add missing `get_result` test with malformed `Operation-Location` URL

**Finding 6 — Recommended**

Cycle 19 was skipped in the original TDD plan. `get_result` parses the `Operation-Location` URL
with `url::Url::parse()` and returns `FoundryError::InvalidEndpoint` on failure. This code path
has no test. A malformed URL like `"not-a-url"` should trigger the error.

- [ ] **RED** — Write the failing test (fails because the test does not exist, not because the
  code is wrong).
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_get_result_with_malformed_url_returns_invalid_endpoint`

  ```rust
  #[tokio::test]
  async fn test_get_result_with_malformed_url_returns_invalid_endpoint() {
      use azure_ai_foundry_core::error::FoundryError;

      let server = MockServer::start().await;
      let client = setup_mock_client(&server).await;

      // No mock needed — the URL parsing fails before any HTTP call is made.
      let err = get_result(&client, "not-a-valid-url")
          .await
          .expect_err("should fail with malformed URL");

      assert!(
          matches!(err, FoundryError::InvalidEndpoint { .. }),
          "expected FoundryError::InvalidEndpoint, got: {err:?}",
      );
      assert!(
          err.to_string().contains("Operation-Location"),
          "error should mention Operation-Location: {err}",
      );
  }
  ```

  Run test — MUST FAIL (test does not exist yet, so currently there is no coverage).
  After adding the test and running it, it will PASS immediately because the `get_result`
  implementation already handles this case correctly. The "RED" here is the absence of the test.

- [ ] **GREEN** — The implementation already handles this. No production code changes. Add the test.

  Run test — MUST PASS.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 12: Verify tracing span field content (vision)

**Finding 11 — Optional**

The existing tracing test `test_analyze_emits_vision_span` only checks that the span name appears
in logs via `logs_contain("foundry::vision::analyze")`. It does not verify that the `features`
field is populated correctly. Add a test that verifies the field value.

- [ ] **RED** — Write the test.
  - File: `sdk/azure_ai_foundry_tools/src/vision.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_analyze_emits_span_with_features_field`

  ```rust
  #[tokio::test]
  #[tracing_test::traced_test]
  async fn test_analyze_emits_span_with_features_field() {
      let server = MockServer::start().await;
      let client = setup_mock_client(&server).await;

      Mock::given(method("POST"))
          .and(match_path("/computervision/imageanalysis:analyze"))
          .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
              "modelVersion": "2024-02-01",
              "metadata": {"width": 100, "height": 100}
          })))
          .mount(&server)
          .await;

      let request = ImageAnalysisRequest::builder()
          .url("https://example.com/img.jpg")
          .features(vec![VisualFeature::Tags, VisualFeature::Caption])
          .build()
          .expect("valid request");

      let _ = analyze(&client, &request).await;

      // Verify the features field value appears in the trace output.
      assert!(logs_contain("tags,caption"));
  }
  ```

  Run test — MUST FAIL (no existing test with this assertion).
  After adding, it should PASS because `tracing::instrument` records `features` using
  `%request.features_query_param()`.

- [ ] **GREEN** — Test is added. Existing implementation already emits the field.

  Run test — MUST PASS.

- [ ] **REFACTOR** — None needed.

---

#### Cycle 13: Verify tracing span field content (document intelligence)

**Finding 11 — Optional (continued)**

Same as Cycle 12, but for `document_intelligence::analyze`. The existing test only checks the span
name. Add a test verifying the `model_id` field is recorded.

- [ ] **RED** — Write the test.
  - File: `sdk/azure_ai_foundry_tools/src/document_intelligence.rs`, inside `#[cfg(test)] mod tests`
  - Test name: `test_analyze_document_emits_span_with_model_id_field`

  ```rust
  #[tokio::test]
  #[tracing_test::traced_test]
  async fn test_analyze_document_emits_span_with_model_id_field() {
      let server = MockServer::start().await;
      let client = setup_mock_client(&server).await;

      let op_location = format!(
          "{}/documentintelligence/documentModels/prebuilt-read/analyzeResults/res-field",
          server.uri(),
      );

      Mock::given(method("POST"))
          .and(match_path(
              "/documentintelligence/documentModels/prebuilt-read:analyze",
          ))
          .respond_with(
              ResponseTemplate::new(202)
                  .append_header("Operation-Location", op_location.as_str()),
          )
          .mount(&server)
          .await;

      let request = DocumentAnalysisRequest::builder()
          .model_id(PREBUILT_READ)
          .url_source("https://example.com/doc.pdf")
          .build()
          .expect("valid request");

      let _ = analyze(&client, &request).await;

      // Verify the model_id field value appears in the trace output.
      assert!(logs_contain("prebuilt-read"));
  }
  ```

  Run test — MUST FAIL (test does not exist).
  After adding, it MUST PASS.

- [ ] **GREEN** — Test is added. Existing implementation already emits the field.

- [ ] **REFACTOR** — None needed.

---

### Phase 4: Required Infrastructure

---

#### Cycle 14: Add `README.md` for `azure_ai_foundry_tools` crate

**Finding 5 — Recommended**

crates.io requires a `README.md` for a crate to render a description page. The `Cargo.toml` does
not have a `readme` key, and no `README.md` file exists in
`sdk/azure_ai_foundry_tools/`. Use the `azure_ai_foundry_agents/README.md` as a template.

- [ ] **RED** — Confirm the file is missing.

  Run: `ls sdk/azure_ai_foundry_tools/` — output should NOT contain `README.md`.

- [ ] **GREEN** — Create the file.
  - File to create: `sdk/azure_ai_foundry_tools/README.md`

  Contents (model after the agents crate README, adapting for tools):

  ```markdown
  # azure_ai_foundry_tools

  [![Crates.io](https://img.shields.io/crates/v/azure_ai_foundry_tools.svg)](https://crates.io/crates/azure_ai_foundry_tools)
  [![docs.rs](https://docs.rs/azure_ai_foundry_tools/badge.svg)](https://docs.rs/azure_ai_foundry_tools)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

  Vision and Document Intelligence clients for the Azure AI Foundry Rust SDK.

  ## Features

  - **Vision** — Image Analysis 4.0: tags, captions, object detection, OCR, smart crops, people
  - **Document Intelligence** — Document Intelligence v4.0: OCR, layout, invoices, receipts,
    ID documents, business cards
  - **Tracing** — Full instrumentation with `tracing` spans

  ## Installation

  ```toml
  [dependencies]
  azure_ai_foundry_core = "0.3"
  azure_ai_foundry_tools = "0.3"
  tokio = { version = "1", features = ["full"] }
  ```

  ## Usage

  ### Analyze an Image

  ```rust,no_run
  use azure_ai_foundry_core::client::FoundryClient;
  use azure_ai_foundry_core::auth::FoundryCredential;
  use azure_ai_foundry_tools::vision::{self, ImageAnalysisRequest, VisualFeature};

  # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  let client = FoundryClient::builder()
      .endpoint("https://your-resource.services.ai.azure.com")
      .credential(FoundryCredential::api_key("your-key"))
      .build()?;

  let request = ImageAnalysisRequest::builder()
      .url("https://example.com/image.jpg")
      .features(vec![VisualFeature::Tags, VisualFeature::Caption])
      .build()?;

  let result = vision::analyze(&client, &request).await?;
  if let Some(caption) = &result.caption_result {
      println!("Caption: {} ({:.1}%)", caption.text, caption.confidence * 100.0);
  }
  # Ok(())
  # }
  ```

  ### Analyze a Document

  ```rust,no_run
  use azure_ai_foundry_core::client::FoundryClient;
  use azure_ai_foundry_core::auth::FoundryCredential;
  use azure_ai_foundry_tools::document_intelligence::{
      self, DocumentAnalysisRequest, PREBUILT_READ,
  };

  # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  let client = FoundryClient::builder()
      .endpoint("https://your-resource.services.ai.azure.com")
      .credential(FoundryCredential::api_key("your-key"))
      .build()?;

  let request = DocumentAnalysisRequest::builder()
      .model_id(PREBUILT_READ)
      .url_source("https://example.com/document.pdf")
      .build()?;

  let operation = document_intelligence::analyze(&client, &request).await?;
  let result = document_intelligence::poll_until_complete(
      &client,
      &operation.operation_location,
      std::time::Duration::from_secs(2),
      60,
  ).await?;

  if let Some(ar) = &result.analyze_result {
      println!("Extracted text: {:?}", ar.content);
  }
  # Ok(())
  # }
  ```

  ## Supported Models (Document Intelligence)

  | Constant | Model ID | Purpose |
  |---|---|---|
  | `PREBUILT_READ` | `prebuilt-read` | General OCR |
  | `PREBUILT_LAYOUT` | `prebuilt-layout` | Layout with tables |
  | `PREBUILT_INVOICE` | `prebuilt-invoice` | Invoice extraction |
  | `PREBUILT_RECEIPT` | `prebuilt-receipt` | Receipt extraction |
  | `PREBUILT_ID_DOCUMENT` | `prebuilt-idDocument` | Passport / ID |
  | `PREBUILT_BUSINESS_CARD` | `prebuilt-businessCard` | Business card |

  ## License

  MIT — see [LICENSE](../../LICENSE).
  ```

  Also add a `readme` key to `Cargo.toml`:

  ```toml
  # sdk/azure_ai_foundry_tools/Cargo.toml
  readme = "README.md"
  ```

  Run `cargo build --workspace` — MUST compile.

- [ ] **REFACTOR** — None needed.

---

## Execution Order Summary

| Cycle | Finding | Severity | File | Time |
|-------|---------|----------|------|------|
| 1 | NaN/Infinity in aspect ratio | Critical | `vision.rs` | 15 min |
| 2 | `url()` getter | Critical | `vision.rs` | 10 min |
| 3 | `features_query_param` visibility | Optional | `vision.rs` | 5 min |
| 4 | Empty-string `url_source`/`base64_source` | Critical | `document_intelligence.rs` | 15 min |
| 5 | Wrong error variant for missing header | Recommended | `document_intelligence.rs` | 15 min |
| 6 | `error` field in `AnalyzeOperationResult` | Recommended | `document_intelligence.rs` | 20 min |
| 7 | `Display` for `AnalyzeResultStatus` | Optional | `document_intelligence.rs` | 10 min |
| 8 | `as_str()` vs serde sync tests | Recommended | both | 15 min |
| 9 | `DocumentAnalysisBody` visibility | Recommended | `document_intelligence.rs` | 5 min |
| 10 | `poll_until_complete` max_attempts | Critical | `document_intelligence.rs` | 30 min |
| 11 | `get_result` malformed URL test | Recommended | `document_intelligence.rs` | 10 min |
| 12 | Tracing field content test (vision) | Optional | `vision.rs` | 10 min |
| 13 | Tracing field content test (doc_intel) | Optional | `document_intelligence.rs` | 10 min |
| 14 | `README.md` and `Cargo.toml` readme key | Recommended | new file | 20 min |

**Total estimated time:** ~3 hours implementation + 30 min verification.

---

## Estimacion Total

- Implementacion: 2.5 hours
- Testing (unit): 30 min (integrated into each cycle)
- Documentation/README: 20 min

## Criterios de Exito

- [ ] `cargo test -p azure_ai_foundry_tools` — all tests pass (target: 42 existing + ~14 new = ~56)
- [ ] `cargo clippy -p azure_ai_foundry_tools --all-targets -- -D warnings` — zero warnings
- [ ] `cargo build --workspace` — zero errors
- [ ] `cargo doc --workspace --no-deps` — no broken doc links
- [ ] `f64::NAN` and `f64::INFINITY` rejected as aspect ratios
- [ ] Empty `url_source`/`base64_source` rejected
- [ ] Missing `Operation-Location` header returns `FoundryError::Api`, not `FoundryError::MissingConfig`
- [ ] `poll_until_complete` with `max_attempts = 3` returns error after 3 polls against a server
  that always returns `"running"`
- [ ] `AnalyzeOperationResult.error` is populated when status is `"failed"`
- [ ] `README.md` exists at `sdk/azure_ai_foundry_tools/README.md`
