# Shared — Core Domain Library

## Purpose

Core domain library for RakuYomi — manga models, database layer (sqlx/SQLite), WASM source manager (wasmi), chapter downloader, settings, image processing, and all use cases.

## Ownership

Owns: domain models (`model.rs`), database schema + migrations (`database.rs`, `migrations/`), source manager (`source/`), chapter downloader/storage, settings schema, image DRM unscrambling, CBZ metadata, use case implementations, and benchmarks.

## Local Contracts

- Optional `sqlx` feature for SQLite support (enabled by default via `all` feature)
- `ffi` feature for JNI-compatible serialization (postcard)
- Build script (`build.rs`) generates settings schema JSON at compile time
- Uses `schemars` for JSON schema generation
- WASM runtime: `wasmi` for source execution
- JS runtime: `boa_engine` for JavaScript-based sources
- Font rendering: `raqote` + `font-kit` + `ab_glyph` for image rendering
- Image decoding: `zune-png`, `zune-jpeg`, `image`
- Image processing: `imageproc`, `mozjpeg`
- EPUB generation: `epub-builder` (forked)

## Work Guidance

### Source system

- `source/mod.rs` (48KB) — main source abstraction, source resolution, scraping
- `source/wasm_store.rs` — wasmi WASM runtime for Aidoku sources
- `source/wasm_imports/` — WASM import bindings
- `source/html_element.rs` — HTML DOM element abstraction
- `source/model.rs` — source-specific data models
- `source/next_reader.rs` — "next" SDK reader support
- `source/source_settings.rs` — per-source settings
- `source/decode_image.rs` — image decoding utilities

### Database

- `database.rs` (127KB) — all SQLite queries via sqlx
- `migrations/` — SQLite migrations
- `build.rs` — compile-time SQLx query verification (`cargo sqlx prepare`)
- dev.db — development database snapshot

### Key modules

- `chapter_downloader.rs` — concurrent chapter downloading with progress
- `chapter_storage.rs` — file storage for downloaded chapters (CBZ/ZIP)
- `settings/` — settings schema + implementation
- `cbz_metadata/` — CBZ metadata extraction/parsing
- `arima_light.rs` — "ARIMA" light novel rendering engine
- `unscrable_image.rs` — image DRM unscrambling
- `usecases/` — all business logic use cases (40+ files)

## Verification

- `cargo test -p shared` — unit tests
- `cargo bench -p shared` — criterion benchmarks (`chapter_downloader_benchmark`, `search_mangas_benchmark`)
- Build script verifies SQLx queries at compile time
- Benchmarks in `benches/` with `pprof` flamegraph support
