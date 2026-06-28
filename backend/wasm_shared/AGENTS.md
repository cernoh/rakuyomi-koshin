# WASM Shared — Shared WASM Interop Types

## Purpose

Shared types for WASM interop across the Rust workspace. Provides serialization contracts between the WASM runtime (wasmi) and the host.

## Ownership

Owns: data types used for communication between Rust host code and WASM guest modules (Aidoku sources).

## Local Contracts

- Thin crate with minimal dependencies
- Used by both `shared` (wasm store) and `wasm_macros` (generated bindings)
- Types serialized via postcard for FFI

## Verification

- Compile-time verification via `cargo check -p wasm_shared`
