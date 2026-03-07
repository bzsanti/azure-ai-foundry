# TDD Plan: v0.8.0 Quality Review Round 2

## Context

Second quality review of `azure_ai_foundry_safety` crate identified 11 findings (2 critical, 6 recommended, 3 optional).
This plan covers all findings using strict TDD methodology (RED -> GREEN -> REFACTOR).

**Branch**: `feature/0.8.0`
**Baseline tests**: 808 (154 agents + 228 models + 153 core + 94 safety + 65 tools + 114 doc-tests)

---

## Execution Order (dependency graph)

```
F8 (centralize constants in models.rs)
 |
 +-> F3 (description validation in BlocklistItemInput — uses constants from F8)
 |
 +-> F1 (remove blocklist_name from body — uses constants from F8)
      |
      +-> F6 (reorder validation in remove_blocklist_items — same file, same context)

F2 (remove encode_query_value + url dep — independent)

F4+F11 (api-version query param tests — independent)

F5 (traced_test for blocklist — independent)

F7 (#[non_exhaustive] on ImageOutputType — independent)

F9 (doc next_link — independent)

F10 (PartialEq/Eq derives — independent)
```

**Order**: F8 -> F3 -> F1 -> F6 -> F2 -> F4+F11 -> F5 -> F7 -> F9 -> F10

---

## F8 — Centralize limit constants in models.rs

**Problem**: `MAX_TEXT_LENGTH` duplicated between `text.rs:13` and `protected_material.rs:13`. Blocklist constants are local to `try_build` bodies.
**Files**: `models.rs`, `text.rs`, `protected_material.rs`, `blocklist.rs`

### Cycle 8.1: RED — test constants exist in models.rs
- File: `sdk/azure_ai_foundry_safety/src/models.rs`
- Test: `test_limit_constants`
  ```rust
  assert_eq!(MAX_TEXT_LENGTH, 10_000);
  assert_eq!(MAX_BLOCKLIST_NAME_LENGTH, 64);
  assert_eq!(MAX_DESCRIPTION_LENGTH, 1_024);
  assert_eq!(MAX_ITEM_TEXT_LENGTH, 128);
  ```
- Expected: FAILS (constants don't exist in models.rs)

### Cycle 8.2: GREEN — add constants to models.rs
- File: `sdk/azure_ai_foundry_safety/src/models.rs`
  ```rust
  pub(crate) const MAX_TEXT_LENGTH: usize = 10_000;
  pub(crate) const MAX_BLOCKLIST_NAME_LENGTH: usize = 64;
  pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 1_024;
  pub(crate) const MAX_ITEM_TEXT_LENGTH: usize = 128;
  ```

### Cycle 8.3: REFACTOR — remove local constants, update imports
- `text.rs`: remove line 13 `const MAX_TEXT_LENGTH`, add to `use crate::models::{..., MAX_TEXT_LENGTH}`
- `protected_material.rs`: remove line 13 `const MAX_TEXT_LENGTH`, add to `use crate::models::{..., MAX_TEXT_LENGTH}`
- `blocklist.rs`: remove `const MAX_BLOCKLIST_NAME_LENGTH/MAX_DESCRIPTION_LENGTH/MAX_ITEM_TEXT_LENGTH` from inside `try_build` bodies, add to `use crate::models::{..., MAX_BLOCKLIST_NAME_LENGTH, MAX_DESCRIPTION_LENGTH, MAX_ITEM_TEXT_LENGTH}`
- Verify: all existing tests pass unchanged

**Tests**: +1 new

---

## F3 — Description max-length validation in BlocklistItemInput

**Problem**: `BlocklistItemInput.description` has no max-length validation (1024 chars) but `BlocklistUpsertRequest.description` does.
**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`

### Cycle 3.1: RED — test description too long
- Test: `test_blocklist_item_description_rejects_too_long`
  ```rust
  let long_desc = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
  let err = BlocklistItemInput::builder()
      .text("badword")
      .description(long_desc)
      .try_build()
      .expect_err("should reject description > 1024 chars");
  assert!(matches!(err, FoundryError::Validation { .. }));
  assert!(err.to_string().contains("description"));
  ```
- Expected: FAILS (no validation on description)

### Cycle 3.2: GREEN — add description validation to BlocklistItemInputBuilder::try_build
- Add after text length check:
  ```rust
  if let Some(ref desc) = self.description {
      if desc.chars().count() > MAX_DESCRIPTION_LENGTH {
          return Err(FoundryError::validation(format!(
              "description exceeds maximum length of {MAX_DESCRIPTION_LENGTH} characters"
          )));
      }
  }
  ```

### Cycle 3.3: RED — test boundary value accepted
- Test: `test_blocklist_item_description_accepts_boundary`
  ```rust
  let boundary_desc = "a".repeat(MAX_DESCRIPTION_LENGTH);
  let result = BlocklistItemInput::builder()
      .text("badword")
      .description(boundary_desc)
      .try_build();
  assert!(result.is_ok());
  ```
- Expected: PASSES (boundary <= 1024)

**Tests**: +2 new

---

## F1 — Remove blocklist_name from BlocklistUpsertRequest body (CRITICAL)

**Problem**: `BlocklistUpsertRequest` serializes `blocklist_name` in the body, but the Azure API expects it ONLY as a URL path parameter. The `create_or_update_blocklist` function already takes `name: &str` for the URL.
**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`

### Cycle 1.1: RED — test body does NOT contain blocklistName
- Test: `test_blocklist_upsert_body_does_not_contain_blocklist_name`
  ```rust
  let request = BlocklistUpsertRequest::builder()
      .description("My filter")
      .try_build()
      .unwrap();
  let json = serde_json::to_value(&request).unwrap();
  assert!(json.get("blocklistName").is_none(),
      "blocklistName must not be in body, got: {json}");
  ```
- Expected: FAILS (current struct serializes blocklistName)

### Cycle 1.2: GREEN — restructure BlocklistUpsertRequest
- Remove `blocklist_name` field from struct
- Remove `blocklist_name` field and method from builder
- Builder only has `description` (with existing max-length validation)
- Move name validation (max 64 chars) to `create_or_update_blocklist` function:
  ```rust
  pub async fn create_or_update_blocklist(...) {
      FoundryClient::validate_resource_id(name)?;
      if name.chars().count() > MAX_BLOCKLIST_NAME_LENGTH {
          return Err(FoundryError::validation(format!(
              "blocklist name exceeds maximum length of {MAX_BLOCKLIST_NAME_LENGTH} characters"
          )));
      }
      // ...
  }
  ```

### Cycle 1.3: RED — test name validation moved to API function
- Test: `test_create_or_update_blocklist_rejects_name_too_long`
  ```rust
  let long_name = "a".repeat(MAX_BLOCKLIST_NAME_LENGTH + 1);
  let request = BlocklistUpsertRequest::builder().build();
  let err = create_or_update_blocklist(&client, &long_name, &request)
      .await
      .expect_err("should reject name > 64 chars");
  assert!(matches!(err, FoundryError::Validation { .. }));
  ```
- Test: `test_create_or_update_blocklist_accepts_name_at_boundary`

### Cycle 1.4: REFACTOR — update all affected tests
- Remove obsolete builder tests: `test_blocklist_upsert_requires_name`, `test_blocklist_upsert_rejects_blank_name`, `test_blocklist_upsert_rejects_name_too_long`, `test_blocklist_upsert_accepts_name_at_boundary`
- Update tests that used `.blocklist_name("...")`: `test_blocklist_upsert_accepts_description_none`, `test_blocklist_upsert_accepts_description_some`, `test_create_or_update_blocklist_success`, `test_create_or_update_blocklist_rejects_path_traversal`
- Update doc example in `create_or_update_blocklist`

**Tests**: +3 new, -4 removed = -1 net

---

## F6 — Reorder validation in remove_blocklist_items

**Problem**: Validates blocklist name AFTER consuming the iterator. Should validate name first.
**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`

### Cycle 6.1: RED — test name validated before empty check
- Test: `test_remove_blocklist_items_validates_name_before_empty_check`
  ```rust
  let empty: &[&str] = &[];
  let err = remove_blocklist_items(&client, "bad/name", empty)
      .await
      .expect_err("should fail");
  // After fix: error should be about invalid name, not empty ids
  assert!(
      err.to_string().contains("invalid") || err.to_string().contains("slash"),
      "error should mention invalid name, got: {err}"
  );
  ```
- Expected: FAILS (currently returns "item_ids must not be empty")

### Cycle 6.2: GREEN — move validate_resource_id before iterator consumption
  ```rust
  pub async fn remove_blocklist_items(...) {
      FoundryClient::validate_resource_id(blocklist_name)?; // FIRST
      let id_strings: Vec<String> = item_ids.into_iter()...collect();
      if id_strings.is_empty() { ... }
      // ...
  }
  ```

**Tests**: +1 new

---

## F2 — Remove encode_query_value and url dependency (CRITICAL)

**Problem**: `encode_query_value` in `lib.rs` is dead code with `#[allow(dead_code)]`. The `url` dep exists only for it.
**Files**: `lib.rs`, `Cargo.toml`

### Cycle 2.1: GREEN — remove dead code
- File: `sdk/azure_ai_foundry_safety/src/lib.rs` — remove `encode_query_value` function and its `#[cfg(test)] mod tests`
- File: `sdk/azure_ai_foundry_safety/Cargo.toml` — remove `url.workspace = true`
- Verify: `cargo clippy -p azure_ai_foundry_safety -- -D warnings` passes

**Tests**: -1 removed (test_encode_query_value_encodes_spaces)

---

## F4+F11 — api-version query param tests

**Problem**: No test verifies `api-version=2024-09-01` is actually sent as query param. The existing `test_api_version_constant_value` is trivial.
**Files**: `text.rs`, `image.rs`, `prompt_shields.rs`, `protected_material.rs`, `blocklist.rs`, `models.rs`

### Cycle 4.1: RED — test api-version in text module
- Test: `test_analyze_text_sends_api_version_query_param`
  ```rust
  use wiremock::matchers::query_param;
  Mock::given(method("POST"))
      .and(path("/contentsafety/text:analyze"))
      .and(query_param("api-version", "2024-09-01"))
      .respond_with(...)
      .expect(1)
      .mount(&server).await;
  ```
- Expected: PASSES (code already sends the param)

### Cycle 4.2-4.5: Same pattern for image, prompt_shields, protected_material, blocklist
- `image.rs`: path `/contentsafety/image:analyze`
- `prompt_shields.rs`: path `/contentsafety/text:shieldPrompt`
- `protected_material.rs`: path `/contentsafety/text:detectProtectedMaterial`
- `blocklist.rs`: path `/contentsafety/text/blocklists/test-list`, method PATCH

### Cycle 4.6: Replace trivial test in models.rs
- Remove `test_api_version_constant_value`
- Replace with `test_api_version_constant_has_correct_format`:
  ```rust
  assert!(CONTENT_SAFETY_API_VERSION.starts_with("api-version="));
  let version = CONTENT_SAFETY_API_VERSION.strip_prefix("api-version=").unwrap();
  assert_eq!(version.len(), 10); // YYYY-MM-DD
  assert_eq!(version, "2024-09-01");
  ```

**Tests**: +5 new, -1 replaced = +4 net

---

## F5 — traced_test for blocklist operations

**Problem**: All other modules have traced_test but blocklist has none.
**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`

### Cycle 5.1: GREEN — add traced_test for create_or_update_blocklist
- Test: `test_create_or_update_blocklist_emits_span`
  ```rust
  #[tokio::test]
  #[tracing_test::traced_test]
  async fn test_create_or_update_blocklist_emits_span() {
      // ... setup mock, call function
      assert!(logs_contain("foundry::safety::create_or_update_blocklist"));
  }
  ```

**Tests**: +1 new

---

## F7 — Add #[non_exhaustive] to ImageOutputType

**Problem**: Single-variant enum without forward-compatibility marker.
**File**: `sdk/azure_ai_foundry_safety/src/models.rs`

### Cycle 7.1: GREEN — add attribute and doc
- Add `#[non_exhaustive]` to `ImageOutputType` enum
- Update doc comment with rationale about future API versions

### Cycle 7.2: Test documenting the variant
- Test: `test_image_output_type_is_non_exhaustive`
  ```rust
  let ot = ImageOutputType::FourSeverityLevels;
  assert_eq!(ot.as_str(), "FourSeverityLevels");
  ```

**Tests**: +1 new

---

## F9 — Document next_link pagination fields

**Problem**: `next_link` fields exist but no documentation on usage pattern.
**File**: `sdk/azure_ai_foundry_safety/src/blocklist.rs`

### Cycle 9.1: GREEN — add doc comments
- Update `BlocklistList` and `BlocklistItemList` doc comments with pagination pattern
- Update `next_link` field doc comments

**Tests**: 0

---

## F10 — Add PartialEq/Eq derives to types

**Problem**: Request/response types lack PartialEq/Eq, preventing direct comparison.
**Files**: `text.rs`, `image.rs`, `blocklist.rs`, `prompt_shields.rs`, `protected_material.rs`

### Cycle 10.1: RED — test PartialEq on request types
- Test in `text.rs`: `test_analyze_text_request_partial_eq`
  ```rust
  let r1 = AnalyzeTextRequest::builder().text("hello").build();
  let r2 = AnalyzeTextRequest::builder().text("hello").build();
  assert_eq!(r1, r2);
  ```
- Test in `blocklist.rs`: `test_blocklist_object_partial_eq`
- Expected: FAILS (no PartialEq)

### Cycle 10.2: GREEN — add derives
- Add `PartialEq, Eq` to all request/response structs
- Also add to private `ImageSource` (contained by `AnalyzeImageRequest`)

**Tests**: +2 new

---

## Summary Table

| Finding | Priority | New Tests | Removed Tests | Net |
|---------|----------|-----------|---------------|-----|
| F8 | Refactor | +1 | 0 | +1 |
| F3 | Recommended | +2 | 0 | +2 |
| F1 | Critical | +3 | -4 | -1 |
| F6 | Recommended | +1 | 0 | +1 |
| F2 | Critical | 0 | -1 | -1 |
| F4+F11 | Recommended | +5 | -1 | +4 |
| F5 | Recommended | +1 | 0 | +1 |
| F7 | Recommended | +1 | 0 | +1 |
| F9 | Optional | 0 | 0 | 0 |
| F10 | Optional | +2 | 0 | +2 |
| **Total** | | **+16** | **-6** | **+10** |

**Expected total after fixes**: ~818 tests

## Success Criteria

- [ ] `cargo build --workspace` — no errors, no warnings
- [ ] `cargo test --workspace` — all tests pass (818+)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --all -- --check` — clean
- [ ] `BlocklistUpsertRequest` body does NOT contain `blocklistName`
- [ ] All limit constants centralized in `models.rs`
- [ ] `encode_query_value` and `url` dependency removed
- [ ] `api-version` query param verified by at least one test per module
- [ ] No regressions in existing tests
