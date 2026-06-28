# External Integrations

**Analysis Date:** 2026-06-28

## APIs & External Services

### Manga Source Runtimes

The server supports two source runtimes for loading manga from external sources (scanlator groups, official platforms, aggregators). These are the primary external integrations — every source `.aix` file is a WASM module that makes its own HTTP requests.

| Component | Technology | Protocol | Configuration | Key File |
|---|---|---|---|---|
| WASM source runtime | wasmi 1.0 | WASM import/export | `.aix` files in sources folder | `backend/shared/src/source/mod.rs` |
| JS source runtime | boa_engine 0.21 | JavaScript -> WASM bridge | `.aix` files with JS payloads | `backend/shared/src/source/mod.rs` |
| WASM FFI serialization | postcard 1.1 | Compact binary | Internal | `backend/shared/src/source/wasm_store.rs` |

**Imports exposed to WASM sources (defined in `backend/shared/src/source/wasm_imports/`):**

| Import Module | Purpose |
|---|---|
| `aidoku` | Aidoku SDK bindings (`register_aidoku_imports`) |
| `net` | HTTP networking (`register_net_imports`), default user-agent |
| `html` | HTML/XML parsing via dom_query CSS selectors |
| `json` | JSON serialization/deserialization |
| `std` | Standard library functions (string, array, etc.) |
| `env` | Environment/context access |
| `defaults` | Default values and constructors |

### Networking Stack

| Component | Technology | Protocol | Configuration | Key File |
|---|---|---|---|---|
| WASM source HTTP | reqwest 0.12 (blocking) | HTTP/HTTPS (rustls-tls) | Embedded in source WASM calls | `backend/shared/src/source/wasm_store.rs` |
| Server HTTP framework | axum 0.8 | HTTP/1.1 | Port/UDS env vars | `backend/server/src/app.rs` |
| Android bridge HTTP | reqwest 0.12 (optional, `ffi` feature) | HTTP/1.1 | TCP 127.0.0.1:8787 | `backend/server/src/jni.rs` |
| Lua frontend HTTP (Android) | LuaSocket (`socket.http`) | HTTP/1.1 | TCP 127.0.0.1:8787 | `frontend/rakuyomi.koplugin/platform/android_platform.lua` |
| Lua frontend HTTP (Unix) | `uds_http_request` binary | Unix domain socket → HTTP/1.1 | `/tmp/rakuyomi.sock` | `frontend/rakuyomi.koplugin/platform/generic_unix_platform.lua` |
| UDS proxy binary | hyper 1.10 / hyperlocal 0.9 | HTTP/1.1 over Unix sockets | Built as separate binary | `backend/uds_http_request/` |

## Data Storage

| Component | Technology | Details | Key File |
|---|---|---|---|
| Database | SQLite via sqlx 0.8 | WAL journal mode, synchronous=NORMAL, cache_size=-2000, temp_store=MEMORY, foreign_keys=ON | `backend/shared/src/database.rs` |
| Settings | JSON file on disk | Default settings written at first boot, read/written via serde, schema generated at build time via schemars | `backend/shared/src/settings/` |
| Sources (installed) | `.aix` WASM files | Stored in `<home_path>/sources/` directory | `backend/shared/src/source_manager.rs` |
| Manga chapters | Raw files on disk | Stored in `<home_path>/chapters/`, managed by `ChapterStorage` | `backend/shared/src/chapter_storage.rs` |
| CBZ metadata | CBZ archive (zip) | Extracted via `zip 6.0` + inline reader binary | `backend/shared/src/cbz_metadata/` |
| Calibration / cache | Local files | Next reader position caching, downloaded chapter tracking | `backend/shared/src/chapter_storage.rs` |

**SQLite configuration:**
- Library: sqlx 0.8 with `sqlite` + `runtime-tokio` features
- Connection pool: `PoolOptions` with sqlx's default connection pooling
- Compile-time query checking: enabled
- Bind limit: 32766 (`BIND_LIMIT` constant)
- Write concurrency: `tokio::sync::RwLock` wraps the pool
- Schema: managed manually (no migrations framework detected)

## Image Processing

| Component | Technology | Purpose | Key File |
|---|---|---|---|
| Image decode/encode | image 0.25, zune-png 0.5, zune-jpeg 0.5 | General manga image handling | `backend/shared/src/source/mod.rs` |
| JPEG encode (mozjpeg) | mozjpeg 0.10 | High-quality JPEG encoding | `backend/shared/src/source/decode_image.rs` |
| Image processing | imageproc 0.26 | Image manipulation utilities | `backend/shared/src/` |
| Image unscrambling | custom | Some sources serve scrambled images; custom decode logic | `backend/shared/src/unscrable_image.rs` |
| Font rendering | raqote 0.8, font-kit 0.14, ab_glyph 0.32 | ARIMA light novel engine text-to-image rendering | `backend/shared/src/arima_light.rs` |

## Document Formats

| Component | Technology | Purpose | Key File |
|---|---|---|---|
| CBZ reading | zip 6.0 (deflate + bzip2) | Comic book archive extraction | `backend/shared/src/cbz_metadata/` |
| EPUB export | epub-builder (git dep) | Library export as EPUB | `backend/shared/src/` |
| HTML parsing | dom_query 0.28 (CSS selectors) | Source page HTML scraping | `backend/shared/src/source/html_element.rs` |
| HTML escaping | html-escape 0.2 | Safe string rendering | `backend/shared/src/` |
| Markdown | markdown 1.0 | Markdown rendering (ARIMA) | `backend/shared/src/` |

## Authentication & Identity

| Component | Technology | Details |
|---|---|---|
| Auth provider | None built-in | Sources manage their own auth internally |
| Source auth | WASM source-level | Some sources support deep links, basic login, web login, key migration (defined in source WASM) |
| Companion app identity | JNI bridge (Android) | `git.shin.rakuyomi_bridge` — the companion app's Java namespace |

## Environment Configuration

**Required env vars (at build time):**
- None strictly required by the build; version is read from `SEMANTIC_RELEASE_VERSION` if available

**Required env vars (at runtime):**
- None strictly required; all have sensible defaults

**Configuration files created on first boot:**
- `<home_path>/settings.json` — application settings
- `<home_path>/database.sqlite` — SQLite database
- `<home_path>/sources/` — installed source WASM files
- `<home_path>/chapters/` — downloaded chapter data
- `<home_path>/logs/` — runtime logs (via server)

**Secrets location:**
- No secrets framework detected. Source-specific credentials (if any) are managed per-source in WASM or stored in source settings JSON.

## CI/CD & Deployment

| Component | Technology | Details |
|---|---|---|
| CI platform | GitHub Actions | 11 workflow files in `.github/workflows/` |
| Build automation | `cross` + Podman | Container-based cross-compilation for all 6 targets |
| Rust build | `cargo build --release` via cross | Per-target release builds |
| Versioning | semantic-release 25 (Node.js) | Conventional Commits → semver, auto-publishes GitHub releases |
| Android build | cargo-ndk + Android NDK r23b | Builds `libserver.so` for aarch64, armv7, x86_64 Android |
| Artifact distribution | GitHub Actions (upload-artifact) | Per-target `.zip` artifacts for each build |
| Release channel | GitHub Releases | Managed by semantic-release with changelog generation |

### CI Workflows

| Workflow | Path | Trigger | Purpose |
|---|---|---|---|
| `Build` | `.github/workflows/build.yml` | push to main, PR | Build all 6 targets, run tests + luacheck, release |
| `Test` | `.github/workflows/test.yml` | push to main, PR, workflow_call | `cargo test --all` |
| `luacheck` | `.github/workflows/luacheck.yml` | push to main (frontend/ changes), PR, workflow_call | Lua linting |
| `Deploy mdBook to GitHub Pages` | `.github/workflows/deploy-pages.yml` | push to main (docs/ changes), workflow_dispatch | Build and deploy documentation site |
| `gemini-dispatch` | `.github/workflows/gemini-dispatch.yml` | (scheduled or triggered) | Gemini AI review dispatch |
| `gemini-invoke` | `.github/workflows/gemini-invoke.yml` | (triggered by dispatch) | Execute Gemini AI review |
| `gemini-review` | `.github/workflows/gemini-review.yml` | (triggered) | AI code review via Gemini |
| `gemini-scheduled-triage` | `.github/workflows/gemini-scheduled-triage.yml` | Scheduled | Periodic issue triage via Gemini |
| `gemini-triage` | `.github/workflows/gemini-triage.yml` | (triggered) | Issue triage via Gemini |
| `issue-label-flow` | `.github/workflows/issue-label-flow.yml` | (issue events) | Automated issue labeling |

### Nix / Dev Shell

| Component | Technology | Details | Key File |
|---|---|---|---|
| Dev shell | Nix flakes (`devShells.default`) | Reproducible development environment | `flake.nix` |
| Nix packages | Nixpkgs (nixos-unstable) | System dependencies: fontconfig, freetype, Lua, gettext, etc. | `flake.nix` devShell |
| Rust toolchain | rust-overlay (Nix) | Rust + cross-compilation targets via Nix | `flake.nix` |
| Binary caching | Cachix | CI cache for Nix derivations | `flake.nix` devShell |
| Build helper lib | Crane | Nix-native Cargo build (used in flake.nix but CI uses `cross` instead) | `flake.nix` |

## Webhooks & Callbacks

**Incoming:** None detected. The server is a standalone HTTP server, does not register external webhooks.

**Outgoing:** None detected. Source network requests go through the WASM runtime's own HTTP client (`reqwest`).

## Testing & Quality

| Component | Technology | Details | Key File |
|---|---|---|---|
| Rust tests | `cargo test` (built-in) | Unit + integration tests via `#[cfg(test)]` | `backend/` |
| Rust benchmarks | criterion 0.5 + pprof 0.15 | Async benchmarks for chapter downloader & search | `backend/shared/benches/` |
| Lua tests | busted + speculate (via `testing.lua`) | Busted-compatible test runner, generates JUnit XML | `frontend/rakuyomi.koplugin/testing.lua` |
| E2E tests | Playwright (Python 3.11+) | Headless KOReader UI tests | `e2e-tests/` |
| E2E test deps | openai, pydantic, pytest, pyautogui, pillow | AI-assisted test agent, screenshot analysis | `e2e-tests/pyproject.toml` |
| Lua linting | luacheck | Static analysis for all frontend Lua | `.github/workflows/luacheck.yml` |
| Rust linting | clippy + rustfmt | Installed as part of CI rust toolchain | `.github/workflows/` |

## Documentation

| Component | Technology | Details | Key File |
|---|---|---|---|
| User guide | mdBook 0.0.28 + mdbook-admonish | Published to GitHub Pages | `docs/` |
| Settings schema | schemars 1.2 (build-time) | Auto-generated from Rust types, used for validation | `backend/shared/src/settings/schema.rs` |

## Internationalization

| Component | Technology | Details |
|---|---|---|
| Framework | Custom `gettext+.lua` | KOReader-compatible gettext implementation |
| Locales | 40+ language directories | Each contains `koreader.po` translation file |
| Template | `.pot` file in `l10n/templates/` | Source strings for translators |
| Translation tools | Custom `Makefile` + `GOOGLE_TRANSLATE.sh` + `restore_po.sh` | Update and manage translation files |
| Locale list | ar, bg_BG, bn, ca, cs, da, de, el, eo, es, eu, fa, fi, fr, gl, he, hi, hr, hu, it_IT, ja, ka, kk, ko_KR, lt_LT, lv, nb_NO, nl_NL, pl, pt_BR, pt_PT, ro, ro_MD, ru, sk, sr, sv, th, tr, uk, vi, zh_CN, zh_TW | 44 locales |

## AI Integration (CI)

| Component | Technology | Details |
|---|---|---|
| Code review | Gemini API | Automated PR review via AI |
| Issue triage | Gemini API | Automated issue categorization and routing |
| E2E test agent | OpenAI API | AI-driven KOReader UI testing agent in `e2e-tests/` |

---

*Integration audit: 2026-06-28*
