# Technology Stack

**Analysis Date:** 2026-06-28

## Languages & Runtimes

| Language/Runtime | Version | Where Used |
|---|---|---|
| Rust | 1.95.0 (edition 2021) | Backend server, all backend crates |
| LuaJIT | 5.1 (KOReader runtime) | Plugin frontend, UI, jobs, widgets |
| Python | 3.11+ | E2E tests (Playwright) |
| JavaScript | ES2020 (via boa_engine 0.21) | JS-based Aidoku source WASM files |
| Nix | Flake-based | Reproducible dev shell via flake devShell |
| Shell (bash) | POSIX | Build scripts, CI scripts |

## Key Frameworks & Libraries

### Rust Backend (`backend/` workspace, 6 crates)

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `server/` | HTTP server binary + Android cdylib | axum 0.8, tokio 1.52, clap 4.6, serde, reqwest 0.12 (opt), jni 0.22 (opt), nix 0.31 |
| `shared/` | Core domain: manga models, DB, WASM sources, downloads, settings, ARIMA | wasmi 1.0, sqlx 0.8, boa_engine 0.21, reqwest 0.12, image 0.25, zip 6.0, raqote, font-kit, ab_glyph, schemars, chrono, aidoku-rs |
| `uds_http_request/` | UDS HTTP proxy binary (Unix: Kindle/Kobo) | hyper 1.10, hyperlocal 0.9, tokio |
| `cbz_metadata_reader/` | CBZ metadata extraction binary | clap 4.6, shared lib |
| `wasm_macros/` | Proc-macro crate for WASM bindings | proc-macro2, quote, syn 2.0, wasmi |
| `wasm_shared/` | Shared WASM interop types | wasmi, chrono |

**Core Rust dependencies across crates:**

- **HTTP framework:** axum 0.8 — HTTP server with FromRef state pattern, typed extractors, middleware support
- **Async runtime:** tokio 1.52 (full features) — multi-threaded runtime, `tokio::runtime::Builder::new_multi_thread()`
- **WASM runtime:** wasmi 1.0 — interprets Aidoku `.aix` WASM source files
- **JS runtime:** boa_engine 0.21 — JavaScript execution for JS-based Aidoku sources
- **Database:** sqlx 0.8 — SQLite with compile-time query checking, WAL journal mode, `runtime-tokio` feature
- **HTTP client:** reqwest 0.12 — `rustls-tls` backend, blocking + streaming support
- **Serialization:** serde 1.0 + serde_json 1.0 — all JSON communication between server and Lua frontend
- **JSON Schema:** schemars 1.2 — generates settings schema from Rust types at build time
- **Image processing:** image 0.25 — decode/encode manga images; mozjpeg 0.10, zune-png 0.5, zune-jpeg 0.5, imageproc 0.26
- **Font rendering (ARIMA):** raqote 0.8 — 2D vector renderer; font-kit 0.14 — system font loading; ab_glyph 0.32 — glyph layout
- **Archive:** zip 6.0 — CBZ (comic book archive) reading with deflate + bzip2 support
- **FFI:** postcard 1.1 — compact binary serialization for WASM FFI
- **DOM parsing:** dom_query 0.28 — CSS selector-based HTML parsing (used by WASM sources)
- **HTML/XML:** quick-xml 0.40 — XML serialization; html-escape
- **EPUB generation:** epub-builder (git dep) — export manga as EPUB
- **Cryptography:** sha2 0.11, base64 0.22
- **Date/time:** chrono 0.44 + chrono-tz 0.10
- **Env overlay:** serde_json_lenient 0.2 — tolerant JSON parsing for third-party source data

### Lua Frontend (`frontend/rakuyomi.koplugin/`)

- **KOReader framework:** `InputContainer`, `UIManager`, `WidgetContainer`, `Menu` — KOReader's custom widget system
- **JSON handling:** `rapidjson` — native C JSON library for LuaJIT; all server communication uses JSON
- **i18n:** `gettext+.lua` — custom gettext implementation with 40+ locale directories (`l10n/`), `.po` files
- **Networking (Android):** `socket.http` (LuaSocket) — TCP HTTP requests to `127.0.0.1:8787`
- **Networking (Unix):** subprocess via `ffi` — executes `uds_http_request` binary for UDS HTTP proxying
- **Testing:** `busted` — Lua unit test framework (`testing.lua` runner), spec files (`*_spec.lua`)

### Dev/CI Tools

- **Cross-compilation:** `cross` + Podman — 5 targets via Docker containers
- **Documentation:** mdBook 0.0.28 + mdbook-admonish — user guide, published to GitHub Pages
- **E2E testing:** Playwright (Python 3.11+) — automated KOReader UI tests
- **Lua linting:** `luacheck` — Lua static analysis, run via GitHub Actions
- **Rust tooling:** rustfmt, clippy — formatting and linting
- **Profiling:** criterion 0.5 + pprof 0.15 — async benchmarks for chapter downloader and search
- **Debugging:** `cargo-debugger` (custom build) — Rust debugging utility, available in dev shell

## Build System

**Workspace:** Cargo workspace at `backend/Cargo.toml` with 6 members (`shared`, `server`, `uds_http_request`, `wasm_macros`, `wasm_shared`, `cbz_metadata_reader`)

**Resolved workspace dependencies:**
- `wasmi = "1.0.9"` — shared across `shared`, `wasm_macros`, `wasm_shared`

**Patches:**
- `tiff` — patched to `image-rs/image-tiff` git repo (version 0.11.3)

**Build profiles (`backend/Cargo.toml`):**

| Profile | Settings |
|---|---|
| `dev` | incremental = true |
| `release` | opt-level = 3, lto = "fat", codegen-units = 1, panic = "abort" |

**Cross-compilation config (set by `scripts/build-all.sh`):**
```toml
[env]
RUST_FONTCONFIG_DLOPEN = "on"
FONTCONFIG_NO_PKG_CONFIG = "1"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
```

**Nix dev shell (`flake.nix` `devShells.default`):**
- Languages: Rust 1.95.0 (via rust-overlay)
- Packages: koreader, mold-wrapped (Linux), Lua 5.1, luacheck, cargo-debugger, gettext, busted, lua-language-server, mdbook
- Post-enter hook: `cargo fetch --manifest-path="$PWD/backend/Cargo.toml"`
- Cachix integration for binary caching

**CI build flow (`.github/workflows/build.yml`):**
1. Install system deps (fontconfig, freetype, podman, gettext)
2. Install Rust 1.95.0 with rustfmt + clippy
3. Install `cross` for cross-compilation
4. Run `npx semantic-release --dry-run` for version detection
5. Run `bash scripts/build-all.sh {target}` — builds per target
6. Package and upload artifacts as `rakuyomi-{target}.zip`
7. For Android: additionally install Android NDK r23b + cargo-ndk, build with `build-rust-android.sh`

**Versioning:** semantic-release 25 (Node.js) with Conventional Commits

**Lockfile:** `backend/Cargo.lock` (committed)

## Deployment Targets

| Build Name | Rust Target | Arch | Libc | Device |
|---|---|---|---|---|
| desktop | `x86_64-unknown-linux-musl` | x86_64 | musl | Linux PC (bridge mode) |
| aarch64 | `aarch64-unknown-linux-musl` | aarch64 | musl | Modern e-readers |
| kindle | `arm-unknown-linux-musleabi` | armv6 | musl | Older Kindle (earlier models) |
| kindlehf | `arm-unknown-linux-musleabihf` | armv7hf | musl | Kobo, early Kindle (with FPU) |
| kindlea9 | `arm-unknown-linux-musleabi` (with Cortex-A9 opts) | armv7 | musl | Kindle with Cortex-A9 CPU |
| android | `aarch64-linux-android` + `armv7-linux-androideabi` + `x86_64-linux-android` | aarch64/armv7/x86_64 | bionic | Android devices (companion app) |

**KindleA9 specifics:** `-C target-cpu=cortex-a9 -C target-feature=+thumb2,+neon` Rust flags

## Platform Architecture by Device

| Platform | Server Binary | Transport | Frontend Mechanism |
|---|---|---|---|
| Kindle / Kobo / reMarkable | Built-in binary (fork/exec) | Unix domain socket (`/tmp/rakuyomi.sock`) | `uds_http_request` binary bridges HTTP → UDS |
| Desktop Linux (bridge) | Systemd user service | TCP `127.0.0.1:8787` | LuaSocket when `RAKUYOMI_USE_BRIDGE=1` |
| Android | `libserver.so` via JNI | TCP `127.0.0.1:8787` | LuaSocket via companion app's network loop |
| Desktop Linux (native) | Built-in binary | Unix domain socket (UDS) | `uds_http_request` binary |

## Runtime Configuration via Environment Variables

| Variable | Platform | Default | Purpose |
|---|---|---|---|
| `RAKUYOMI_USE_TCP` | Unix | absent | Force TCP listener over UDS |
| `RAKUYOMI_TCP_PORT` | Unix/Android | 8787 | TCP listener port |
| `RAKUYOMI_UNIX_SOCKET_PATH` | Unix | `/tmp/rakuyomi.sock` | Custom UDS path |
| `RAKUYOMI_USE_BRIDGE` | Unix | absent | Use systemd bridge (TCP) instead of direct UDS |
| `RAKUYOMI_SERVER_COMMAND_OVERRIDE` | Unix | — | Custom server binary path |
| `RAKUYOMI_SERVER_WORKING_DIRECTORY` | Unix | — | Server working directory |
| `RAKUYOMI_UDS_HTTP_REQUEST_COMMAND_OVERRIDE` | Unix | — | Custom UDS proxy binary path |
| `RAKUYOMI_UDS_HTTP_REQUEST_WORKING_DIRECTORY` | Unix | — | UDS proxy working directory |
| `RAKUYOMI_SERVER_STARTUP_TIMEOUT` | Both | 5 | Server startup timeout in seconds |
| `RAKUYOMI_DISABLE_LOGGING` | Both | false | Disable server log capture |
| `SEMANTIC_RELEASE_VERSION` | CI | — | Version override for builds (from semantic-release) |

---

*Stack analysis: 2026-06-28*
