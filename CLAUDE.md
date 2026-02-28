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
│   ├── chat.rs               # Chat completions + streaming
│   └── embeddings.rs         # Vector embeddings
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

## Session Status (2026-02-28)

**v0.1.0 Status:** RELEASED
**v0.2.0 Status:** RELEASED
**v0.3.0 Status:** RELEASED
**v0.4.0 Status:** RELEASED

Published to crates.io:
- https://crates.io/crates/azure_ai_foundry_core/0.4.0
- https://crates.io/crates/azure_ai_foundry_models/0.4.0
- https://crates.io/crates/azure_ai_foundry_agents/0.4.0
- https://crates.io/crates/azure_ai_foundry_tools/0.4.0

**v0.4.0 Highlights:**
- New `azure_ai_foundry_tools` crate (Vision + Document Intelligence)
- docs.rs documentation improvements (include_str!, README as crate docs)
- 14 quality fixes applied via TDD cycles

**Test Summary:**
- 347 tests passing (113 core + 77 models + 42 agents + 60 tools + 55 doc-tests)
- All clippy checks passing (0 warnings)
- All formatting checks passing
