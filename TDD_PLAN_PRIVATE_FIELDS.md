# TDD Plan: Make FoundryClient Fields Private

## Context

`FoundryClient` currently exposes its six internal fields with `pub(crate)` visibility:

```rust
pub struct FoundryClient {
    pub(crate) http: HttpClient,
    pub(crate) endpoint: Url,
    pub(crate) credential: FoundryCredential,
    pub(crate) api_version: String,
    pub(crate) retry_policy: RetryPolicy,
    pub(crate) streaming_timeout: Duration,
}
```

The goal is to make all fields truly private (no visibility modifier) and expose only what is needed through explicit public accessor methods. This enforces encapsulation: callers must use the client's public API instead of reaching into its internals.

**Stack detectado**: Rust workspace (MSRV 1.88), `reqwest`, `tokio`, `serde`, `tracing`
**Convenciones**: TDD obligatorio, builder pattern, `#[cfg(test)] mod tests` inline en cada archivo
**Afecta hot path**: No. Este cambio es puramente estructural (visibilidad de campos). No modifica ninguna ruta de procesamiento de datos ni lógica de red.

---

## Analisis del Estado Actual

Tras leer los tres archivos involucrados, el estado real difiere de lo descrito en el enunciado:

| Campo | Accessors existentes | Usado fuera de FoundryClient |
|---|---|---|
| `endpoint` | `pub fn endpoint(&self) -> &Url` | Solo en tests de `client.rs` via el accessor |
| `api_version` | `pub fn api_version(&self) -> &str` | Solo en tests de `client.rs` via el accessor |
| `retry_policy` | `pub fn retry_policy(&self) -> &RetryPolicy` | Solo en tests de `client.rs` via el accessor |
| `streaming_timeout` | `pub fn streaming_timeout(&self) -> Duration` | Solo en tests de `client.rs` via el accessor |
| `http` | **Ninguno** | Solo usado en `self.http.*` dentro de `FoundryClient` (acceso privado valido) |
| `credential` | **Ninguno** | Solo usado en `self.credential.resolve()` dentro de `FoundryClient` (acceso privado valido) |

**Conclusion critica**: `azure_ai_foundry_models` (chat.rs, embeddings.rs) NO accede directamente a ningun campo. Solo llama a metodos publicos: `client.post()`, `client.post_stream()`. Por tanto, ese crate no requiere ninguna modificacion.

El cambio real consiste unicamente en quitar `pub(crate)` de los seis campos en `client.rs`. Los metodos `get()`, `post()`, `post_stream()` ya son metodos de `FoundryClient` mismo, por lo que tienen acceso a campos privados sin necesidad de visibilidad especial.

---

## Decisiones Previas Necesarias

Ninguna. El cambio es mecanico y no requiere decisiones arquitectonicas.

---

## Plan de Ejecucion

### Fase 1: Verificar la Compilacion Actual (Baseline)

**Objetivo**: Confirmar que los tests pasan antes de cualquier cambio.

1. [ ] Ejecutar suite de tests completa para establecer baseline (15 min)
   - Comando: `cargo test --workspace`
   - Output esperado: 160 tests pasando, 0 warnings

2. [ ] Verificar clippy limpio (5 min)
   - Comando: `cargo clippy --workspace --all-targets -- -D warnings`
   - Output esperado: 0 warnings, 0 errors

---

### Fase 2: Ciclo TDD — Campos `http` y `credential` no necesitan accessor

**Racional**: `http` y `credential` son detalles de implementacion interna. Solo se usan dentro de los propios metodos de `FoundryClient`. No hace falta exponer accessors para ellos.

#### Ciclo 1: Verificar que `pub(crate)` en `http` y `credential` no es usado externamente

- **RED**: Escribir un test que compile sin acceso directo a `.http` ni `.credential`

  Archivo: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

  En el modulo `#[cfg(test)] mod tests`, agregar al final:

  ```rust
  // Ciclo 1: Verificar encapsulacion de http y credential
  // Este test verifica que FoundryClient funciona correctamente
  // sin exponer .http ni .credential directamente.
  // El test ya existia de forma implicita: todos los tests de get/post
  // usan la API publica. Este test lo hace explicito.
  #[test]
  fn client_internals_are_not_accessible_without_accessor() {
      // Verifica que el cliente se puede construir y usar
      // sin necesidad de acceder a http ni credential directamente
      let client = FoundryClient::builder()
          .endpoint("https://test.services.ai.azure.com")
          .credential(FoundryCredential::api_key("test"))
          .build()
          .expect("should build");

      // La API publica del cliente funciona sin acceso a campos internos
      assert!(client.url("/test").is_ok());
      assert_eq!(client.api_version(), DEFAULT_API_VERSION);
  }
  ```

  Ejecutar: `cargo test --workspace client_internals_are_not_accessible_without_accessor`
  Estado esperado: PASA (el test ya es valido con la estructura actual)

- **GREEN**: Quitar `pub(crate)` de `http` y `credential` en `FoundryClient`

  Archivo: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

  Cambio:
  ```rust
  // ANTES
  pub struct FoundryClient {
      pub(crate) http: HttpClient,
      pub(crate) endpoint: Url,
      pub(crate) credential: FoundryCredential,
      pub(crate) api_version: String,
      pub(crate) retry_policy: RetryPolicy,
      pub(crate) streaming_timeout: Duration,
  }

  // DESPUES (solo http y credential en este ciclo)
  pub struct FoundryClient {
      http: HttpClient,
      pub(crate) endpoint: Url,
      credential: FoundryCredential,
      pub(crate) api_version: String,
      pub(crate) retry_policy: RetryPolicy,
      pub(crate) streaming_timeout: Duration,
  }
  ```

  Ejecutar: `cargo build --workspace`
  Estado esperado: Compila sin errores

  Ejecutar: `cargo test --workspace`
  Estado esperado: 160+ tests pasando

- **REFACTOR**: No se necesita refactorizacion. El cambio es minimo y correcto.

---

#### Ciclo 2: Eliminar `pub(crate)` de los campos con accessors existentes

Los campos `endpoint`, `api_version`, `retry_policy`, `streaming_timeout` ya tienen accessors publicos. Quitar `pub(crate)` es seguro porque todo acceso externo ya pasa por esos metodos.

- **RED**: Verificar que los tests de los accessors existentes siguen pasando

  Los tests existentes en `client.rs` ya validan los accessors:
  - `builder_accepts_endpoint` -> usa `client.endpoint()`
  - `builder_uses_default_api_version` -> usa `client.api_version()`
  - `builder_accepts_retry_policy` -> usa `client.retry_policy()`
  - `test_builder_accepts_streaming_timeout` -> usa `client.streaming_timeout()`

  Ejecutar: `cargo test --workspace builder_accepts_endpoint builder_uses_default_api_version builder_accepts_retry_policy test_builder_accepts_streaming_timeout`
  Estado esperado: PASAN (confirmar que los accessors funcionan antes del cambio)

- **GREEN**: Quitar `pub(crate)` de los cuatro campos restantes

  Archivo: `/Volumes/WD_BLACK/repos/MojoBytes/azure-ai-foundry/sdk/azure_ai_foundry_core/src/client.rs`

  Cambio (resultado final del struct):
  ```rust
  pub struct FoundryClient {
      http: HttpClient,
      endpoint: Url,
      credential: FoundryCredential,
      api_version: String,
      retry_policy: RetryPolicy,
      streaming_timeout: Duration,
  }
  ```

  Ejecutar: `cargo build --workspace`
  Estado esperado: Compila sin errores

  Ejecutar: `cargo clippy --workspace --all-targets -- -D warnings`
  Estado esperado: 0 warnings

  Ejecutar: `cargo test --workspace`
  Estado esperado: Mismo numero de tests pasando que el baseline

- **REFACTOR**: Verificar que los doc comments del struct reflejan la encapsulacion

  El doc comment de `FoundryClient` dice "cheaply cloneable". Esto sigue siendo correcto.
  No se necesita modificar la documentacion existente.

---

### Fase 3: Verificacion de Integridad Cross-Crate

**Objetivo**: Confirmar que `azure_ai_foundry_models` no requirio ningun cambio.

1. [ ] Compilar el workspace completo (5 min)
   - Comando: `cargo build --workspace`
   - Output esperado: 0 errores, 0 warnings

2. [ ] Ejecutar tests de `azure_ai_foundry_models` (10 min)
   - Comando: `cargo test -p azure_ai_foundry_models`
   - Output esperado: 65 tests pasando + 16 doc-tests

3. [ ] Ejecutar tests de `azure_ai_foundry_core` (10 min)
   - Comando: `cargo test -p azure_ai_foundry_core`
   - Output esperado: 79 tests pasando

4. [ ] Verificar formato (5 min)
   - Comando: `cargo fmt --all -- --check`
   - Output esperado: Sin diferencias

---

### Fase 4: Verificacion Final

1. [ ] Suite completa (5 min)
   - Comando: `cargo test --workspace`
   - Output esperado: 160 tests pasando

2. [ ] Clippy final (5 min)
   - Comando: `cargo clippy --workspace --all-targets -- -D warnings`
   - Output esperado: 0 warnings, 0 errors

---

## Estimacion Total

- Implementacion: 30 minutos
- Testing funcional: 20 minutos
- Testing de rendimiento: No aplica (no afecta hot path)

**Total**: ~50 minutos

---

## Criterios de Exito

- [ ] Los 6 campos de `FoundryClient` son privados (sin modificador de visibilidad)
- [ ] Los 4 accessors existentes (`endpoint`, `api_version`, `retry_policy`, `streaming_timeout`) permanecen sin cambios
- [ ] No se agregan accessors para `http` ni `credential` (no son necesarios)
- [ ] `azure_ai_foundry_models` no requirio ninguna modificacion
- [ ] Todos los tests del workspace pasan (160 en total)
- [ ] Clippy pasa sin warnings con `-D warnings`
- [ ] `cargo fmt --check` no reporta diferencias

---

## Notas de Implementacion

### Por que NO agregar accessor para `http`

`reqwest::Client` es un detalle de transporte. Exponerlo publicamente crea una dependencia de implementacion: los crates dependientes podrian empezar a construir requests directamente con el `HttpClient`, evitando la logica de autenticacion y reintentos de `FoundryClient`. El campo debe permanecer privado y sin accessor.

### Por que NO agregar accessor para `credential`

`FoundryCredential` contiene informacion sensible (API keys, tokens). Exponerlo publicamente violaria el principio de menor privilegio. Los crates dependientes deben autenticar sus requests unicamente a traves de los metodos `get()`, `post()`, `post_stream()` de `FoundryClient`, que manejan la autenticacion internamente.

### Secuencia de cambios recomendada

Hacer los dos ciclos en un solo commit atomico, ya que son cambios en el mismo struct. El commit debe incluir solo la modificacion del struct en `client.rs` y el nuevo test de verificacion. No hay cambios en `chat.rs` ni en `embeddings.rs`.
