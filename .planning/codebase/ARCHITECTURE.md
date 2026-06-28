# Architecture

**Analysis Date:** 2026-06-28

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER (Lua/KOReader)                 │
│                                                                     │
│  ┌────────────┐ ┌──────────────┐ ┌──────────────────┐ ┌──────────┐ │
│  │ LibraryView │ │ ChapterListing││ MangaSearchResults │ │ MangaRead│ │
│  │  38KB      │ │  40KB        │ │  13KB             │ │  8KB     │ │
│  ├────────────┤ ├──────────────┤ ├──────────────────┤ ├──────────┤ │
│  │Menu/Widgets│ │ SettingItem  │ │ NotificationView  │ │ Dialogs  │ │
│  │            │ │ patch/*      │ │ UpdateChecker     │ │          │ │
│  └────────────┘ └──────────────┘ └──────────────────┘ └──────────┘ │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Backend.lua (32KB)                               │   │
│  │  HTTP/JSON request/response to server via Platform dispatch   │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│              ┌───────────────┴───────────────┐                      │
│              ▼                               ▼                      │
│  ┌─────────────────────┐   ┌─────────────────────────┐             │
│  │ AndroidPlatform     │   │ GenericUnixPlatform      │             │
│  │ TCP 127.0.0.1:8787  │   │ fork+exec server binary  │             │
│  │ LuaSocket HTTP       │   │ UDS (/tmp/rakuyomi.sock)│             │
│  └─────────────────────┘   │ uds_http_request bridge  │             │
│                            └─────────────────────────┘             │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ HTTP / JSON
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    API LAYER (axum HTTP Server)                      │
│                                                                     │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌───────┐ ┌──────┐ ┌──────┐│
│  │ manga/  │ │ source/  │ │settings/ │ │system/│ │job/  │ │update││
│  │ routes  │ │ routes   │ │ routes   │ │routes │ │routes│ │routes││
│  └────┬────┘ └────┬─────┘ └────┬─────┘ └───┬───┘ └──┬───┘ └──┬───┘│
│       │           │            │           │        │        │     │
│       └───────────┴────────────┴───────────┴────────┴────────┘     │
│                              │                                      │
│                        ┌─────┴─────┐                                │
│                        │ State     │ ← FromRef<JobState>            │
│                        │ (axum)    │                                │
│                        └─────┬─────┘                                │
│                              │                                      │
│              ┌───────────────┴───────────────┐                      │
│              ▼                               ▼                      │
│  ┌─────────────────────┐   ┌─────────────────────────┐             │
│  │ JNI bridge (ffi)    │   │ pick_listener            │             │
│  │ Android companion   │   │ UDS or TCP listener      │             │
│  │ app loads libserver │   │ via env vars             │             │
│  └─────────────────────┘   └─────────────────────────┘             │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ function calls
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    DATA LAYER (shared crate)                         │
│                                                                     │
│  ┌──────────────┐ ┌──────────────────┐ ┌─────────────────────────┐  │
│  │ Database     │ │ SourceManager    │ │ usecases (40+)          │  │
│  │ sqlx/SQLite  │ │ WASM (wasmi)    │ │ ┌─────────────────────┐ │  │
│  │ 3192 lines   │ │ JS (boa_engine) │ │ │ add_manga_to_library│ │  │
│  │              │ │ 1478 lines      │ │ │ search_mangas       │ │  │
│  │              │ │ Aidoku SDK      │ │ │ fetch_manga_chapter │ │  │
│  │              │ │ next SDK 0.7    │ │ │ … 40 more           │ │  │
│  └──────┬───────┘ └────────┬─────────┘ │ └─────────────────────┘ │  │
│         │                  │           │                          │  │
│         ▼                  ▼           └──────────┬───────────────┘  │
│  ┌──────────────┐ ┌──────────────────┐            │                  │
│  │ Chapter      │ │ Image Processing │            ▼                  │
│  │ Downloader   │ │ unscrable_image  │  ┌─────────────────────┐     │
│  │ 622 lines    │ │ DRM unscrambling │  │ arima_light         │     │
│  ├──────────────┤ ├──────────────────┤  │ ARIMA-based chapter │     │
│  │ Chapter      │ │ CBZ/EPUB export  │  │ release forecasting │     │
│  │ Storage      │ │ (zip/epub-builder)│ │ 737 lines           │     │
│  │ 733 lines    │ │                  │  └─────────────────────┘     │
│  └──────────────┘ └──────────────────┘                              │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Backend.lua | Central HTTP/JSON client for all server communication | `frontend/rakuyomi.koplugin/Backend.lua` |
| Platform.lua | Dispatches to Android or Unix server startup | `frontend/rakuyomi.koplugin/Platform.lua` |
| GenericUnixPlatform | fork+exec server binary + UDS HTTP request proxy | `frontend/rakuyomi.koplugin/platform/generic_unix_platform.lua` |
| AndroidPlatform | Starts companion app foreground service, TCP HTTP | `frontend/rakuyomi.koplugin/platform/android_platform.lua` |
| LibraryView | Main manga library listing (grid/list) | `frontend/rakuyomi.koplugin/LibraryView.lua` |
| ChapterListing | Chapter list for a manga with download/read | `frontend/rakuyomi.koplugin/ChapterListing.lua` |
| MangaSearchResults | Search results with pagination | `frontend/rakuyomi.koplugin/MangaSearchResults.lua` |
| MangaReader | Wraps KOReader's ReaderUI for manga reading | `frontend/rakuyomi.koplugin/MangaReader.lua` |
| Job.lua | Async job polling abstraction | `frontend/rakuyomi.koplugin/jobs/Job.lua` |
| server crate (axum) | HTTP server binary + cdylib for Android JNI | `backend/server/src/` |
| shared crate | Domain model + SQLite DB + WASM/JS sources + use cases | `backend/shared/src/` |
| Database (sqlx) | SQLite database with 10 migrations | `backend/shared/src/database.rs` |
| SourceManager | WASM/JS source lifecycle, Aidoku SDK bridge | `backend/shared/src/source_manager.rs` |
| Source module | WASM execution (wasmi) + JS execution (boa_engine) | `backend/shared/src/source/` |
| ChapterDownloader | Async chapter download, CBZ/EPUB creation | `backend/shared/src/chapter_downloader.rs` |
| ChapterStorage | Filesystem chapter caching with RAM/tmpfs support | `backend/shared/src/chapter_storage.rs` |

## Pattern Overview

**Overall:** Three-tier client-server architecture with a thick Rust server layer.

**Key Characteristics:**
- Lua plugin acts as a thin UI client — all business logic, data storage, and external source fetching happens in the Rust server
- Server runs as a separate process (Unix) or JNI native library (Android), communicating via HTTP/JSON
- Sources (manga content providers) are WASM or JS binaries loaded at runtime by the server
- Asynchronous job system for long-running operations (downloads, library refresh) with polling from Lua
- SQLite database with WAL mode, connection pool (max 4), compiled via sqlx

## Layers

**Presentation Layer (Lua plugin):**
- Purpose: KOReader UI integration — views, menus, dialogs
- Location: `frontend/rakuyomi.koplugin/`
- Contains: ~40 Lua files — views, jobs, widgets, patches, utilities, translations
- Depends on: KOReader framework (`ui/`, `device/`, `ffi/`, `document/`), `rapidjson`, `socket.http`
- Used by: End user via KOReader's main menu or toolbar

**API Layer (server crate):**
- Purpose: HTTP API server exposing domain operations via RESTful JSON endpoints
- Location: `backend/server/src/`
- Contains: axum router with route modules (manga, source, settings, system, playlists, job, update), JNI bridge
- Depends on: shared crate, axum, tokio, serde
- Used by: Lua frontend via HTTP/JSON

**Data Layer (shared crate):**
- Purpose: Domain model, persistence, source execution, business logic use cases
- Location: `backend/shared/src/`
- Contains: Database (sqlx/SQLite), SourceManager (WASM wasmi + JS boa_engine), 40+ use cases, chapter downloader/storage, image processing, ARIMA light novel engine
- Depends on: wasmi, boa_engine, reqwest, sqlx, zip, image, epub-builder, aidoku-rs
- Used by: server crate

## Data Flow

### Primary Request Path (e.g., searching manga)

1. User interacts with Lua view (`LibraryView.lua` / `MangaSearchResults.lua`)
2. View calls `Backend.searchMangas()` in `Backend.lua`
3. `Backend.requestJson()` serializes request via `rapidjson` and calls `Server:request()`
4. Platform-specific `Server` sends HTTP request:
   - Unix: `uds_http_request` binary → Unix domain socket `/tmp/rakuyomi.sock`
   - Android: `socket.http` → `127.0.0.1:8787`
5. axum router dispatches to `manga::routes::search_mangas` handler
6. Handler calls `usecases::search_mangas()` which calls `Source::search()` on the WASM/JS source
7. Source makes external HTTP requests (reqwest) to real manga websites, parses HTML/JSON
8. Results flow back: Source → usecase → route handler → JSON response
9. Lua `Backend.requestJson()` decodes response (rapidjson), returns to view
10. View updates items and renders via KOReader framework

### Async Job Flow (e.g., downloading a chapter)

1. `ChapterListing.lua` creates `DownloadChapter` job object
2. `Backend.createDownloadChapterJob()` → POST `/jobs/download-chapter`
3. Server creates job, spawns background tokio task, returns `job_id`
4. `Job:poll()` in `jobs/Job.lua` loops calling `Backend.getJobDetails(job_id)` → GET `/jobs/{id}`
5. Server returns `{ type: "PENDING" }` with progress while running, `{ type: "COMPLETED" }` with result when done
6. Job runs `download_chapter` usecase → `ensure_chapter_is_in_storage()` → chapter downloaded as CBZ/EPUB
7. On completion, Lua view updates to show downloaded chapter

**State Management:**
- SQLite database (sqlx pooled connections, WAL mode) for persistent manga/chapter/source state
- `CancellationToken` store for cancelable operations (search, download, refresh)
- `tokio::sync::Mutex` for shared mutable state (SourceManager, Settings, ChapterStorage, cancel map)
- `Semaphore` for download concurrency limiting
- Startup log buffer for reporting server initialization issues to the Lua UI

## Key Abstractions

**Source (WASM/JS source provider):**
- Purpose: Represents a manga content source loaded from an `.aix` (WASM) or `.js` file
- Examples: `backend/shared/src/source/mod.rs`, `source_manager.rs`
- Pattern: WASM compiled with `wasmi` interpreter or JS with `boa_engine`, both wrapped in `Source` struct with common interface (search, fetch manga details, fetch chapters, fetch pages)
- SDK versions: legacy Aidoku SDK + next SDK 0.7 (next/) with additional imports

**UseCase:**
- Purpose: Single-responsibility business logic functions that coordinate DB + source operations
- Examples: `backend/shared/src/usecases/` (40+ files, each one pub fn)
- Pattern: Pure async functions taking `&Database`, `&Source`, or other dependencies, returning `Result<T>`

**Job:**
- Purpose: Long-running async work with polling-based progress reporting
- Location: `backend/server/src/job/`
- Pattern: Each job type implements a `run()` async fn, stores progress state, returns `JobDetail` with type-tagged response (`PENDING` | `COMPLETED` | `ERRORED`)

**Platform:**
- Purpose: Abstracts how the server starts and how HTTP requests are sent
- Location: `frontend/rakuyomi.koplugin/platform/`
- Files: `android_platform.lua`, `generic_unix_platform.lua`
- Pattern: Factory via `Platform.lua` that detects Android and returns the appropriate platform module. Each implements `startServer()` → `Server` interface with `request()`, `getLogBuffer()`, `stop()`

## Entry Points

**Server binary:**
- Location: `backend/server/src/main.rs`
- Triggers: Command line with `home_path` argument, parses with `clap`
- Responsibilities: Build tokio runtime, call `server::run(home_path)` which picks a listener (UDS or TCP via env vars) and serves the axum router

**Server cdylib (Android JNI):**
- Location: `backend/server/src/jni.rs`
- Triggers: Java companion app loads `librakuyomi_server.so` via `System.loadLibrary()`
- Responsibilities: Start server in background tokio runtime via `nativeStart`, stop via `nativeStop`, bridge network requests from WASM through Java (since Android blocks native HTTP)

**Lua plugin:**
- Location: `frontend/rakuyomi.koplugin/main.lua`
- Triggers: KOReader loads plugin; registered in main menu as "rakuyomi"
- Responsibilities: Initialize Backend (start server process or connect), show LibraryView on open

**CBA (standalone binary):**
- Location: `backend/cbz_metadata_reader/src/main.rs`
- Triggers: CLI with CBZ file path argument
- Responsibilities: Extract ComicInfo.xml from CBZ, transform to KOReader metadata JSON, print to stdout

**UDS HTTP proxy:**
- Location: `backend/uds_http_request/src/main.rs`
- Triggers: Called by generic_unix_platform.lua when Lua needs to make HTTP requests to the server
- Responsibilities: Receives JSON request via stdin, performs HTTP request to Unix domain socket, returns JSON response via stdout

## Architectural Constraints

- **Threading:** Tokio multi-thread runtime (`tokio::runtime::Builder::new_multi_thread`). WASM source execution happens on async tasks. SQLite pool has max 4 connections. Download semaphore limits concurrent downloads.
- **Global state:** `JAVA_VM`, `SERVER_CLASS`, `NET_PENDING` static singletons in JNI module (`backend/server/src/jni.rs`). `OnceLock`-protected. `ServerState` singleton behind `AsyncMutex<Option<ServerState>>` for the JNI lifecycle.
- **Circular imports:** Not detected between crates. The workspace dependency graph is a DAG: `server → shared`, `server ← wasm_macros ← wasm_shared`, `cbz_metadata_reader → shared`.
- **File system access:** Server binary receives a `home_path` argument where it creates `sources/`, `database.db`, `settings.json`, `downloads/`. All server state is relative to this path. No global config.
- **Cross-compilation:** Rust 1.95.0, 5 targets via `cross` + Podman: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `arm-unknown-linux-musleabi`, `arm-unknown-linux-musleabihf`, `aarch64-linux-android`. Nix flakes for native dev shell (`flake.nix` devShells.default).

## Anti-Patterns

### Source clone in route handlers

**What happens:** `SourceExtractor` (`backend/server/src/source_extractor.rs`) clones the `Source` from `SourceManager`. The code has a FIXME comment noting this relies on `Source` being internally `Arc`-based to be performant.
**Why it's wrong:** The clone API is misleading — it isn't a deep copy but depends on internal reference counting. If `Source` internals change, this could become extremely expensive.
**Do this instead:** Extract only the pieces needed per request, or use `Arc<Source>` explicitly.

### Backend.requestJson uses LuaSocket for Unix platform

**What happens:** The Unix platform (`generic_unix_platform.lua`) forks `uds_http_request` process per request — this means each HTTP call spawns a subprocess, serializes/deserializes JSON via stdin/stdout, and runs a full HTTP client inside that process.
**Why it's wrong:** High per-request overhead (process spawn + JSON roundtrip + TCP to UDS). For rapid polling (job progress checks every 1 second), this creates significant overhead on constrained e-ink devices.
**Do this instead:** Consider LuaSocket UDS support directly, or a persistent UDS connection from Lua.

### Job polling architecture

**What happens:** Long-running jobs are polled from Lua by calling `Backend.getJobDetails(id)` every 1 second (`jobs/Job.lua` line 29: `JOB_POLLING_INTERVAL_SECONDS = 1`). Each poll triggers a subprocess spawn + HTTP request on Unix platforms.
**Why it's wrong:** Polling at 1Hz on e-ink devices wastes battery and CPU. The architecture could use server-sent events or a push mechanism.
**Do this instead:** Increase polling interval to 5-10 seconds for constrained devices, or implement a long-poll/comet endpoint.

## Error Handling

**Strategy:** Rust-side uses `anyhow::Result` + custom `AppError` enum (`backend/server/src/error.rs`) with HTTP status code mapping. Lua side wraps all requests in `requestJson()` returning tagged union `{ type: 'SUCCESS' | 'ERROR', ... }`.

**Patterns:**
- `AppError` → axum `IntoResponse` with status code + JSON error body `{ message: "..." }`
- `SourceNotFound` → HTTP 404, `NotFound` → 404, `NetworkFailure` → 502, `Other` → 500
- Lua always checks `response.type` before accessing `response.body`
- `ErrorDialog:show()` displays server errors to user
- `rapidjson.null` values from JSON are replaced with `nil` recursively in Lua

## Cross-Cutting Concerns

**Logging:** Rust uses `log` crate facade + `tracing_subscriber` with `env-filter` (default `RUST_LOG=info`). Lua uses KOReader's `logger` module. Server logs captured via pipe to Lua for display in UI.

**Validation:** JSON body deserialization via `serde` with `#[derive(Deserialize)]` — axum rejects malformed requests automatically. WASM/JS source function arguments validated via `TryFromWasmValues` trait.

**Authentication:** None. The server listens only on `127.0.0.1` (TCP) or `/tmp/rakuyomi.sock` (UDS). It is a local-only service.

## Platform Deployment Architectures

### Unix (Kindle/Kobo/reMarkable/Linux)

```
Lua (KOReader)
  │
  ├─ Platform: generic_unix_platform.lua
  │    ├─ fork() → exec(server_binary)       # Start server process
  │    │   server_binary: axum, UDS listener
  │    │   Listens on /tmp/rakuyomi.sock
  │    │
  │    └─ HTTP request path:
  │         Lua → execute_binary_fast(uds_http_request, json_stdin)
  │              uds_http_request: hyper client
  │                   → HTTP to Unix Domain Socket /tmp/rakuyomi.sock
  │                        → axum handler → response
  │              json_stdout ← response back to Lua
  │
  └─ On Kobo: os.execute("ifconfig lo 127.0.0.1") to ensure loopback
```

### Android (Companion App)

```
Lua (KOReader on Android)
  │
  ├─ Platform: android_platform.lua
  │    ├─ Opens "rakuyomi_bridge://start" via android.openLink()
  │    │   This starts foreground service in Rakuyomi Bridge app
  │    │   Bridge loads librakuyomi_server.so via System.loadLibrary()
  │    │   Server binds to TCP 127.0.0.1:8787
  │    │
  │    └─ HTTP request path:
  │         Lua → socket.http → TCP 127.0.0.1:8787 → axum handler
  │
  └─ JNI bridge (backend/server/src/jni.rs):
       nativeStart(homePath, port) → background tokio runtime
       nativeStop() → graceful shutdown via oneshot channel
       nativeIsRunning() → status check
       nativeSendNetworkResponse() / nativeSendNetworkError()
         → Bridges WASM source HTTP requests through Android Java layer
           (since Android restricts native sockets)
```

### Linux Bridge Mode

```
Lua (KOReader on desktop Linux with RAKUYOMI_USE_BRIDGE=1)
  │
  └─ Server runs as systemd user service or manually started
       Binds to TCP 127.0.0.1:8787
       LuaSocket HTTP via generic_unix_platform.lua socket.http
```

---

*Architecture analysis: 2026-06-28*
