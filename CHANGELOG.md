# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2025-02-15

### Added

#### Robustness
- Streaming timeout configuration (5-minute default, separate from HTTP timeout)
- Pre-stream retry logic with exponential backoff and jitter for 503/429 errors
- Builder validations for parameter ranges (temperature, top_p, penalties, dimensions)
- High concurrency tests (100+ concurrent tasks) for token refresh verification
- Documentation examples with error handling for `complete()` and `complete_stream()`

### Security

- **SSE Buffer Limit**: Maximum 1MB buffer per SSE line to prevent DoS attacks (CWE-400)
- **Error Sanitization**: Bearer tokens and API keys are automatically redacted from error messages (CWE-209)

### Changed

- `post_stream()` now uses dedicated streaming timeout instead of default HTTP timeout
- Builder `try_build()` methods now validate parameter ranges before construction

## [0.1.0] - 2025-02-14

### Added

#### Authentication
- `FoundryCredential` enum supporting API key and Microsoft Entra ID authentication
- `FoundryCredential::api_key()` for API key authentication
- `FoundryCredential::entra_id()` for Azure AD/Entra ID authentication via `azure_identity`
- `FoundryCredential::from_env()` for automatic credential detection from environment variables
- Environment variable support: `AZURE_AI_FOUNDRY_ENDPOINT` and `AZURE_AI_FOUNDRY_API_KEY`

#### HTTP Client
- `FoundryClient` with builder pattern for configuration
- Support for custom API versions
- Automatic endpoint URL construction
- Error handling with `FoundryError` enum covering HTTP, API, auth, and stream errors

#### Chat Completions
- `ChatCompletionRequest` with builder pattern
- Support for all standard parameters: temperature, top_p, max_tokens, stop sequences, presence/frequency penalty
- `Message` type with `system()`, `user()`, and `assistant()` constructors
- `complete()` function for synchronous completions
- `complete_stream()` function returning `impl Stream<Item = FoundryResult<ChatCompletionChunk>>`
- SSE parsing optimized with `memchr` for high-throughput streaming

#### Embeddings
- `EmbeddingRequest` with builder pattern
- Support for single and multiple input texts
- `EncodingFormat` enum (Float, Base64)
- Optional `dimensions` parameter for dimension reduction
- `embed()` function for generating embeddings

#### Developer Experience
- `try_build()` methods returning `Result` for fallible builder construction
- Comprehensive error types with `thiserror`
- Full documentation with examples
- 97 unit tests with `wiremock` for HTTP mocking

### Security
- API keys wrapped with `secrecy` crate to prevent accidental logging
- Error message truncation to prevent sensitive data leakage

[Unreleased]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/bzsanti/azure-ai-foundry/releases/tag/v0.1.0
