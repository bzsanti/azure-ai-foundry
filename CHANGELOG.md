# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0] - 2026-03-08

### Added

#### New Crate: `azure_ai_foundry_safety` (Content Safety API v2024-09-01)

**Text Content Analysis**
- `analyze_text()` — detect harmful content (hate, self-harm, sexual, violence)
- `AnalyzeTextRequest` builder with categories, blocklist names, halt-on-blocklist, output type
- Configurable severity output: `FourSeverityLevels` or `EightSeverityLevels`
- Maximum text length validation (10,000 Unicode code points)

**Image Content Analysis**
- `analyze_image()` — detect harmful content in images
- `AnalyzeImageRequest` builder with base64 content or Azure Blob URL input
- `has_base64_content()` / `has_blob_url()` accessors

**Prompt Shields**
- `shield_prompt()` — detect jailbreak and injection attacks
- `ShieldPromptRequest` builder with user prompt and optional documents

**Protected Material Detection**
- `detect_protected_material()` — detect copyrighted content (lyrics, articles, code)
- `ProtectedMaterialRequest` builder with text length validation

**Blocklist Management (CRUD)**
- `create_or_update_blocklist()` — create/update via PATCH with `application/merge-patch+json`
- `get_blocklist()`, `list_blocklists()`, `delete_blocklist()`
- `add_or_update_blocklist_items()`, `get_blocklist_item()`, `list_blocklist_items()`
- `remove_blocklist_items()` — accepts `impl IntoIterator<Item = impl AsRef<str>>`
- `BlocklistUpsertRequest` builder (body contains description only; name is a URL path parameter)
- `BlocklistItemInput` builder with text, description, is_regex fields
- Paginated list responses with `next_link` for cursor-based navigation

**Shared Types**
- `HarmCategory` enum (Hate, SelfHarm, Sexual, Violence)
- `OutputType` enum (FourSeverityLevels, EightSeverityLevels)
- `ImageOutputType` enum (`#[non_exhaustive]`, FourSeverityLevels)
- `CategoryAnalysis` with category and severity fields
- All request/response types derive `PartialEq, Eq`

**Validation**
- Client-side max-length enforcement: text (10,000), blocklist name (64), description (1,024), item text (128)
- All length errors use `FoundryError::Validation` (not `Builder`)
- Name validation order: resource ID check before iterator consumption
- Centralized limit constants in `models.rs`

**Core Crate Enhancement**
- `FoundryClient::patch()` method for PATCH requests with `application/merge-patch+json`
- Serialization errors use `FoundryError::Serialization` (was incorrectly `Api`)

**Quality**
- Builder methods accept `impl IntoIterator` (categories, blocklist_names, documents)
- `api-version=2024-09-01` query parameter verified by tests in every module
- Tracing instrumentation on all API functions with `foundry::safety::*` spans
- 104 unit tests + 6 doc-tests in the safety crate
- 818 total workspace tests

## [0.7.0] - 2026-03-04

### Changed

#### Quality Refactor — 4 Rounds (52 findings resolved)

**Round 1 — Foundation quality (M1-M7)**
- `FoundryError::Validation` variant for runtime validation errors
- Workspace-level Clippy lints (`unsafe_code=deny`, `clippy::all=warn`)
- Removed panic paths in `auth.rs` and `client.rs`
- Optimized `sanitize_error_message` (O(n) instead of O(n²))
- URL path injection validation for all resource IDs
- Extracted `execute_with_retry` to eliminate retry loop duplication (~350 lines removed)
- `poll_until_complete` accepts `max_attempts: Option<u32>`
- Removed `Clone` from audio/image request types with `Vec<u8>` data
- `file::upload` uses `impl Into<bytes::Bytes>` for zero-copy
- `stop()` builder methods accept `impl IntoIterator`
- `Display` impls for `RunStatus`, `VectorStoreStatus`
- `ResponseMessage::role` typed as `Role` enum
- `Debug` on all model builders (manual impl for byte-holding builders)
- Borrowed `DocumentAnalysisBody<'a>` to avoid clones
- Standardized `build()` / `try_build()` across all builders
- Unified `RunUsage` with `azure_ai_foundry_core::models::Usage`

**Round 2 — Deep quality fixes (M1-M6, 12 findings)**
- Audio bytes migration (`TranscriptionRequest`/`TranslationRequest` → `bytes::Bytes`)
- File upload zero-copy (`Part::stream_with_length` instead of `to_vec()`)
- Stringly-typed enums replaced: `RequiredActionType`, `ToolType`, `ToolCallType`, `MessageRole`
- Poll timeout returns `FoundryError::Validation` instead of `Api`
- `Display` impls for `RunStepStatus`, `StepType`
- Uniform empty-string validation (`trim().is_empty()`) across all builders

**Round 3 — 18 findings (M1-M8)**
- Field `n` → `count` with serde rename in image requests
- Doc comments on `EmbeddingData`, `ResponseUsage`, `OUTPUT_TEXT_TYPE` (now `pub const`)
- `AgentUpdateRequest` rejects all-None (Validation error)
- `Display` for `FilePurpose` and `AudioResponseFormat`
- Doc lifetime note on `fetch_fresh_token_with_options`
- Replaced `unreachable!()` with explicit error in `execute_with_retry`

**Round 4 — 11 findings (M1-M12)**
- **Critical fix**: UTF-8 boundary panic in `truncate_message`
- Query params percent-encoding in tools crate (vision + document intelligence)
- `previous_response_id` empty/whitespace validation
- `ImageEditRequest` `Vec<u8>` → `bytes::Bytes` migration (O(1) clone for retries)
- `FileObject.status` → `FileStatus` typed enum with `#[serde(other)]`
- `ResponseOutput.output_type` → `ResponseOutputType` typed enum
- `ResponseContent.content_type` → `ResponseContentType` typed enum
- `EmbeddingData` added `object` field for API parity
- `RetryPolicy::new` error variant `Builder` → `Validation`
- `Role` enum added `Hash` derive
- `FileList.has_more` pagination cursor documentation with example

### Breaking Changes
- `ImageEditRequest.image` / `.mask`: `Vec<u8>` → `bytes::Bytes`
- `FileObject.status`: `Option<String>` → `Option<FileStatus>`
- `ResponseOutput.output_type`: `String` → `ResponseOutputType`
- `ResponseContent.content_type`: `String` → `ResponseContentType`
- `EmbeddingData`: new required field `object: String`
- `RetryPolicy::new` returns `FoundryError::Validation` (was `Builder`)
- Image request field `n` renamed to `count` (serde alias preserves JSON compatibility)
- `RunUsage` removed in favor of `azure_ai_foundry_core::models::Usage`

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

[Unreleased]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/bzsanti/azure-ai-foundry/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/bzsanti/azure-ai-foundry/releases/tag/v0.1.0
