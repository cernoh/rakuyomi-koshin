# WASM Macros — Proc-Macro Crate

## Purpose

Procedural macros for generating WASM binding code between Rust host and Aidoku source WASM modules.

## Ownership

Owns: proc-macro attributes and derive macros used by the shared library's WASM runtime to generate host↔guest bindings.

## Local Contracts

- Proc-macro crate (compiled before dependent crates)
- Generates wasmi import/export glue code
- Used by `shared/src/source/wasm_store.rs` and related modules
- Input/output types defined in `wasm_shared`

## Verification

- Compile-time verification via `cargo check -p wasm_macros`
- Tested implicitly by `shared` crate tests that exercise WASM sources
