# Contributing to Azure AI Foundry SDK for Rust

Thank you for your interest in contributing! This document provides guidelines for contributing to this project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/bzsanti/azure-ai-foundry.git`
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Run lints: `cargo clippy --workspace -- -D warnings`
7. Format code: `cargo fmt --all`
8. Commit and push
9. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75 or later (`rustup update stable`)
- An Azure AI Foundry resource (for integration tests)

### Running Tests

```bash
# Unit tests (no Azure credentials needed)
cargo test --workspace

# Integration tests (requires Azure credentials)
export AZURE_AI_FOUNDRY_ENDPOINT="https://your-resource.services.ai.azure.com"
export AZURE_AI_FOUNDRY_API_KEY="your-key"
cargo test --workspace --features integration-tests
```

## Code Style

- Follow standard Rust conventions (`rustfmt` defaults)
- All public items must have doc comments
- All modules must have module-level documentation
- Use `thiserror` for error types
- Use `tracing` for logging (not `log` or `println!`)

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation changes
- `test:` adding or updating tests
- `refactor:` code refactoring
- `ci:` CI/CD changes
- `chore:` maintenance tasks

## Pull Request Process

1. Update documentation if you changed public APIs
2. Add tests for new functionality
3. Ensure CI passes
4. Request review from a maintainer

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
