## Domain

SQLite schema for manga tracking, Rust domain types, OAuth authentication flows for AniList (implicit) and MyAnimeList (PKCE), QR code generation for e-ink auth, and HTTP API routes for service discovery and auth lifecycle.

Phase 1 delivers the foundation — DB storage + auth plumbing + QR. No sync logic, no tracker search, no Lua UI. Those are Phase 2-3.

## Canonical Refs

- `.planning/ROADMAP.md` — Phase tasks and success criteria
- `.planning/REQUIREMENTS.md` — Full requirements traceability (DB-01 through QR-06)
- `.planning/PROJECT.md` — Key decisions (OAuth clients, tech stack, platform constraints)
- `.planning/codebase/ARCHITECTURE.md` — Three-tier architecture, state pattern, route module conventions
- `.planning/codebase/INTEGRATIONS.md` — External integrations, SQLite config, no existing auth framework
- `.planning/codebase/STACK.md` — Crate deps (axum 0.8, sqlx 0.8, reqwest 0.12), build profiles
- `.planning/codebase/STRUCTURE.md` — Route module pattern, use case pattern, model.rs patterns
- `.planning/codebase/CONVENTIONS.md` — Rust naming, axum FromRef state, error handling patterns
- `backend/shared/src/database.rs` — Existing DB setup (sqlx migrations, WAL mode, pool config)
- `backend/shared/src/model.rs` — Existing domain model patterns (SourceId, MangaId, ChapterId newtypes)
- `backend/server/src/app.rs` — build_router() merging route modules
- `backend/server/src/state.rs` — FromRef<State> pattern, Arc<Mutex<...>> for shared state
- `backend/server/src/manga/routes.rs` — Route handler pattern to follow
- `backend/shared/migrations/` — Existing migration files (10 migrations, timestamp-prefixed)

## Code Context

### Reusable Assets

- **sqlx migration pipeline** already wired in `Database::new()` at `backend/shared/src/database.rs:62`. Add new migration files to `backend/shared/migrations/` with `YYYYMMDDHHMMSS_description.sql` naming.
- **Route module pattern** established: create `server/src/track/mod.rs` re-exporting `routes`, `server/src/track/routes.rs` with `pub fn routes() -> Router<State>`. Wire into `build_router()` in `app.rs`.
- **State pattern**: add `TrackState` to `server/src/state.rs` with `FromRef<State>`. Holds PKCE state map.
- **AppError** in `server/src/error.rs` — add new variants for OAuth errors if needed, they auto-convert via `Into<anyhow::Error>`.
- **reqwest 0.12** already a dep (optional behind `ffi` feature in server). Can be used for OAuth HTTP calls.
- **image 0.25** already a dep in shared crate. QR code PNG rendering uses it.
- **Existing newtype pattern** (`SourceId`, `MangaId`) — new tracking types follow same `#[derive]` and serde style.

### Integration Points

- Server `build_router()` at `backend/server/src/app.rs:45` — add `.nest("/track", track::routes())`
- Server `build_state()` at `backend/server/src/app.rs:93` — construct `TrackState` and add to `State` struct
- `State` struct at `backend/server/src/state.rs:43` — add `track_state: TrackState` field
- Shared crate `src/lib.rs` — add `pub mod track` (feature-gated behind `all` like database/usecases)
- Shared crate `Cargo.toml` — no new deps needed for types-only `track` module
- Server crate `Cargo.toml` — needs `qrcode` crate + `image` (for QR PNG rendering)

### Patterns to Follow

- **Use case pattern**: one `pub async fn` per operation in `shared::usecases/`. For Phase 1, auth operations live in `server::track` (not shared), since they're server-protocol-layer concerns.
- **Error handling**: `AppError` enum implements `IntoResponse`. New OAuth errors as `AppError::OAuth(String)` or similar.
- **Migration files**: `sqlx::migrate!()` discovers files by timestamp prefix. One migration file per logical change.

## Prior Decisions

### Project-Level (from PROJECT.md)

| Decision | Value |
|----------|-------|
| Rust backend for tracker API | Uses existing axum pattern |
| SQLite for credential storage | Existing DB infra |
| QR-code auth for e-ink | Phone-as-browser standard pattern |
| Public OAuth client IDs | AniList `client_id=16329`, MAL `client_id=c46c9e24640a64dad5be5ca7a1a53a0f` |
| No automatic background sync | e-ink battery constraints |
| Mihon API contracts as reference | Well-established status/score mappings |

### From Prior Phases

(None — this is the first phase)

## Decisions

### DB Schema Design

- **FK strategy**: Composite `(source_id TEXT, manga_id TEXT)` matching existing `MangaId` pattern. No auto-increment rowid FK. Columns mirror the domain model, not internal IDs.
- **Score storage**: Normalize all scores to 0-10 INTEGER. AniList /10 conversion in Rust. MAL 0-10 direct. `score` column is INTEGER.
- **TrackStatus enum**: Mihon-compatible — `CURRENTLY_READING`, `COMPLETED`, `ON_HOLD`, `DROPPED`, `PLAN_TO_READ`, `REPEATING`. Stored as TEXT in DB.
- **SyncDirection enum**: `Push`, `Pull`, `TwoWay` variants. TwoWay = call push then pull sequentially.
- **Migration organization**: One migration file creating both `track` and `tracker_auth` tables with indices on `(tracker_id, manga_id)`, `(state)` for PKCE lookups. Named `YYYYMMDDHHMMSS_create_tracking_tables.sql`.
- **`track` table columns**: id INTEGER PRIMARY KEY AUTOINCREMENT, manga_source_id TEXT NOT NULL, manga_id TEXT NOT NULL, tracker_id TEXT NOT NULL, remote_id TEXT, library_id TEXT, title TEXT, last_chapter_read INTEGER DEFAULT 0, total_chapters INTEGER, status TEXT, score INTEGER, start_date TEXT, finish_date TEXT, tracking_url TEXT, private INTEGER DEFAULT 0, updated_at TEXT NOT NULL DEFAULT (datetime('now')), UNIQUE(manga_source_id, manga_id, tracker_id)
- **`tracker_auth` table columns**: id INTEGER PRIMARY KEY AUTOINCREMENT, tracker_id TEXT NOT NULL UNIQUE, token_json TEXT NOT NULL, expires_at TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now'))

### OAuth Module Architecture

- **Trait-based**: `TrackerAuth` trait with `generate_auth_url()`, `exchange_code()`, `refresh_token()` methods.
- **Code location**: Trait + implementations in `server::track::auth`. Types only in `shared::track::types`.
- **Service implementations**: `AniListAuth` and `MalAuth` structs implementing `TrackerAuth`.
- **Why trait**: Phase 2 full API clients benefit from same abstraction. Avoids refactor churn.
- **No shared::track::client**: HTTP client code stays in server crate. Shared crate only has types.

### PKCE State Management

- **Verdict**: In-memory HashMap behind `Arc<Mutex<HashMap<String, PkceSession>>>`.
- **Key**: Random 32-byte hex `state` string (returned in auth URL).
- **Value**: `{ code_verifier: String, tracker_id: String, created_at: Instant }`.
- **Cleanup**: Entries expire after 15 minutes (checked lazily on /auth). Prevents memory leak from abandoned flows.
- **Location**: `TrackState` struct in server state. Added to `State` via `FromRef`.
- **Why not persistent**: OAuth flow is short-lived (minutes). Server restart during auth is edge case. Simplifies schema.

### QR Code Delivery Format

- **Two-step delivery**: `POST /track/{tracker}/auth-url` returns JSON `{ url: "...", qr_id: "..." }`. `GET /track/qr/{qr_id}` returns `image/png` bytes.
- **Why separate**: Allows caching on Lua side. Smaller first response. Cleaner API boundary.
- **QR lifetime**: QR image tied to PKCE session. Same 15-minute TTL. QR ID = state string.
- **QR library**: `qrcode` crate for QR encoding + `image` crate for PNG rendering at 300x300px (greyscale for e-ink).
- **Fallback**: URL is always returned alongside for text-based display (QR-04).

## Deferred Ideas

- **OAuth via embedded WebView**: Deferred by PROJECT.md — not possible on e-ink. QR auth is the correct solution.
- **Multiple tracker accounts per service**: Out of scope for v1. `tracker_auth` has `UNIQUE(tracker_id)` — one account per service.
- **Persistent PKCE state**: If in-memory proves unreliable (server restarts mid-flow on Android), can migrate to DB.

## Folded Todos

(None — no matching todos found)

## Reviewed Todos

(None)
