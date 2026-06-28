# Backend — Rust Workspace

## Purpose

Rust workspace containing the HTTP server, shared domain library, and helper binaries for the RakuYomi manga reader backend.

## Ownership

Owns all Rust source: server binary/cdylib, shared library, WASM interop crates, UDS proxy, CBZ metadata reader, and cross-compilation config.

## Local Contracts

- Workspace resolver 2, Cargo.toml at `backend/Cargo.toml`
- Members: `shared`, `server`, `uds_http_request`, `wasm_macros`, `wasm_shared`, `cbz_metadata_reader`
- Release profile: `opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic="abort"`
- Cross-compile via `backend/.cross/` Dockerfiles
- Tiff patch: `tiff` crate from git

## Work Guidance

### Commands

```sh
cargo build                     # debug build (host)
cargo build --release           # release build (host)
cargo test                      # run all tests + benchmarks
```

### Cross-compilation

```sh
scripts/build-all.sh <target>   # cross-compile + package plugin
scripts/build-rust-android.sh   # build libserver.so + APK
```

### Android

- `server` is built as both `bin` (Unix) and `cdylib` (Android JNI)
- JNI code in `server/src/jni.rs` behind `#[cfg(target_os = "android")]`
- Feature `ffi` enables JNI + serde_bytes + reqwest for Android
- Feature `api_18` disables nix (mount/fs) for API 18+ compatibility

## Verification

- `cargo test` — unit + integration + doc tests
- `cargo clippy` — lint
- Benchmarks via criterion in `shared/benches/`

## Child DOX Index

| Path | Scope | Owner |
|---|---|---|
| `shared/` | Core domain: manga models, DB, source manager, downloader, settings | Core shared library |
| `server/` | HTTP server (axum), binary + cdylib, routes | HTTP server |
| `wasm_shared/` | Shared WASM interop types across workspace | WASM types |
| `wasm_macros/` | Proc-macro crate for WASM bindings | WASM macros |
| `uds_http_request/` | Standalone UDS HTTP proxy binary | UDS proxy |
| `cbz_metadata_reader/` | CBZ metadata extraction binary | CBZ reader |
