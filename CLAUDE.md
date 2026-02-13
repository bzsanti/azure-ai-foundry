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

## Session Status (2026-02-13)

**Branch:** `feat/real-auth`

**Completed:**
- Implemented real authentication with `azure_identity` (`Arc<dyn TokenCredential>`)
- Added support for API key and Entra ID authentication
- Scope: `https://cognitiveservices.azure.com/.default`
- Test coverage: **94.24%** (180/191 lines)

**Test Summary:**
- 70 tests passing (44 core + 17 models + 9 doc-tests)
- `auth.rs`: 86.2% coverage
- `client.rs`: 96.4% coverage
- `error.rs`: 100% coverage
- `chat.rs`: 100% coverage
