# Codebase Structure

**Analysis Date:** 2026-06-28

## Directory Layout

```
rakuyomi-koshin/
├── backend/                  # Rust workspace — HTTP server + tools
│   ├── server/               #   axum HTTP server (binary + cdylib)
│   ├── shared/               #   Core library (domain, DB, sources, use cases)
│   ├── uds_http_request/     #   Unix domain socket HTTP proxy binary
│   ├── cbz_metadata_reader/  #   CBZ metadata extraction binary
│   ├── wasm_macros/          #   Proc-macro crate for WASM bindings
│   ├── wasm_shared/          #   Shared WASM interop types/traits
│   └── .cross/               #   Cross-compilation Dockerfiles
├── frontend/
│   └── rakuyomi.koplugin/    # KOReader Lua plugin (~40 files)
│       ├── platform/         #   Platform dispatch (Android vs Unix)
│       ├── jobs/             #   Async job implementations
│       ├── widgets/          #   Custom UI widgets
│       ├── patch/            #   KOReader UI patch modules
│       ├── utils/            #   Utility functions
│       ├── chapters/         #   Chapter navigation logic
│       ├── extensions/       #   Document extension (CBZ)
│       ├── handlers/         #   Event handlers
│       └── l10n/             #   Translations (40+ languages via gettext)
├── docs/                     # mdBook documentation site
│   ├── src/                  #   Documentation markdown sources
│   │   ├── user-guide/       #   User documentation
│   │   └── contributing/     #   Contribution guide
│   └── theme/                #   Custom mdBook theme
├── e2e-tests/                # Python/Playwright end-to-end tests
│   └── tests/                #   Test scripts + driver
├── scripts/                  # Build scripts
├── tools/                    # Developer utilities
├── ci/                       # CI helper scripts
├── .github/                  # GitHub Actions workflows + templates
│   ├── workflows/            #   11 CI/CD workflow files
│   ├── commands/             #   GitHub command configurations
│   └── ISSUE_TEMPLATE/       #   Bug report & feature request templates
├── packages/                 # Nix/KOReader packaging
│   └── patches/              #   Patches for KOReader
└── .vscode/                  # Editor configuration
```

## Directory Purposes

### Root Directory

- **Purpose:** Project root with Rust workspace, Lua plugin, documentation, CI/CD, and packaging
- **Contains:** Workspace config, build system, development environment, CI/CD configuration
- **Key files:**
  - `README.md` — Project overview, installation links, feature highlights
  - `flake.nix` — Nix flake for dev shell (devShells.default) and cross-compilation packages
  - `rust-toolchain.toml` — Rust 1.95.0 with clippy, rust-analyzer
  - `.releaserc.yaml` — semantic-release configuration
  - `package.json` — Node.js semantic-release dependencies
  - `CHANGELOG.md` — Release changelog
  - `LICENSE` — License file (33.7KB)
  - `.luacheckrc` — Lua linting configuration
  - `.coderabbit.yaml` — CodeRabbit AI review config
  - `.envrc.dist` — Example direnv environment
  - `AGENTS.md` — DOX framework agent configuration (17 AGENTS.md files across tree)

### `backend/` — Rust Workspace

**Purpose:** Rust monorepo with 6 crates — HTTP server, core library, auxiliary tools
**Contains:** Cargo workspace with workspace-level dependency versions and release profile settings

**Key files:**
- `Cargo.toml` — Workspace manifest (members: shared, server, uds_http_request, wasm_macros, wasm_shared, cbz_metadata_reader)
- `Cargo.lock` — Dependency lockfile (153.9KB, 6400+ lines)
- `.gitignore` — Ignores target/

#### `backend/server/` — HTTP Server Crate

**Purpose:** axum HTTP server built as both a binary (Unix) and cdylib (Android JNI)
**Contains:** Route modules, state management, JNI bridge, listener resolution

**Key files:**
- `Cargo.toml` — Dependencies: axum 0.8, tokio, shared, serde, jni (optional), reqwest (optional)
- `src/main.rs` — CLI entry point with clap arg parsing, creates tokio runtime, calls `server::run()`
- `src/lib.rs` — Library root re-exporting all modules and key functions (build_router, build_state, run, serve)
- `src/app.rs` — Router construction (merge all route modules), state initialization, server bootstrapping
- `src/state.rs` — Axum `FromRef`-based state with SourceManager, Database, ChapterStorage, Settings, JobState
- `src/jni.rs` — JNI entry points for Android companion app (nativeStart, nativeStop, nativeIsRunning, network bridging)
- `src/listener.rs` — Transport selection: UDS vs TCP based on env vars (RAKUYOMI_TCP_PORT / RAKUYOMI_UNIX_SOCKET_PATH)
- `src/error.rs` — AppError enum with axum IntoResponse (maps to HTTP status codes + JSON error body)
- `src/model.rs` — Response DTOs for Lua frontend (Manga, Chapter, SourceInformation serialization)
- `src/source_extractor.rs` — Axum FromRequestParts extractor that fetches Source from SourceManager by source_id path param

**Route modules:**
- `src/manga/` — Main business routes (library CRUD, chapters, notifications, sync, search)
- `src/source/` — Source lifecycle (list available/installed, install, uninstall, settings)
- `src/settings/` — App settings get/put, tmpfs mount
- `src/system/` — System stats, startup log retrieval
- `src/playlists/` — Playlist CRUD, add/remove mangas
- `src/job/` — Async job creation and polling (download chapter, refresh library, etc.)
- `src/update/` — Check and install server updates

#### `backend/shared/` — Core Library Crate

**Purpose:** Domain model, database, source execution, use cases, chapter storage, image/ARIMA processing
**Contains:** All business logic — the largest and most complex crate

**Key files:**
- `Cargo.toml` — Dependencies: wasmi, boa_engine, reqwest, sqlx, zip, image, aidoku-rs, 50+ crates
- `build.rs` — Build-time settings.schema.json generation via schemars, triggers on migration changes
- `src/lib.rs` — Module declarations, feature-gated: `database` and `usecases` behind `feature = "all"`, `arima_light` behind `feature = "all"`
- `src/database.rs` — SQLite database (3192 lines): 10 migrations, CRUD for manga/chapter/source/playlist state, WAL mode, 4-connection pool
- `src/model.rs` — Core domain types: SourceId, MangaId, ChapterId, SourceInformation, Manga, Chapter, ChapterState, MangaState, Playlist, NotificationInformation
- `src/source/mod.rs` — WASM source execution (1478 lines): Aidoku SDK imports (net, json, html, std, env), next SDK 0.7 imports (canvas, js, std), wasmi Store management, ImageRef/image pipeline
- `src/source_manager.rs` — Source lifecycle (install from .aix file, uninstall, get by id, update settings, load from folder)
- `src/source/model.rs` — Source domain types: Filter, SettingDefinition variants (group, select, switch, text, login, etc.), MangaPageResult, Page
- `src/source/wasm_store.rs` — WASM memory store + operation context (RequestBuildingState, ResponseData, caching)
- `src/source/wasm_imports/` — WASM import modules: aidoku.rs (Aidoku SDK), defaults.rs, env.rs, html.rs, json.rs, net.rs, std.rs
- `src/source/wasm_imports/next/` — Next SDK 0.7 import modules: canvas.rs, defaults.rs, env.rs, html.rs, js.rs, net.rs, std.rs
- `src/source/decode_image.rs` — Image decoding: decode_argb_to_rgb, decode_image_fast (via zune-jpeg/mozjpeg)
- `src/usecases/` — 40+ single-responsibility use case modules (each one pub fn):

| Use Case | File |
|----------|------|
| add_manga_to_library | `add_manga_to_library.rs` |
| add_manga_to_playlist | `add_manga_to_playlist.rs` |
| check_mangas_update | `check_mangas_update.rs` |
| check_update | `check_update.rs` |
| clear_notifications | `clear_notifications.rs` |
| create_playlist | `create_playlist.rs` |
| delete_notification | `delete_notification.rs` |
| delete_playlist | `delete_playlist.rs` |
| fetch_manga_chapter | `fetch_manga_chapter.rs` |
| fetch_manga_chapters_in_batch | `fetch_manga_chapters_in_batch.rs` |
| find_orphan_or_read_files | `find_orphan_or_read_files.rs` |
| get_cached_manga_chapters | `get_cached_manga_chapters.rs` |
| get_cached_manga_details | `get_cached_manga_details.rs` |
| get_count_notifications | `get_count_notifications.rs` |
| get_manga_library | `get_manga_library.rs` |
| get_manga_preferred_scanlator | `get_manga_preferred_scanlator.rs` |
| get_mangas_in_playlist | `get_mangas_in_playlist.rs` |
| get_notifications | `get_notifications.rs` |
| get_playlists | `get_playlists.rs` |
| get_source_setting_definitions | `get_source_setting_definitions.rs` |
| get_source_stored_settings | `get_source_stored_settings.rs` |
| install_source | `install_source.rs` |
| install_update | `install_update.rs` |
| list_available_sources | `list_available_sources.rs` |
| list_installed_sources | `list_installed_sources.rs` |
| mark_chapter_as_read | `mark_chapter_as_read.rs` |
| mark_chapters_as_read | `mark_chapters_as_read.rs` |
| refresh_manga_chapters | `refresh_manga_chapters.rs` |
| refresh_manga_details | `refresh_manga_details.rs` |
| remove_manga_from_library | `remove_manga_from_library.rs` |
| remove_manga_from_playlist | `remove_manga_from_playlist.rs` |
| rename_playlist | `rename_playlist.rs` |
| revoke_manga_chapter | `revoke_manga_chapter.rs` |
| search_mangas | `search_mangas.rs` |
| set_manga_preferred_scanlator | `set_manga_preferred_scanlator.rs` |
| set_source_stored_settings | `set_source_stored_settings.rs` |
| sync_database | `sync_database.rs` |
| uninstall_source | `uninstall_source.rs` |
| update_last_read_chapter | `update_last_read_chapter.rs` |
| update_settings | `update_settings.rs` |

- `src/chapter_downloader.rs` — Async chapter download pipeline (622 lines): concurrent page downloads via reqwest stream, CBZ/EPUB creation, image optimization, progress reporting, DRM unscrambling
- `src/chapter_storage.rs` — File-based chapter cache (733 lines): CBZ/EPUB files on disk or RAM/tmpfs, storage size tracking with cache eviction, SHA-256 content hashing
- `src/unscrable_image.rs` — Image DRM unscrambling (83 lines): pixel-block rearrangement based on source-defined Block array
- `src/arima_light.rs` — ARIMA(1,1,1) forecasting for light novel chapter release prediction (737 lines): conditional least squares fitting with coordinate descent, rolling window
- `src/settings/mod.rs` — Settings re-exports (ChapterSortingMode, LibrarySortingMode, etc.)
- `src/settings/implementation.rs` — Settings file IO (from_file, save_to_file)
- `src/settings/schema.rs` — Settings struct with all config fields + schemars derive
- `src/cbz_metadata/mod.rs` — ComicInfo.xml schema + reader (244 lines)
- `src/util.rs` — Shared utilities (HTML generation, image helpers, download_all_images, request_with_forced_referer)
- `src/source_collection.rs` — SourceCollection utility (aggregating sources by ID for available sources listing)
- `migrations/` — 10 SQLite migrations (2024-2026 date-stamped)
- `benches/` — Criterion benchmarks for chapter_downloader and search_mangas
- `fonts/` — Font files for ARIMA/epub rendering
- `dev.db` — Development SQLite database
- `.sqlx/` — sqlx offline query cache

#### `backend/uds_http_request/` — UDS HTTP Proxy Binary

**Purpose:** Standalone binary that reads a JSON request from stdin, performs an HTTP request to a Unix domain socket, and writes JSON response to stdout. Called by generic_unix_platform.lua for each API call.
**Key files:**
- `src/main.rs` — hyper + hyperlocal client, reads stdin JSON, performs UDS request, writes stdout JSON response
- `Cargo.toml` — Dependencies: hyper, hyperlocal, tokio, serde, anyhow

#### `backend/cbz_metadata_reader/` — CBZ Metadata Tool Binary

**Purpose:** CLI tool that reads ComicInfo.xml from a CBZ file and outputs KOReader-compatible metadata JSON
**Key files:**
- `src/main.rs` — clap CLI, reads CBZ via shared::cbz_metadata, transforms to KoReaderMetadata, prints JSON
- `Cargo.toml` — Dependencies: shared, clap, anyhow, serde

#### `backend/wasm_macros/` — Proc-Macro Crate

**Purpose:** `#[aidoku_wasm_function]` attribute macro that generates WASM binding code for Aidoku SDK functions
**Key files:**
- `src/lib.rs` — proc_macro implementation (201 lines): generates register_wasm_function fn, internal fn, parameter type inference via FromWasmValues trait, return type encoding

#### `backend/wasm_shared/` — WASM Interop Types

**Purpose:** Shared WASM interop traits and types used by both shared and wasm_macros crates
**Key files:**
- `src/lib.rs` — FromWasmValues trait, TryFromWasmValues (330 lines): memory reader, WASM value conversion for String, Vec<u8>, DateTime, i32, f64, etc.
- `src/memory_reader.rs` — WASM linear memory reading helpers (read_string, read_bytes)

#### `backend/.cross/`

**Purpose:** Cross-compilation Dockerfiles for use with `cross` tool + Podman
**Key files:**
- `arm-unknown-linux-musleabi.Dockerfile` — armv5te musl cross-compilation environment

### `frontend/rakuyomi.koplugin/` — Lua Plugin Directory

**Purpose:** KOReader plugin implementing the manga reader UI — ~40 Lua source files
**Contains:** Views, platform dispatch, async jobs, custom widgets, KOReader patches, utilities, translations, tests

**Key files:**
- `main.lua` — Plugin entry point: extends `InputContainer`, registers with KOReader main menu, initializes Backend on library view open
- `Backend.lua` — Central HTTP/JSON API client (972 lines): 40+ API methods wrapping `requestJson()`, server lifecycle management (initialize, running, cleanup)
- `Platform.lua` — Platform detection and dispatch: checks if Android is available, loads android_platform or generic_unix_platform
- `LibraryView.lua` — Main library view (1387 lines): manga grid/list display, search, import, library management, update checking
- `ChapterListing.lua` — Chapter list view (1443 lines): chapter listing with sorting, scanlator filtering, download, read, mark read
- `MangaSearchResults.lua` — Search results view (468 lines): paginated search results with view mode cycling
- `MangaReader.lua` — Book reader integration (242 lines): wraps KOReader's ReaderUI to display CBZ/EPUB files
- `MangaInfoWidget.lua` — Detailed manga info panel (17.5KB)
- `Settings.lua` — App settings view (568 lines): settings UI with RAM info, storage management
- `SourceSettings.lua` — Per-source settings view (8.2KB)
- `AvailableSourcesListing.lua` — Source browser and installer (6.7KB)
- `InstalledSourcesListing.lua` — Installed source management (5.2KB)
- `NotificationView.lua` — Notification list view (200 lines)
- `PlaylistDialog.lua` — Playlist management dialog (9.8KB)
- `UpdateChecker.lua` — Server update checking UI (149 lines)
- `ErrorDialog.lua` — Error display dialog (669B)
- `LoadingDialog.lua` — Loading spinner dialog (7.4KB)
- `BasicJobDialog.lua` — Job progress dialog (7.6KB)
- `CheckboxDialog.lua` — Multi-select dialog (1.5KB)
- `CustomDialog.lua` — Generic custom dialog (3.1KB)
- `OfflineAlertDialog.lua` — Offline notification dialog (2.3KB)
- `DownloadUnreadChaptersJobDialog.lua` — Batch download dialog (822B)
- `Paths.lua` — Path resolution utilities (635B): home directory, plugin directory
- `Icons.lua` — Icon definitions (1.5KB): material design icons
- `gettext+.lua` — Translation library (11.1KB): gettext-style l10n with .po file loading
- `testing.lua` — Test infrastructure (275 lines): UI snapshot, event emission for E2E testing

#### `frontend/rakuyomi.koplugin/platform/`

**Purpose:** Server startup and HTTP communication platform abstraction
**Key files:**
- `_meta.lua` — EmmyLua type annotations for Platform/Server interface
- `android_platform.lua` — Android: starts companion app via android.openLink, TCP HTTP via socket.http to 127.0.0.1:8787
- `generic_unix_platform.lua` — Unix: fork+exec server binary, UDS HTTP via uds_http_request subprocess, pipe-based log capture
- `util.lua` — Platform utilities: SubprocessOutputCapturer (non-blocking pipe I/O via poll/dup2), must()

#### `frontend/rakuyomi.koplugin/jobs/`

**Purpose:** Async job implementations that wrap the server's job API via `Job.lua` polling pattern
**Key files:**
- `Job.lua` — Base job class with `poll()`, `runUntilCompletion()`, `requestCancellation()`
- `DownloadChapter.lua` — Download a single chapter job
- `DownloadUnreadChapters.lua` — Download all unread chapters job
- `DownloadScanlatorChapters.lua` — Download chapters by scanlator job
- `RefreshLibraryChapters.lua` — Refresh chapter list for all library manga
- `RefreshLibraryDetails.lua` — Refresh manga details for all library manga

#### `frontend/rakuyomi.koplugin/widgets/`

**Purpose:** Custom UI widgets extending KOReader's widget system
**Key files:**
- `Menu.lua` — Rakuyomi's menu base class (1.8KB)
- `SettingItem.lua` — Individual setting widget (1.9KB)
- `SettingItemValue.lua` — Setting value picker (11.6KB)
- `SpacedBetweenHorizontalGroup.lua` — Layout widget (848B)

#### `frontend/rakuyomi.koplugin/patch/`

**Purpose:** KOReader UI patch modules for enhanced manga display
**Key files:**
- `MenuItemCover.lua` — Cover image rendering in menu items (18.4KB)
- `MenuItemGrid.lua` — Grid layout for manga items (3.8KB)
- `MenuCustom.lua` — Custom menu extensions (8.7KB)

#### `frontend/rakuyomi.koplugin/utils/`

**Purpose:** Utility functions used across views
**Key files:**
- `calcLastReadText.lua` — Human-readable last-read time text
- `filterChaptersByLang.lua` — Language-based chapter filtering
- `findEntries.lua` — Chapter entry search helpers
- `findLastRead.lua` — Find the last-read chapter in a list
- `getChapterDisplayName.lua` — Formatted chapter display name
- `isBeforeChapter.lua` — Chapter ordering comparison
- `executeBinaryFast.lua` — Fast subprocess execution for UDS proxy
- `urlContent.lua` — URL content helpers
- `beforeWifi.lua` — WiFi requirement helper
- `hasValue.lua` — Table value check

#### `frontend/rakuyomi.koplugin/chapters/`

**Purpose:** Chapter navigation logic (next/previous chapter detection)
**Key files:**
- `findNextChapter.lua` — Find next chapter in sequence
- `findPreviousChapter.lua` — Find previous chapter in sequence
- `findNextChapter_spec.lua` — Test specification (3.0KB)

#### `frontend/rakuyomi.koplugin/extensions/`

**Purpose:** Document format extensions
**Key files:**
- `CbzDocument.lua` — CBZ file format support for KOReader's document registry (4.4KB)

#### `frontend/rakuyomi.koplugin/handlers/`

**Purpose:** Event handlers
**Key files:**
- `addToPlaylist.lua` — Add manga to playlist handler (665B)

#### `frontend/rakuyomi.koplugin/l10n/`

**Purpose:** Internationalization via gettext .po files — 40+ languages
**Key files:**
- `templates/koreader.pot` — Translation template (23.8KB)
- `Makefile` — Translation build system (6.1KB)
- `restore_po.sh` — .po file restoration utility
- `GOOGLE_TRANSLATE.sh` — Machine translation automation

### `docs/` — mdBook Documentation Site

**Purpose:** User guide and contributing documentation
**Key files:**
- `book.toml` — mdBook configuration (493B)
- `src/README.md` — Documentation home page
- `src/SUMMARY.md` — Book table of contents
- `src/user-guide/` — Installation, quickstart, offline mode, reader settings
- `src/contributing/` — Setting up development environment
- `src/images/` — Screenshots and demo GIF
- `theme/book.js` — Custom mdBook JavaScript

### `e2e-tests/` — End-to-End Test Suite

**Purpose:** Python/Playwright E2E tests for the Rakuyomi plugin
**Key files:**
- `pyproject.toml` — Python project config with Poetry
- `poetry.lock` — Python dependency lockfile
- `tests/conftest.py` — Test configuration
- `tests/koreader_driver.py` — KOReader test harness driver (Playwright-based)
- `tests/test_library_view.py` — Library view tests
- `tests/test_open_chapter.py` — Chapter opening tests
- `tests/test_search_view_modes.py` — Search view mode tests
- `tests/agent.py` — AI agent integration for test automation
- `tests/fixtures.py` — Test fixtures
- `tests/phase_report_hook.py` — Test phase reporting
- `tests/queries/` — Test queries directory

### `scripts/` — Build Scripts

**Purpose:** Build, packaging, and code generation scripts
**Key files:**
- `build-all.sh` — Build all targets via `cross` + Podman (desktop/aarch64/kindle/kindlehf/kindlea9 + android plugin packaging)
- `build-plugin.sh` — Build and package the Lua plugin (with version stamping)
- `build-rust-android.sh` — Build Rust shared library for Android targets (aarch64-linux-android, etc.)
- `generate-settings-schema.sh` — Generate settings JSON schema

### `tools/` — Developer Tools

**Purpose:** Development utilities for testing and deployment
**Key files:**
- `install-into-remote-koreader.py` — Deploy plugin to a remote KOReader device via SSH
- `run-koreader-with-plugin.sh` — Launch KOReader with the Rakuyomi plugin locally
- `prepare-sqlx-queries.sh` — Prepare sqlx compile-time query checking
- `setup-macos.sh` — macOS development environment setup
- `dev-macos.sh` — macOS development launcher

### `ci/` — CI Scripts

**Purpose:** Continuous integration helper scripts
**Key files:**
- `lua-language-server-check.py` — LuaLS type checking CI script
- `run-e2e-tests.sh` — E2E test runner for CI

### `.github/` — GitHub CI/CD

**Purpose:** GitHub Actions workflows, commands, and issue templates
**Key files:**
- `workflows/build.yml` — Build workflow for all targets
- `workflows/test.yml` — Test workflow
- `workflows/luacheck.yml` — Lua code quality check
- `workflows/deploy-pages.yml` — Deploy mdBook documentation to GitHub Pages
- `workflows/gemini-dispatch.yml` — Gemini AI dispatch workflow (7.6KB)
- `workflows/gemini-invoke.yml` — Gemini AI invocation (4.5KB)
- `workflows/gemini-review.yml` — Gemini AI code review (4.1KB)
- `workflows/gemini-triage.yml` — Gemini AI issue triage (6.1KB)
- `workflows/gemini-scheduled-triage.yml` — Scheduled AI triage (7.9KB)
- `workflows/issue-label-flow.yml%` — Issue label automation (partial)
- `commands/gemini-invoke.toml` — Gemini invoke command config
- `commands/gemini-review.toml` — Gemini review command config
- `commands/gemini-triage.toml` — Gemini triage command config
- `commands/gemini-scheduled-triage.toml` — Scheduled triage command config
- `ISSUE_TEMPLATE/bug_report.md` — Bug report template
- `ISSUE_TEMPLATE/feature_request.md` — Feature request template
- `FUNDING.yml` — GitHub funding configuration

### `packages/` — Packaging & Patches

**Purpose:** KOReader packaging and patches for the Rakuyomi integration
**Key files:**
- `koreader.nix` — Nix expression for building KOReader with Rakuyomi
- `koreader-macos-arm64.zip` — Pre-built KOReader for macOS ARM64 (27.1MB)
- `patches/fontlist-use-bitser.patch` — Font list serialization optimization
- `patches/datastorage-isolate-storage.patch` — Data storage isolation for Rakuyomi

### `.vscode/` — Editor Configuration

**Purpose:** VS Code workspace settings
**Key files:**
- `settings.json` — Editor settings
- `extensions.json` — Recommended extensions

## Naming Conventions

**Files:**
- Rust files: `snake_case.rs` (e.g., `chapter_downloader.rs`, `source_manager.rs`)
- Lua files: `PascalCase.lua` for classes (e.g., `LibraryView.lua`, `MangaSearchResults.lua`), `camelCase.lua` for utilities (e.g., `calcLastReadText.lua`, `findEntries.lua`)
- Rust source directory: `snake_case/` matching module name (e.g., `source/wasm_imports/`)
- Migration files: Unix timestamp prefixed (e.g., `20250601202635_create_manga_state_table.sql`)

**Directories:**
- `snake_case/` for Rust modules (e.g., `usecases/`, `wasm_imports/`)
- Directory names in Lua plugin are short identifiers (e.g., `jobs/`, `utils/`, `widgets/`)
- `.github/workflows/` uses `kebab-case.yml`

## Where to Add New Code

**New Feature (e.g., new API endpoint):**
- Route handler: `backend/server/src/{category}/routes.rs` (add route + handler function)
- Use case: `backend/shared/src/usecases/{new_usecase}.rs` (single-responsibility async fn)
- Database query: `backend/shared/src/database.rs` (add method to `Database` impl)
- Lua API method: `frontend/rakuyomi.koplugin/Backend.lua` (add `Backend.xxx()` method)
- Lua view: `frontend/rakuyomi.koplugin/` (new view file or extend existing)

**New Source platform (WASM):**
- WASM imports: `backend/shared/src/source/wasm_imports/{name}.rs`
- Or next SDK: `backend/shared/src/source/wasm_imports/next/{name}.rs`
- WASM store types: `backend/shared/src/source/wasm_store.rs`

**New Source platform (JS):**
- Boa engine integration: `backend/shared/src/source/` (extend `Source` struct)

**New UI Screen:**
- Implementation: `frontend/rakuyomi.koplugin/` (PascalCase named file extending Menu or MenuCustom)
- Add to main.lua registration if needed

**New Async Job:**
- Job impl: `backend/server/src/job/` (new module with `run()` fn)
- Job state: `backend/server/src/job/state.rs` (add variant to RunningJob enum)
- Route: `backend/server/src/job/routes.rs` (add endpoint)
- Lua job wrapper: `frontend/rakuyomi.koplugin/jobs/` (extend Job.lua)

**Tests:**
- Rust unit tests: co-located in each `.rs` file with `#[cfg(test)] mod tests { ... }`
- Rust benchmarks: `backend/shared/benches/`
- Lua tests: `testing.lua` via busted, or spec files like `chapters/findNextChapter_spec.lua`
- E2E tests: `e2e-tests/tests/` (Python + Playwright)

## Special Directories

**`backend/target/`:**
- Purpose: Rust build artifacts
- Generated: Yes (by cargo)
- Committed: No (gitignored)

**`backend/shared/migrations/`:**
- Purpose: SQLite schema migrations
- Generated: No (hand-written)
- Committed: Yes

**`frontend/rakuyomi.koplugin/l10n/`:**
- Purpose: Gettext translation files (40+ languages)
- Generated: `.pot` template generated from Lua sources; `.po` files manually maintained or machine-translated
- Committed: Yes

**`packages/koreader-macos-arm64.zip`:**
- Purpose: Pre-built KOReader binary for macOS development
- Generated: Downloaded artifact
- Committed: Yes (large binary, 27MB)

**`backend/shared/dev.db`:**
- Purpose: Development SQLite database
- Generated: Yes (by sqlx)
- Committed: Yes (92KB, for offline query checking)

**`.planning/`:**
- Purpose: GSD planning artifacts
- Generated: Yes (by GSD workflow)
- Committed: Yes

---

*Structure analysis: 2026-06-28*
