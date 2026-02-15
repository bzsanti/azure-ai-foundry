# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run a single test
cargo test --workspace test_name

# Check compilation without building
cargo check --workspace --all-targets

# Lint with clippy (warnings as errors)
cargo clippy --workspace --all-targets -- -D warnings

# Format code
cargo fmt --all

# Format check (CI mode)
cargo fmt --all -- --check

# Generate docs
cargo doc --workspace --no-deps

# Integration tests (requires Azure credentials)
export AZURE_AI_FOUNDRY_ENDPOINT="https://your-resource.services.ai.azure.com"
export AZURE_AI_FOUNDRY_API_KEY="your-key"
cargo test --workspace --features integration-tests
```

## Architecture

This is a **Rust workspace** providing an unofficial SDK for Microsoft AI Foundry. MSRV is **1.88**.

### Crate Structure

```
sdk/
├── azure_ai_foundry_core     # Auth, HTTP client, shared types, error handling
│   ├── auth.rs               # FoundryCredential (API key / Entra ID)
│   ├── client.rs             # FoundryClient builder
│   ├── error.rs              # FoundryError (thiserror)
│   └── models.rs             # Common types
│
└── azure_ai_foundry_models   # Model inference APIs (depends on core)
    ├── chat.rs               # Chat completions + streaming
    └── embeddings.rs         # Vector embeddings
```

### Key Patterns

- **Builder pattern** for `FoundryClient` and request types
- **Async-first** with `tokio` runtime
- **thiserror** for typed errors, **tracing** for logging
- **secrecy** for sensitive values (API keys)
- All public items require doc comments

### Planned Crates (not yet implemented)

- `azure_ai_foundry_agents` - Agent Service (v0.2.0)
- `azure_ai_foundry_tools` - Vision, Document Intelligence (v0.3.0)

## Code Style

- Follow `rustfmt` defaults
- Use Conventional Commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `chore:`
- CI runs with `RUSTFLAGS="-D warnings"` — all warnings are errors

## TDD Methodology (MANDATORY)

**All implementation plans MUST follow strict Test-Driven Development.**

### Planning Phase (planning-agent)
When creating implementation plans, structure them as TDD cycles:

1. **RED Phase** - Write failing test first
   - Define test case with clear assertions
   - Specify expected behavior
   - Test MUST fail initially (code doesn't exist yet)

2. **GREEN Phase** - Minimal implementation
   - Write the minimum code to make the test pass
   - No extra features, no premature optimization
   - Focus only on passing the current test

3. **REFACTOR Phase** - Improve code quality
   - Clean up implementation
   - Remove duplication
   - Improve naming and structure
   - Tests must still pass

### Plan Format
Every plan must be structured as:
```
## Task: [Feature Name]

### Cycle 1: [Smallest testable unit]
- RED: Write test for [specific behavior]
- GREEN: Implement [minimal code]
- REFACTOR: [improvements if needed]

### Cycle 2: [Next testable unit]
- RED: Write test for [next behavior]
- GREEN: Implement [minimal code]
- REFACTOR: [improvements if needed]

[Continue cycles...]
```

### Implementation Phase
When implementing:
1. **Write the test FIRST** - before any production code
2. **Run the test** - verify it fails
3. **Write minimal code** - only enough to pass
4. **Run the test** - verify it passes
5. **Refactor if needed** - keep tests green
6. **Repeat** for next test case

### Rules
- NEVER write production code without a failing test
- NEVER skip the RED phase
- Each test should test ONE behavior
- Tests must be independent and isolated

## Session Status (2026-02-15)

**Branch:** `docs/session-update`

**v0.1.0 Status:** RELEASED

**Release Tag:** `v0.1.0` pushed to GitHub, release workflow triggered.

**Completed Features:**
- Real authentication with `azure_identity` (`Arc<dyn TokenCredential>`)
- API key and Entra ID authentication
- Chat completions (sync + streaming)
- SSE parsing optimized with `memchr`
- Embeddings API (`embed()` function with builder pattern)

**Quality Improvements v0.1.0 (9 phases):**
1. Test isolation with `serial_test` crate
2. Builder pattern with `try_build()` returning `Result`
3. SSE performance optimized (memchr, Vec<u8> buffer)
4. Security logging with message truncation
5. Documentation for internal types
6. Centralized test helpers
7. API consistency with `IntoIterator` bounds
8. Error tests with pattern matching
9. Test constants centralization

**Quality Improvements v0.2.0 (TDD Plan execution):**

Phase 1 - Security:
1. SSE Buffer Limit: DoS prevention with 1MB buffer limit
2. Error Sanitization: Automatic redaction of Bearer tokens and API keys

Phase 2 - Robustness:
3. Streaming Timeouts: 5-minute default timeout for streaming responses
4. Token Race Condition: Already implemented in v0.1.0 (verified)
5. Builder Validations: Range validation for temperature, top_p, penalties, dimensions
6. Streaming Retry Logic: Pre-stream retry with exponential backoff

Phase 3 - Optional:
7. Clone Optimization: ❌ Discarded (negligible impact, clone is required)
8. Doc Examples: ✓ Error handling examples for complete() and complete_stream()
9. Tracing Instrumentation: ⏳ Deferred to v0.3.0 (will be implemented with Agent Service)
10. High Concurrency Tests: ✓ Tests with 100+ concurrent tasks for thread-safety

**Release Infrastructure:**
- GitHub Actions release workflow (`.github/workflows/release.yml`)
- CHANGELOG.md following Keep a Changelog format
- Automatic crates.io publishing on tag push

**Documentation:**
- README.md for `azure_ai_foundry_core` crate
- README.md for `azure_ai_foundry_models` crate
- Doc examples with error handling patterns

**Test Summary:**
- 160 tests passing (79 core + 65 models + 16 doc-tests)
- All clippy checks passing (0 warnings)
