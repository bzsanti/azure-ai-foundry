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
├── azure_ai_foundry_models   # Model inference APIs (depends on core)
│   ├── audio.rs              # Transcription, translation, text-to-speech
│   ├── chat.rs               # Chat completions + streaming
│   ├── embeddings.rs         # Vector embeddings
│   ├── images.rs             # Image generation + editing
│   └── responses.rs          # Responses API (create, get, delete)
│
├── azure_ai_foundry_agents   # Agent Service APIs (depends on core)
│   ├── agent.rs              # Create, get, list, delete agents
│   ├── thread.rs             # Thread management
│   ├── message.rs            # Message management
│   ├── run.rs                # Run execution and polling
│   └── models.rs             # Shared types
│
└── azure_ai_foundry_tools    # Vision & Document Intelligence (depends on core)
    ├── vision.rs             # Image Analysis 4.0
    ├── document_intelligence.rs  # Document Intelligence v4.0
    └── models.rs             # Shared types (BoundingBox, ImageMetadata, etc.)
```

### Key Patterns

- **Builder pattern** for `FoundryClient` and request types
- **Async-first** with `tokio` runtime
- **thiserror** for typed errors, **tracing** for logging
- **secrecy** for sensitive values (API keys)
- All public items require doc comments

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

## Session Status (2026-03-03)

**v0.1.0 Status:** RELEASED
**v0.2.0 Status:** RELEASED
**v0.3.0 Status:** RELEASED
**v0.4.0 Status:** RELEASED
**v0.5.0 Status:** MERGED TO MAIN (pending release)
**v0.6.0 Status:** MERGED TO MAIN (pending release)

Published to crates.io:
- https://crates.io/crates/azure_ai_foundry_core/0.4.0
- https://crates.io/crates/azure_ai_foundry_models/0.4.0
- https://crates.io/crates/azure_ai_foundry_agents/0.4.0
- https://crates.io/crates/azure_ai_foundry_tools/0.4.0

**v0.5.0 Highlights:**
- File upload/download/list/delete (/files API)
- Vector stores CRUD with file and batch operations
- Run steps list and get
- Submit tool outputs with polling
- Agent, thread, and message update operations
- post_multipart() and get_bytes() on FoundryClient

**v0.6.0 Highlights:**
- Audio transcription/translation (Whisper) and text-to-speech (TTS)
- Image generation and editing (DALL-E)
- Responses API (create, get, delete)
- 13 quality fixes via TDD cycles

**v0.7.0 Status:** IN PROGRESS (branch `feature/0.7.0`)
- Quality refactor rounds 1-3: all milestones complete
- Version bumped to 0.7.0 across all crates
- Quality review round 4: 12 new findings identified, TDD plan in `.claude/plan.md`
- Pending: Execute round 4 milestones (M1-M12, see plan)
- Pending: Update CHANGELOG.md, create PR, merge to main

**v0.7.0 Highlights (completed):**
- Round 1 (M1-M7): Foundation quality fixes
  - `FoundryError::Validation` variant for runtime validation errors
  - Workspace-level Clippy lints (unsafe_code=deny, clippy::all=warn)
  - Removed panic paths in auth.rs and client.rs
  - Optimized `sanitize_error_message` (O(n) instead of O(n²))
  - URL path injection validation for all resource IDs
  - Extracted `execute_with_retry` to eliminate retry loop duplication (~350 lines removed)
  - `poll_until_complete` now accepts `max_attempts: Option<u32>`
  - Removed `Clone` from audio/image request types with Vec<u8> data
  - `file::upload` uses `impl Into<bytes::Bytes>` for zero-copy
  - `stop()` builder methods accept `impl IntoIterator`
  - `Display` impls for `RunStatus`, `VectorStoreStatus`
  - `ResponseMessage::role` typed as `Role` enum
  - `Debug` on all model builders (manual impl for byte-holding builders)
  - Borrowed `DocumentAnalysisBody<'a>` to avoid clones
  - Standardized `build()` / `try_build()` across all builders
  - `FoundryError::Validation` for runtime validation (distinct from `Builder`)
  - Unified `RunUsage` with `azure_ai_foundry_core::models::Usage`
- Round 2 (M1-M6): Deep quality fixes (12 findings)
- Round 3 (M1-M8): 18 findings resolved
  - Audio bytes migration (TranscriptionRequest/TranslationRequest → bytes::Bytes)
  - File upload zero-copy (Part::stream_with_length instead of to_vec())
  - Stringly-typed enums replaced: RequiredActionType, ToolType, ToolCallType, MessageRole, ResponseOutput::role
  - Poll timeout returns FoundryError::Validation instead of Api
  - Display impls for RunStepStatus, StepType
  - Uniform empty-string validation (trim().is_empty()) across all builders
  - Field `n` → `count` with serde rename in image requests
  - Doc comments on EmbeddingData, ResponseUsage, OUTPUT_TEXT_TYPE (now pub const)
  - AgentUpdateRequest rejects all-None (Validation error)
  - Display for FilePurpose and AudioResponseFormat
  - Doc lifetime note on fetch_fresh_token_with_options
  - Replaced unreachable!() with explicit error in execute_with_retry

**Test Summary:**
- 676 tests passing (149 agents + 214 models + 145 core + 61 tools + 107 doc-tests)
- All clippy checks passing (0 warnings)
- All formatting checks passing
