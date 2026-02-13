# Instrucciones: Crear el repo azure-ai-foundry en GitHub

## Paso 1: Crear el repo en GitHub

```bash
# Crear el repo público con descripción y licencia
gh repo create azure-ai-foundry \
  --public \
  --description "🦀 Unofficial Rust SDK for Microsoft Foundry (Azure AI Foundry) — Chat completions, embeddings, agents, and tools" \
  --license MIT \
  --clone
```

## Paso 2: Copiar el scaffold del proyecto

Copia todo el contenido del scaffold que te he generado dentro del directorio `azure-ai-foundry/` que se acaba de clonar. La estructura debería quedar así:

```
azure-ai-foundry/
├── .github/
│   ├── workflows/
│   │   └── ci.yml
│   └── ISSUE_TEMPLATE/
│       ├── bug_report.md
│       └── feature_request.md
├── sdk/
│   ├── azure_ai_foundry_core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth.rs
│   │       ├── client.rs
│   │       ├── error.rs
│   │       └── models.rs
│   └── azure_ai_foundry_models/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── chat.rs
│           └── embeddings.rs
├── examples/
├── .gitignore
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

## Paso 3: Personalizar

Reemplaza `bzsanti` y `YOUR_NAME` en estos archivos:

```bash
# Reemplazar en todos los archivos de una vez
# Sustituye TU_USUARIO por tu username real de GitHub
# Sustituye TU_NOMBRE por tu nombre real

find . -type f \( -name "*.toml" -o -name "*.md" -o -name "*.yml" \) \
  -exec sed -i 's/bzsanti/TU_USUARIO/g' {} +

find . -type f -name "LICENSE" \
  -exec sed -i 's/YOUR_NAME/TU_NOMBRE/g' {} +
```

## Paso 4: Verificar que compila

```bash
cd azure-ai-foundry
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
cargo test --workspace
```

## Paso 5: Primer commit y push

```bash
git add -A
git commit -m "feat: initial project scaffold

- Workspace structure with core and models crates
- FoundryClient with builder pattern and auth support
- Chat completion types and API function
- CI pipeline with check, test, fmt, clippy, docs, and MSRV
- MIT license, README, CONTRIBUTING guide
- GitHub issue templates"

git push origin main
```

## Paso 6: Configurar el repo en GitHub

```bash
# Añadir topics para visibilidad
gh repo edit --add-topic rust,azure,ai,foundry,microsoft,sdk,openai,machine-learning

# Activar GitHub Discussions (opcional, bueno para comunidad)
gh repo edit --enable-discussions

# Proteger la rama main (opcional pero recomendable)
# Esto se hace mejor desde la web: Settings > Branches > Add rule
```

## Paso 7: Verificar que CI pasa

Después del push, ve a:
`https://github.com/TU_USUARIO/azure-ai-foundry/actions`

El workflow de CI debería ejecutar 6 jobs: Check, Test, Format, Clippy, Docs, y MSRV. Todos deberían pasar en verde.

## Siguiente paso

Con el repo creado y CI en verde, el siguiente paso es completar la implementación de `azure_ai_foundry_core` y `azure_ai_foundry_models` para el release v0.1.0 en crates.io. Las prioridades son:

1. Implementar la autenticación real con `azure_identity` (Entra ID)
2. Completar chat completions con streaming (SSE)
3. Implementar embeddings
4. Añadir tests unitarios con `wiremock`
5. Publicar v0.1.0 en crates.io
