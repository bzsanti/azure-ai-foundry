# TDD Plan: Tracing Instrumentation

## Overview

This plan adds structured observability to the Azure AI Foundry Rust SDK via proper
`tracing` instrumentation. The `tracing` crate is already declared as a workspace
dependency in both crates (`azure_ai_foundry_core` and `azure_ai_foundry_models`),
but zero instrumentation exists in the source code today — no `#[tracing::instrument]`
attributes, no span creation, no event macros.

The goal is to instrument every significant operation so that operators running the
SDK can plug in any `tracing` subscriber (Jaeger, OpenTelemetry, JSON logs, etc.) and
get structured traces with zero changes to their application code.

**Stack**: Rust 1.88, `tracing 0.1`, `tokio` async runtime, `reqwest` HTTP client,
`wiremock` + `tracing-test` for test assertions.

**Convenciones observadas en el proyecto**:
- Tests live in `#[cfg(test)] mod tests` inside each source file (same-file tests).
- Builder pattern with `try_build()` / `build()`.
- Errors via `thiserror`, sensitive data protected by `secrecy`.
- `wiremock` for HTTP mocking, `serial_test` for env-var isolation.
- `async-trait` for mock trait impls in auth tests.

**Afecta hot path**: No. Tracing spans add microsecond-level overhead per call;
the SDK hot path is network I/O (milliseconds to seconds). No throughput benchmarks
are required.

---

## Decisions Needed Before Implementation

None. All decisions are clear from the codebase inspection:

- Use `#[tracing::instrument]` with `skip(...)` to exclude sensitive args.
- Use `tracing::debug!` / `tracing::warn!` / `tracing::error!` for events within spans.
- The test crate for span assertions will be `tracing-test 0.2` (dev-dependency only).
- Span names will follow the pattern `foundry::<module>::<operation>` for consistency
  with OpenTelemetry conventions.

---

## Execution Plan

### Phase 1: Test Infrastructure

#### Cycle 1: Add `tracing-test` to dev-dependencies

- **RED**: Write test `test_tracing_subscriber_captures_spans` in
  `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`
  that uses the `#[traced_test]` attribute macro and asserts that a named span is
  emitted when `FoundryClient::get()` is called.
  - File: `sdk/azure_ai_foundry_core/src/client.rs`
  - Assert: `logs_contain("foundry::client::get")` — fails because no instrumentation exists.
  - The test compiles but the assertion fails (RED state confirmed by `cargo test`).

- **GREEN**: Add `tracing-test = "0.2"` to the `[dev-dependencies]` section in
  `sdk/azure_ai_foundry_core/Cargo.toml`.
  - File: `sdk/azure_ai_foundry_core/Cargo.toml`
  - Action: Add `tracing-test = "0.2"` entry.
  - This alone does not make the test pass (instrumentation still missing). The test
    will only pass after Cycle 4.

- **REFACTOR**: None — this cycle is purely additive.

---

### Phase 2: Core Crate Instrumentation (`azure_ai_foundry_core`)

#### Cycle 2: Instrument `FoundryCredential::resolve()`

- **RED**: Write test `test_resolve_emits_auth_span` in
  `sdk/azure_ai_foundry_core/src/auth.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `FoundryCredential::api_key("test-key")`
  - Action: Call `credential.resolve().await`
  - Assert: `logs_contain("foundry::auth::resolve")`
  - Fails because `resolve()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::auth::resolve", skip(self), fields(credential_type))]`
  to `FoundryCredential::resolve()` in `sdk/azure_ai_foundry_core/src/auth.rs`.
  - Inside the function, record the credential type as a span field:
    `tracing::Span::current().record("credential_type", match self { Self::ApiKey(_) => "api_key", ... });`
  - Do NOT log the key value or token value anywhere.

- **REFACTOR**: Extract the credential type string to a private helper method
  `credential_type_name(&self) -> &'static str` to avoid repeating the `match`
  in multiple instrumented functions.

---

#### Cycle 3: Instrument `FoundryClient::post()` — sync path

- **RED**: Write test `test_post_emits_http_span` in
  `sdk/azure_ai_foundry_core/src/client.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning 200 OK.
  - Action: Call `client.post("/test", &body).await`
  - Assert: `logs_contain("foundry::client::post")`
  - Fails because `post()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::client::post", skip(self, body), fields(path, attempt, status_code))]`
  to `FoundryClient::post()` in `sdk/azure_ai_foundry_core/src/client.rs`.
  - Record `path` at span entry.
  - Record `attempt` and `status_code` inside the retry loop after each response.
  - Emit `tracing::debug!("http.request.sent", attempt = attempt)` before each send.
  - Emit `tracing::warn!("http.request.retry", status = status, attempt = attempt)`
    when a retriable status is encountered.
  - Do NOT log the `Authorization` header value.

- **REFACTOR**: Extract the retry backoff calculation into a private function
  `compute_backoff(attempt: u32, initial_backoff: Duration) -> Duration` to reduce
  duplication between `post()`, `get()`, and `post_stream()` — three copies of the
  same jitter logic exist today. This is a correctness improvement that also simplifies
  later instrumentation cycles.

---

#### Cycle 4: Instrument `FoundryClient::get()`

- **RED**: Write test `test_get_emits_http_span` in
  `sdk/azure_ai_foundry_core/src/client.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning 200 OK.
  - Action: Call `client.get("/test").await`
  - Assert: `logs_contain("foundry::client::get")`
  - This is the test written in Cycle 1 (now also causes Cycle 1 test to pass).

- **GREEN**: Add `#[tracing::instrument(name = "foundry::client::get", skip(self), fields(path, attempt, status_code))]`
  to `FoundryClient::get()` in `sdk/azure_ai_foundry_core/src/client.rs`.
  - Same event pattern as `post()`.
  - Use `compute_backoff()` refactored in Cycle 3.

- **REFACTOR**: None.

---

#### Cycle 5: Instrument `FoundryClient::post_stream()` — streaming path

- **RED**: Write test `test_post_stream_emits_http_span` in
  `sdk/azure_ai_foundry_core/src/client.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning SSE response with `content-type: text/event-stream`.
  - Action: Call `client.post_stream("/test", &body).await`
  - Assert: `logs_contain("foundry::client::post_stream")`
  - Fails because `post_stream()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::client::post_stream", skip(self, body), fields(path, attempt, status_code))]`
  to `FoundryClient::post_stream()`.
  - Record `streaming_timeout_secs` as a span field for diagnostics.
  - Emit `tracing::debug!("stream.started")` on success response.
  - Use `compute_backoff()`.

- **REFACTOR**: None.

---

#### Cycle 6: Instrument error paths — sensitive data must not leak

- **RED**: Write test `test_error_events_do_not_contain_bearer_tokens` in
  `sdk/azure_ai_foundry_core/src/client.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning 401 with body containing `Bearer sk-secret123`.
  - Action: Call `client.get("/test").await` (expect `Err`)
  - Assert: `!logs_contain("sk-secret123")` — the raw token must never appear in logs.
  - Fails if any event emits unsanitized error bodies.

- **GREEN**: Ensure all `tracing::error!` and `tracing::warn!` events in the error path
  pass the body through `FoundryClient::sanitize_error_message()` before recording.
  - File: `sdk/azure_ai_foundry_core/src/client.rs`
  - `check_response()` already sanitizes via `truncate_message()`. Verify that the
    tracing events added in Cycles 3, 4, 5 record the sanitized body, not the raw one.

- **REFACTOR**: Consolidate error logging into a single private helper
  `fn emit_error_event(status: u16, raw_body: &str)` that sanitizes and then emits
  the tracing event. Call it from `check_response()`.

---

### Phase 3: Models Crate Instrumentation (`azure_ai_foundry_models`)

#### Cycle 7: Instrument `chat::complete()`

- **RED**: Write test `test_complete_emits_chat_span` in
  `sdk/azure_ai_foundry_models/src/chat.rs`.
  - Add `tracing-test = "0.2"` to `sdk/azure_ai_foundry_models/Cargo.toml` dev-dependencies.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning a valid `ChatCompletionResponse`.
  - Action: Call `complete(&client, &request).await`
  - Assert: `logs_contain("foundry::chat::complete")`
  - Fails because `complete()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::chat::complete", skip(client, request), fields(model, prompt_tokens, completion_tokens))]`
  to `complete()` in `sdk/azure_ai_foundry_models/src/chat.rs`.
  - Record `model = request.model` at span entry.
  - After the response is deserialized, record token usage:
    `span.record("prompt_tokens", response.usage.as_ref().map(|u| u.prompt_tokens));`
  - Do NOT record message content (user data).

- **REFACTOR**: None.

---

#### Cycle 8: Instrument `chat::complete_stream()` — stream initiation span

- **RED**: Write test `test_complete_stream_emits_chat_stream_span` in
  `sdk/azure_ai_foundry_models/src/chat.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning SSE with `data: [DONE]`.
  - Action: Call `complete_stream(&client, &request).await` (just initiation, not consuming).
  - Assert: `logs_contain("foundry::chat::complete_stream")`
  - Fails because `complete_stream()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::chat::complete_stream", skip(client, request), fields(model))]`
  to `complete_stream()` in `sdk/azure_ai_foundry_models/src/chat.rs`.
  - Record `model = request.model` at span entry.
  - Emit `tracing::debug!("stream.initiated")` after `post_stream()` returns Ok.
  - The SSE parsing loop (`parse_sse_stream`) runs outside this span — that is correct
    because the stream is returned to the caller, not consumed internally.

- **REFACTOR**: None.

---

#### Cycle 9: Instrument `chat::parse_sse_stream()` — chunk counter event

- **RED**: Write test `test_sse_stream_emits_chunk_events` in
  `sdk/azure_ai_foundry_models/src/chat.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning SSE with 3 chunks + `[DONE]`.
  - Action: Consume the full stream via `StreamExt::collect()`.
  - Assert: `logs_contain("foundry::chat::sse_chunk")` appears 3 times.
  - Fails because `parse_sse_stream()` emits no events.

- **GREEN**: Inside `parse_sse_line()` in `sdk/azure_ai_foundry_models/src/chat.rs`,
  emit `tracing::trace!(target: "foundry::chat::sse_chunk", "chunk parsed")` for each
  successfully parsed chunk.
  - Note: `parse_sse_line()` is a pure sync function — use `tracing::trace!` (not a span)
    to avoid overhead on the tight inner loop.
  - Also emit `tracing::warn!("foundry::chat::sse_chunk", error = %e)` for parse errors.

- **REFACTOR**: None.

---

#### Cycle 10: Instrument `embeddings::embed()`

- **RED**: Write test `test_embed_emits_embeddings_span` in
  `sdk/azure_ai_foundry_models/src/embeddings.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `wiremock::MockServer` returning a valid `EmbeddingResponse`.
  - Action: Call `embed(&client, &request).await`
  - Assert: `logs_contain("foundry::embeddings::embed")`
  - Fails because `embed()` has no instrumentation.

- **GREEN**: Add `#[tracing::instrument(name = "foundry::embeddings::embed", skip(client, request), fields(model, input_count, prompt_tokens))]`
  to `embed()` in `sdk/azure_ai_foundry_models/src/embeddings.rs`.
  - Record `model = request.model` at span entry.
  - Record `input_count` based on `EmbeddingInput` variant:
    `Single` = 1, `Multiple(v)` = v.len()`.
  - After response is deserialized, record `prompt_tokens = response.usage.prompt_tokens`.
  - Do NOT record the input text content (user data).

- **REFACTOR**: None.

---

### Phase 4: Token Refresh Observability

#### Cycle 11: Emit span events for token cache hits and misses

- **RED**: Write test `test_token_cache_hit_emits_debug_event` in
  `sdk/azure_ai_foundry_core/src/auth.rs`.
  - Attribute: `#[tokio::test]` + `#[traced_test]`
  - Setup: `CountingTokenCredential` (already exists in auth.rs test module) with 1h TTL.
  - Action: Call `credential.resolve().await` twice.
  - Assert: `logs_contain("token.cache_hit")` appears at least once (second call).
  - Fails because no event is emitted for cache hits.

- **GREEN**: Inside `FoundryCredential::resolve()`, within the
  `TokenCredential { .. }` branch, emit:
  - `tracing::debug!("token.cache_hit")` when the cached token is still valid.
  - `tracing::debug!("token.cache_miss")` when a new token must be fetched.
  - `tracing::debug!("token.cache_refresh")` when the token is within the expiry buffer.
  - Do NOT emit the token value or the `auth_header` string.
  - File: `sdk/azure_ai_foundry_core/src/auth.rs`

- **REFACTOR**: None.

---

### Phase 5: Compilation and Lint Validation

#### Cycle 12: Verify zero warnings under `clippy` with `-D warnings`

- **RED**: Run `cargo clippy --workspace --all-targets -- -D warnings`.
  - Expected failure: clippy may warn about unused `tracing::Span::current()` calls
    or incorrectly skipped fields.

- **GREEN**: Fix all clippy warnings introduced by the instrumentation.
  - Common fixes: ensure all `fields(...)` in `#[tracing::instrument]` are actually
    recorded inside the function; if a field cannot be known at entry (e.g.,
    `status_code` before the HTTP response), declare it with a placeholder via
    `tracing::field::Empty` and record it later with `Span::current().record(...)`.

- **REFACTOR**: Replace any `tracing::Span::current().record(...)` patterns with
  `let span = tracing::Span::current(); span.record(...)` for readability across all
  instrumented functions.

---

## File Map

| File | Actions |
|------|---------|
| `sdk/azure_ai_foundry_core/Cargo.toml` | Add `tracing-test = "0.2"` to `[dev-dependencies]` |
| `sdk/azure_ai_foundry_models/Cargo.toml` | Add `tracing-test = "0.2"` to `[dev-dependencies]` |
| `sdk/azure_ai_foundry_core/src/auth.rs` | Instrument `resolve()`, `get_token()`, `get_token_with_options()`; add cache events |
| `sdk/azure_ai_foundry_core/src/client.rs` | Instrument `get()`, `post()`, `post_stream()`; extract `compute_backoff()`; add `emit_error_event()` |
| `sdk/azure_ai_foundry_models/src/chat.rs` | Instrument `complete()`, `complete_stream()`; add trace events in `parse_sse_line()` |
| `sdk/azure_ai_foundry_models/src/embeddings.rs` | Instrument `embed()` |

---

## Security Constraints (Non-Negotiable)

The following data must NEVER appear in any tracing event or span field:

1. API keys (the value wrapped by `SecretString` in `FoundryCredential::ApiKey`).
2. Bearer tokens (the value returned by `resolve()`).
3. Raw HTTP response bodies before sanitization (use `sanitize_error_message()`).
4. User message content from `ChatCompletionRequest.messages`.
5. Input text from `EmbeddingRequest.input`.

Enforce this by:
- Always using `skip(self)` or `skip(client, request)` in `#[tracing::instrument]`.
- Never recording fields derived from `SecretString` or `AccessToken.token`.
- Routing all error body strings through `FoundryClient::truncate_message()` before
  recording them as span fields or emitting them in events.

---

## Span Field Reference

| Span name | Fields |
|-----------|--------|
| `foundry::auth::resolve` | `credential_type` = `"api_key"` or `"token_credential"` |
| `foundry::client::get` | `path`, `attempt`, `status_code` (recorded after response) |
| `foundry::client::post` | `path`, `attempt`, `status_code` |
| `foundry::client::post_stream` | `path`, `attempt`, `status_code`, `streaming_timeout_secs` |
| `foundry::chat::complete` | `model`, `prompt_tokens`, `completion_tokens` |
| `foundry::chat::complete_stream` | `model` |
| `foundry::embeddings::embed` | `model`, `input_count`, `prompt_tokens` |

---

## Estimation

| Phase | Cycles | Estimated time |
|-------|--------|---------------|
| 1: Test infrastructure | 1 | 15 min |
| 2: Core crate | 5 | 2 h |
| 3: Models crate | 4 | 1.5 h |
| 4: Token observability | 1 | 30 min |
| 5: Lint validation | 1 | 30 min |
| **Total** | **12** | **~4.5 h** |

---

## Success Criteria

- [ ] All 12 TDD cycles complete (RED confirmed before GREEN implemented).
- [ ] `cargo test --workspace` passes with all tests green (target: 160+ tests).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` reports zero warnings.
- [ ] `cargo fmt --all -- --check` reports no formatting issues.
- [ ] No sensitive data (keys, tokens, user content) appears in any span field or event.
- [ ] All new instrumented functions have their span names documented in the public doc
  comment (so operators know what to filter on).
