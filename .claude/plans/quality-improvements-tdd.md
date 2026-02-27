# Plan TDD: Mejoras de Calidad v0.3.0

## Contexto

Este plan implementa los hallazgos críticos y mejoras identificados durante la revisión de calidad del Azure AI Foundry SDK. El objetivo es mejorar la robustez, seguridad y ergonomía del SDK siguiendo estrictamente la metodología TDD.

**Stack detectado**: Rust 1.88, async/await con tokio, reqwest HTTP client
**Convenciones**: Builder pattern, thiserror para errores, tracing para logging, tests unitarios con wiremock
**Afecta hot path**: No (estas son mejoras de validación y refactorización)

## Decisiones Previas Necesarias

No hay decisiones arquitectónicas bloqueantes. Todas las mejoras son incrementales y mantienen compatibilidad con la API actual.

## Plan de Ejecución

---

## Task 1: Validación de max_tokens en ChatCompletionRequestBuilder

### Contexto
Actualmente el builder acepta `max_tokens = 0`, lo cual es inválido para la API de Azure AI Foundry. Necesitamos validar que si se especifica `max_tokens`, debe ser mayor que 0.

### Cycle 1: Validar max_tokens > 0 cuando está presente

- **RED**: Escribir test `test_builder_rejects_zero_max_tokens` en `sdk/azure_ai_foundry_models/src/chat.rs`
  - Ubicación: En el módulo `#[cfg(test)] mod tests` después de `test_builder_accepts_boundary_values` (línea ~991)
  - Assert: `try_build()` debe devolver `Err(FoundryError::Builder(_))` cuando `max_tokens(0)`
  - Assert: El mensaje de error debe contener "max_tokens" y "greater than 0"

- **GREEN**: Implementar validación en `ChatCompletionRequestBuilder::try_build()`
  - Ubicación: Archivo `sdk/azure_ai_foundry_models/src/chat.rs`, método `try_build()` (línea ~184)
  - Agregar validación después de validación de `frequency_penalty` (línea ~230):
    ```rust
    // Validate max_tokens (must be > 0)
    if let Some(max) = self.max_tokens {
        if max == 0 {
            return Err(FoundryError::Builder(
                "max_tokens must be greater than 0".into(),
            ));
        }
    }
    ```

- **REFACTOR**: No necesario - código simple y directo

### Cycle 2: Validar boundary case max_tokens = 1

- **RED**: Escribir test `test_builder_accepts_min_max_tokens`
  - Ubicación: Después del test anterior
  - Assert: `max_tokens(1)` debe ser aceptado (mínimo válido)
  - Assert: `request.max_tokens == Some(1)`

- **GREEN**: La implementación del Cycle 1 ya cubre este caso (validación `== 0`)

- **REFACTOR**: No necesario

### Cycle 3: Verificar error message clarity

- **RED**: Extender `test_builder_rejects_zero_max_tokens`
  - Assert adicional: `err.to_string().contains("must be greater than 0")`

- **GREEN**: Ya implementado en Cycle 1

- **REFACTOR**: No necesario

---

## Task 2: Timeout Global para Streaming

### Contexto
Las operaciones de streaming actualmente solo tienen timeout por chunk individual (via reqwest). Necesitamos un timeout total para toda la operación de streaming para prevenir streams infinitos.

### Cycle 1: Test de timeout en streaming

- **RED**: Escribir test `test_complete_stream_timeout_total_operation` en `sdk/azure_ai_foundry_models/src/chat.rs`
  - Ubicación: En módulo `tests`, después de `test_complete_stream_emits_chat_stream_span` (línea ~1911)
  - Setup: MockServer que responde con delay > streaming_timeout
  - Assert: El stream debe producir un error de timeout
  - Assert: El error debe mencionar "timeout" o "timed out"

- **GREEN**: Implementar timeout wrapper en `complete_stream()`
  - Ubicación: Función `complete_stream()` en `sdk/azure_ai_foundry_models/src/chat.rs` (línea ~537)
  - Modificar para envolver el stream con `tokio::time::timeout`:
    ```rust
    use futures::stream::{self, Stream, StreamExt};
    use std::pin::Pin;

    // Dentro de complete_stream, después de obtener parse_sse_stream:
    let timeout_duration = client.streaming_timeout();
    let stream_with_timeout = stream::unfold(
        (parse_sse_stream(response), tokio::time::Instant::now()),
        move |(mut inner_stream, start_time)| async move {
            // Check total elapsed time
            if start_time.elapsed() >= timeout_duration {
                return Some((
                    Err(FoundryError::stream("streaming operation timed out")),
                    (inner_stream, start_time),
                ));
            }

            // Get next chunk with timeout
            match tokio::time::timeout(
                timeout_duration.saturating_sub(start_time.elapsed()),
                inner_stream.next()
            ).await {
                Ok(Some(chunk)) => Some((chunk, (inner_stream, start_time))),
                Ok(None) => None,
                Err(_) => Some((
                    Err(FoundryError::stream("streaming operation timed out")),
                    (inner_stream, start_time),
                )),
            }
        }
    );
    ```

- **REFACTOR**: Considerar extraer lógica de timeout a función helper si el código crece

### Cycle 2: Test de streaming normal dentro del timeout

- **RED**: Escribir test `test_complete_stream_completes_within_timeout`
  - Ubicación: Después del test anterior
  - Setup: Stream normal que completa en 1 segundo con timeout de 5 segundos
  - Assert: El stream debe completar exitosamente sin errores de timeout
  - Assert: Todos los chunks deben ser recibidos

- **GREEN**: La implementación del Cycle 1 ya cubre este caso

- **REFACTOR**: No necesario

### Cycle 3: Configurabilidad del timeout

- **RED**: Escribir test `test_client_builder_custom_streaming_timeout`
  - Ubicación: En `sdk/azure_ai_foundry_core/src/client.rs`, módulo `tests`
  - Assert: `FoundryClientBuilder::streaming_timeout(Duration::from_secs(120))`
  - Assert: `client.streaming_timeout() == Duration::from_secs(120)`

- **GREEN**: Ya existe `streaming_timeout()` en `FoundryClientBuilder` (verificar en código)
  - Si no existe, agregar campo y método setter

- **REFACTOR**: No necesario

---

## Task 3: Refactorización de client.rs (2,454 líneas)

### Contexto
El archivo `client.rs` tiene 2,454 líneas. Necesitamos separarlo en módulos lógicos: `retry.rs` (lógica de retry), `streaming.rs` (helpers de streaming), `http.rs` (métodos HTTP).

### Cycle 1: Extraer módulo retry.rs - Tests primero

- **RED**: Crear `sdk/azure_ai_foundry_core/src/client/retry.rs` con tests
  - Tests para `is_retriable_status()`:
    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_is_retriable_status_429() {
            assert!(is_retriable_status(429));
        }

        #[test]
        fn test_is_retriable_status_5xx() {
            assert!(is_retriable_status(500));
            assert!(is_retriable_status(502));
            assert!(is_retriable_status(503));
            assert!(is_retriable_status(504));
        }

        #[test]
        fn test_is_retriable_status_non_retriable() {
            assert!(!is_retriable_status(200));
            assert!(!is_retriable_status(400));
            assert!(!is_retriable_status(404));
        }

        #[test]
        fn test_compute_backoff_first_attempt() {
            let backoff = compute_backoff(0, Duration::from_millis(100));
            // Should be ~100ms with jitter (75-125ms)
            assert!(backoff >= Duration::from_millis(75));
            assert!(backoff <= Duration::from_millis(125));
        }

        #[test]
        fn test_compute_backoff_exponential() {
            let backoff = compute_backoff(2, Duration::from_millis(100));
            // 2^2 * 100ms = 400ms with jitter (300-500ms)
            assert!(backoff >= Duration::from_millis(300));
            assert!(backoff <= Duration::from_millis(500));
        }

        #[test]
        fn test_compute_backoff_capped_at_max() {
            let backoff = compute_backoff(100, Duration::from_secs(10));
            // Should be capped at MAX_BACKOFF (60 seconds)
            assert!(backoff <= MAX_BACKOFF);
        }

        #[test]
        fn test_retry_policy_validation() {
            let policy = RetryPolicy::new(5, Duration::from_secs(1));
            assert!(policy.is_ok());

            let policy = RetryPolicy::new(11, Duration::from_secs(1));
            assert!(policy.is_err());

            let policy = RetryPolicy::new(3, Duration::from_secs(61));
            assert!(policy.is_err());
        }
    }
    ```

- **GREEN**: Mover funciones de retry desde `client.rs` a `retry.rs`:
  - Mover: `is_retriable_status()` (línea ~98)
  - Mover: `compute_backoff()` (línea ~122)
  - Mover: `extract_retry_after_delay()` (línea ~145)
  - Mover: `RetryPolicy` struct y impl (línea ~154-217)
  - Mover: Constantes `MAX_BACKOFF` (línea ~103)
  - Hacer públicos los items necesarios con `pub use`

- **REFACTOR**:
  - Actualizar `client.rs` para usar `mod retry; pub use retry::*;`
  - Verificar que todos los tests pasen

### Cycle 2: Extraer módulo http.rs - Métodos HTTP

- **RED**: Crear `sdk/azure_ai_foundry_core/src/client/http.rs` con tests básicos
  - Tests para construcción de URL:
    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_build_url_with_query_params() {
            // Test que la función de query params funciona correctamente
        }
    }
    ```

- **GREEN**: Mover métodos HTTP desde `client.rs`:
  - Mantener solo firma pública de métodos en `client.rs`
  - Implementaciones privadas (con `#[instrument]`) mover a `http.rs`
  - Métodos: `get()`, `post()`, `post_stream()`, `delete()`, `put()` (si existe)

- **REFACTOR**:
  - Asegurar que el módulo `http` sea privado (`mod http;`)
  - Re-exportar solo lo necesario
  - Verificar que trazas de tracing funcionen correctamente

### Cycle 3: Documentar estructura de módulos

- **RED**: Escribir doc-test en `client.rs` que muestre que la API no ha cambiado
  - Ubicación: En doc-comment de `FoundryClient`
  - Assert: Código de ejemplo compila y funciona igual que antes

- **GREEN**: Agregar documentación de módulos:
  ```rust
  //! HTTP client implementation for Azure AI Foundry.
  //!
  //! # Module structure
  //!
  //! - [`retry`] - Retry policy and backoff logic
  //! - [`http`] - Low-level HTTP request methods (private)
  ```

- **REFACTOR**: Revisar que toda la documentación pública sea clara

---

## Task 4: Optimizar Clones Innecesarios en auth.rs

### Contexto
Revisar `auth.rs` para identificar clones que pueden evitarse mediante referencias o `Arc::clone()` explícito.

### Cycle 1: Benchmark baseline de auth operations

- **RED**: Crear benchmark `benches/auth_bench.rs`
  - Ubicación: Crear archivo nuevo `sdk/azure_ai_foundry_core/benches/auth_bench.rs`
  - Benchmark: `resolve_api_key()` - tiempo de resolver credential 1000 veces
  - Benchmark: `resolve_token_credential()` - con token cacheado
  - Establecer baseline de rendimiento

- **GREEN**: Implementar benchmarks con `criterion`:
  ```rust
  use criterion::{black_box, criterion_group, criterion_main, Criterion};
  use azure_ai_foundry_core::auth::FoundryCredential;

  fn bench_api_key_resolve(c: &mut Criterion) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      let cred = FoundryCredential::api_key("test-key");

      c.bench_function("resolve_api_key", |b| {
          b.to_async(&rt).iter(|| async {
              black_box(cred.resolve().await)
          });
      });
  }

  criterion_group!(benches, bench_api_key_resolve);
  criterion_main!(benches);
  ```

- **REFACTOR**: No necesario

### Cycle 2: Identificar y eliminar clones innecesarios

- **RED**: No aplica (refactorización guiada por análisis estático)

- **GREEN**: Revisar `auth.rs` línea por línea:
  - Buscar `.clone()` calls
  - Evaluar si se puede usar referencia
  - Evaluar si `Arc::clone()` sería más explícito
  - Documentar por qué cada clone es necesario o eliminarlo

- **REFACTOR**:
  - Ejecutar benchmarks después de cambios
  - Verificar que no hay regresión de performance
  - Umbral: No más de 5% overhead

### Cycle 3: Verificar benchmarks post-optimización

- **RED**: Test que compara resultados de benchmark
  - Assert: Nueva versión es <= 105% del baseline (máximo 5% overhead)

- **GREEN**: Ejecutar `cargo bench` y comparar resultados

- **REFACTOR**: Documentar optimizaciones en código con comentarios

---

## Task 5: Rate Limiting Local con Governor

### Contexto
Agregar rate limiting opcional en el cliente para prevenir exceder cuotas de Azure AI. Usar el crate `governor` para implementación robusta.

### Cycle 1: Agregar dependencia y configuración básica

- **RED**: Test de rate limiter en `client.rs`
  - Ubicación: `sdk/azure_ai_foundry_core/src/client.rs` módulo tests
  - Test: `test_rate_limiter_allows_within_quota`
  - Setup: Cliente con rate limit de 10 req/s
  - Assert: 10 requests en 1 segundo pasan sin delay

- **GREEN**:
  1. Agregar a `Cargo.toml`:
     ```toml
     [dependencies]
     governor = { version = "0.7", optional = true }

     [features]
     rate-limiting = ["governor"]
     ```
  2. Agregar campo opcional en `FoundryClient`:
     ```rust
     rate_limiter: Option<Arc<governor::RateLimiter<...>>>,
     ```
  3. Agregar método en builder:
     ```rust
     pub fn rate_limit(mut self, requests_per_second: u32) -> Self {
         #[cfg(feature = "rate-limiting")]
         {
             self.rate_limiter = Some(...);
         }
         self
     }
     ```

- **REFACTOR**: No necesario

### Cycle 2: Test de throttling cuando se excede límite

- **RED**: Test `test_rate_limiter_throttles_excess_requests`
  - Setup: Rate limit de 5 req/s
  - Assert: Hacer 10 requests debe tomar >= 2 segundos
  - Assert: Todas las requests eventualmente completan

- **GREEN**: Implementar check de rate limit en métodos HTTP:
  ```rust
  #[cfg(feature = "rate-limiting")]
  if let Some(limiter) = &self.rate_limiter {
      limiter.until_ready().await;
  }
  ```

- **REFACTOR**: Extraer lógica a método helper si se repite

### Cycle 3: Test de rate limit deshabilitado por defecto

- **RED**: Test `test_client_without_rate_limit_unrestricted`
  - Setup: Cliente sin rate limit configurado
  - Assert: 100 requests en rápida sucesión completan en < 1 segundo

- **GREEN**: Ya implementado (rate_limiter es `Option`, default `None`)

- **REFACTOR**: Documentar feature flag en README y docs

---

## Task 6: Helper collect_text() para ChatCompletionChunk

### Contexto
Agregar método ergonómico `ChatCompletionChunk::collect_text()` para extraer fácilmente el contenido de texto de un chunk.

### Cycle 1: Implementar collect_text() básico

- **RED**: Test `test_chunk_collect_text_simple` en `chat.rs`
  - Ubicación: Módulo tests, cerca de otros tests de chunk (línea ~1320)
  - Setup: Chunk con `delta.content = Some("Hello")`
  - Assert: `chunk.collect_text() == Some("Hello".to_string())`

- **GREEN**: Implementar método en `ChatCompletionChunk`:
  ```rust
  impl ChatCompletionChunk {
      /// Collect text content from the first choice's delta.
      ///
      /// This is a convenience method that extracts the content from
      /// `choices[0].delta.content` if present.
      ///
      /// # Returns
      ///
      /// `Some(String)` if the first choice has content, `None` otherwise.
      pub fn collect_text(&self) -> Option<String> {
          self.choices
              .first()
              .and_then(|c| c.delta.content.clone())
      }
  }
  ```

- **REFACTOR**: Considerar si debe retornar `Option<&str>` en lugar de `Option<String>` (evitar clone)

### Cycle 2: Test de chunk sin contenido

- **RED**: Test `test_chunk_collect_text_empty_delta`
  - Setup: Chunk con `delta = Delta::default()` (sin content)
  - Assert: `chunk.collect_text() == None`

- **GREEN**: Ya cubierto por implementación del Cycle 1

- **REFACTOR**: No necesario

### Cycle 3: Test de chunk sin choices

- **RED**: Test `test_chunk_collect_text_no_choices`
  - Setup: Chunk con `choices = vec![]`
  - Assert: `chunk.collect_text() == None`

- **GREEN**: Ya cubierto por `.first()` que retorna `None`

- **REFACTOR**: Considerar cambiar a retornar `&str` para evitar clone:
  ```rust
  pub fn collect_text(&self) -> Option<&str> {
      self.choices
          .first()
          .and_then(|c| c.delta.content.as_deref())
  }
  ```

### Cycle 4: Documentar uso en streaming example

- **RED**: Doc-test en el método `collect_text()`
  - Assert: Código compila y muestra uso en streaming loop

- **GREEN**: Agregar ejemplo:
  ```rust
  /// # Example
  ///
  /// ```rust,no_run
  /// # use azure_ai_foundry_models::chat::*;
  /// # use futures::StreamExt;
  /// # async fn example(stream: impl futures::Stream<Item = Result<ChatCompletionChunk, azure_ai_foundry_core::error::FoundryError>>) {
  /// let mut stream = std::pin::pin!(stream);
  /// while let Some(chunk) = stream.next().await {
  ///     if let Ok(chunk) = chunk {
  ///         if let Some(text) = chunk.collect_text() {
  ///             print!("{}", text);
  ///         }
  ///     }
  /// }
  /// # }
  /// ```
  pub fn collect_text(&self) -> Option<&str> { ... }
  ```

- **REFACTOR**: No necesario

---

## Estimación Total

- **Task 1 (max_tokens validation)**: 1 hora
  - Implementación: 30 min
  - Testing: 30 min

- **Task 2 (streaming timeout)**: 2.5 horas
  - Implementación: 1.5 horas
  - Testing: 1 hora

- **Task 3 (refactorización client.rs)**: 4 horas
  - Separación de módulos: 2 horas
  - Testing y verificación: 1.5 horas
  - Documentación: 30 min

- **Task 4 (optimizar clones)**: 2 horas
  - Benchmarks: 1 hora
  - Optimización: 1 hora

- **Task 5 (rate limiting)**: 3 horas
  - Implementación: 2 horas
  - Testing: 1 hora

- **Task 6 (collect_text helper)**: 1 hora
  - Implementación: 30 min
  - Testing y docs: 30 min

**Total: 13.5 horas**

---

## Criterios de Éxito

- [ ] Todos los tests unitarios pasan (`cargo test --workspace`)
- [ ] Todos los tests de integración pasan (si aplica)
- [ ] Clippy no reporta warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
- [ ] Formato correcto (`cargo fmt --all -- --check`)
- [ ] Documentación actualizada (doc-tests pasan)
- [ ] Benchmarks muestran que no hay regresión de rendimiento (Task 4)
- [ ] Cobertura de tests: Todas las nuevas validaciones tienen tests
- [ ] Compatibilidad: API pública no ha cambiado (excepto adiciones)

---

## Notas de Implementación

### Orden de Ejecución Recomendado

1. **Task 1** (max_tokens) - Más simple, bajo riesgo
2. **Task 6** (collect_text) - Pequeño, independiente, agrega valor inmediato
3. **Task 4** (clones) - Optimización sin cambios de API
4. **Task 2** (streaming timeout) - Mejora de robustez crítica
5. **Task 3** (refactorización) - Mayor complejidad, hacer cuando otras estén estables
6. **Task 5** (rate limiting) - Feature opcional, hacer al final

### Riesgos y Mitigaciones

**Riesgo**: Refactorización de `client.rs` puede romper dependencias internas
**Mitigación**: Ejecutar todos los tests después de cada movimiento de código

**Riesgo**: Streaming timeout puede romper streams largos legítimos
**Mitigación**: Timeout configurable y valor por defecto generoso (5 minutos)

**Riesgo**: Rate limiting puede causar deadlocks si se implementa mal
**Mitigación**: Usar `governor` crate battle-tested, feature flag opcional

### Testing

- Tests unitarios para cada función pública
- Tests de integración con wiremock para APIs HTTP
- Benchmarks para cambios de performance (Task 4)
- Doc-tests para ejemplos de uso

### Compatibilidad

- Todas las mejoras son backwards-compatible
- Rate limiting es opt-in via feature flag
- Métodos nuevos son adiciones, no cambios breaking
