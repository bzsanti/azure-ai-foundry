# TDD Implementation Plan: v0.2.0 Quality Improvements

## Overview

Este plan implementa 4 mejoras de calidad críticas para el SDK Azure AI Foundry siguiendo metodología TDD estricta:

1. **Token Caching** - Caché thread-safe en `FoundryClient` para reducir llamadas innecesarias a Entra ID
2. **HTTP Timeouts Configurables** - Configuración de timeouts en el builder para control de latencia
3. **Retry Logic con Exponential Backoff** - Reintentos automáticos para errores transitorios (429, 503, 504)
4. **Error Source Preservation** - Preservación de la cadena de errores original con `#[source]`

**Stack**: Rust 1.88, async tokio, reqwest HTTP client, azure_identity para auth
**Convenciones**: Builder pattern con `try_build()`, errors con `thiserror`, tests con `wiremock` y `serial_test`

---

## Feature 1: Token Caching

**Objetivo**: Implementar caché thread-safe de tokens Entra ID en `FoundryClient` para evitar llamadas redundantes.

### Cycle 1.1: Estructura de caché básica

**RED**: Write test `test_token_cache_stores_valid_token`
- Setup: Crear `MockTokenCredential` que cuenta llamadas a `get_token()`
- Action: Llamar `credential.resolve()` dos veces
- Assert: Verificar que `get_token()` se llama solo 1 vez (segunda usa caché)

**GREEN**: Implementar caché básica
- Añadir campo `token_cache: Arc<Mutex<Option<CachedToken>>>` a `FoundryCredential`
- Struct `CachedToken { token: String, expires_at: OffsetDateTime }`
- Modificar `resolve()` para guardar token en caché tras obtenerlo

**REFACTOR**: Extraer lógica de caché a método privado `get_or_refresh_token()`

---

### Cycle 1.2: Expiración de tokens

**RED**: Write test `test_token_cache_expires_after_ttl`
- Setup: Mock que devuelve token expirando en 1 segundo
- Action: Llamar `resolve()`, esperar 2 segundos, llamar de nuevo
- Assert: `get_token()` llamado 2 veces (el segundo refresh por expiración)

**GREEN**: Implementar lógica de expiración
- En `get_or_refresh_token()`, verificar `expires_at > OffsetDateTime::now_utc()`
- Si expirado, limpiar caché y solicitar nuevo token

**REFACTOR**: N/A

---

### Cycle 1.3: Thread-safety del caché

**RED**: Write test `test_token_cache_thread_safe`
- Setup: Cliente compartido entre 10 tasks concurrentes
- Action: Todas las tasks llaman `resolve()` simultáneamente
- Assert: Solo 1 llamada a `get_token()` (sin race conditions)

**GREEN**: Usar `tokio::sync::Mutex` para proteger caché
- Cambiar de `std::sync::Mutex` a `tokio::sync::Mutex<Option<CachedToken>>`
- Lock durante verificación + potencial refresh

**REFACTOR**: Documentar thread-safety en doc comments

---

### Cycle 1.4: Buffer de expiración (safety margin)

**RED**: Write test `test_token_cache_refreshes_before_expiry`
- Setup: Token que expira en 5 minutos
- Action: Avanzar tiempo a 4:30 min (30s antes de expirar)
- Assert: Siguiente `resolve()` hace refresh (no espera a expiración real)

**GREEN**: Añadir buffer de 60 segundos
- Modificar condición: `expires_at - Duration::from_secs(60) > now`

**REFACTOR**: Hacer buffer configurable como constante `const TOKEN_EXPIRY_BUFFER: Duration`

---

### Cycle 1.5: Caché no afecta API keys

**RED**: Write test `test_api_key_credential_no_cache`
- Setup: `FoundryCredential::api_key("test")`
- Action: Llamar `resolve()` múltiples veces
- Assert: Siempre devuelve `"Bearer test"` inmediatamente (sin caché)

**GREEN**: En `resolve()`, match sobre `Self::ApiKey` devuelve directo
- Solo rama `Self::TokenCredential` usa `get_or_refresh_token()`

**REFACTOR**: N/A

---

## Feature 2: HTTP Timeouts Configurables

**Objetivo**: Permitir configurar timeouts de conexión y lectura en el `FoundryClientBuilder`.

### Cycle 2.1: Timeout de conexión configurable

**RED**: Write test `test_builder_accepts_connect_timeout`
- Setup: `FoundryClient::builder().connect_timeout(Duration::from_secs(5))`
- Action: Build client
- Assert: HTTP client interno tiene timeout de conexión de 5s

**GREEN**: Implementar configuración de timeout
- Añadir campo `connect_timeout: Option<Duration>` a `FoundryClientBuilder`
- Método `pub fn connect_timeout(mut self, timeout: Duration) -> Self`
- En `build()`, si `http_client` no está set, crear con `reqwest::ClientBuilder::new().connect_timeout(timeout)`

**REFACTOR**: N/A

---

### Cycle 2.2: Timeout de lectura configurable

**RED**: Write test `test_builder_accepts_read_timeout`
- Setup: `builder().read_timeout(Duration::from_secs(30))`
- Action: Build client
- Assert: Timeout de lectura configurado en HTTP client

**GREEN**: Añadir campo `read_timeout` + método builder
- En `build()`, aplicar con `ClientBuilder::timeout(read_timeout)`

**REFACTOR**: Extraer creación de `reqwest::Client` a método privado `build_http_client()`

---

### Cycle 2.3: Timeouts por defecto

**RED**: Write test `test_default_timeouts_applied`
- Setup: Builder SIN configurar timeouts
- Action: Build client
- Assert: Tiene timeouts por defecto (connect=10s, read=60s)

**GREEN**: Definir constantes
```rust
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);
```
- En `build()`, usar `unwrap_or(DEFAULT_*)`

**REFACTOR**: N/A

---

### Cycle 2.4: Timeout personalizado no sobrescribe http_client custom

**RED**: Write test `test_custom_http_client_ignores_timeout_config`
- Setup: Cliente custom con timeouts específicos + `builder().connect_timeout(5s)`
- Action: Build con `http_client(custom_client)`
- Assert: Usa cliente custom (ignora timeout del builder)

**GREEN**: En `build()`, verificar orden de precedencia
```rust
let http = self.http_client.unwrap_or_else(|| self.build_http_client());
```

**REFACTOR**: Documentar precedencia en doc comments de `http_client()`

---

### Cycle 2.5: Integration test con wiremock timeout

**RED**: Write test `test_request_times_out_with_configured_timeout`
- Setup: Wiremock que demora 5 segundos, client con `read_timeout(1s)`
- Action: `client.get("/slow")`
- Assert: Falla con `FoundryError::Request` por timeout

**GREEN**: N/A (ya funciona por reqwest)

**REFACTOR**: N/A

---

## Feature 3: Retry Logic con Exponential Backoff

**Objetivo**: Reintentos automáticos para errores transitorios (429, 503, 504) con backoff exponencial.

### Cycle 3.1: Detectar errores retriables

**RED**: Write test `test_identifies_retriable_http_errors`
- Setup: Helper `is_retriable_error(status: u16) -> bool`
- Assert: `is_retriable_error(429) == true`
- Assert: `is_retriable_error(503) == true`
- Assert: `is_retriable_error(504) == true`
- Assert: `is_retriable_error(400) == false`

**GREEN**: Implementar función privada
```rust
fn is_retriable_error(status: u16) -> bool {
    matches!(status, 429 | 503 | 504)
}
```

**REFACTOR**: N/A

---

### Cycle 3.2: Retry policy configurable

**RED**: Write test `test_builder_accepts_retry_policy`
- Setup: `RetryPolicy { max_retries: 3, initial_backoff: Duration::from_millis(100) }`
- Action: `builder().retry_policy(policy).build()`
- Assert: Cliente tiene política configurada

**GREEN**: Crear struct
```rust
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(500),
        }
    }
}
```
- Añadir campo a `FoundryClient` y builder

**REFACTOR**: N/A

---

### Cycle 3.3: Retry con exponential backoff en GET

**RED**: Write test `test_get_retries_on_503_with_backoff`
- Setup: Wiremock que falla 503 dos veces, luego 200
- Action: `client.get("/endpoint")`
- Assert: 3 requests totales (2 retries + 1 success)
- Assert: Delays entre retries son ~500ms, ~1000ms (exponencial)

**GREEN**: Modificar `get()` para usar retry loop
```rust
pub async fn get(&self, path: &str) -> FoundryResult<reqwest::Response> {
    let url = self.url(path)?;
    let auth = self.credential.resolve().await?;

    for attempt in 0..=self.retry_policy.max_retries {
        let response = self.http.get(url.clone())
            .header("Authorization", &auth)
            .header("api-version", &self.api_version)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status().as_u16();
        if !is_retriable_error(status) || attempt == self.retry_policy.max_retries {
            return Self::check_response(response).await;
        }

        let backoff = self.retry_policy.initial_backoff * 2_u32.pow(attempt);
        tokio::time::sleep(backoff).await;
    }

    unreachable!()
}
```

**REFACTOR**: Extraer retry logic a método genérico `retry_request()`

---

### Cycle 3.4: Retry en POST

**RED**: Write test `test_post_retries_on_429_rate_limit`
- Setup: Wiremock 429 con header `Retry-After: 1`, luego 200
- Action: `client.post("/endpoint", &body)`
- Assert: Retry exitoso después de 429

**GREEN**: Aplicar misma lógica de retry a `post()` usando `retry_request()`

**REFACTOR**: N/A

---

### Cycle 3.5: Respetar header Retry-After

**RED**: Write test `test_respects_retry_after_header`
- Setup: 429 con `Retry-After: 2`
- Action: Retry
- Assert: Delay es mínimo 2 segundos (overrides exponential backoff)

**GREEN**: Parsear header en retry loop
```rust
if let Some(retry_after) = response.headers().get("retry-after") {
    let delay = parse_retry_after(retry_after)?;
    tokio::time::sleep(delay).await;
} else {
    // exponential backoff normal
}
```

**REFACTOR**: Implementar `parse_retry_after()` que soporta segundos y HTTP-date

---

### Cycle 3.6: No retry en streaming

**RED**: Write test `test_post_stream_does_not_retry`
- Setup: `post_stream()` que recibe 503
- Action: Llamada falla
- Assert: NO hay retries (streaming consume body, no es idempotente)

**GREEN**: `post_stream()` NO usa retry logic (mantener código actual)

**REFACTOR**: Documentar en doc comment por qué streaming no hace retry

---

### Cycle 3.7: Jitter en backoff

**RED**: Write test `test_retry_backoff_includes_jitter`
- Setup: 10 retries consecutivos
- Action: Medir delays
- Assert: Delays varían ligeramente (no exactamente 500, 1000, 2000...)

**GREEN**: Añadir jitter aleatorio ±25%
```rust
use rand::Rng;
let jitter = rand::thread_rng().gen_range(0.75..1.25);
let backoff = self.retry_policy.initial_backoff * 2_u32.pow(attempt);
tokio::time::sleep(backoff.mul_f64(jitter)).await;
```

**REFACTOR**: N/A

---

## Feature 4: Error Source Preservation

**Objetivo**: Preservar la cadena de errores original usando `#[source]` de `thiserror` para mejor debugging.

### Cycle 4.1: Preservar source en Auth errors

**RED**: Write test `test_auth_error_preserves_source`
- Setup: Crear `azure_core::Error` con mensaje "token expired"
- Action: Convertir a `FoundryError::Auth`
- Assert: `error.source()` contiene el error original de Azure

**GREEN**: Modificar variante `Auth`
```rust
#[error("Authentication failed: {message}")]
Auth {
    message: String,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
},
```
- Actualizar constructores para aceptar `source`

**REFACTOR**: Crear helper `FoundryError::auth_with_source(msg, source)`

---

### Cycle 4.2: Preservar source en HTTP errors

**RED**: Write test `test_http_error_preserves_reqwest_source`
- Setup: `reqwest::Error` de timeout
- Action: Convertir a `FoundryError::Http`
- Assert: `error.source()` es el `reqwest::Error` original

**GREEN**: Añadir `#[source]` a `Http` variant
```rust
#[error("HTTP error: {status} - {message}")]
Http {
    status: u16,
    message: String,
    #[source]
    source: Option<reqwest::Error>,
},
```

**REFACTOR**: N/A

---

### Cycle 4.3: Chain completa de errores

**RED**: Write test `test_error_chain_preserves_all_sources`
- Setup: Azure error → Auth error → Request error
- Action: Unwrap toda la cadena con `error.source().unwrap().source()...`
- Assert: Cadena completa accesible hasta error raíz

**GREEN**: Ya funciona con `#[source]` en todas las variantes

**REFACTOR**: N/A

---

### Cycle 4.4: InvalidEndpoint preserva url::ParseError

**RED**: Write test `test_invalid_endpoint_preserves_parse_error`
- Setup: `Url::parse("not a url")` falla
- Action: Convertir a `FoundryError::InvalidEndpoint`
- Assert: `error.source()` es el `url::ParseError`

**GREEN**: Modificar `InvalidEndpoint`
```rust
#[error("Invalid endpoint URL: {message}")]
InvalidEndpoint {
    message: String,
    #[source]
    source: Option<url::ParseError>,
},
```

**REFACTOR**: Actualizar usos en `client.rs` para pasar source

---

### Cycle 4.5: Stream errors preservan source

**RED**: Write test `test_stream_error_preserves_source`
- Setup: Error de deserialización en SSE parsing
- Action: Crear `FoundryError::Stream`
- Assert: Source es el `serde_json::Error` original

**GREEN**: Añadir source a `Stream`
```rust
#[error("Stream error: {message}")]
Stream {
    message: String,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
},
```

**REFACTOR**: N/A

---

### Cycle 4.6: Backward compatibility de error messages

**RED**: Write test `test_error_display_unchanged_for_existing_code`
- Setup: Crear errores con los nuevos fields opcionales
- Action: Llamar `to_string()`
- Assert: Mensajes de error idénticos a versión anterior (no breaking change)

**GREEN**: Todos los `#[source]` son `Option<T>` (retrocompatibles)

**REFACTOR**: N/A

---

## Execution Order

### Fase 1: Foundation (Features 2 y 4 primero)
**Justificación**: HTTP timeouts y error preservation son bases necesarias para retry logic.

1. **Feature 4: Error Source Preservation** (6 cycles, ~2 horas)
   - Ciclos 4.1 → 4.6
   - Sin dependencias, mejora debugging inmediato

2. **Feature 2: HTTP Timeouts** (5 cycles, ~1.5 horas)
   - Ciclos 2.1 → 2.5
   - Necesario para tests de retry

### Fase 2: Advanced Features (Features 3 y 1)

3. **Feature 3: Retry Logic** (7 cycles, ~3 horas)
   - Ciclos 3.1 → 3.7
   - Depende de: timeouts configurables (Feature 2)
   - Depende de: error sources (Feature 4, para retry decisions)

4. **Feature 1: Token Caching** (5 cycles, ~2 horas)
   - Ciclos 1.1 → 1.5
   - Independiente pero se beneficia de retry logic
   - Debe ir última para no interferir con tests de retry

### Fase 3: Integration Testing

5. **End-to-End Tests**
   - Test combinado: Client con timeouts + retry + token cache
   - Test de rendimiento: Verificar que token cache reduce latencia >30%
   - Test de resiliencia: 10 requests con 50% 503 errors, todos deben tener éxito

### Fase 4: Documentation & Examples

6. **Update Examples**
   - Añadir ejemplo de configuración de timeouts en `client.rs` doc
   - Ejemplo de retry policy custom
   - Mencionar token caching automático en `auth.rs` doc

---

## Dependencies & Crates

### Nuevas dependencias necesarias

**Cargo.toml additions**:
```toml
[dependencies]
# Para retry jitter
rand = "0.8"
```

### Compatibilidad

- **MSRV**: 1.88 (sin cambios)
- **Breaking changes**: NINGUNO (todas las features son opt-in o compatibles)
- **Semver**: v0.2.0 (minor bump, features nuevas sin breaking changes)

---

## Criterios de Éxito

### Funcionales
- [ ] 100% tests passing (unit + integration)
- [ ] Token cache reduce llamadas a Entra ID en >90% en escenarios multi-request
- [ ] Retry logic maneja 429/503/504 con max 3 retries por defecto
- [ ] Timeouts configurables sin romper API pública existente
- [ ] Error chains completas accesibles vía `.source()`

### Rendimiento
- [ ] Throughput no degrada >5% vs baseline
- [ ] Token cache reduce latencia de auth en ≥30% (multi-request)
- [ ] Request sin retries NO añade overhead medible

### Calidad
- [ ] `cargo clippy -- -D warnings` pasa
- [ ] `cargo fmt --check` pasa
- [ ] Documentación actualizada en todos los items públicos
- [ ] CHANGELOG.md actualizado para v0.2.0

---

## TDD Compliance Verification

- ✅ Cada cycle tiene RED → GREEN → REFACTOR explícito
- ✅ Tests especificados ANTES de implementación
- ✅ Assertions concretas y verificables
- ✅ Implementación mínima descrita en GREEN phase
- ✅ REFACTOR phase solo cuando mejora calidad

**OBLIGATORIO**: NO escribir código de producción sin test RED que falle primero
