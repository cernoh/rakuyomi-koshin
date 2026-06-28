# Coding Conventions

**Analysis Date:** 2026-06-28

## Rust Conventions

### Toolchain & Build

- **Edition:** 2021
- **Toolchain:** 1.95.0 (defined in `rust-toolchain.toml`)
- **Components:** clippy, rust-analyzer
- **Workspace resolver:** 2 (`backend/Cargo.toml`)
- **Workspace members:** `shared`, `server`, `uds_http_request`, `wasm_macros`, `wasm_shared`, `cbz_metadata_reader`
- **Full release profile** in `backend/Cargo.toml`:
  - `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`
- **Dev profile:** incremental = true
- **Cross-compilation:** via `cross` + Podman using Dockerfiles in `backend/.cross/`

### Naming Patterns

| Element | Convention | Example |
|---------|-----------|---------|
| Types/Enums | `CamelCase` | `SourceId`, `MangaId`, `AppError` |
| Functions/Methods | `snake_case` | `search_mangas`, `build_state` |
| Variables | `snake_case` | `source_path`, `manga_id` |
| Modules | `snake_case` | `chapter_storage`, `source_manager` |
| Error types | `Error` | `pub enum Error` with thiserror |

### Error Handling

- **Binaries** (`server/src/main.rs`): `use anyhow::Result; fn main() -> Result<()>`
- **Libraries** (`backend/shared/src/`): `thiserror` for typed error enums
  - Pattern: `#[derive(thiserror::Error, Debug)] pub enum Error { ... }`
  - Example: `fetch_manga_chapter.rs` — `Error::DownloadError(#[source] anyhow::Error)` and `Error::Other(#[from] anyhow::Error)`
- **Server error layer** (`server/src/error.rs`): `AppError` enum that implements `IntoResponse` for axum, with blanket `From<E: Into<anyhow::Error>>` conversion
- **JSON error responses:** `ErrorResponse { message: String }` serialized struct

### Axum Patterns

- **State pattern:** `FromRef<State>` trait for extracting sub-states into route handlers
  - `impl FromRef<State> for JobState` in `server/src/state.rs`
- **Route modules:** Each domain (`manga/`, `settings/`, `source/`, `system/`, `playlists/`, `job/`, `update/`) has a `routes() -> Router<State>` function
- **Router composition:** `app::build_router()` merges all `.route()` chains from domain modules
- **REST endpoints:** `axum::routing::{get, post, delete}` with path parameters via `Path<(String, String)>` or `Query<T>` extractors

### Async & Concurrency

- **Runtime:** tokio multi-threaded, enabled via `tokio = { features = ["full"] }`
- **Thread naming:** `thread_name("rakuyomi-main")` in `main.rs`
- **Cancellation:** `tokio_util::sync::CancellationToken` passed through use cases
- **Semaphores:** `tokio::sync::Semaphore` for rate-limiting (download concurrency)
- **Concurrent streams:** `futures::stream::StreamExt::buffered()` for parallel HTTP requests
- **Sharing:** `Arc<Mutex<T>>` for mutable shared state (SourceManager, ChapterStorage, Settings)
- **CancellationToken store:** `Arc<Mutex<HashMap<usize, CancellationToken>>>` for cancel-by-ID

### Android / JNI

- JNI module at `server/src/jni.rs` behind `#[cfg(feature = "ffi")]` + `#[cfg(target_os = "android")]`
- Server built as both binary (`bin` for Unix) and `cdylib` (loaded via JNI by companion app)
- Feature `ffi` enables: `serde_bytes`, `reqwest` (rustls-tls), `jni`
- Feature `api_18` disables `nix` (mount/fs) for Android API 18+ compatibility

### Module Structure

- **Use cases** (`backend/shared/src/usecases/`): One file per operation, all re-exported via `pub mod` + `pub use` in `mod.rs`
- **Domain model** (`backend/shared/src/model.rs`): Newtype wrappers with serde derives:
  - `SourceId(String)` — `#[serde(transparent)]`
  - `MangaId { source_id: SourceId, manga_id: String }`
  - `ChapterId { manga_id: MangaId, chapter_id: String }`
- **Database** (`backend/shared/src/database.rs`): `Database` struct wrapping `sqlx::Pool<Sqlite>` behind `Arc<RwLock<>>`
  - SQLite WAL journal mode, synchronous=NORMAL, cache_size=-2000, temp_store=MEMORY
- Feature-gated modules: `database`, `usecases`, `arima_light` behind `#[cfg(feature = "all")]`

### Comment Style

- **JSDoc-style doc comments** (`///`) on public items
- **FIXME comments** used liberally for known technical debt (e.g., `// FIXME this looks awful`, `// FIXME what the fuck why`)
- Some `// TODO` markers for planned improvements

## Lua Conventions

### Language & Runtime

- **LuaJIT 5.1 compatibility** — KOReader uses LuaJIT. Checked at runtime: `if _VERSION == "Lua 5.1" then`
- **InputContainer widget pattern** from KOReader: `local Foo = InputContainer:extend { name = "foo", ... }`

### Module & Import Patterns

- **Require-based modules:** `local Foo = require("Foo")` — relative requires for intra-plugin modules
- **Returning tables:** Each module returns a table (e.g., `return ErrorDialog` or `return TestHarness`)
- **CamelCase for module/class names:** `LibraryView`, `MangaSearchResults`, `ChapterListing`
- **snake_case for locals and functions:** `local function waitUntilHttpServerIsReady()`, `local disable_logging`

### EmmyLua Annotations

- **All public APIs annotated** with EmmyLua:
  - `--- @class ClassName` — class declarations
  - `--- @field name type` — field types
  - `--- @param name type` — parameter types
  - `--- @return type` — return types
  - `--- @alias TypeName ActualType` — type aliases
  - `--- @enum EnumName` — enum definitions
- Example from `Backend.lua`:
  ```lua
  --- @class Chapter
  --- @field id string The ID of this chapter.
  --- @field source_id string The ID of the source for this chapter.
  ```
- Diagnostic control comments used:
  - `---@diagnostic disable-next-line: different-requires`
  - `--- @diagnostic disable: undefined-global, undefined-field`

### KOReader Widget Patterns

- **UI display:** `UIManager:show(dialog)` — never direct widget rendering
- **Dialog patterns:** `ConfirmBox:new { ... }`, `InfoMessage:new { ... }`
- **Widget extension:** `InputContainer:extend { ... }`
- **Menu registration:** adding entries to `menu_items` table in `addToMainMenu()` callbacks
- **Event handlers:** methods named `onEventName` (e.g., `onStartLibraryView`)
- **Translations:** `local _ = require("gettext+")` — underscore convention for gettext

### Lua Lint / Static Analysis

- **Config:** `.luacheckrc` at repo root:
  - `max_line_length = 300`
  - `stds.lua51` with `read_globals = { "self" }`
  - `ignore = { "212/self", "__" }` (212 = unused argument)
  - `globals = { "G_defaults", "G_reader_settings", "PublishingStatus", "MangaContentRating", "MangaViewer" }`
  - `exclude_files = { "frontend/rakuyomi.koplugin/platform/_meta.lua" }`
- **Language server CI config:** `frontend/.luarc.ci.json` sets LuaJIT runtime, `G_reader_settings` global
- **VSCode globals:** `.vscode/settings.json` — `"Lua.diagnostics.globals": ["G_defaults", "G_reader_settings"]`
- **Editorconfig:** `frontend/.editorconfig` — `indent_style = space`, `indent_size = 2` for `*.lua`

### Platform Dispatch

- `Platform.lua` selects `platform/android_platform.lua` or `platform/generic_unix_platform.lua`
- Android: TCP `127.0.0.1:8787` via LuaSocket
- Unix: fork/exec server binary, UDS (`/tmp/rakuyomi.sock`)
- Linux bridge mode: `RAKUYOMI_USE_BRIDGE=1` env var triggers LuaSocket to TCP

### Server Communication (`Backend.lua`)

- **HTTP/JSON API** via `rapidjson` for JSON handling
  - `replaceRapidJsonNullWithNilRecursively()` to scrub `null` values
- **Request wrapper:** `Backend.requestJson { path, method, body, query_params, timeout } -> { type = 'SUCCESS', body = ... } | { type = 'ERROR', status, message }`
- **LuaSocket on Unix:** `http.request` with Unix domain socket path `/tmp/rakuyomi.sock`
- **Server lifecycle:** `Backend.initialize()`, `Backend.running()`, `Backend.cleanup()`

## Python / E2E Conventions

- **pytest** with `asyncio_mode = "auto"` in `pyproject.toml`
- **Pydantic** for response models from AI query agent
- **Pydantic TypeAdapter** for generic response class dispatch
- **Async fixtures** using `@pytest.fixture` + `async def` with `AsyncGenerator`
- **Type imports:** `from typing import AsyncGenerator, TypeVar, Type, overload`
- **env vars for configuration:** `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL`

## DOX Framework Conventions

- **AGENTS.md hierarchy:** 17 AGENTS.md files as binding work contracts
  - Root `AGENTS.md` -> subtree AGENTS.md files (backend, frontend, docs, scripts, e2e-tests, tools, ci, .github, packages)
  - Each AGENTS.md contains: Purpose, Ownership, Local Contracts, Work Guidance, Verification, Child DOX Index
- **Read-before-edit chain:** root AGENTS.md then nearest subtree AGENTS.md
- **Local contracts** inherit from parent to child

## Project Rules

- **No emojis in code or comments** — enforced by DOX root contract
- **Loosely coupled** Rust backend + Lua frontend via JSON/HTTP API
- **semantic-release** commit message format (`chore(release):`, `fix:`, `feat:`)
  - Config in `.releaserc.yaml` — `@semantic-release/commit-analyzer`, changelog, git tag, GitHub release
- **Versioning:** Tags generated by semantic-release, CHANGELOG.md auto-generated
- **AI review:** CodeRabbit (`chill` profile, poem enabled) in `.coderabbit.yaml`
- **Build packaging:** `scripts/build-all.sh` cross-compiles + packages `rakuyomi.koplugin/` into zip
- **Nix dev shell:** `flake.nix` + `devenv.nix` for reproducible development environment
- **direnv:** `.envrc.dist` loads devenv environment

---

*Convention analysis: 2026-06-28*
