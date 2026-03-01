# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-03-01

### Added

#### Audio API (`azure_ai_foundry_models`)
- `audio` module: transcription, translation, and text-to-speech
- `TranscriptionRequest` builder with language, prompt, response_format, temperature
- `TranslationRequest` builder (always outputs English)
- `SpeechRequest` builder with voice, response_format (mp3/opus/aac/flac/wav/pcm16), speed
- `transcribe()` and `translate()` via multipart POST to `/openai/v1/audio/*`
- `speak()` returns raw audio bytes via JSON POST to `/openai/v1/audio/speech`
- `AudioResponseFormat` enum (json, text, srt, vtt, verbose_json)
- `SpeechFormat` enum (mp3, opus, aac, flac, wav, pcm16)
- `VerboseTranscriptionResponse` with segments and timestamps
- `MAX_SPEECH_INPUT_LENGTH` constant (4096 characters)

#### Images API (`azure_ai_foundry_models`)
- `images` module: image generation and editing
- `ImageGenerationRequest` builder with size, quality, response_format, output_format, n
- `ImageEditRequest` builder with image, mask, multipart upload
- `generate()` via JSON POST to `/openai/v1/images/generations`
- `edit()` via multipart POST to `/openai/v1/images/edits`
- `ImageSize` enum (256x256, 512x512, 1024x1024, 1536x1024, 1024x1536, auto)
- `ImageQuality` enum (standard, hd, low, medium, high, auto)
- `ImageResponseFormat` enum (url, b64_json)
- `ImageOutputFormat` enum (png, jpeg, webp)

#### Responses API (`azure_ai_foundry_models`)
- `responses` module: unified Responses API
- `CreateResponseRequest` builder with temperature, top_p, max_output_tokens, penalties, stop, stream, previous_response_id
- `ResponseInput` enum: simple text or structured messages
- `ResponseMessage` with user(), system(), assistant() constructors
- `create()` via JSON POST to `/openai/v1/responses`
- `get()` via GET to `/openai/v1/responses/{id}`
- `delete()` via DELETE to `/openai/v1/responses/{id}`
- `Response::output_text()` convenience method
- `ResponseStatus` enum (completed, failed, in_progress, cancelled)

## [0.5.0] - 2026-03-01

### Added

#### Files API (`azure_ai_foundry_agents`)
- `file` module: upload, get, list, delete, and download files
- `FilePurpose` enum with `as_str()` method
- `MAX_FILE_SIZE_BYTES` constant for client-side 512 MB validation
- Multipart file upload via `FoundryClient::post_multipart()`

#### Vector Stores (`azure_ai_foundry_agents`)
- `vector_store` module: create, get, list, update, delete vector stores
- Vector store files: add, get, list, delete files in a vector store
- Vector store file batches: create and get batch operations
- `VectorStoreCreateRequest` and `VectorStoreUpdateRequest` builders

#### Run Steps (`azure_ai_foundry_agents`)
- `run_step` module: list and get run steps
- `StepType` enum (`MessageCreation`, `ToolCalls`) — type-safe step classification
- `ToolCallType` enum (`Function`, `CodeInterpreter`, `FileSearch`) — type-safe tool call classification
- `RunStepUsage` for token usage per step

#### Submit Tool Outputs (`azure_ai_foundry_agents`)
- `run::submit_tool_outputs()` for providing tool results back to the agent
- `run::submit_tool_outputs_and_poll()` convenience function with polling
- `ToolOutput` type for structured tool output submission
- Validation: non-empty outputs, non-empty tool_call_id

#### Update Operations (`azure_ai_foundry_agents`)
- `agent::update()` with `AgentUpdateRequest` builder (model, name, instructions, tools, temperature, top_p)
- `thread::update()` with `ThreadUpdateRequest` builder (metadata)
- `message::update()` with `MessageUpdateRequest` builder (metadata)

#### Core Improvements
- `FoundryClient::post_multipart()` for multipart form uploads with retry support
- `FoundryClient::get_bytes()` for raw binary downloads with retry support
- `create_and_poll()` and `submit_tool_outputs_and_poll()` tracing instrumentation

### Changed
- `create_file_batch()` now accepts `&[impl AsRef<str>]` instead of `&[String]` for ergonomics
- `get_bytes()` error path uses `expect_err` pattern (consistent with other client methods)
- Upload closure wraps file data in `Arc<Vec<u8>>` to avoid redundant clones on retry

## [0.4.0] - 2026-02-28

### Added

#### New Crate: `azure_ai_foundry_tools`
- `vision` module: Image Analysis 4.0 (tags, captions, object detection, OCR, smart crops, people)
- `document_intelligence` module: Document Intelligence v4.0 (OCR, layout, invoices, receipts, ID documents, business cards)
- Full tracing instrumentation for all Vision and Document Intelligence API calls

#### Documentation
- `include_str!` pattern for README as crate-level documentation on docs.rs
- Comprehensive doc-tests across all crates

### Changed
- 14 quality fixes applied via TDD cycles
- Improved builder validation messages

## [0.3.0] - 2025-02-21

### Added

#### New Crate: `azure_ai_foundry_agents`
- `agent` module: create, get, list, delete agents
- `thread` module: create, get, delete conversation threads
- `message` module: create, list, get messages in threads
- `run` module: create, get, create_thread_and_run, poll_until_complete

#### Tracing Instrumentation
- Full `tracing` spans across all API calls (chat, embeddings, agents)
- Span fields for model, token usage, agent_id, thread_id, run_id, etc.

#### Core Improvements
- `FoundryClient::delete()` method for DELETE requests
- `FoundryClient` fields made private (encapsulation)
- Input validation in builders (empty strings, parameter ranges)

### Security
- **HTTPS Required**: Endpoint URLs must use HTTPS (except localhost for development)
- **Token Refresh Buffer**: Increased from 60s to 120s to prevent race conditions in slow networks
- **SSE Parsing**: Defensive checks for empty/malformed lines

### Changed
- `get_token()` deprecated → use `fetch_fresh_token()`
- `EmbeddingUsage` removed → use `Usage` from core
- `AzureSdk` error variant changed from `(String)` to `{ message, source }`

### Breaking Changes
- `FoundryClient::builder().endpoint("http://...")` now fails with `InvalidEndpoint` error
- HTTP is only allowed for localhost/127.0.0.1 for local development

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

[Unreleased]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/bzsanti/azure-ai-foundry/releases/tag/v0.1.0
