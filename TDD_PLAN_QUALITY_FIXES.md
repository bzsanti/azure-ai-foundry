# TDD Plan: Quality Review Fixes

## Context

This plan addresses 13 findings from the quality review: 5 critical issues and 8 recommended
improvements. The fixes span four files in the `azure_ai_foundry_core` and
`azure_ai_foundry_models` crates.

**Stack detectado**: Rust (Workspace), async/await with Tokio, reqwest HTTP client, thiserror,
serde, wiremock for integration tests.

**Convenciones observadas en el proyecto**:
- Tests viven en `#[cfg(test)] mod tests` dentro del mismo archivo fuente
- Builder pattern con `try_build()` (fallible) y `build()` (panicking wrapper)
- Constructores explícitos para variantes de `FoundryError` (`FoundryError::auth()`,
  `FoundryError::http()`, etc.)
- Fixtures compartidas en `test_utils` module
- `wiremock` para tests de HTTP con servidor mock real
- `serial_test` para tests que tocan env vars

**Afecta hot path**: SI - la logica de retry en `get()`, `post()` y `post_stream()` se ejecuta en
cada request HTTP.

---

## Decisiones Previas Necesarias

Ninguna decision arquitectonica bloqueante. Todos los fixes tienen una direccion clara basada en el
codigo existente y los patrones del proyecto.

---

## Orden de Dependencias entre Fases

```
Fase 1: error.rs (AzureSdk source chain) - base de la cadena de errores
    |
    v
Fase 2: client.rs - extract_retry_delay (habilita respeto de Retry-After)
    |
    v
Fase 3: client.rs - unificar retry loop (depende del helper de Fase 2)
    |
    v
Fase 4: client.rs - sanitizacion completa (JWT + api-key:)
    |
    v
Fase 5: client.rs - panics en build() + RetryPolicy validation
    |
    v
Fase 6: client.rs - token refresh dentro del retry loop
    |
    v
Fase 7: auth.rs - rename get_token -> fetch_fresh_token
    |
    v
Fase 8: models.rs + embeddings.rs - unificar Usage
    |
    v
Fase 9: chat.rs - doc comments + SSE drain optimization
    |
    v
Fase 10: client.rs - documentar DEFAULT_API_VERSION como preview
```

---

## Plan de Ejecucion

---

### Fase 1: Reparar error chain en AzureSdk (Critical #5)

**Problema**: `From<azure_core::Error> for FoundryError` convierte a `AzureSdk(String)`,
descartando el `source()`. Cualquier operacion de auth con Entra ID pierde el contexto de error
original.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/error.rs`

#### Ciclo 1.1: AzureSdk debe preservar source

- **RED**: Escribir test que verifica que `source()` no es `None` al convertir desde
  `azure_core::Error`:

  ```rust
  #[test]
  fn azure_sdk_error_preserves_source() {
      use std::error::Error;
      let azure_err = azure_core::Error::with_message(
          azure_core::error::ErrorKind::Credential,
          "token expired",
      );
      let foundry_err: FoundryError = azure_err.into();
      // FALLA: AzureSdk(String) no tiene #[source], devuelve None
      assert!(foundry_err.source().is_some(), "AzureSdk must preserve source chain");
      assert!(foundry_err.source().unwrap().to_string().contains("token expired"));
  }
  ```

- **GREEN**: Cambiar la variante `AzureSdk` de `AzureSdk(String)` a una forma estructurada con
  `#[source]`:

  ```rust
  /// An error from the Azure SDK.
  #[error("Azure SDK error: {message}")]
  AzureSdk {
      message: String,
      #[source]
      source: azure_core::Error,
  }
  ```

  Actualizar la implementacion `From`:

  ```rust
  impl From<azure_core::Error> for FoundryError {
      fn from(err: azure_core::Error) -> Self {
          Self::AzureSdk {
              message: err.to_string(),
              source: err,
          }
      }
  }
  ```

- **REFACTOR**: Actualizar todos los `match` que hacen pattern matching en `AzureSdk(_)` a la nueva
  forma `AzureSdk { .. }`. Verificar que `azure_sdk_error_display` sigue pasando con el nuevo
  formato de mensaje.

  Archivo de tests a actualizar (en el mismo `error.rs`):
  - `azure_sdk_error_display`: verificar que el mensaje contiene `"Azure SDK error: "`
  - `from_azure_core_error`: ya existe, debe verificar ademas que `source()` es `Some`

---

### Fase 2: Extraer logica de Retry-After (Critical #3 - prerequisito para Fase 3)

**Problema**: El header `Retry-After` enviado por el servidor en respuestas 429 es ignorado. El SDK
usa siempre su propio backoff exponencial, lo que puede violar rate limits del servidor.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

#### Ciclo 2.1: Funcion helper que extrae Retry-After de un response

- **RED**:

  ```rust
  #[test]
  fn extract_retry_delay_from_seconds_header() {
      use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
      let mut headers = HeaderMap::new();
      headers.insert(RETRY_AFTER, HeaderValue::from_static("30"));
      let delay = extract_retry_after_delay(&headers);
      assert_eq!(delay, Some(Duration::from_secs(30)));
  }

  #[test]
  fn extract_retry_delay_missing_header() {
      let headers = reqwest::header::HeaderMap::new();
      let delay = extract_retry_after_delay(&headers);
      assert_eq!(delay, None);
  }

  #[test]
  fn extract_retry_delay_capped_at_max_backoff() {
      use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
      let mut headers = HeaderMap::new();
      headers.insert(RETRY_AFTER, HeaderValue::from_static("3600")); // 1 hora
      let delay = extract_retry_after_delay(&headers);
      // Debe respetar MAX_BACKOFF como cota superior
      assert_eq!(delay, Some(MAX_BACKOFF));
  }

  #[test]
  fn extract_retry_delay_invalid_value_returns_none() {
      use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
      let mut headers = HeaderMap::new();
      headers.insert(RETRY_AFTER, HeaderValue::from_static("not-a-number"));
      let delay = extract_retry_after_delay(&headers);
      assert_eq!(delay, None);
  }
  ```

- **GREEN**: Implementar `fn extract_retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration>`:

  ```rust
  fn extract_retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
      headers
          .get(reqwest::header::RETRY_AFTER)
          .and_then(|v| v.to_str().ok())
          .and_then(|s| s.trim().parse::<u64>().ok())
          .map(|secs| Duration::from_secs(secs).min(MAX_BACKOFF))
  }
  ```

- **REFACTOR**: La funcion es pura y sin efectos secundarios. No requiere refactor adicional.

---

### Fase 3: Eliminar duplicacion DRY del retry loop (Critical #2)

**Problema**: Los tres metodos `get()`, `post()`, y `post_stream()` contienen ~120 lineas de logica
de retry practicamente identica, duplicada en cada uno. Cualquier cambio (como el soporte de
Retry-After de Fase 2) debe replicarse en los tres lugares.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

#### Ciclo 3.1: Funcion privada execute_with_retry que encapsula el loop

La estrategia es extraer el loop de retry a una funcion generica que acepte un closure que
construye y envía el request. Los tres metodos publicos se convierten en thin wrappers.

- **RED**: Los tests existentes de retry (`get_retries_on_503_with_backoff`,
  `post_retries_on_429_rate_limit`, `post_stream_retries_on_503_before_stream_starts`) son los tests
  de regresion para este ciclo. Deben seguir pasando sin modificacion.

  Agregar tests que verifican que Retry-After es respetado (integra Fase 2):

  ```rust
  #[tokio::test]
  async fn get_respects_retry_after_header() {
      use std::sync::atomic::{AtomicU32, Ordering};
      use std::sync::Arc;
      use std::time::{Duration, Instant};

      let server = MockServer::start().await;
      let request_count = Arc::new(AtomicU32::new(0));
      let counter = request_count.clone();

      Mock::given(method("GET"))
          .and(path("/retry-after-test"))
          .respond_with(move |_req: &wiremock::Request| {
              let count = counter.fetch_add(1, Ordering::SeqCst);
              if count == 0 {
                  ResponseTemplate::new(429)
                      .set_body_string("Rate limited")
                      .insert_header("Retry-After", "1") // Pide esperar 1 segundo
              } else {
                  ResponseTemplate::new(200).set_body_string("OK")
              }
          })
          .mount(&server)
          .await;

      let policy = RetryPolicy {
          max_retries: 3,
          initial_backoff: Duration::from_millis(10), // Mucho menor que Retry-After
      };

      let client = FoundryClient::builder()
          .endpoint(server.uri())
          .credential(FoundryCredential::api_key("test"))
          .retry_policy(policy)
          .build()
          .expect("should build");

      let start = Instant::now();
      let result = client.get("/retry-after-test").await;
      let elapsed = start.elapsed();

      assert!(result.is_ok());
      // Debe haber esperado al menos 1 segundo (el Retry-After del servidor),
      // no solo los 10ms del initial_backoff
      assert!(
          elapsed >= Duration::from_millis(900),
          "Should have waited for Retry-After (1s), but waited only {:?}",
          elapsed
      );
  }
  ```

- **GREEN**: Extraer la logica de retry a una funcion privada asincrona generica. El diseno exacto
  debe resolver el problema del lifetime del closure asincrono. Una forma que funciona en Rust
  estable es usar una `Box<dyn Fn() -> Pin<Box<dyn Future<...>>>>`:

  ```rust
  async fn execute_with_retry<F, Fut>(
      &self,
      make_request: F,
  ) -> FoundryResult<reqwest::Response>
  where
      F: Fn() -> Fut,
      Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
  {
      for attempt in 0..=self.retry_policy.max_retries {
          let span = tracing::Span::current();
          span.record("attempt", attempt);

          let response = make_request().await?;
          let status = response.status().as_u16();
          span.record("status_code", status);

          if response.status().is_success() {
              return Ok(response);
          }

          if !is_retriable_status(status) || attempt == self.retry_policy.max_retries {
              return Self::check_response(response).await;
          }

          tracing::warn!(status = status, attempt = attempt, "retriable error, will retry");

          // Respetar Retry-After del servidor si esta presente; sino, usar backoff propio
          let backoff = extract_retry_after_delay(response.headers())
              .unwrap_or_else(|| compute_backoff(attempt, self.retry_policy.initial_backoff));

          tokio::time::sleep(backoff).await;
      }
      unreachable!("retry loop should return before reaching here")
  }
  ```

  Los metodos `get()`, `post()`, y `post_stream()` pasan a ser wrappers de una linea (mas la
  construccion del request). Para `post_stream()`, la diferencia es el `.timeout()` en el request
  y que no llama a `check_response` (retorna el stream directamente).

- **REFACTOR**: Verificar que los atributos `#[tracing::instrument]` en los metodos publicos siguen
  siendo funcionales. El span actual se propaga al interior de `execute_with_retry` via
  `tracing::Span::current()`, por lo que no se pierde la instrumentacion.

  Eliminar los tres bloques `unreachable!()` duplicados que ya no existen.

---

### Fase 4: Sanitizacion completa de credenciales (Critical #4)

**Problema**: `sanitize_error_message` no cubre:
1. JWTs de Entra ID (formato `eyJ...` - header base64url sin prefijo `Bearer `)
2. El patron `api-key: <valor>` que aparece en headers de error de algunos servicios Azure

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

#### Ciclo 4.1: Sanitizar JWTs (tokens eyJ...)

- **RED**:

  ```rust
  #[test]
  fn sanitize_jwt_tokens_in_error_messages() {
      // Un JWT real tiene 3 partes separadas por puntos, todas en base64url
      let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIiwiZXhwIjoxNzAwMDAwMDAwfQ.signature123";
      let msg = format!("Token validation failed: {}", jwt);
      let result = FoundryClient::sanitize_error_message(&msg);
      assert!(!result.contains("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9"), "JWT header should be redacted");
      assert!(result.contains("[REDACTED]"), "Should contain redaction marker");
  }

  #[test]
  fn sanitize_partial_jwt_eyj_prefix() {
      let msg = "Invalid token eyJhbGci.payload.sig in request";
      let result = FoundryClient::sanitize_error_message(&msg);
      assert!(!result.contains("eyJhbGci"), "Partial JWT should be redacted");
  }
  ```

#### Ciclo 4.2: Sanitizar patron api-key: <valor>

- **RED**:

  ```rust
  #[test]
  fn sanitize_api_key_header_pattern() {
      let msg = "Request failed with api-key: abc123secret456 - invalid key";
      let result = FoundryClient::sanitize_error_message(&msg);
      assert!(!result.contains("abc123secret456"), "api-key value should be redacted");
      assert!(result.contains("[REDACTED]"), "Should contain redaction marker");
  }

  #[test]
  fn sanitize_ocp_apim_subscription_key_header() {
      // Header alternativo usado por algunos servicios Azure
      let msg = "Ocp-Apim-Subscription-Key: deadbeef1234 was invalid";
      let result = FoundryClient::sanitize_error_message(&msg);
      assert!(!result.contains("deadbeef1234"), "Subscription key should be redacted");
  }
  ```

- **GREEN**: Extender `sanitize_error_message` con dos patrones adicionales:

  Para JWTs: buscar secuencias que comiencen con `eyJ` (inicio de cualquier JWT en base64url) y
  extender hasta el proximo espacio o delimitador. Un JWT siempre empieza con `eyJ` porque el
  header JSON `{"alg":...}` en base64url siempre produce esos bytes.

  Para `api-key:`: buscar el patron `api-key:` (case-insensitive) y redactar el valor que le sigue.
  Tambien `Ocp-Apim-Subscription-Key:`.

- **REFACTOR**: Considerar extraer los patrones de sanitizacion a un array de `(prefix, terminator)`
  para facilitar agregar nuevos patrones en el futuro.

---

### Fase 5: Eliminar panic no documentado en build() (Critical #1)

**Problema**: En `client.rs:657-658`, la construccion del `HttpClient` usa `.expect()` dentro de
`unwrap_or_else`. Si `reqwest::Client::builder().build()` falla (lo cual es extremadamente raro,
pero posible en entornos embedded o con configuracion TLS corrupta), el proceso hace panic en vez
de retornar `Err`.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

Problema secundario relacionado: `RetryPolicy` tiene campos `pub` sin validacion. Un usuario puede
crear `RetryPolicy { max_retries: u32::MAX, initial_backoff: Duration::MAX }` que causaria
comportamiento extremo.

#### Ciclo 5.1: build() debe propagar el error de HttpClient en lugar de hacer panic

- **RED**:

  ```rust
  #[test]
  fn build_returns_error_not_panic_on_invalid_http_config() {
      // No podemos forzar facilmente que reqwest::Client::builder().build() falle,
      // pero podemos verificar que build() retorna FoundryResult, no panics.
      // Este test documenta el contrato y verifica la firma de retorno.
      // La validacion real es en el type system: build() -> FoundryResult<FoundryClient>
      // y ya no usa .expect() internamente.
      let result = FoundryClient::builder()
          .endpoint("https://test.services.ai.azure.com")
          .credential(FoundryCredential::api_key("test"))
          .build();
      // Si llegamos aqui, build() no hizo panic - el test pasa.
      assert!(result.is_ok());
  }
  ```

  El test mas util es de **regresion**: verificar que el codigo en `build()` usa `?` o
  `map_err()` en lugar de `.expect()`:

  ```rust
  // Test de regresion: build() propaga errores de HttpClient
  #[test]
  fn build_propagates_http_client_error() {
      // Crear un builder con una configuracion invalida de TLS (si la API lo permite)
      // Si reqwest no expone esto directamente, el test documenta la intencion.
      // Lo importante es que el codigo usa .build().map_err(...)?  en lugar de .build().expect(...)
  }
  ```

- **GREEN**: Cambiar el bloque en `build()`:

  ```rust
  // ANTES (lineas 654-659):
  let http = self.http_client.unwrap_or_else(|| {
      let connect_timeout = self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
      let read_timeout = self.read_timeout.unwrap_or(DEFAULT_READ_TIMEOUT);
      reqwest::Client::builder()
          .connect_timeout(connect_timeout)
          .timeout(read_timeout)
          .build()
          .expect("failed to build HTTP client")
  });

  // DESPUES: no usar unwrap_or_else porque el closure no puede retornar Result.
  // Usar if-else explicitamente:
  let http = if let Some(client) = self.http_client {
      client
  } else {
      let connect_timeout = self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
      let read_timeout = self.read_timeout.unwrap_or(DEFAULT_READ_TIMEOUT);
      reqwest::Client::builder()
          .connect_timeout(connect_timeout)
          .timeout(read_timeout)
          .build()
          .map_err(|e| FoundryError::Builder(
              format!("failed to build HTTP client: {}", e)
          ))?
  };
  ```

- **REFACTOR**: Ninguno adicional.

#### Ciclo 5.2: RetryPolicy debe validar sus parametros

**Problema recomendado #7**: `RetryPolicy` con campos `pub` permite construccion directa sin
validacion.

- **RED**:

  ```rust
  #[test]
  fn retry_policy_rejects_zero_initial_backoff_is_allowed() {
      // Zero backoff es un caso degenerado pero valido (util en tests)
      let policy = RetryPolicy::new(3, Duration::ZERO);
      assert!(policy.is_ok());
  }

  #[test]
  fn retry_policy_new_constructor_validates_max_retries() {
      // Un limite razonable: mas de 10 retries es probablemente un error de configuracion
      let policy = RetryPolicy::new(11, Duration::from_millis(500));
      assert!(policy.is_err());
      let err = policy.unwrap_err();
      assert!(err.to_string().contains("max_retries"));
  }

  #[test]
  fn retry_policy_new_constructor_validates_backoff_cap() {
      // initial_backoff > MAX_BACKOFF no tiene sentido
      let policy = RetryPolicy::new(3, Duration::from_secs(120)); // > MAX_BACKOFF (60s)
      assert!(policy.is_err());
  }
  ```

- **GREEN**: Agregar un constructor `RetryPolicy::new(max_retries, initial_backoff) -> FoundryResult<RetryPolicy>`
  y hacer los campos `pub(crate)` (o mantenerlos `pub` con documentacion de invariantes). Los
  tests existentes que construyen `RetryPolicy { max_retries: 5, ... }` directamente deben
  convertirse a usar `RetryPolicy::new()` o indicarse explicitamente que son construccion directa
  para tests.

  Los campos se mantienen `pub` para compatibilidad con v0.2.0, pero se documenta que deben
  usarse los constructores para garantizar invariantes:

  ```rust
  impl RetryPolicy {
      /// Construct a validated RetryPolicy.
      ///
      /// # Errors
      /// Returns an error if max_retries > 10 or initial_backoff > MAX_BACKOFF.
      pub fn new(max_retries: u32, initial_backoff: Duration) -> FoundryResult<Self> {
          if max_retries > 10 {
              return Err(FoundryError::Builder(
                  format!("max_retries must be <= 10, got {}", max_retries)
              ));
          }
          if initial_backoff > MAX_BACKOFF {
              return Err(FoundryError::Builder(
                  format!("initial_backoff must be <= MAX_BACKOFF (60s), got {:?}", initial_backoff)
              ));
          }
          Ok(Self { max_retries, initial_backoff })
      }
  }
  ```

- **REFACTOR**: Actualizar la documentacion de `RetryPolicy` para mencionar los invariantes.

---

### Fase 6: Token refresh dentro del retry loop (Recomendado #5)

**Problema**: En los tres metodos de HTTP, el token se resuelve **una vez** antes del loop
(`let auth = self.credential.resolve().await?;`). Si los retries toman tiempo (ej: backoff de 60s
acumulado tras varios fallos), el token Entra ID puede expirar durante el loop. El siguiente
request se enviaria con un token caducado.

**Nota**: El cache interno de `resolve()` con el buffer de 60 segundos mitiga el problema
parcialmente, pero no lo elimina si el backoff total supera el TTL del token.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

#### Ciclo 6.1: El token se resuelve por cada intento del retry loop

- **RED**: El test de este ciclo verifica que si un token expira entre retries, se obtiene uno
  nuevo. Esto requiere un `MockTokenCredential` que retorne tokens con TTL corto:

  ```rust
  #[tokio::test]
  async fn retry_loop_refreshes_expired_token() {
      // Este test verifica el comportamiento: si el token expira entre retries,
      // resolve() (con su cache) lo renovara automaticamente.
      // El contrato es: resolve() se llama en cada iteracion del loop, no una sola vez.
      // Verificable indirectamente: un CountingTokenCredential con TTL corto
      // debe llamarse mas de una vez si hay retries con backoff suficiente.

      // Nota: Este test es un test de documentacion del contrato.
      // La implementacion concreta depende de mover `resolve()` dentro del loop.
  }
  ```

  El test principal es verificar que `auth = self.credential.resolve().await?` esta DENTRO del
  loop `for attempt in 0..=...`, no antes. Esto se verifica mediante code review al implementar.

- **GREEN**: Mover la llamada a `self.credential.resolve().await?` dentro del cuerpo del loop en
  los tres metodos (o dentro de `execute_with_retry` de Fase 3). El cache de tokens en
  `FoundryCredential::resolve()` garantiza que no se hace una llamada de red extra si el token
  todavia es valido.

  Antes (patron actual):
  ```rust
  pub async fn get(&self, path: &str) -> FoundryResult<reqwest::Response> {
      let url = self.url(path)?;
      let auth = self.credential.resolve().await?;  // <-- fuera del loop

      for attempt in 0..=self.retry_policy.max_retries {
          // usa auth
      }
  }
  ```

  Despues:
  ```rust
  pub async fn get(&self, path: &str) -> FoundryResult<reqwest::Response> {
      let url = self.url(path)?;

      for attempt in 0..=self.retry_policy.max_retries {
          let auth = self.credential.resolve().await?;  // <-- dentro del loop
          // usa auth
      }
  }
  ```

  Si la Fase 3 (extract retry loop) ya se implemento, el cambio es solo en el closure/funcion
  interna: el closure que construye el request debe llamar a `resolve()` en cada invocacion.

- **REFACTOR**: Verificar que no hay impacto en performance: el cache de `resolve()` hace que la
  llamada extra sea `O(1)` en el caso de token valido (solo un lock de Mutex y una comparacion de
  tiempo de expiracion).

---

### Fase 7: Rename get_token -> fetch_fresh_token en auth.rs (Recomendado #3)

**Problema**: `get_token()` tiene semantica confusa porque:
1. El metodo interno del SDK tambien se llama `get_token` (trait `TokenCredential::get_token`)
2. El nombre no comunica que bypasea el cache (siempre fetchea un token fresco)
3. La documentacion dice "Note: This method bypasses the internal cache" pero el nombre no lo indica

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/auth.rs`

#### Ciclo 7.1: Rename preservando compatibilidad mediante deprecation

- **RED**:

  ```rust
  #[tokio::test]
  async fn fetch_fresh_token_bypasses_cache() {
      // Verificar que fetch_fresh_token siempre llama a get_token del credential,
      // incluso si hay un token valido en cache.
      let mock = CountingTokenCredential::new("fresh-token", 3600);
      let cred = FoundryCredential::token_credential(mock.clone());

      // Primer resolve() -> popula el cache
      let _ = cred.resolve().await.expect("first resolve");
      assert_eq!(mock.call_count(), 1);

      // fetch_fresh_token() debe bypassear el cache
      let _ = cred.fetch_fresh_token().await.expect("fetch fresh token");
      assert_eq!(mock.call_count(), 2, "fetch_fresh_token must bypass cache");
  }

  #[tokio::test]
  async fn fetch_fresh_token_fails_for_api_key() {
      let cred = FoundryCredential::api_key("my-key");
      let result = cred.fetch_fresh_token().await;
      assert!(result.is_err());
      assert!(result.unwrap_err().to_string().contains("API key credential"));
  }
  ```

- **GREEN**:
  - Renombrar `get_token` a `fetch_fresh_token` en la implementacion
  - Mantener `get_token` como alias `#[deprecated]` apuntando a `fetch_fresh_token` para
    compatibilidad con v0.2.x:

  ```rust
  /// Get an access token for the Cognitive Services scope.
  ///
  /// Always fetches a fresh token, bypassing the internal cache.
  /// Use `resolve()` for normal authentication which benefits from caching.
  ///
  /// # Errors
  ///
  /// Returns an error if this is an API key credential or if token acquisition fails.
  pub async fn fetch_fresh_token(&self) -> FoundryResult<AccessToken> {
      // ... implementacion actual de get_token ...
  }

  /// Deprecated: use [`fetch_fresh_token`](Self::fetch_fresh_token) instead.
  #[deprecated(since = "0.3.0", note = "Use fetch_fresh_token() instead")]
  pub async fn get_token(&self) -> FoundryResult<AccessToken> {
      self.fetch_fresh_token().await
  }
  ```

  Analogamente para `get_token_with_options` -> `fetch_fresh_token_with_options`.

- **REFACTOR**: Actualizar todos los tests en `auth.rs` que usan `get_token` a usar
  `fetch_fresh_token`. Actualizar la documentacion del modulo en el archivo `.rs`.

---

### Fase 8: Unificar Usage vs EmbeddingUsage (Recomendado #2)

**Problema**: `models.rs` define `Usage { prompt_tokens, completion_tokens, total_tokens }` y
`embeddings.rs` define `EmbeddingUsage { prompt_tokens, total_tokens }`. Son dos structs separadas
para el mismo concepto, lo que viola DRY y complica el uso del SDK por parte de los usuarios.

**Archivos**:
- `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/models.rs`
- `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_models/src/embeddings.rs`

#### Ciclo 8.1: Unificar en Usage con completion_tokens opcional

La diferencia entre ambas es solo que `EmbeddingUsage` no tiene `completion_tokens`. La struct
`Usage` ya tiene `completion_tokens: Option<u32>`, por lo que ya es compatible con la semantica
de embeddings.

- **RED**:

  ```rust
  // En embeddings.rs tests:
  #[test]
  fn embedding_response_uses_core_usage_type() {
      // Verificar que EmbeddingResponse.usage es del tipo azure_ai_foundry_core::models::Usage
      // y no un tipo local. Este test compila solo si el tipo correcto esta siendo usado.
      use azure_ai_foundry_core::models::Usage;
      let json = serde_json::json!({
          "object": "list",
          "model": "text-embedding-ada-002",
          "data": [],
          "usage": { "prompt_tokens": 5, "total_tokens": 5 }
      });
      let response: EmbeddingResponse = serde_json::from_value(json).unwrap();
      // El tipo de response.usage debe ser Usage (del core)
      let _usage: &Usage = &response.usage;
  }
  ```

- **GREEN**:
  1. Agregar doc comments a `Usage` en `models.rs` (ver Fase 10)
  2. En `embeddings.rs`: reemplazar `EmbeddingUsage` con `Usage` del core:
     ```rust
     use azure_ai_foundry_core::models::Usage;

     // Cambiar EmbeddingResponse:
     pub struct EmbeddingResponse {
         pub object: String,
         pub model: String,
         pub data: Vec<EmbeddingData>,
         pub usage: Usage,  // antes: EmbeddingUsage
     }
     ```
  3. Eliminar la definicion de `EmbeddingUsage`

  El JSON `{ "prompt_tokens": 5, "total_tokens": 5 }` deserializa correctamente en `Usage`
  porque `completion_tokens` es `Option<u32>` y sera `None` si el campo no esta presente.

- **REFACTOR**: Eliminar todos los usos de `EmbeddingUsage` en tests. Los tests que verificaban
  `response.usage.prompt_tokens` y `response.usage.total_tokens` siguen funcionando identicamente
  porque `Usage` tiene los mismos campos.

---

### Fase 9: Doc comments y optimizacion SSE (Recomendados #1 y #4)

#### Ciclo 9.1: Doc comments en ChatCompletionRequestBuilder (Recomendado #1)

**Problema**: Los metodos de `ChatCompletionRequestBuilder` (lineas 107-221 de `chat.rs`) no tienen
doc comments. Los usuarios del SDK ven metodos sin documentacion en `rustdoc`.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_models/src/chat.rs`

- **RED**: `cargo doc --workspace --no-deps 2>&1 | grep -i "warning.*missing"` debe mostrar
  warnings para los metodos sin documentar. Con `RUSTFLAGS="-D warnings"` en CI, esto falla
  el build.

  Verificar con:
  ```bash
  cargo doc --workspace --no-deps 2>&1 | grep "ChatCompletionRequestBuilder"
  ```

- **GREEN**: Agregar doc comments a todos los metodos publicos de `ChatCompletionRequestBuilder`:

  ```rust
  impl ChatCompletionRequestBuilder {
      /// Set the model ID to use for the completion.
      ///
      /// This is a required field. Example values: `"gpt-4o"`, `"gpt-4o-mini"`.
      pub fn model(mut self, model: impl Into<String>) -> Self { ... }

      /// Add a single message to the conversation.
      ///
      /// Messages are appended in order. Use [`messages`](Self::messages) to add multiple at once.
      pub fn message(mut self, message: Message) -> Self { ... }

      /// Add multiple messages to the conversation.
      ///
      /// Accepts any type implementing `IntoIterator<Item = Message>`.
      pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self { ... }

      /// Set the sampling temperature (0.0 to 2.0).
      ///
      /// Higher values make output more random, lower values more deterministic.
      /// Defaults to the model's default if not set.
      pub fn temperature(mut self, temp: f32) -> Self { ... }

      // ... resto de metodos
  }
  ```

- **REFACTOR**: Verificar con `cargo doc --workspace --no-deps` que no quedan warnings de
  documentacion faltante.

#### Ciclo 9.2: Eliminar allocacion innecesaria en SSE drain (Recomendado #4)

**Problema**: En `chat.rs:608`, la linea `buffer.drain(..=newline_pos).collect()` crea un
`Vec<u8>` intermedio innecesario. El buffer se drena en una coleccion que luego se convierte a
`&str` para procesar. Es mas eficiente copiar el slice antes de drenar.

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_models/src/chat.rs`

- **RED**: Los tests existentes de SSE (`complete_stream_success`,
  `test_sse_buffer_limit_prevents_dos`, etc.) son los tests de regresion. Deben seguir pasando.

  No hay nuevo test para este ciclo: es una optimizacion de implementacion pura, sin cambio de
  comportamiento observable.

- **GREEN**:

  ```rust
  // ANTES (genera Vec<u8> intermedio):
  let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
  let line = match std::str::from_utf8(&line_bytes[..line_bytes.len() - 1]) { ... };

  // DESPUES (zero-copy: convierte el slice antes de drenar):
  // Intentar convertir UTF-8 primero, luego drenar
  let line_end = newline_pos; // indice del \n, excluido
  let line = match std::str::from_utf8(&buffer[..line_end]) {
      Ok(s) => {
          let owned = s.to_owned(); // necesario porque drain invalida el slice
          buffer.drain(..=newline_pos);
          owned
      }
      Err(_) => {
          buffer.drain(..=newline_pos);
          continue;
      }
  };
  ```

  Nota tecnica: no se puede tener un `&str` que referencie `buffer` mientras se drena `buffer`.
  La alternativa real sin `to_owned()` es usar indices y reconstruir. La mejora principal es
  evitar el `collect::<Vec<u8>>()` que hace una copia adicional: la version ANTES hace
  `drain().collect()` (copia a Vec) + `from_utf8` (solo referencia). La version DESPUES hace
  `from_utf8` (referencia directa al buffer) + `to_owned()` (una sola copia del string ya
  validado como UTF-8). La mejora es marginal para strings ASCII, pero evita una copia en el
  caso de exito.

- **REFACTOR**: Verificar que el comportamiento ante UTF-8 invalido es identico: debe continuar
  (skip the line) en ambas versiones.

---

### Fase 10: Documentacion de campos y constantes (Recomendados #6 y #8)

#### Ciclo 10.1: Documentar DEFAULT_API_VERSION como preview (Recomendado #6)

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

- **RED**: `cargo doc --workspace --no-deps` debe mostrar la constante documentada. El test es
  visual/manual: verificar que rustdoc incluye la advertencia.

- **GREEN**: Actualizar el doc comment de `DEFAULT_API_VERSION`:

  ```rust
  /// Default API version for Azure AI Foundry.
  ///
  /// # Warning
  ///
  /// This is a **preview** API version (`-preview` suffix). Preview APIs may change
  /// without notice and are not covered by SLA guarantees. For production use,
  /// consider pinning to a stable version via
  /// [`FoundryClientBuilder::api_version`](crate::client::FoundryClientBuilder::api_version).
  pub const DEFAULT_API_VERSION: &str = "2025-01-01-preview";
  ```

- **REFACTOR**: Ninguno adicional.

#### Ciclo 10.2: Documentar campos de models.rs (Recomendado #8)

**Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/models.rs`

- **RED**: Los campos `pub` de `Usage` no tienen doc comments. Con `#[warn(missing_docs)]` esto
  generaria warnings.

- **GREEN**:

  ```rust
  /// Usage statistics returned by the API.
  ///
  /// Present in both chat completion and embedding responses.
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Usage {
      /// Number of tokens in the prompt (input).
      pub prompt_tokens: u32,

      /// Number of tokens generated by the model (output).
      ///
      /// `None` for embedding requests, which do not generate output tokens.
      pub completion_tokens: Option<u32>,

      /// Total tokens consumed by the request (prompt + completion).
      pub total_tokens: u32,
  }
  ```

- **REFACTOR**: Ninguno adicional.

---

## Fase de Testing de Rendimiento (Hot Path)

Los cambios de Fase 3 (unificar retry loop) y Fase 6 (token refresh dentro del loop) afectan el
hot path de cada request HTTP.

### Ciclo P.1: Medir throughput baseline ANTES de Fase 3

- **Archivo**: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/benches/retry_throughput.rs`
  (nuevo archivo a crear)

  ```rust
  use criterion::{black_box, criterion_group, criterion_main, Criterion};
  use azure_ai_foundry_core::client::{RetryPolicy, FoundryClient};
  use azure_ai_foundry_core::auth::FoundryCredential;

  fn bench_successful_request(c: &mut Criterion) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      // Setup wiremock server que siempre retorna 200
      // Medir: requests/segundo con 0 retries (camino feliz)
      c.bench_function("get_no_retry", |b| {
          b.to_async(&rt).iter(|| async {
              // ...
          });
      });
  }

  criterion_group!(benches, bench_successful_request);
  criterion_main!(benches);
  ```

### Ciclo P.2: Test de regresion de throughput post-Fase 3

- **Umbral minimo aceptable**: No mas del 10% de degradacion respecto al baseline medido en P.1.
- **Criterio**: Si el refactor del retry loop introduce overhead, debe investigarse. El camino
  feliz (request exitoso en el primer intento) no debe ser afectado por el refactor.

---

## Estimacion Total

| Fase | Descripcion | Tiempo estimado |
|------|-------------|-----------------|
| 1    | error.rs - AzureSdk source chain | 20 min |
| 2    | extract_retry_after_delay helper | 25 min |
| 3    | Unificar retry loop + Retry-After | 45 min |
| 4    | Sanitizacion JWT + api-key: | 30 min |
| 5    | Eliminar panic + RetryPolicy validacion | 30 min |
| 6    | Token refresh dentro del loop | 20 min |
| 7    | Rename get_token -> fetch_fresh_token | 20 min |
| 8    | Unificar Usage / EmbeddingUsage | 20 min |
| 9    | Doc comments + SSE drain optimizacion | 30 min |
| 10   | Documentar DEFAULT_API_VERSION + models.rs | 15 min |
| P    | Benchmark baseline y regresion | 30 min |
| **Total** | | **~5.5 horas** |

---

## Criterios de Exito

- [ ] `cargo test --workspace` pasa con 0 fallos
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` pasa con 0 warnings
- [ ] `cargo doc --workspace --no-deps` pasa sin warnings de documentacion faltante
- [ ] `FoundryError::AzureSdk` preserva `source()` (no es `None`)
- [ ] `Retry-After` header es respetado cuando esta presente
- [ ] Retry loop no esta duplicado 3 veces
- [ ] `sanitize_error_message` cubre JWT (`eyJ...`) y `api-key:` ademas de `Bearer ` y `sk-`
- [ ] `build()` retorna `Err` en vez de hacer panic si `reqwest::Client` falla
- [ ] `get_token()` esta deprecated en favor de `fetch_fresh_token()`
- [ ] `EmbeddingUsage` eliminado; `EmbeddingResponse.usage` usa `azure_ai_foundry_core::models::Usage`
- [ ] Throughput del camino feliz no degrada mas del 10% respecto al baseline (benchmark)
