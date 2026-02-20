# TDD Plan: azure_ai_foundry_agents

## Overview

This crate implements the Azure AI Foundry Agent Service REST API client. It provides Rust bindings for creating, managing, and running AI agents with threads, messages, and runs. The Agent Service enables cloud-hosted AI workflows that pair large language models with tools to execute complex tasks.

**Stack detected**: Rust async ecosystem (tokio, reqwest, serde)
**Convenciones**: Builder pattern, async-first, thiserror errors, tracing instrumentation
**¿Afecta hot path?**: No - This is an orchestration/control plane API, not a data plane hot path

## API Research

Based on [Azure AI Foundry Agent Service REST API reference](https://learn.microsoft.com/en-us/rest/api/aifoundry/aiagents/), the API provides the following endpoints:

### Base Endpoint
- Format: `https://<resource>.services.ai.azure.com/api/projects/<project>`
- All paths are appended to this base
- API version: `v1` (uses `2025-05-15-preview` internally)

### Agents
- `POST /assistants?api-version=v1` - Create agent
- `GET /assistants/{assistantId}?api-version=v1` - Get agent
- `GET /assistants?api-version=v1` - List agents
- `POST /assistants/{assistantId}?api-version=v1` - Update agent
- `DELETE /assistants/{assistantId}?api-version=v1` - Delete agent

### Threads
- `POST /threads?api-version=v1` - Create thread
- `GET /threads/{threadId}?api-version=v1` - Get thread
- `DELETE /threads/{threadId}?api-version=v1` - Delete thread

### Messages
- `POST /threads/{threadId}/messages?api-version=v1` - Create message
- `GET /threads/{threadId}/messages?api-version=v1` - List messages
- `GET /threads/{threadId}/messages/{messageId}?api-version=v1` - Get message

### Runs
- `POST /threads/{threadId}/runs?api-version=v1` - Create run
- `POST /threads/runs?api-version=v1` - Create thread and run
- `GET /threads/{threadId}/runs/{runId}?api-version=v1` - Get run
- `GET /threads/{threadId}/runs?api-version=v1` - List runs

### Run Steps
- `GET /threads/{threadId}/runs/{runId}/steps?api-version=v1` - List run steps
- `GET /threads/{threadId}/runs/{runId}/steps/{stepId}?api-version=v1` - Get run step

## Decisiones Previas Necesarias

None - The API structure follows OpenAI Assistants API protocol, which is well-documented. Implementation follows existing patterns from `azure_ai_foundry_models`.

## Plan de Ejecución

### Phase 1: Crate Structure and Dependencies

#### Cycle 1: Create crate skeleton
- **RED**: Write test `test_crate_builds` in `sdk/azure_ai_foundry_agents/tests/build.rs`
  - Assert: `cargo build -p azure_ai_foundry_agents` succeeds
- **GREEN**: Create `sdk/azure_ai_foundry_agents/Cargo.toml` with dependencies
  - Add to workspace members in root `Cargo.toml`
  - Create `sdk/azure_ai_foundry_agents/src/lib.rs` with basic docs
- **REFACTOR**: Ensure doc comments match style from other crates

#### Cycle 2: Module structure
- **RED**: Write test `test_modules_exist` that imports all planned modules
  - Assert: `use azure_ai_foundry_agents::{agent, thread, message, run};` compiles
- **GREEN**: Create empty module files:
  - `sdk/azure_ai_foundry_agents/src/agent.rs`
  - `sdk/azure_ai_foundry_agents/src/thread.rs`
  - `sdk/azure_ai_foundry_agents/src/message.rs`
  - `sdk/azure_ai_foundry_agents/src/run.rs`
  - `sdk/azure_ai_foundry_agents/src/models.rs` (shared types)
- **REFACTOR**: Add module docs with usage examples (doc tests will validate later)

### Phase 2: Agent Types and Builder

#### Cycle 3: Agent request type
- **RED**: Write test `test_agent_request_serialization`
  - Assert: `AgentCreateRequest` serializes to expected JSON structure with model, name, instructions
- **GREEN**: Implement `AgentCreateRequest` struct in `agent.rs`
  - Fields: `model: String`, `name: Option<String>`, `instructions: Option<String>`, `tools: Option<Vec<Tool>>`
  - Derive `Serialize`
- **REFACTOR**: Add doc comments for each field

#### Cycle 4: Agent request builder
- **RED**: Write test `test_agent_builder_requires_model`
  - Assert: `AgentCreateRequestBuilder::default().build()` returns error
  - Assert: Builder with model set succeeds
- **GREEN**: Implement `AgentCreateRequestBuilder`
  - Method: `model(impl Into<String>) -> Self`
  - Method: `name(impl Into<String>) -> Self`
  - Method: `instructions(impl Into<String>) -> Self`
  - Method: `build() -> FoundryResult<AgentCreateRequest>`
- **REFACTOR**: Extract validation logic to helper functions

#### Cycle 5: Agent response type
- **RED**: Write test `test_agent_response_deserialization`
  - Assert: Mock JSON response from API deserializes to `Agent`
  - Assert: Required fields (`id`, `object`, `created_at`, `model`) are present
- **GREEN**: Implement `Agent` struct
  - Fields: `id: String`, `object: String`, `created_at: u64`, `model: String`, `name: Option<String>`, `instructions: Option<String>`
  - Derive `Deserialize, Debug, Clone`
- **REFACTOR**: None needed (minimal struct)

### Phase 3: Agent API Functions

#### Cycle 6: Create agent function
- **RED**: Write test `test_create_agent_success` using wiremock
  - Mock: `POST /assistants?api-version=v1` returns 200 with agent JSON
  - Assert: Response deserializes correctly
  - Assert: Request includes Authorization header
- **GREEN**: Implement `async fn create(client: &FoundryClient, request: &AgentCreateRequest) -> FoundryResult<Agent>`
  - Call `client.post("/assistants?api-version=v1", request)`
  - Deserialize response
- **REFACTOR**: Add tracing span `foundry::agents::create` with `model` field

#### Cycle 7: Get agent function
- **RED**: Write test `test_get_agent_success`
  - Mock: `GET /assistants/{id}?api-version=v1` returns agent
  - Assert: Correct URL is constructed with ID
- **GREEN**: Implement `async fn get(client: &FoundryClient, agent_id: &str) -> FoundryResult<Agent>`
  - Construct path with agent_id
  - Call `client.get(path)`
- **REFACTOR**: Add tracing span `foundry::agents::get` with `agent_id` field

#### Cycle 8: List agents function
- **RED**: Write test `test_list_agents_success`
  - Mock: `GET /assistants?api-version=v1` returns list
  - Assert: Response contains array of agents
- **GREEN**: Implement `AgentList` struct and `async fn list(client: &FoundryClient) -> FoundryResult<AgentList>`
  - `AgentList` fields: `object: String`, `data: Vec<Agent>`, `first_id: Option<String>`, `last_id: Option<String>`, `has_more: bool`
- **REFACTOR**: Add doc comments explaining pagination (future enhancement)

#### Cycle 9: Delete agent function
- **RED**: Write test `test_delete_agent_success`
  - Mock: `DELETE /assistants/{id}?api-version=v1` returns deletion response
  - Assert: Response confirms deletion
- **GREEN**: Implement `AgentDeletionResponse` struct and `async fn delete(client: &FoundryClient, agent_id: &str) -> FoundryResult<AgentDeletionResponse>`
  - `AgentDeletionResponse` fields: `id: String`, `deleted: bool`, `object: String`
- **REFACTOR**: Add tracing span

### Phase 4: Thread Types and Functions

#### Cycle 10: Thread creation
- **RED**: Write test `test_create_thread_minimal`
  - Mock: `POST /threads?api-version=v1` with empty body returns thread
  - Assert: Thread has `id`, `object`, `created_at`
- **GREEN**: Implement `Thread` struct and `async fn create_thread(client: &FoundryClient) -> FoundryResult<Thread>`
  - `Thread` fields: `id: String`, `object: String`, `created_at: u64`, `metadata: Option<serde_json::Value>`
- **REFACTOR**: None needed

#### Cycle 11: Thread with metadata
- **RED**: Write test `test_create_thread_with_metadata`
  - Mock: POST with metadata returns thread with metadata
  - Assert: Metadata is preserved in request and response
- **GREEN**: Implement `ThreadCreateRequest` with optional metadata
  - Add `async fn create_thread_with_metadata(client: &FoundryClient, metadata: serde_json::Value) -> FoundryResult<Thread>`
- **REFACTOR**: Merge into single function with `Option<serde_json::Value>` parameter

#### Cycle 12: Get and delete thread
- **RED**: Write tests `test_get_thread_success` and `test_delete_thread_success`
  - Assert: Correct paths are constructed
- **GREEN**: Implement `async fn get_thread` and `async fn delete_thread`
  - Similar pattern to agent functions
- **REFACTOR**: Add tracing spans

### Phase 5: Message Types and Functions

#### Cycle 13: Message types
- **RED**: Write test `test_message_serialization`
  - Assert: User message serializes with role="user" and content
  - Assert: Assistant message serializes correctly
- **GREEN**: Implement `Message` struct and `MessageRole` enum
  - `Message` fields: `id: String`, `object: String`, `created_at: u64`, `thread_id: String`, `role: MessageRole`, `content: Vec<MessageContent>`
  - `MessageRole`: `User`, `Assistant`
  - `MessageContent` struct for text content
- **REFACTOR**: None needed

#### Cycle 14: Create message function
- **RED**: Write test `test_create_message_success`
  - Mock: `POST /threads/{thread_id}/messages?api-version=v1` returns message
  - Assert: Message is created in correct thread
- **GREEN**: Implement `MessageCreateRequest` and `async fn create_message`
  - Request fields: `role: MessageRole`, `content: String`
  - Path construction with thread_id
- **REFACTOR**: Add builder pattern for `MessageCreateRequest`

#### Cycle 15: List messages function
- **RED**: Write test `test_list_messages_success`
  - Mock: `GET /threads/{thread_id}/messages?api-version=v1` returns message list
  - Assert: Messages are ordered correctly
- **GREEN**: Implement `MessageList` struct and `async fn list_messages`
  - Similar to `AgentList` structure
- **REFACTOR**: Add tracing span

### Phase 6: Run Types and Functions

#### Cycle 16: Run request and response types
- **RED**: Write test `test_run_creation_serialization`
  - Assert: Run request includes `assistant_id` and optional instructions
- **GREEN**: Implement `RunCreateRequest` and `Run` structs
  - `RunCreateRequest` fields: `assistant_id: String`, `instructions: Option<String>`
  - `Run` fields: `id: String`, `object: String`, `created_at: u64`, `thread_id: String`, `assistant_id: String`, `status: RunStatus`, `required_action: Option<RequiredAction>`, `last_error: Option<RunError>`
  - `RunStatus` enum: `Queued`, `InProgress`, `RequiresAction`, `Cancelling`, `Cancelled`, `Failed`, `Completed`, `Expired`
- **REFACTOR**: Add comprehensive doc comments for statuses

#### Cycle 17: Create run function
- **RED**: Write test `test_create_run_success`
  - Mock: `POST /threads/{thread_id}/runs?api-version=v1` returns run
  - Assert: Run is created with correct agent
- **GREEN**: Implement `async fn create_run(client: &FoundryClient, thread_id: &str, request: &RunCreateRequest) -> FoundryResult<Run>`
- **REFACTOR**: Add tracing span with `assistant_id` and `thread_id`

#### Cycle 18: Get run function
- **RED**: Write test `test_get_run_success`
  - Mock: `GET /threads/{thread_id}/runs/{run_id}?api-version=v1` returns run with status
  - Assert: Run status changes are reflected
- **GREEN**: Implement `async fn get_run(client: &FoundryClient, thread_id: &str, run_id: &str) -> FoundryResult<Run>`
- **REFACTOR**: None needed

#### Cycle 19: Create thread and run (combined operation)
- **RED**: Write test `test_create_thread_and_run_success`
  - Mock: `POST /threads/runs?api-version=v1` returns run with new thread
  - Assert: Single API call creates both thread and run
- **GREEN**: Implement `ThreadAndRunRequest` and `async fn create_thread_and_run`
  - Request fields: `assistant_id: String`, `thread: Option<ThreadCreateRequest>`, `instructions: Option<String>`
- **REFACTOR**: Add builder pattern

### Phase 7: Error Handling

#### Cycle 20: API error responses
- **RED**: Write test `test_agent_not_found_error`
  - Mock: `GET /assistants/nonexistent?api-version=v1` returns 404
  - Assert: Error is `FoundryError::Http` with status 404
- **GREEN**: No implementation needed - uses existing `FoundryClient` error handling
- **REFACTOR**: Add integration test examples in docs

#### Cycle 21: Validation errors
- **RED**: Write test `test_agent_builder_validation`
  - Assert: Empty model returns `FoundryError::Builder`
  - Assert: Invalid run status transition returns error
- **GREEN**: Implement validation in builders
  - Check non-empty strings
  - Validate enum values
- **REFACTOR**: Extract validation to shared helpers in `models.rs`

### Phase 8: Tracing Instrumentation

#### Cycle 22: Add tracing to all functions
- **RED**: Write test `test_tracing_spans_emitted` using `tracing-test`
  - Assert: `foundry::agents::create` span is recorded
  - Assert: Span contains `model` field
- **GREEN**: Add `#[tracing::instrument]` to all public async functions
  - Spans for: `create`, `get`, `list`, `delete` (agents)
  - Spans for: `create_thread`, `get_thread`, `delete_thread`
  - Spans for: `create_message`, `list_messages`
  - Spans for: `create_run`, `get_run`, `create_thread_and_run`
- **REFACTOR**: Ensure consistent field names across spans

### Phase 9: Documentation

#### Cycle 23: Module-level documentation
- **RED**: Write `cargo doc --no-deps -p azure_ai_foundry_agents` and check for missing docs warnings
  - Assert: All public items have doc comments
- **GREEN**: Add comprehensive module docs to:
  - `lib.rs` - Crate overview with quickstart example
  - `agent.rs` - Agent management examples
  - `thread.rs` - Thread lifecycle examples
  - `message.rs` - Message handling examples
  - `run.rs` - Run execution examples
- **REFACTOR**: Add cross-references between related types

#### Cycle 24: Doc tests
- **RED**: Write doc tests that demonstrate typical workflows
  - Assert: Doc tests compile (use `no_run` for async examples)
- **GREEN**: Add doc test examples:
  - Create agent and run a conversation
  - Handle run status polling
  - Error handling patterns
- **REFACTOR**: Extract common setup to test utilities

### Phase 10: Integration Tests (Optional)

#### Cycle 25: Integration test infrastructure
- **RED**: Write integration test `test_agent_lifecycle_integration`
  - Skip if `AZURE_AI_FOUNDRY_ENDPOINT` not set
  - Assert: Can create, get, and delete agent
- **GREEN**: Create `sdk/azure_ai_foundry_agents/tests/integration.rs`
  - Use feature flag `integration-tests`
  - Read credentials from environment
- **REFACTOR**: Share test setup with other crates

## Estimación Total

- **Fase 1-2 (Estructura y tipos básicos)**: 2 horas
- **Fase 3 (Agent API)**: 2 horas
- **Fase 4 (Thread API)**: 1.5 horas
- **Fase 5 (Message API)**: 1.5 horas
- **Fase 6 (Run API)**: 2 horas
- **Fase 7 (Error handling)**: 1 hora
- **Fase 8 (Tracing)**: 1 hora
- **Fase 9 (Documentación)**: 1.5 horas
- **Fase 10 (Tests de integración)**: 1 hora

**Total estimado**: ~13.5 horas de implementación

## Criterios de Éxito

- [ ] Tests funcionales pasan (objetivo: 100+ tests)
- [ ] Todos los módulos públicos tienen doc comments
- [ ] `cargo clippy` sin warnings
- [ ] `cargo fmt --check` pasa
- [ ] Doc tests compilan
- [ ] Tracing instrumentation en todas las funciones públicas
- [ ] Crate se puede publicar a crates.io (metadata correcta)
- [ ] Ejemplos en docs demuestran workflows comunes

## Archivos a Crear/Modificar

### Nuevos archivos:
- `sdk/azure_ai_foundry_agents/Cargo.toml`
- `sdk/azure_ai_foundry_agents/src/lib.rs`
- `sdk/azure_ai_foundry_agents/src/agent.rs`
- `sdk/azure_ai_foundry_agents/src/thread.rs`
- `sdk/azure_ai_foundry_agents/src/message.rs`
- `sdk/azure_ai_foundry_agents/src/run.rs`
- `sdk/azure_ai_foundry_agents/src/models.rs`
- `sdk/azure_ai_foundry_agents/tests/agent_tests.rs`
- `sdk/azure_ai_foundry_agents/tests/thread_tests.rs`
- `sdk/azure_ai_foundry_agents/tests/message_tests.rs`
- `sdk/azure_ai_foundry_agents/tests/run_tests.rs`

### Archivos a modificar:
- `Cargo.toml` (root) - Agregar `azure_ai_foundry_agents` a members
- `CLAUDE.md` - Actualizar session status después de implementar

## Notas de Implementación

1. **Seguir patrón existente**: Todos los tipos y funciones deben seguir el patrón establecido en `azure_ai_foundry_models`
2. **Builder pattern**: Usar builders para tipos de request complejos
3. **Error handling**: Reutilizar `FoundryError` existente, no crear nuevos tipos de error
4. **Async-first**: Todas las funciones públicas son `async fn`
5. **API version**: Hardcodear `?api-version=v1` en las URLs (puede ser configurable en el futuro)
6. **Project path**: Asumir que el endpoint ya incluye `/api/projects/{project}`, solo agregar paths relativos
7. **Retry logic**: Heredado de `FoundryClient`, no reimplementar
8. **Streaming**: No es necesario para agentes (solo para chat completions)

## Referencias

### Documentación Azure:
- [Azure AI Foundry Agent Service REST API](https://learn.microsoft.com/en-us/rest/api/aifoundry/aiagents/)
- [Threads, Runs, and Messages concepts](https://learn.microsoft.com/en-us/azure/ai-foundry/agents/concepts/threads-runs-messages)
- [Agent Service Quickstart](https://learn.microsoft.com/en-us/azure/ai-foundry/agents/quickstart)

### Crates de referencia:
- `azure_ai_foundry_core` - Cliente HTTP, auth, errores
- `azure_ai_foundry_models` - Patrón de implementación
