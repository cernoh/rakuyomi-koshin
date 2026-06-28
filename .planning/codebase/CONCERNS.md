# Codebase Concerns

**Analysis Date:** 2026-06-28

## Technical Debt

### database.rs: 3191-line monolithic SQL layer — HIGH
- **Issue:** `backend/shared/src/database.rs` (3191 lines) contains every database operation in a single `impl Database` block — queries for manga library, chapters, sources, settings, playlists, notifications, and more all in one file. 28 distinct public methods plus ~170 internal `sqlx::query!` calls.
- **Impact:** Makes the file hard to navigate, test, and modify. Any schema change requires touching this one file. No inline `#[cfg(test)]` blocks exist — zero unit tests for any query.
- **Fix approach:** Split into per-domain modules (e.g., `database/manga.rs`, `database/chapters.rs`, `database/settings.rs`). Extract query helpers into a shared submodule.

### source/mod.rs: 1477 lines mixing WASM execution, source parsing, and metadata extraction — HIGH
- **Issue:** `backend/shared/src/source/mod.rs` (1477 lines) contains WASM source instantiation (`BlockingSource`), the `Source` wrapper with `spawn_blocking` dispatch, source manifest parsing, metadata extraction, HACK comments for Aidoku compatibility (`line 693`), and the `call_cleanup!` macro.
- **Impact:** Difficult to reason about the WASM execution lifecycle. The `call_cleanup!` macro (`line 110`) does manual memory management for WASM descriptors with error-prone clone-and-free semantics.
- **Fix approach:** Split by concern: `source/execution.rs` for WASM lifecycle, `source/metadata.rs` for manifest and feature parsing, `source/fallback.rs` for Aidoku SDK compatibility shims.

### wasm_store.rs: 789 lines with 6 unsafe impls — HIGH
- **Issue:** `backend/shared/src/source/wasm_store.rs` contains:
  - 6 `unsafe impl Send/Sync` for `Html`, `JsContext`, `Canvas` types (lines 61-62, 190-191, 195-196) — the comments say "THIS IS BORKED AS FUCK" (line 60) and "See above." (line 66)
  - 5 `static OnceLock` function pointers for webview hooks (`WEBVIEW_LOAD`, `WEBVIEW_WAIT_FOR_LOAD`, `WEBVIEW_EVAL`, `WEBVIEW_DESTROY`, `REQUEST_TRY_FROM`) — late-initialized globals that panic if accessed before initialization
  - Multiple `#[allow(clippy::large_enum_variant, dead_code)]` suppressions
  - Blocking reqwest API usage with a comment "Maybe we should give up on using the blocking reqwest APIs" (line 716)
- **Impact:** Thread-safety of the WASM store is unverified. The unsafe impls are acknowledged hacks. If webview callbacks aren't set before a source uses them, the runtime panics with `.expect()`.
- **Fix approach:** Audit the `Send/Sync` impls — use wrapper types that properly validate thread safety. Replace `OnceLock` panics with fallible APIs returning `Result`. Move blocking reqwest calls to `spawn_blocking`.

### 112 unwrap() calls and 14 expect() calls across Rust code — MEDIUM
- **Issue:** Files with highest unwrap density: `wasm_imports/next/std.rs` (14), `server/src/job/dto.rs` (11), `wasm_imports/html.rs` (10), `source/html_element.rs` (10), `database.rs` (7). Most `expect()` calls are in webview/reuqest callback globals that would panic if uninitialized.
- **Impact:** Any of these could produce an unhandled panic in production. The `dto.rs` usage (11 unwraps in a data-transfer module) is especially risky — malformed DTOs would crash the server.
- **Fix approach:** Audit and replace with `?` operator or `.context()` from anyhow. At minimum, document which unwraps are provably infallible.

### 69 TODO/FIXME/HACK markers across codebase — MEDIUM
- **Issue:** Comment markers scattered across Rust (48) and Lua (21) code. Notable clusters:
  - `wasm_store.rs`: 4 FIXMEs + 1 TODO (threading, response state, settings handling)
  - `source/mod.rs`: 8 FIXMEs + 3 HACKs (Aidoku compatibility, scoping, chapter counting)
  - `database.rs`: 3 FIXMEs (error handling, read field override)
  - `ChapterListing.lua`: 6 FIXMEs (async refactoring, chapter counting, error assumption)
  - `download_unread_chapters.rs`: 3 FIXMEs — one says "THIS IS SO WRONG" (line 127)
- **Impact:** Many FIXMEs indicate known design problems that haven't been addressed. Some reflect correctness uncertainty ("is it ok to assume there are no errors here?").
- **Fix approach:** Triage by risk. Address the "THIS IS SO WRONG" and error-assumption FIXMEs first.

### Large Lua UI files (>30KB each) — MEDIUM
- `ChapterListing.lua`: 1442 lines (1442) — handles chapter listing, filtering, sorting, reading, multi-select operations
- `LibraryView.lua`: 1386 lines (1386) — main library display, search, filtering, source management
- `Backend.lua`: 971 lines — server communication, JSON serialization, error handling
- `MangaInfoWidget.lua`: 626 lines, `Settings.lua`: 567 lines
- **Impact:** Large files make maintenance harder. High risk of merge conflicts. ~9881 total lines of Lua across the plugin.
- **Fix approach:** Extract reusable widgets, split LibraryView into sub-modules (search, filtering, display), move server communication patterns into Backend.lua utilities.

### Workflow backup/stale files in .github/workflows/ — LOW
- **Issue:** `test.yml%` (1350 bytes) and `issue-label-flow.yml%` (2827 bytes) present alongside `test.yml` (900 bytes). `test.yml%` appears to be a newer nix-based workflow draft (with format/lint/frontend-test/e2e steps) that was never renamed to replace `test.yml`. `issue-label-flow.yml%` exists without a non-% counterpart.
- **Impact:** Dead files in CI directory create confusion about which workflow is active. `test.yml` only runs `cargo test --all` while `test.yml%` would run format + lint + frontend tests + E2E tests.
- **Fix approach:** Decide which workflow to use, rename/remove the stale copy.

### .whitesource file still present — LOW
- **Issue:** `.whitesource` contains a Mend (formerly Whitesource) config for dependency scanning. The tool was acquired by Mend and renamed. The config has empty `baseBranches` — likely unused or migrated.
- **Impact:** If the project no longer uses Mend, the file is dead config. If still used, it should reference the current product name.
- **Fix approach:** Verify with team; remove if unused.

### 17 #[allow(dead_code)] and other lint suppressions — LOW
- **Issue:** Files with `#[allow(dead_code)]`: `database.rs` (lines 3069, 3112, 3146), `source/mod.rs` (line 282), `wasm_imports/next/canvas.rs` (50, 59), `wasm_imports/next/defaults.rs` (30), `wasm_imports/next/html.rs` (56), `wasm_imports/next/net.rs` (53), `wasm_imports/std.rs` (29), `wasm_store.rs` (42). Also `#[allow(clippy::large_enum_variant)]` (3 files), `#[allow(clippy::too_many_arguments)]` (2 files), `#[allow(unused_imports)]` (benchmarks).
- **Impact:** Dead code can rot without detection. Suppressed lints may mask real issues.
- **Fix approach:** Remove dead code or add `#[allow(dead_code)]` with a documented reason. Move dead-code structs to their usage site.

## Cross-Platform Risk

### Three platform implementations with different transports — HIGH
- **Issue:** Platform dispatch in `frontend/rakuyomi.koplugin/Platform.lua` dynamically requires either `platform/android_platform.lua` (125 lines) or `platform/generic_unix_platform.lua` (170 lines). The Rust server side resolves to UDS on Unix, TCP on bridge mode (`RAKUYOMI_TCP_PORT`), or JNI on Android (libserver.so loaded by companion app).
  - Unix: fork/exec server binary, UDS at `/tmp/rakuyomi.sock`, `uds_http_request` bridges HTTP→UDS
  - Android: JNI via `server/src/jni.rs` (493 lines) with Android API 18 polyfills, TCP 127.0.0.1:8787
  - Linux bridge: TCP 127.0.0.1:8787 via systemd user service
- **Impact:** Bugs may be platform-specific and hard to reproduce. The JNI path uses `.try_lock()` on `OnceLock<AsyncMutex>` (lines 273, 362, 483) which returns `INTERNAL_ERROR` on contention but could deadlock under load. Blocking in JNI on `block_on()` (line 254) from a JNI thread is risky.
- **Fix approach:** Standardize on TCP across all platforms if possible. Add integration tests for each platform transport. Document the JNI threading model clearly.

### Android API 18 polyfills — MEDIUM
- **Issue:** `server/src/jni.rs` has a `#[cfg(feature = "api_18")]` module (`lines 46-84`) that polyfills `epoll_create1`, `dl_iterate_phdr`, and `signal` — APIs missing on Android API 18. Android 18 (Jelly Bean 4.3, 2013) is long deprecated.
- **Impact:** The polyfills use raw syscall wrappers with limited error handling. The signal polyfill is a no-op that returns `0`, which could mask signal handling issues.
- **Fix approach:** Consider bumping minimum API level to a more modern version and removing the polyfill module.

### Cross-compilation complexity — MEDIUM
- **Issue:** Five targets in `scripts/build-all.sh`: desktop (musl), aarch64, kindle (arm musleabi), kindlehf (arm musleabihf), kindlea9 (Cortex-A9 optimized). Uses `cross` with Podman. Separate `scripts/build-rust-android.sh` for Android with 3 ABIs (arm64-v8a, armeabi-v7a, x86_64). Build plugin script copies binaries and removes translation build artifacts.
- **Impact:** Each target adds build CI time. The Cortex-A9 aggressive optimization path and musl vs bionic differences can introduce subtle platform bugs. Build failures can be hard to reproduce locally.
- **Fix approach:** Maintain CI caching aggressively. Consider reducing targets if certain devices are no longer supported. Document required test matrix.

### platform/_meta.lua excluded from luacheck — LOW
- **Issue:** `frontend/rakuyomi.koplugin/platform/_meta.lua` (34 lines, EmmyLua type definitions) is explicitly excluded from luacheck via `.luacheckrc`: `exclude_files = { "frontend/rakuyomi.koplugin/platform/_meta.lua" }`.
- **Impact:** The meta file contains the Platform and Server interface contracts. Being excluded from linting means type stubs can drift from actual implementation.
- **Fix approach:** Fix the lint issues rather than excluding. The file likely just needs `-- @type` annotations or `_` prefix cleanup.

## Dependency Risk

### Forked git dependencies — HIGH
- **Issue:** Three dependencies pinned to git branches rather than crates.io versions:
  - `pared`: `https://github.com/hanatsumi/pared.git` branch `feat/unwrap-or-clone` — a feature branch that may never be merged upstream
  - `epub-builder`: `https://github.com/tachibana-shin/epub-builder.git` branch `main` — outside the mainline epub-builder project
  - `aidoku`: `https://github.com/Aidoku/aidoku-rs.git` branch `main` — external SDK bindings
- **Impact:** Git dependencies can break with upstream force-pushes or become unavailable. The `pared` dependency on an unreleased feature branch is particularly risky — no version pin, no semver guarantees.
- **Fix approach:** Fork and pin to a specific commit hash. Alternatively, vendor the patches needed. For `pared`, evaluate if the `unwrap-or-clone` behavior can be inlined.

### Patched tiff crate via [patch.crates-io] — MEDIUM
- **Issue:** `backend/Cargo.toml` patches `tiff = { git = "https://github.com/image-rs/image-tiff", version = "0.11.3" }` — pulls the crate directly from git rather than crates.io.
- **Impact:** Pinned to upstream git, which could introduce breaking changes. If the patch was needed for a specific bugfix, it may be stale once a new version hits crates.io.
- **Fix approach:** Document the reason for the patch in a comment. Check if a newer crates.io version resolves the issue.

### boa_engine 0.21 — JS interpreter dependency — MEDIUM
- **Issue:** `backend/shared/Cargo.toml` depends on `boa_engine = "0.21.1"` — a full JavaScript engine pulled in for WASM source JS evaluation (webview/JS bridge in `wasm_imports/next/js.rs`). The `boa_engine` crate and its ~12 sub-crates (`boa_ast`, `boa_parser`, `boa_gc`, etc.) contribute significantly to build time and binary size.
- **Impact:** Large dependency tree for a niche use case (evaluating JS in WASM sources). Security surface of a full JS engine in a manga reader.
- **Fix approach:** Evaluate if JS evaluation is used by all sources or only a few. Consider feature-gating the JS engine behind a Cargo feature flag.

### wasmi — interpreted WASM runtime — LOW
- **Issue:** WASM sources run via `wasmi` (v1.0.9), an interpreter-based WASM runtime. No JIT/compilation tier.
- **Impact:** Manga source parsing is on the critical path for user experience. Interpreted WASM is typically 10-50x slower than native. For CPU-intensive source parsers, this can cause noticeable UI lag (the Lua side handles this via `spawn_blocking`, but it still blocks a thread).
- **Fix approach:** Monitor if sources with complex parsing (e.g., those using Canvas/image manipulation via WASM) cause slowdowns. Consider `wasmtime` (JIT) as a drop-in replacement if performance becomes an issue.

### Large dependency tree — LOW
- **Issue:** ~300+ transitive dependencies in `backend/Cargo.lock`. Image processing stack contributes heavily: `image` → `exr`, `gif`, `jpeg-decoder`, `png`, `tiff`, `webp`, `avif-serialize`, `rav1e`, etc. Also font rendering: `font-kit`, `raqote`, `ab_glyph`, `freetype-sys`.
- **Impact:** Long CI build times. Potential for supply chain vulnerabilities in the broad tree.
- **Fix approach:** Use `cargo-udeps` to remove unused deps. Feature-gate image format support to formats actually used. Already using `rustls` over openssl — good.

## Safety & Correctness

### Unsafe Send/Sync impls with acknowledged hacks — HIGH
- **Issue:** In `backend/shared/src/source/wasm_store.rs`:
  - `unsafe impl Send for Html {}` / `unsafe impl Sync for Html {}` — comment says "THIS IS BORKED AS FUCK"
  - `unsafe impl Send for JsContext {}` / `unsafe impl Sync for JsContext {}` — wraps `boa_engine::JsNativeObject`
  - `unsafe impl Send for Canvas {}` / `unsafe impl Sync for Canvas {}` — wraps `DrawTarget`
- **Impact:** These are unchecked assertions that non-thread-safe types are safe to transfer across threads. If the underlying types (particularly `dom_query::Document` for Html, or raqote's `DrawTarget` for Canvas) have interior mutability or thread-unsafe state, this can cause data races.
- **Fix approach:** Audit each type to verify it's truly `Send + Sync`. Add explicit documentation justifying each `unsafe impl`. Consider wrapping in `Arc<Mutex<>>` instead of lying to the compiler.

### Static OnceLock function pointer globals — MEDIUM
- **Issue:** 10+ `static OnceLock` globals across `wasm_store.rs`, `wasm_imports/net.rs`, `wasm_imports/next/env.rs`, `wasm_imports/next/defaults.rs`, `server/src/jni.rs` — all store function pointers or trait objects that must be set before WASM execution:
  - `WEBVIEW_LOAD`, `WEBVIEW_WAIT_FOR_LOAD`, `WEBVIEW_EVAL`, `WEBVIEW_DESTROY`, `REQUEST_TRY_FROM`
  - `NET_SEND`, `SEND_PARTIAL_RESULT`, `LOG_PRINT`, `DEFAULTS_SET`, `DEFAULTS_GET`
  - JNI: `JAVA_VM`, `SERVER_CLASS`, `NET_PENDING`, `LOG_QUEUE`, `SERVER`
- **Impact:** Each `OnceLock::get().expect(...)` is a potential runtime panic if initialization ordering is wrong. The registration pattern is implicit — no compile-time guarantee that dependencies are satisfied before WASM execution.
- **Fix approach:** Replace with a single init function that takes all required callbacks as parameters and returns a struct. Or use `Arc`-based dependency injection through the `WasmStore`.

### JNI threading model — MEDIUM
- **Issue:** `server/src/jni.rs` uses `.try_lock()` on `OnceLock<AsyncMutex>` for the server state (lines 273, 362, 483). If the lock is contended, returns `INTERNAL_ERROR`. The network bridge uses `block_on()` (line 254) inside a JNI callback to await a `oneshot` channel — blocking a JNI thread.
- **Impact:** Under load, `try_lock()` failures cause operation failures rather than waiting. `block_on()` from a JNI thread can cause thread-pool starvation if the tokio runtime's threads are busy with other tasks.
- **Fix approach:** Use non-blocking lock acquisition. Consider a dedicated thread pool for JNI→Rust communication.

### source/mod.rs: reqwest::blocking in async context — MEDIUM
- **Issue:** `BlockingSource` uses reqwest's blocking client, dispatched via `tokio::task::spawn_blocking` through the `wrap_blocking_source_fn!` macro (line 97-102). The comment at line 83-84 notes: "all calls to reqwest::blocking methods from an async context causes the program to panic."
- **Impact:** The macro correctly wraps every exposed method in `spawn_blocking`, but the codebase has 15+ additional `block_on()` calls in WASM import functions (`wasm_imports/net.rs`, `wasm_imports/next/net.rs`, `wasm_imports/next/js.rs`) that block the current thread directly. These run inside `spawn_blocking` threads, but if a source calls networking from a non-blocking WASM context, it could panic.
- **Fix approach:** The `block_on` calls in WASM imports should be unified into the `spawn_blocking` dispatch. Add clear documentation about which contexts are safe for blocking calls.

## Maintainability

### Minimal test coverage — HIGH
- **Coverage gaps:**
  - **database.rs:** Zero inline tests. 0 `#[cfg(test)]` or `#[test]` functions in a 3191-line file that defines the entire data access layer.
  - **Lua frontend:** Only 1 spec file: `chapters/findNextChapter_spec.lua` (95 lines). ~9881 lines of Lua, ~95 lines of test = ~1% test coverage.
  - **Rust total:** 36 `#[test]` functions across 5 files (`chapter_storage.rs`, `arima_light.rs`, `source_settings.rs`, `html_element.rs`, `wasm_imports/std.rs`). 0 tests in the server crate.
  - **No integration tests:** No Rust integration tests for HTTP routes. E2E tests exist in Python (Playwright) but require KOReader + display server to run.
- **Impact:** Database changes cannot be validated without running the full app. UI changes risk regressions with no safety net. Refactoring is high-risk.
- **Fix approach:** Add `#[cfg(test)]` module to database.rs with in-memory SQLite tests. Add route-level integration tests using `axum::test`. Add busted Lua tests for individual widgets.

### SQLx requires compile-time database for query verification — MEDIUM
- **Issue:** `backend/shared/build.rs` includes `println!("cargo:rerun-if-changed=migrations");` — the sqlx `query!` macro connects to a live database at compile time to verify SQL. `tools/prepare-sqlx-queries.sh` runs `cargo sqlx prepare` to generate `.sqlx/` cache.
- **Impact:** If the `.sqlx/` cache is stale or the database schema doesn't match, the project won't compile. CI must either have the `.sqlx/` cache committed or run `sqlx prepare` as a build step. Currently `test.yml` does not run `sqlx prepare`.
- **Fix approach:** Commit the `.sqlx/` cache directory. Add `sqlx prepare` to CI build steps. Consider using `query_as` (runtime) instead of `query_as!` (compile-time) for dynamic queries.

### 41+ translation files to maintain — LOW
- **Issue:** `frontend/rakuyomi.koplugin/l10n/` contains 41 locale directories (ar, bg_BG, bn...zh_TW), with `.po` files, generated `.mo` files, `Makefile`, and `GOOGLE_TRANSLATE.sh` (112 lines). The build script (`build-plugin.sh`) runs `make mo` and then `rm -rf */*.po .gitignore *.sh Makefile *.md *.po` — which means the .po sources are deleted during build.
- **Impact:** If someone runs `build-plugin.sh` locally, the `.po` files are destroyed. The `GOOGLE_TRANSLATE.sh` script suggests machine translation, which may produce low-quality translations.
- **Fix approach:** The build script should copy, not modify in-place. Consider a dedicated translation platform (e.g., Crowdin, Weblate) to reduce maintainer burden.

### Nix + devenv + flake.lock — multiple dev environment definitions — LOW
- **Issue:** The project uses `devenv.nix` (120 lines) + `devenv.yaml` + `devenv.lock` + `flake.lock` for the primary dev shell. This provides Rust 1.95.0, KOReader, busted, lua-language-server, cargo-flamegraph, python, poetry, and various scripts (`check-format`, `check-lint`, `test-frontend`, `test-e2e`, etc.).
- **Impact:** The Nix setup is fairly comprehensive but adds complexity. New contributors need Nix installed. The lock files must be updated regularly. CI also uses Nix (in `test.yml%`).
- **Fix approach:** Document the dev environment setup clearly. Keep lock files updated. The CI test.yml (non-%) doesn't use Nix, so there are two parallel environment definitions.

### Consistent CI fragmentation — LOW
- **Issue:** 11 workflows in `.github/workflows/`: `build.yml` (171 lines), `test.yml` (40 lines), `test.yml%` (50 lines, nix-based), `luacheck.yml` (27 lines), `deploy-pages.yml` (60 lines), and 4 gemini-* workflows. The gemini workflows (`gemini-dispatch.yml`, `gemini-invoke.yml`, `gemini-review.yml`, `gemini-triage.yml`, `gemini-scheduled-triage.yml`) use AI for code review/triage.
- **Impact:** The gemini review pipeline adds external API dependencies to CI. If the API key or service is unavailable, CI can be blocked. The workflow fragmentation makes it hard to understand the full CI pipeline.
- **Fix approach:** Consolidate related workflows. Document the gemini review dependency. Ensure the `test.yml` workflow runs all necessary checks without depending on external AI services.

## Performance

### Chapter downloader CPU usage from image transcoding — MEDIUM
- **Issue:** `backend/shared/src/chapter_downloader.rs` (621 lines) performs image format conversion (to JPEG) and resizing during chapter download. Uses `mozjpeg` for encoding, `image` crate for decoding. The `chapter_storage.rs` handles RAM/TMPFS caching with manual eviction (`evict_least_recently_modified_chapter`).
- **Impact:** Image transcoding during bulk chapter downloads (e.g., downloading an entire scanlated manga) is CPU-bound on the e-ink device's limited CPU. The LRU eviction algorithm scans filesystem mtimes sequentially, which is O(n) per eviction.
- **Fix approach:** Consider background transcoding with lower priority. The RAM cache with TMPFS is a solid optimization — consider making its size configurable in the UI.

### wasmi interpreted execution for all source operations — LOW
- **Issue:** Every source function call (search, get_manga_details, get_chapter_list, get_page_list) requires WASM interpretation via wasmi. The `call_cleanup!` macro adds descriptor allocation and free overhead per call.
- **Impact:** For sources with complex parsing (HTML/document traversal), the WASM overhead compounds. The Canvas/Rendering API (`wasm_imports/next/canvas.rs`, 520 lines) uses software rasterization via raqote, adding more CPU work.
- **Fix approach:** Already mitigated via `spawn_blocking`. If sources become performance-critical, consider caching parsed results or batching WASM calls.

### Potential memory usage from image caching — LOW
- **Issue:** `chapter_storage.rs` supports a RAM disk (TMPFS) for chapter caching, sized via `enable_ram(size_mb)`. Image data is stored decoded (RGB) and re-encoded to JPEG. Chapter eviction checks full storage size each time.
- **Impact:** On devices with limited RAM (Kindle: ~256MB), aggressive caching could trigger OOM. The `tmpfs_full_storage()` check (line 243) is the only guard.
- **Fix approach:** Add hard memory limits. The current LRU eviction is sound but O(n) in total files.

## Missing Critical Features

### No database migration rollback path — MEDIUM
- **Issue:** The project uses `sqlx::migrate!()` which only runs forward migrations. The `hot_replace()` method in `database.rs` (lines 46-140) has a comprehensive backup/restore mechanism for database replacement, but there's no way to roll back a schema migration without manual SQL.
- **Impact:** If a migration breaks, the only recovery is restoring a backup from disk. No `down.sql` files exist in `backend/shared/migrations/` (9 migration files, all `up` only).
- **Fix approach:** Add down migrations for future schema changes. Document the manual rollback procedure for existing databases.

### No structured error handling across Lua ↔ Rust boundary — MEDIUM
- **Issue:** `Backend.lua` serializes errors via HTTP response as generic JSON with type `'ERROR'`. The Rust `server/src/error.rs` (119 lines) defines application errors but they're flattened to HTTP status codes. No structured error codes or user-facing error messages are passed through.
- **Impact:** When a request fails, the Lua UI can only show the HTTP status code and a generic message. Debugging failures requires server logs.
- **Fix approach:** Define a shared error schema with codes, messages, and (optionally) recovery hints. Pass structured errors through the JSON response body.

### No observability for offline/cached state — LOW
- **Issue:** The server provides no health endpoint or cache status API. `shared/src/usecases/get_cached_manga_details.rs` (39 lines) and `get_cached_manga_chapters.rs` (15 lines) handle cache lookups but expose no metrics.
- **Impact:** Users have no visibility into whether data is from cache or fresh. Debugging stale-data issues requires server log inspection.
- **Fix approach:** Add a `/system/health` endpoint. Expose cache hit rates or staleness in the response metadata.

---

*Concerns audit: 2026-06-28*
