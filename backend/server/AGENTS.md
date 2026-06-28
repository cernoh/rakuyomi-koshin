# Server — HTTP Server

## Purpose

HTTP server (axum) serving the RakuYomi Lua frontend. Runs as a standalone binary on Unix platforms or as a cdylib loaded via JNI on Android.

## Ownership

Owns: route modules (manga, source, settings, system, playlists, job, update), axum app wiring, JNI bridge, error types, build info, TCP/UDS listener setup.

## Local Contracts

- Crate type: `["lib", "cdylib"]` — lib for binary, cdylib for Android JNI
- Binary entry: `src/main.rs`; Library entry: `src/lib.rs`
- `ffi` feature gates JNI + serde_bytes + reqwest (Android companion communication)
- `api_18` feature gates for older Android API (disables nix mount/fs)
- axum `FromRef` state pattern for dependency injection
- Routes organized by domain module: `manga/routes.rs`, `source/routes.rs`, `job/routes.rs`, etc.

## Work Guidance

### Route modules

- `manga/routes.rs` (22KB) — library CRUD, chapter listings, reading progress, search
- `source/routes.rs` — source management, installation, search dispatch
- `job/routes.rs` — async job management (downloads, refreshes)
- `settings/routes.rs` — global settings CRUD
- `playlists/routes.rs` — playlist CRUD
- `system/routes.rs` — system info, health
- `update/routes.rs` — self-update

### Key modules

- `app.rs` — axum router construction, middleware, state initialization
- `state.rs` — application state (shared via `FromRef`)
- `error.rs` — HTTP error responses
- `listener.rs` — TCP/UDS listener setup
- `jni.rs` — Android JNI bridge (Android-only)
- `model.rs` — API-level request/response types
- `build_info.rs` — compile-time build metadata
- `source_extractor.rs` — source archive extraction

## Verification

- `cargo test -p server` — unit tests for routes and handlers
- HTTP response format tests in route modules
- JNI module behind `#[cfg]` — compile-checked on Linux, tested on Android
