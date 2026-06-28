# DOX framework

- DOX is highly performant AGENTS.md hierarchy installed here
- Agent must follow DOX instructions across any edits

## Core Contract

- AGENTS.md files are binding work contracts for their subtrees
- Work products, source materials, instructions, records, assets, and durable docs must stay understandable from the nearest applicable AGENTS.md plus every parent AGENTS.md above it

## Purpose

RakuYomi is a manga reader plugin for KOReader. Rust HTTP server backend + Lua plugin frontend. The Rust backend handles sources (WASM/JS), downloads, DB (SQLite); the Lua plugin provides UI within KOReader.

Architecture: `Backend.lua` (Lua) → HTTP/JSON → `server` (axum, Rust) → SQLite + WASM sources.

## Ownership

This root AGENTS.md owns project-wide rules, conventions, and the top-level Child DOX Index. All source code, documentation, tests, CI, and build infrastructure fall under this tree.

## Local Contracts

- No emojis in code or comments
- Keep Rust backend + Lua frontend loosely coupled via JSON API
- Platform architectures:
  - **Unix** (Kindle, Kobo, etc.): fork/exec `server` binary, UDS (`/tmp/rakuyomi.sock`), `uds_http_request` binary bridges HTTP→UDS
  - **Android**: `libserver.so` loaded via JNI in companion app, TCP `127.0.0.1:8787`
  - **Linux (bridge mode)**: systemd user service runs `server` with TCP on `127.0.0.1:8787`, Lua plugin connects via LuaSocket when `RAKUYOMI_USE_BRIDGE=1`
- Data directory: `$KOREARCHIVE_DIR/rakuyomi/` (Unix) or `/storage/emulated/0/koreader/rakuyomi` (Android)
- Versioning: `semantic-release` from commit messages

## Work Guidance

### Rust Conventions

- Edition 2021, toolchain 1.95.0
- snake_case functions/vars, CamelCase types
- `anyhow::Result` in binaries, `thiserror` for library error enums
- axum with `FromRef` state pattern
- tokio multi-threaded async throughout
- JNI code in `server/src/jni.rs` behind `#[cfg(target_os = "android")]`
- Release profile: `opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic="abort"`
- Cross-compile targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `arm-unknown-linux-musleabi[hf]`, `aarch64-linux-android` etc.

### Lua Conventions

- LuaJIT 5.1 compatibility (KOReader uses LuaJIT)
- Require-based modules returning tables
- CamelCase for module names/classes, snake_case for locals/functions
- EmmyLua annotations on all public APIs (`--- @class`, `--- @param`, `--- @return`)
- KOReader widget pattern: `local Foo = InputContainer:extend { ... }`
- UI via `UIManager:show()`, frame containers, etc.
- `.luacheckrc` enforces: max_line_length 300, standard lua51, exclude `platform/_meta.lua`

### Build

```sh
scripts/build-all.sh <target>   # cross-compile + package plugin
scripts/build-android.sh        # build libserver.so + APK
```

CI (root): `.github/workflows/build.yml` — 5 targets via `cross` + Podman.
Builds Rust `.so` via `scripts/build-rust-android.sh`, then runs Gradle
lint/test/assemble for the Android companion app.

### Update translations

```sh
cd frontend/rakuyomi.koplugin/l10n
make update-trans
```

### Lua lint CI

`.github/workflows/luacheck.yml` runs `ci/lua-language-server-check.py` on PRs.

### E2E tests

`e2e-tests/` — Python/Playwright tests run against a real KOReader via `ci/run-e2e-tests.sh`.

### Nix development environment

`flake.nix` (devShells.default) — reproducible dev shell via `nix develop` or `direnv`.

## Verification

- `cargo test` in `backend/` for Rust tests (unit + integration + benches)
- `.github/workflows/test.yml` runs `luacheck` on Lua files
- E2E tests in `e2e-tests/` via Playwright

## Child DOX Index

| Path | Scope | Owner |
|---|---|---|
| `backend/` | Rust workspace: server, shared, wasm_macros, wasm_shared, uds_http_request, cbz_metadata_reader | Rust backend |
| `backend/shared/` | Core domain library: manga models, DB (sqlx/SQLite), source manager (wasmi), downloader, settings | Core shared lib |
| `backend/server/` | HTTP server (axum), binary + cdylib (Android JNI), route modules | HTTP server |
| `backend/wasm_shared/` | Shared WASM interop types across workspace | WASM types |
| `backend/wasm_macros/` | Proc-macro crate for WASM bindings | WASM macros |
| `backend/uds_http_request/` | Standalone UDS HTTP proxy binary | UDS proxy |
| `backend/cbz_metadata_reader/` | CBZ metadata extraction binary | CBZ reader |
| `frontend/` | Lua plugin frontend + configs | Frontend root |
| `frontend/rakuyomi.koplugin/` | KOReader plugin: UI views, platform dispatch, jobs, widgets, l10n | Plugin |
| `docs/` | mdBook documentation site | Docs |
| `scripts/` | Build and dev scripts | Scripts |
| `e2e-tests/` | Python/Playwright end-to-end tests | E2E tests |
| `tools/` | Development helper scripts | Tools |
| `ci/` | CI helper scripts (lua-ls check, e2e runner) | CI scripts |
| `.github/` | GitHub workflows, issue templates, commands | GitHub config |
| `packages/` | Nix packages and patches for KOReader | Nix packages |
