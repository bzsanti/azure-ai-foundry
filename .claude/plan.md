# TDD Plan: v0.8.0 Quality Review Fixes

## Context

Quality review of `azure_ai_foundry_safety` crate identified 10 findings across 3 priority levels.
This plan covers all findings using strict TDD methodology (RED → GREEN → REFACTOR).

**Branch**: `feature/0.8.0`
**Baseline tests**: 795 (154 agents + 228 models + 152 core + 82 safety + 65 tools + 114 doc-tests)

---

## Finding 1 (Critical): `patch` method uses manual serialization

**File**: `sdk/azure_ai_foundry_core/src/client.rs:511-514`
**Problem**: `patch()` uses `serde_json::to_vec(body)` + manual `Content-Type` header + `FoundryError::Api` for serialization errors, while `post()` uses `.json(body)` (reqwest handles serialization + content-type automatically). The `patch` method should use `.json(body)` but override Content-Type to `application/merge-patch+json`.
**Risk**: Inconsistent error types for serialization failures, extra allocation from `to_vec` + `clone`.

### Cycle 1.1: RED — test that patch sends correct Content-Type header
- File: `sdk/azure_ai_foundry_core/src/client.rs` (test module)
- Test: `test_patch_sends_merge_patch_content_type`
  - Use wiremock `header("Content-Type", "application/merge-patch+json")` matcher
  - If the implementation changes to `.json(body)`, reqwest sets `application/json` by default
  - So the test must verify the final Content-Type is `application/merge-patch+json`
- Expected: test passes with current implementation (this is a regression guard)

### Cycle 1.2: GREEN — refactor `patch` to use `.json(body)` with Content-Type override
- Replace:
  ```rust
  let json_body = serde_json::to_vec(body).map_err(|e| FoundryError::Api { ... })?;
  // ...
  .header("Content-Type", "application/merge-patch+json")
  .body(json_body.clone())
  ```
- With:
  ```rust
  .json(body)
  .header("Content-Type", "application/merge-patch+json")
  ```
  Note: `.json(body)` sets Content-Type to `application/json`, then `.header()` overrides it.
- Remove the `serde_json::to_vec` block entirely
- Existing tests must still pass

### Cycle 1.3: REFACTOR — verify clippy + fmt
- `cargo clippy -p azure_ai_foundry_core -- -D warnings`
- `cargo fmt --all -- --check`
- Existing `test_patch_request_success` and `test_patch_request_error_propagation` must pass

**Tests added**: 1 new + 2 existing = 3 total covering patch

---

## Finding 2 (Critical): MAX_TEXT_LENGTH errors use wrong error variant

**Files**:
- `sdk/azure_ai_foundry_safety/src/text.rs:98` — `FoundryError::Builder` for text too long
- `sdk/azure_ai_foundry_safety/src/protected_material.rs:58` — same issue

**Problem**: Text length exceeding `MAX_TEXT_LENGTH` is a **runtime data validation** error (the user provided valid builder config but invalid data), not a builder configuration error. Should use `FoundryError::Validation { field, message }`.

### Cycle 2.1: RED — test that text too long returns Validation error
- File: `sdk/azure_ai_foundry_safety/src/text.rs`
- Modify existing `test_analyze_text_rejects_text_too_long`:
  ```rust
  let err = result.expect_err("should reject long text");
  assert!(matches!(err, FoundryError::Validation { .. }), "error: {err}");
  ```
- Expected: FAILS (current code returns `FoundryError::Builder`)

### Cycle 2.2: GREEN — change text.rs to use Validation
- File: `sdk/azure_ai_foundry_safety/src/text.rs:97-100`
- Replace:
  ```rust
  return Err(FoundryError::Builder(format!(...)));
  ```
- With:
  ```rust
  return Err(FoundryError::validation(format!(
      "text exceeds maximum length of {MAX_TEXT_LENGTH} characters"
  )));
  ```

### Cycle 2.3: RED — same fix for protected_material.rs
- File: `sdk/azure_ai_foundry_safety/src/protected_material.rs`
- Modify `test_protected_material_rejects_text_too_long`:
  ```rust
  assert!(matches!(err, FoundryError::Validation { .. }), "error: {err}");
  ```
- Expected: FAILS

### Cycle 2.4: GREEN — change protected_material.rs
- File: `sdk/azure_ai_foundry_safety/src/protected_material.rs:57-60`
- Same pattern as text.rs

### Cycle 2.5: REFACTOR
- Verify all existing tests still pass
- `cargo test -p azure_ai_foundry_safety`

**Tests modified**: 2 (strengthened assertions)

---

## Finding 3 (Recommended): Builders accept `Vec<T>` instead of `impl IntoIterator`

**Files**:
- `sdk/azure_ai_foundry_safety/src/text.rs:67` — `categories(Vec<HarmCategory>)`
- `sdk/azure_ai_foundry_safety/src/text.rs:73` — `blocklist_names(Vec<String>)`
- `sdk/azure_ai_foundry_safety/src/image.rs:69` — `categories(Vec<HarmCategory>)`
- `sdk/azure_ai_foundry_safety/src/prompt_shields.rs:53` — `documents(Vec<String>)`

**Problem**: Accepting `Vec<T>` forces callers to allocate a Vec even when they have an iterator, array, or slice. The pattern `impl IntoIterator<Item = T>` is more ergonomic and consistent with the agents/models crates.

### Cycle 3.1: RED — test that categories accepts arrays (not just Vec)
- File: `sdk/azure_ai_foundry_safety/src/text.rs`
- New test: `test_analyze_text_categories_accepts_array`
  ```rust
  let request = AnalyzeTextRequest::builder()
      .text("test")
      .categories([HarmCategory::Hate, HarmCategory::Violence])
      .build();
  let json = serde_json::to_value(&request).unwrap();
  assert_eq!(json["categories"], serde_json::json!(["Hate", "Violence"]));
  ```
- Expected: FAILS (array is not `Vec<HarmCategory>`)

### Cycle 3.2: GREEN — change text.rs builders to accept IntoIterator
- `text.rs:67`:
  ```rust
  pub fn categories(mut self, categories: impl IntoIterator<Item = HarmCategory>) -> Self {
      self.categories = Some(categories.into_iter().collect());
      self
  }
  ```
- `text.rs:73`:
  ```rust
  pub fn blocklist_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
      self.blocklist_names = Some(names.into_iter().map(Into::into).collect());
      self
  }
  ```

### Cycle 3.3: RED — test image.rs categories accepts array
- File: `sdk/azure_ai_foundry_safety/src/image.rs`
- New test: `test_analyze_image_categories_accepts_array`

### Cycle 3.4: GREEN — change image.rs
- Same pattern as text.rs

### Cycle 3.5: RED — test prompt_shields.rs documents accepts array
- File: `sdk/azure_ai_foundry_safety/src/prompt_shields.rs`
- New test: `test_shield_prompt_documents_accepts_array`

### Cycle 3.6: GREEN — change prompt_shields.rs
- ```rust
  pub fn documents(mut self, documents: impl IntoIterator<Item = impl Into<String>>) -> Self {
      self.documents = Some(documents.into_iter().map(Into::into).collect());
      self
  }
  ```

### Cycle 3.7: REFACTOR — verify all existing tests still pass
- Existing tests use `Vec<T>` which implements `IntoIterator`, so no breakage expected.

**Tests added**: 3 new

---

## Finding 4 (Recommended): Missing length validations in blocklist builders

**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`
**Problem**: Doc comments mention max lengths (name: 64, description: 1024, item text: 128) but builders don't enforce them. The API will reject oversized values, but client-side validation provides better UX and is consistent with `MAX_TEXT_LENGTH` validation in text.rs.

### Cycle 4.1: RED — test blocklist name max length
- File: `sdk/azure_ai_foundry_safety/src/blocklist.rs`
- New test: `test_blocklist_upsert_rejects_name_too_long`
  ```rust
  let long_name = "a".repeat(65);
  let err = BlocklistUpsertRequest::builder()
      .blocklist_name(long_name)
      .try_build()
      .expect_err("should reject name > 64 chars");
  assert!(matches!(err, FoundryError::Validation { .. }));
  ```
- Expected: FAILS (no length check)

### Cycle 4.2: GREEN — add name length validation
- File: `sdk/azure_ai_foundry_safety/src/blocklist.rs`
- In `BlocklistUpsertRequestBuilder::try_build()`, after the empty check:
  ```rust
  const MAX_BLOCKLIST_NAME_LENGTH: usize = 64;
  if blocklist_name.chars().count() > MAX_BLOCKLIST_NAME_LENGTH {
      return Err(FoundryError::validation(format!(
          "blocklist_name exceeds maximum length of {MAX_BLOCKLIST_NAME_LENGTH} characters"
      )));
  }
  ```

### Cycle 4.3: RED — test description max length
- New test: `test_blocklist_upsert_rejects_description_too_long`
  ```rust
  let long_desc = "a".repeat(1025);
  let err = BlocklistUpsertRequest::builder()
      .blocklist_name("valid")
      .description(long_desc)
      .try_build()
      .expect_err("should reject description > 1024 chars");
  assert!(matches!(err, FoundryError::Validation { .. }));
  ```

### Cycle 4.4: GREEN — add description length validation
- ```rust
  const MAX_DESCRIPTION_LENGTH: usize = 1024;
  if let Some(ref desc) = self.description {
      if desc.chars().count() > MAX_DESCRIPTION_LENGTH {
          return Err(FoundryError::validation(format!(
              "description exceeds maximum length of {MAX_DESCRIPTION_LENGTH} characters"
          )));
      }
  }
  ```

### Cycle 4.5: RED — test item text max length
- New test: `test_blocklist_item_rejects_text_too_long`
  ```rust
  let long_text = "a".repeat(129);
  let err = BlocklistItemInput::builder()
      .text(long_text)
      .try_build()
      .expect_err("should reject text > 128 chars");
  assert!(matches!(err, FoundryError::Validation { .. }));
  ```

### Cycle 4.6: GREEN — add item text length validation
- In `BlocklistItemInputBuilder::try_build()`:
  ```rust
  const MAX_ITEM_TEXT_LENGTH: usize = 128;
  if text.chars().count() > MAX_ITEM_TEXT_LENGTH {
      return Err(FoundryError::validation(format!(
          "text exceeds maximum length of {MAX_ITEM_TEXT_LENGTH} characters"
      )));
  }
  ```

### Cycle 4.7: RED — test boundary values (accepted)
- New tests:
  - `test_blocklist_upsert_accepts_name_at_boundary` — 64 chars => Ok
  - `test_blocklist_item_accepts_text_at_boundary` — 128 chars => Ok

### Cycle 4.8: GREEN — already passes (boundary is <=, not <)

### Cycle 4.9: REFACTOR
- Extract constants to top of file with doc comments
- Verify all existing tests pass

**Tests added**: 5 new

---

## Finding 5 (Recommended): `ImageOutputType` single-variant enum

**File**: `sdk/azure_ai_foundry_safety/src/models.rs:84-98`
**Problem**: `ImageOutputType` has only one variant (`FourSeverityLevels`). The `output_type` builder method in image.rs is effectively useless since there's only one valid value. The API may add more variants in the future, so keeping the enum is fine, but the builder should document this.

### Cycle 5.1: RED — test that image output_type defaults correctly
- File: `sdk/azure_ai_foundry_safety/src/image.rs`
- New test: `test_analyze_image_output_type_absent_by_default`
  ```rust
  let request = AnalyzeImageRequest::builder()
      .blob_url("https://example.com/img.jpg")
      .build();
  let json = serde_json::to_value(&request).unwrap();
  assert!(json.get("outputType").is_none(), "outputType should be absent");
  ```
- Expected: PASSES (already correct behavior)

### Cycle 5.2: GREEN — add doc comment to output_type builder method
- File: `sdk/azure_ai_foundry_safety/src/image.rs:74-78`
- Update doc:
  ```rust
  /// Sets the output type for image analysis.
  ///
  /// Currently only `FourSeverityLevels` is supported by the API.
  /// This field is optional and defaults to `FourSeverityLevels` when omitted.
  ```

### Cycle 5.3: REFACTOR — no code change needed

**Tests added**: 1 new (guard test)

---

## Finding 6 (Recommended): `CONTENT_SAFETY_API_VERSION` embeds param name

**File**: `sdk/azure_ai_foundry_safety/src/models.rs:6`
**Problem**: `CONTENT_SAFETY_API_VERSION = "api-version=2024-09-01"` embeds the query parameter name in the constant. This couples the constant to URL query string usage. Better to split into version-only constant and use it explicitly in format strings.

### Cycle 6.1: RED — test new constant exists
- File: `sdk/azure_ai_foundry_safety/src/models.rs`
- New test: `test_content_safety_version_value`
  ```rust
  assert_eq!(CONTENT_SAFETY_VERSION, "2024-09-01");
  ```
- Expected: FAILS (constant doesn't exist)

### Cycle 6.2: GREEN — add version-only constant, update API version constant
- File: `sdk/azure_ai_foundry_safety/src/models.rs`
  ```rust
  /// Content Safety API version.
  pub(crate) const CONTENT_SAFETY_VERSION: &str = "2024-09-01";

  /// API version query parameter for Content Safety API requests.
  pub(crate) const CONTENT_SAFETY_API_VERSION: &str = "api-version=2024-09-01";
  ```
  Keep `CONTENT_SAFETY_API_VERSION` for backwards compatibility — all callers use it in
  `format!("/path?{CONTENT_SAFETY_API_VERSION}")`. The new `CONTENT_SAFETY_VERSION` is
  available for any future use that needs just the version string.

### Cycle 6.3: REFACTOR — update existing test
- Modify `test_api_version_constant_value` to also assert the new constant.

**Tests added**: 1 modified

---

## Finding 7 (Recommended): Doc comments on `patch` method may be incomplete

**File**: `sdk/azure_ai_foundry_core/src/client.rs:477-497`
**Problem**: The `patch` doc comment doesn't mention the `application/merge-patch+json` Content-Type in the description. Users need to know this is not a regular JSON POST.

### Cycle 7.1: No test needed — documentation only
- File: `sdk/azure_ai_foundry_core/src/client.rs`
- Update the doc comment for `patch`:
  ```rust
  /// Send a PATCH request with a JSON body using `application/merge-patch+json` content type.
  ///
  /// Uses [RFC 7396 Merge Patch](https://tools.ietf.org/html/rfc7396) semantics.
  /// Automatically adds authentication headers and API version.
  /// Retries on retriable HTTP errors (429, 500, 502, 503, 504) with exponential backoff.
  ```

**Tests added**: 0

---

## Finding 8 (Optional): Tracing test flakiness risk

**Problem**: Tracing tests using `#[tracing_test::traced_test]` can be flaky in multi-threaded test runs due to global subscriber conflicts. This is a known issue (see `test_resolve_emits_auth_span` in core).

### Action: No code change
- This is a known limitation documented in MEMORY.md
- The safety crate's tracing tests are simpler (just `logs_contain(span_name)`) and less prone to conflicts
- Monitor for flakiness; if it appears, add `#[serial_test::serial]` attribute

**Tests added**: 0

---

## Finding 9 (Optional): Inconsistent public accessors

**Problem**: `ProtectedMaterialRequest::text()` and `ShieldPromptRequest::user_prompt()` and `AnalyzeTextRequest::text()` are exposed as public accessors, but `AnalyzeImageRequest` doesn't expose its fields. Minor inconsistency.

### Cycle 9.1: RED — test accessor exists
- File: `sdk/azure_ai_foundry_safety/src/image.rs`
- New test: `test_analyze_image_request_accessors`
  ```rust
  let request = AnalyzeImageRequest::builder()
      .blob_url("https://example.com/img.jpg")
      .build();
  // No accessor to test — this cycle adds one
  ```

### Cycle 9.2: GREEN — add accessors to AnalyzeImageRequest
- File: `sdk/azure_ai_foundry_safety/src/image.rs`
  ```rust
  impl AnalyzeImageRequest {
      /// Returns whether this request uses base64 content.
      pub fn has_base64_content(&self) -> bool {
          self.image.content.is_some()
      }

      /// Returns whether this request uses a blob URL.
      pub fn has_blob_url(&self) -> bool {
          self.image.blob_url.is_some()
      }
  }
  ```

**Tests added**: 1 new

---

## Finding 10 (Optional): `remove_blocklist_items(&[&str])` ergonomics

**Problem**: `remove_blocklist_items` takes `&[&str]` for item IDs. This works but is less ergonomic than `impl IntoIterator<Item = impl AsRef<str>>`.

### Cycle 10.1: RED — test that remove accepts String vec
- File: `sdk/azure_ai_foundry_safety/src/blocklist.rs`
- New test: `test_remove_blocklist_items_accepts_string_vec`
  - This tests calling with `&["id1".to_string(), "id2".to_string()]` — currently fails
    because `&[String]` doesn't coerce to `&[&str]`.

### Cycle 10.2: GREEN — change signature
- Change from `ids: &[&str]` to `ids: impl IntoIterator<Item = impl AsRef<str>>`:
  ```rust
  pub async fn remove_blocklist_items(
      client: &FoundryClient,
      name: &str,
      ids: impl IntoIterator<Item = impl AsRef<str>>,
  ) -> FoundryResult<()> {
      let id_strings: Vec<String> = ids.into_iter().map(|s| s.as_ref().to_string()).collect();
      if id_strings.is_empty() {
          return Err(FoundryError::validation("blocklist_item_ids must not be empty"));
      }
      // ...
  }
  ```

### Cycle 10.3: REFACTOR — verify existing callers still work
- Existing tests use `&["id1", "id2"]` which implements `IntoIterator<Item = &&str>` — `&&str` implements `AsRef<str>`, so no breakage.

**Tests added**: 1 new

---

## Summary Table

| Finding | Priority | Scope | New Tests | Modified Tests |
|---------|----------|-------|-----------|----------------|
| F1 | Critical | `client.rs` patch method | 1 | 0 |
| F2 | Critical | text.rs + protected_material.rs error variants | 0 | 2 |
| F3 | Recommended | IntoIterator on builders | 3 | 0 |
| F4 | Recommended | Blocklist length validations | 5 | 0 |
| F5 | Recommended | ImageOutputType documentation | 1 | 0 |
| F6 | Recommended | API version constant split | 0 | 1 |
| F7 | Recommended | patch doc comments | 0 | 0 |
| F8 | Optional | Tracing flakiness | 0 | 0 |
| F9 | Optional | Image request accessors | 1 | 0 |
| F10 | Optional | remove_blocklist_items ergonomics | 1 | 0 |
| **Total** | | | **12** | **3** |

## Execution Order

```
F1 (patch refactor — core crate, no deps)
  |
  +-- F7 (patch docs — same file, do together with F1)
  |
F2 (error variant fix — safety crate, independent)
  |
F3 (IntoIterator — safety crate, independent)
  |
F4 (length validations — safety crate, independent)
  |
F5 (ImageOutputType docs — safety crate, independent)
  |
F6 (version constant — safety crate, independent)
  |
F9 (image accessors — safety crate, independent)
  |
F10 (remove ergonomics — safety crate, independent)
```

F1+F7 first (core crate), then F2-F6, F9-F10 (safety crate, can be done in any order).
F8 is monitor-only, no action needed.

## Estimated Impact

- **New tests**: ~12
- **Modified tests**: ~3
- **Expected total after fixes**: ~807+ tests
- **Implementation time**: ~2 hours
- **Risk**: Low — all changes are localized, no architectural impact

## Success Criteria

- [ ] `cargo build --workspace` — no errors, no warnings
- [ ] `cargo test --workspace` — all tests pass (807+)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --all -- --check` — clean
- [ ] `patch` method uses `.json(body)` consistently with `post`
- [ ] Text length errors use `FoundryError::Validation`, not `Builder`
- [ ] All builder methods accepting collections use `impl IntoIterator`
- [ ] Blocklist builders enforce documented max lengths
- [ ] No regressions in existing 795 tests
