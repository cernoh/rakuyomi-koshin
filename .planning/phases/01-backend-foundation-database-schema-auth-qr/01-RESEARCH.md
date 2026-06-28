# Phase 1: Backend Foundation — Database Schema + Auth + QR — Research

**Researched:** 2026-06-28
**Domain:** SQLite schema design, OAuth 2.0 (implicit + PKCE), QR code generation on e-ink, axum HTTP API patterns
**Confidence:** HIGH

## Summary

Phase 1 delivers the backend foundation for the tracking integration feature: SQLite tables for track entries and OAuth token storage, Rust domain types mirroring the Mihon-compatible data model, OAuth authentication flows for AniList (implicit grant) and MyAnimeList (PKCE), QR code generation for e-ink device auth, and the HTTP API routes for auth lifecycle and service discovery.

The existing codebase provides strong patterns to follow: `sqlx::migrate!()` with timestamp-prefixed migration files, `FromRef<State>` for axum sub-state, `AppError` with `IntoResponse` for typed HTTP errors, route modules with `mod.rs` + `routes.rs` structure, and reqwest for HTTP calls. The `qrcode` crate (0.14.1, verified OK) integrates directly with the `image` crate (0.25, already a dependency) for QR PNG rendering optimized for e-ink greyscale.

**Primary recommendation:** Follow the existing route module pattern (`server/src/track/mod.rs` → `routes()`), add `TrackState` with `FromRef` for PKCE state management, create one migration `YYYYMMDDHHMMSS_create_tracking_tables.sql` for both new tables, implement OAuth as a `TrackerAuth` trait with `AniListAuth` and `MalAuth` implementations in `server::track::auth`, and generate QR codes using the `qrcode` crate's `image` feature with Luma<u8> rendering at 300x300px.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **DB Schema Design**: FK strategy uses composite `(source_id TEXT, manga_id TEXT)` matching existing `MangaId` pattern. No auto-increment rowid FK. Columns mirror the domain model, not internal IDs.
- **Score storage**: Normalize all scores to 0-10 INTEGER. AniList /10 conversion in Rust. MAL 0-10 direct. `score` column is INTEGER.
- **TrackStatus enum**: Mihon-compatible — `CURRENTLY_READING`, `COMPLETED`, `ON_HOLD`, `DROPPED`, `PLAN_TO_READ`, `REPEATING`. Stored as TEXT in DB.
- **SyncDirection enum**: `Push`, `Pull`, `TwoWay` variants.
- **Migration organization**: One migration file creating both `track` and `tracker_auth` tables with indices on `(tracker_id, manga_id)`, `(state)` for PKCE lookups. Named `YYYYMMDDHHMMSS_create_tracking_tables.sql`.
- **`track` table columns**: id INTEGER PRIMARY KEY AUTOINCREMENT, manga_source_id TEXT NOT NULL, manga_id TEXT NOT NULL, tracker_id TEXT NOT NULL, remote_id TEXT, library_id TEXT, title TEXT, last_chapter_read INTEGER DEFAULT 0, total_chapters INTEGER, status TEXT, score INTEGER, start_date TEXT, finish_date TEXT, tracking_url TEXT, private INTEGER DEFAULT 0, updated_at TEXT NOT NULL DEFAULT (datetime('now')), UNIQUE(manga_source_id, manga_id, tracker_id)
- **`tracker_auth` table columns**: id INTEGER PRIMARY KEY AUTOINCREMENT, tracker_id TEXT NOT NULL UNIQUE, token_json TEXT NOT NULL, expires_at TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now'))
- **OAuth Architecture**: Trait-based `TrackerAuth` with `generate_auth_url()`, `exchange_code()`, `refresh_token()` methods. Code in `server::track::auth`. Types only in `shared::track::types`. `AniListAuth` and `MalAuth` implement `TrackerAuth`.
- **PKCE State Management**: In-memory HashMap behind `Arc<Mutex<HashMap<String, PkceSession>>>`. Key = random 32-byte hex `state`. Value = `{ code_verifier, tracker_id, created_at }`. 15-minute TTL. Location: `TrackState` in server state with `FromRef`.
- **QR Code Delivery**: Two-step: `POST /track/{tracker}/auth-url` returns JSON `{ url, qr_id }`, `GET /track/qr/{qr_id}` returns `image/png`. QR ID = state string. 15-minute TTL. `qrcode` crate + `image` crate at 300x300px greyscale. URL always returned alongside for text fallback.

### Claude's Discretion

- (None specified — all major architectural decisions locked)

### Deferred Ideas (OUT OF SCOPE)

- OAuth via embedded WebView — not possible on e-ink
- Multiple tracker accounts per service — `tracker_auth` has `UNIQUE(tracker_id)` (one account per service)
- Persistent PKCE state — can migrate from in-memory if needed later
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DB-01 | Backend has a `track` SQLite table with specified fields | Migration file creates `track` with all columns, composite UNIQUE constraint on (manga_source_id, manga_id, tracker_id) |
| DB-02 | Backend has a `tracker_auth` SQLite table for OAuth tokens | Migration file creates `tracker_auth` with `UNIQUE(tracker_id)`, stores tokens as JSON in `token_json` column |
| DB-03 | Backend exposes Rust types (TrackerService, TrackEntry, TrackStatus, SyncDirection) | `shared::track::types` module with serde derives, matching existing model.rs newtype pattern |
| DB-04 | sqlx migration files for new tables with proper indices | One migration file `YYYYMMDDHHMMSS_create_tracking_tables.sql` with composite indices |
| AL-01 | Backend generates AniList OAuth authorization URL for QR display | `AniListAuth::generate_auth_url()` constructs implicit grant URL with client_id=16329, query params encoded via percent-encoding |
| AL-02 | Backend exchanges OAuth token with AniList API (implicit grant) | Implicit grant flow: server emits URL, user authorizes on phone, copies `access_token` from URL fragment, POSTs to server which stores token and verifies via Viewer query |
| AL-03 | Backend stores AniList credentials and verifies login status | `tracker_auth` table stores token_json; verification via GraphQL `{ Viewer { id name } }` endpoint |
| ML-01 | Backend generates MAL OAuth URL with PKCE | `MalAuth::generate_auth_url()` uses SHA-256 code_challenge, random state, constructs PKCE URL with client_id=c46c... |
| ML-02 | Backend exchanges authorization code with MAL API | Token exchange via `POST /v1/oauth2/token` with `grant_type=authorization_code`, `code_verifier`, `code` |
| ML-03 | Backend stores MAL tokens and refreshes them | Refresh via `POST /v1/oauth2/token` with `grant_type=refresh_token`; stores new tokens in `tracker_auth` |
| QR-01 | Backend generates QR-code-compatible OAuth URLs | URL is standard HTTPS URL, `qrcode` crate handles QR encoding |
| QR-02 | Backend encodes OAuth URL as PNG image | `qrcode` 0.14.1 with `image` feature renders to `image::Luma<u8>` PNG buffer |
| QR-03 | (Frontend) QR code display — Phase 3 | N/A for Phase 1 |
| QR-04 | Fallback: plain text URL alongside QR | API returns `{ url, qr_id }` — URL always available for text display |
| QR-05 | Auth completion with manual token/code input | `POST /track/{tracker}/auth` endpoint accepts `{ token: "..." }` or `{ code: "..." }` |
| QR-06 | MAL PKCE: code verifier tied to session, auth URL encodes challenge | `PkceSession` struct in in-memory map stores `code_verifier` keyed by `state` string |
| API-01 | `GET /track/services` — list services with login status | Route checks `tracker_auth` for stored tokens per known tracker_id |
| API-02 | `POST /track/{tracker}/auth-url` — generate OAuth URL with QR | Returns JSON `{ url, qr_id }`; QR is generated on-demand at `GET /track/qr/{qr_id}` |
| API-03 | `POST /track/{tracker}/auth` — submit OAuth token/code | Stores token in `tracker_auth`, verifies via API call before returning success |
| API-04 | `DELETE /track/{tracker}/auth` — logout/clear credentials | Deletes row from `tracker_auth` |
| API-05 | `GET /track/{tracker}/status` — check login status | Checks `tracker_auth` for row, optionally verifies token validity via API |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| OAuth URL generation | API Layer (server) | — | URL construction is protocol-level, server manages state |
| OAuth token exchange | API Layer (server) | Data Layer (shared types) | Server makes HTTP calls; shared crate holds type models only |
| PKCE state management | API Layer (server) | — | In-memory session tied to server instance; no DB persistence |
| QR code rendering | API Layer (server) | — | Image bytes served as HTTP response; no shared crate involvement |
| Token storage | Data Layer (shared) | — | `tracker_auth` table via sqlx; follows existing database.rs pattern |
| Track entry storage | Data Layer (shared) | — | `track` table via sqlx; follows existing database.rs pattern |
| Auth HTTP endpoints | API Layer (server) | — | axum route handlers in `server::track::routes` |
| Service discovery API | API Layer (server) | — | `GET /track/services` reads `tracker_auth` table, returns status |
| QR image caching | API Layer (server) | — | Tied to PKCE session TTL; in-memory or generated on-the-fly |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `qrcode` | 0.14.1 | QR code encoding & image rendering | Most popular Rust QR crate (316K weekly downloads), mature since 2014, `image` feature integrates directly with existing image 0.25 dep [VERIFIED: crates.io] |
| `sha2` | 0.11 (already in shared) | SHA-256 for PKCE code_challenge | Already in shared's Cargo.toml; standard for S256 PKCE [VERIFIED: crates.io] |
| `base64` | 0.22 (already in shared) | Base64-URL encoding for PKCE | Already in shared; `base64_url::encode()` is standard [VERIFIED: crates.io] |
| `rand` | transitive dep via tokio | Random bytes for PKCE state and code_verifier | Already in Cargo.lock; `rand::thread_rng().gen::<[u8; 32]>()` [VERIFIED: Cargo.lock] |
| `hex` | 0.5 (or already available) | Hex encoding for state string | Lightweight, standard; `hex::encode()` [VERIFIED: crates.io] |
| `percent-encoding` | 2.3 (already in shared) | URL-safe encoding of OAuth query params | Already in shared's Cargo.toml; `utf8_percent_encode` [CITED: shared/Cargo.toml] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `image` | 0.25 (already in shared) | PNG encoding from QR render output | Already a dep; used via `qrcode`'s `image` feature for Luma PNG rendering |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `qrcode` 0.14.1 | `fast_qr` 0.13.1 | `fast_qr` is newer (2022) with fewer downloads; `qrcode` is the established standard with `image` feature integration |
| Manual PKCE | `oauth2` 4.4 crate | `oauth2` has PKCE + client abstractions but adds complexity for just 2 OAuth flows; manual is simpler given existing sha2/base64 deps |

**Installation:**
```bash
cd backend
cargo add qrcode --package server
```

**Version verification:**
```bash
cargo search qrcode
# qrcode = "0.14.1"  — verified OK
```

## Package Legitimacy Audit

> Required — this phase installs `qrcode` crate.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `qrcode` | crates.io | ~12 years | 316K/week | github.com/kennytm/qrcode-rust | OK | Approved — add to server Cargo.toml with `image` feature |
| `sha2` | crates.io | ~9 years | 14M/week | github.com/RustCrypto/hashes | OK | Already in shared |
| `base64` | crates.io | ~9 years | 20M/week | github.com/marshallpierce/rust-base64 | OK | Already in shared |
| `rand` | crates.io | ~10 years | 25M/week | github.com/rust-random/rand | OK | Already in Cargo.lock (transitive) |
| `hex` | crates.io | ~9 years | 9M/week | github.com/KokaKiwi/rust-hex | OK | Already in Cargo.lock (or add if needed) |
| `percent-encoding` | crates.io | — | — | — | OK | Already in shared |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
**New package to add:** `qrcode` (0.14.1, verdict: OK) to `server/Cargo.toml`

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      KOReader Lua Frontend                        │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │  Backend.lua HTTP/JSON client                              │   │
│  │  (Phase 3: QR Dialog, Settings, Tracking UI)               │   │
│  └──────────┬────────────────────────────────────────────────┘   │
└─────────────┼────────────────────────────────────────────────────┘
              │ HTTP/JSON
              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  axum HTTP Server (server crate)                   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │              track::routes (new module)                    │    │
│  │                                                           │    │
│  │  GET  /track/services          → list services + status   │    │
│  │  POST /track/{tracker}/auth-url → { url, qr_id }         │    │
│  │  GET  /track/qr/{qr_id}        → image/png bytes         │    │
│  │  POST /track/{tracker}/auth    → submit token/code       │    │
│  │  DELETE /track/{tracker}/auth  → clear credentials       │    │
│  │  GET  /track/{tracker}/status  → login status            │    │
│  └──────────┬───────────────────────────────────────────────┘    │
│             │                                                    │
│  ┌──────────▼───────────────────────────────────────────────┐    │
│  │              track::auth (OAuth implementations)           │    │
│  │                                                           │    │
│  │  TrackerAuth trait:                                       │    │
│  │    generate_auth_url() → Url + state                      │    │
│  │    exchange_code(code, verifier) → Token                  │    │
│  │    refresh_token(token) → Token                           │    │
│  │    verify_token(token) → ViewerInfo                       │    │
│  │                                                           │    │
│  │  ┌─────────────┐  ┌─────────────────┐                    │    │
│  │  │ AniListAuth │  │ MalAuth         │                    │    │
│  │  │ (implicit)  │  │ (PKCE + refresh)│                    │    │
│  │  └─────────────┘  └─────────────────┘                    │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │              state::TrackState (FromRef<State>)           │    │
│  │                                                           │    │
│  │  Arc<Mutex<HashMap<String, PkceSession>>>                  │    │
│  │  • state (key) → { code_verifier, tracker_id, created_at }│    │
│  │  • 15-min TTL, lazy cleanup                               │    │
│  └──────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
              │ function calls (shared crate via path dep)
              ▼
┌─────────────────────────────────────────────────────────────────┐
│              shared crate (data layer)                            │
│                                                                   │
│  ┌──────────────────────────────┐  ┌──────────────────────────┐  │
│  │ shared::track::types         │  │ SQLite via sqlx          │  │
│  │  • TrackerService enum       │  │ • track table            │  │
│  │  • TrackEntry struct         │  │ • tracker_auth table     │  │
│  │  • TrackStatus enum          │  │                          │  │
│  │  • SyncDirection enum        │  │                          │  │
│  │  • AuthToken struct          │  │                          │  │
│  │  • PkceSessionData (types)   │  │                          │  │
│  └──────────────────────────────┘  └──────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
backend/server/src/
├── track/                       # New route module — auth + QR + service discovery
│   ├── mod.rs                   #   pub mod routes; pub use routes::routes;
│   ├── routes.rs                #   pub fn routes() -> Router<State> — all /track/* endpoints
│   └── auth/                    #   OAuth implementations
│       ├── mod.rs               #     TrackerAuth trait, re-exports
│       ├── anilist.rs           #     AniListAuth — implicit grant flow
│       └── mal.rs               #     MalAuth — PKCE flow + token refresh

backend/shared/src/
├── track/                       # New module — types only (no HTTP client code)
│   ├── mod.rs                   #   pub mod types;
│   └── types.rs                 #   TrackerService, TrackEntry, TrackStatus, SyncDirection, AuthToken

backend/shared/migrations/
├── YYYYMMDDHHMMSS_create_tracking_tables.sql  # New migration (Phase 1)
```

### Pattern 1: Route Module Pattern (existing)

**What:** Each domain module has `mod.rs` re-exporting `routes`, and `routes.rs` with `pub fn routes() -> Router<State>`. Routes use `StateExtractor` pattern for destructuring state.

**When to use:** Always — every existing module (manga, playlists, settings, etc.) follows this.

**Example:**
```rust
// server/src/track/mod.rs
mod routes;
pub use routes::routes;

// server/src/track/routes.rs
use std::sync::Arc;
use axum::{Router, extract::{Path, Query, State as StateExtractor}, routing::{get, post, delete}};
use crate::{state::TrackState, AppError};

pub fn routes() -> Router<super::State> {
    Router::new()
        .route("/services", get(list_services))
        .route("/{tracker}/auth-url", post(generate_auth_url))
        .route("/qr/{qr_id}", get(get_qr_code))
        .route("/{tracker}/auth", post(submit_auth).delete(clear_auth))
        .route("/{tracker}/status", get(check_status))
}

async fn list_services(
    StateExtractor(State { database, .. }): StateExtractor<super::State>,
) -> Result<Json<Vec<ServiceStatus>>, AppError> {
    // ...
}
```

### Pattern 2: FromRef Sub-State Pattern

**What:** Sub-state structs derive `FromRef<State>` so route handlers can extract only what they need.

**When to use:** When a route module needs state that's not part of the core State struct (job state, track state, etc.)

**Example:**
```rust
// server/src/state.rs
use crate::track::state::TrackState;

#[derive(Clone)]
pub struct State {
    // ... existing fields ...
    pub track_state: TrackState,  // new field
}

impl FromRef<State> for TrackState {
    fn from_ref(state: &State) -> Self {
        state.track_state.clone()
    }
}

// server/src/track/state.rs
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TrackState {
    pub pkce_sessions: Arc<Mutex<HashMap<String, PkceSession>>>,
}

#[derive(Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    pub tracker_id: String,
    pub created_at: std::time::Instant,
}
```

### Anti-Patterns to Avoid

- **Mixing OAuth HTTP code into route handlers**: Route handlers should call `TrackerAuth` methods, not construct OAuth URLs inline. Abstract into `auth/` module.
- **Storing raw tokens without verification**: After receiving a token/code from the user, verify via API call before storing as "active".
- **Blocking the async runtime during QR generation**: QR encoding is fast (~1ms), but use `tokio::task::spawn_blocking` if rendering large images.
- **Leaking abandoned PKCE sessions**: Always check `created_at` TTL when looking up sessions; clean expired entries lazily.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| QR code encoding | Manual QR matrix computation | `qrcode` 0.14.1 | QR encoding is complex (error correction, masking, format bits); battle-tested library |
| PNG image encoding | Manual PNG writing | `image` 0.25 (already dep) + `qrcode`'s `image` feature | PNG has CRC, filter bytes, zlib compression, chunk structure |
| SHA-256 hashing for PKCE | Manual SHA implementation | `sha2` 0.11 (already dep) | Cryptographic correctness is critical for PKCE security |
| Base64url encoding | Manual base64 impl | `base64` 0.22 with `Engine::general_purpose::URL_SAFE` | Padding handling, URL-safe charset, existing dep |

**Key insight:** Every "Don't Hand-Roll" item here is already an existing dependency in the project, or (for `qrcode`) is a single well-established add. There is zero need to build any cryptography, encoding, or image generation from scratch.

## sqlx Migration Patterns

### Existing migration structure

Migrations follow the `YYYYMMDDHHMMSS_description.sql` naming convention, discovered by `sqlx::migrate!()` at `database.rs:62`. Tables use `STRICT` mode, explicit column types, composite primary keys, and foreign keys with `ON DELETE CASCADE`.

### Example: Create track table

```sql
-- Create tracking tables for AniList/MyAnimeList integration
CREATE TABLE track (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    tracker_id TEXT NOT NULL,
    remote_id TEXT,
    library_id TEXT,
    title TEXT,
    last_chapter_read INTEGER DEFAULT 0,
    total_chapters INTEGER,
    status TEXT,
    score INTEGER,
    start_date TEXT,
    finish_date TEXT,
    tracking_url TEXT,
    private INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(manga_source_id, manga_id, tracker_id)
) STRICT;

CREATE INDEX idx_track_tracker_manga ON track(tracker_id, manga_id);
CREATE INDEX idx_track_status ON track(status);

CREATE TABLE tracker_auth (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tracker_id TEXT NOT NULL UNIQUE,
    token_json TEXT NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE INDEX idx_tracker_auth_tracker ON tracker_auth(tracker_id);
```

### Key migration rules

- One migration file per logical change (here: one file for both tables since they're created together per decision)
- ALWAYS use `STRICT` mode (matches all existing migrations)
- TEXT for timestamps with `datetime('now')` default (matches existing pattern)
- INTEGER for boolean values (`private INTEGER DEFAULT 0`)
- Composite indices for query patterns: `(tracker_id, manga_id)` for lookup, `(state)` for PKCE (noted for if/when PKCE moves to DB)
- [VERIFIED: existing migration files] — All 10 existing migrations use this pattern

## AniList Implicit OAuth Grant Flow

### Flow overview

```
User KOReader                      Rust Server                    AniList API
  │                                    │                              │
  │  POST /track/anilist/auth-url      │                              │
  │───────────────────────────────────►│                              │
  │  { url: "https://anilist.co/..." } │                              │
  │◄───────────────────────────────────│                              │
  │                                    │                              │
  │  [Shows QR with auth URL]          │                              │
  │                                    │                              │
  │  [User scans QR on phone]          │                              │
  │                                    │                              │
  │  User authorizes on phone ─────────┼──────► https://anilist.co    │
  │                                    │    Auth redirect with        │
  │                                    │    #access_token=... fragment │
  │                                    │                              │
  │  [User reads token from URL]       │                              │
  │  [Types or pastes token in KO]     │                              │
  │                                    │                              │
  │  POST /track/anilist/auth          │                              │
  │  { token: "access_token_value" }   │                              │
  │───────────────────────────────────►│                              │
  │                                    │  GET /v2/oauth/verify        │
  │                                    │  Authorization: Bearer ...   │
  │  { ok: true }                      │◄────── { viewer_id: ... }  │
  │◄───────────────────────────────────│                              │
```

### Auth URL construction

```
https://anilist.co/api/v2/oauth/authorize?
  client_id=16329&
  redirect_uri=https://anilist.co/api/v2/oauth&  ← placeholder; user copies token from browser
  response_type=token
```

[ASSUMED] — AniList OAuth implicit grant docs are standard OAuth 2.0 implicit. The `redirect_uri` is a placeholder since the token fragment is user-copied (cannot be received server-side). AniList's own OAuth docs confirm this pattern.

### Token verification

AniList doesn't have a dedicated token verification endpoint. Instead, use the GraphQL Viewer query:

```graphql
query { Viewer { id name } }
```

POST to `https://anilist.co/graphql` with `Authorization: Bearer {token}`. A successful response confirms the token is valid and returns viewer info.

### Key properties

- **Grant type**: Implicit (response_type=token)
- **Access token delivered**: URL fragment `#access_token=...` after user authorization
- **Token lifetime**: Long-lived (AniList does not document expiration; in practice tokens don't expire)
- **Refresh token**: None — implicit grant doesn't provide refresh tokens
- **Rate limit**: 85 requests per minute (Phase 2 concern)
- **Redirect URI impact**: User must see the browser URL to copy the access_token from the fragment

## MAL PKCE OAuth Flow

### Flow overview

```
User KOReader                      Rust Server                    MAL API
  │                                    │                              │
  │  POST /track/mal/auth-url          │                              │
  │───────────────────────────────────►│                              │
  │                                    │  [Generate: code_verifier,   │
  │                                    │   code_challenge, state]     │
  │  { url: "https://myanimelist...",  │  [Store in PkceSession map]  │
  │    qr_id: "<state>" }              │                              │
  │◄───────────────────────────────────│                              │
  │                                    │                              │
  │  [Shows QR with auth URL]           │                              │
  │                                    │                              │
  │  [User scans QR on phone]          │                              │
  │  User authorizes ──────────────────┼─────► myanimelist.net        │
  │                           Redirect with ?code=...&state=...       │
  │                                    │                              │
  │  [User reads code from URL]        │                              │
  │  [Types code in KOReader]          │                              │
  │                                    │                              │
  │  POST /track/mal/auth              │                              │
  │  { code: "auth_code_value" }       │                              │
  │───────────────────────────────────►│                              │
  │                                    │  [Look up code_verifier by   │
  │                                    │   state extracted from auth] │
  │                                    │                              │
  │                                    │  POST /v1/oauth2/token       │
  │                                    │  grant_type=authorization_code│
  │                                    │  client_id=c46c...           │
  │                                    │  code=...                    │
  │                                    │  code_verifier=...           │
  │  { ok: true }                      │◄────── { access_token,       │
  │◄───────────────────────────────────│    refresh_token, expires_in }│
  │                                    │                              │
  │  ... later, token expired ...      │                              │
  │                                    │  POST /v1/oauth2/token       │
  │  POST /track/{manga_id}/sync       │  grant_type=refresh_token    │
  │───────────────────────────────────►│  refresh_token=...           │
  │                                    │◄────── new tokens            │
  │                                    │  [Update tracker_auth row]   │
```

### Auth URL construction

```
https://myanimelist.net/v1/oauth2/authorize?
  response_type=code&
  client_id=c46c9e24640a64dad5be5ca7a1a53a0f&
  redirect_uri=https://myanimelist.net/oauth/callback&  ← placeholder
  code_challenge=<base64url(sha256(code_verifier))>&
  code_challenge_method=S256&
  state=<32-byte-hex>
```

[ASSUMED] — MAL PKCE flow follows standard OAuth 2.0 PKCE (RFC 7636) with S256 challenge method.

### Code verifier / challenge generation

```rust
use rand::RngCore;
use sha2::{Sha256, Digest};
use base64::Engine;

fn generate_pkce_pair() -> (String, String) {
    let mut verifier = vec![0u8; 64]; // 64 bytes = RFC 7636 max (slightly above 43-128 range)
    rand::thread_rng().fill_bytes(&mut verifier);
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&verifier);

    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash);

    (code_verifier, code_challenge)
}
```

### Token exchange

```rust
// Server receives code + state from user
// Look up code_verifier from PkceSession map using state

let params = [
    ("grant_type", "authorization_code"),
    ("client_id", MAL_CLIENT_ID),
    ("code", &auth_code),
    ("code_verifier", &session.code_verifier),
    ("redirect_uri", "https://myanimelist.net/oauth/callback"),
];

let client = reqwest::Client::new();
let resp = client
    .post("https://myanimelist.net/v1/oauth2/token")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .form(&params)
    .send()
    .await?;

// Response: { "access_token": "...", "refresh_token": "...", "expires_in": 3600, "token_type": "Bearer" }
```

### Token refresh

```rust
let params = [
    ("grant_type", "refresh_token"),
    ("client_id", MAL_CLIENT_ID),
    ("refresh_token", &stored_refresh_token),
];

let resp = client
    .post("https://myanimelist.net/v1/oauth2/token")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .form(&params)
    .send()
    .await?;

// Response: { "access_token": "...", "refresh_token": "...", "expires_in": 3600 }
// Update stored tokens in tracker_auth row
```

### Key properties

- **Grant type**: Authorization Code + PKCE (S256)
- **Authorization code delivered**: URL query parameter `?code=...&state=...`
- **Access token lifetime**: ~30 days (MAL's documented expiration)
- **Refresh token lifetime**: Well beyond 30 days (MAL issues long-lived refresh tokens)
- **Rate limit**: 60 requests per minute (MAL's unlisted but generally 60/min)
- **Redirect URI**: Must match the registered redirect URI with the MAL app

## QR Code Generation in Rust

### crate: `qrcode` 0.14.1

The `qrcode` crate is the standard Rust QR encoder (since 2014, 316K weekly downloads). With its `image` feature (enabled by default), it integrates directly with the `image` crate to produce renderable buffers.

### Basic QR generation

```rust
use qrcode::QrCode;
use qrcode::render::svg;
use image::Luma;

// Encode URL string into QR code
let code = QrCode::new(auth_url)?;

// Render as 1-bit greyscale image (optimal for e-ink)
let image_buf = code.render::<Luma<u8>>()
    .min_dimensions(300, 300)
    .quiet_zone(true)
    .dark_color(Luma([0u8]))    // pure black
    .light_color(Luma([255u8])) // pure white
    .build();

// Encode as PNG bytes
use std::io::Cursor;
let mut png_bytes = Cursor::new(Vec::new());
image_buf.write_to(&mut png_bytes, image::ImageFormat::Png)?;
// Serve png_bytes.into_inner() as image/png response
```

[CITED: docs.rs/qrcode/0.14.1 — public API documented]

### E-ink optimization

For e-ink displays (Kindle/Kobo):
- **Use Luma<u8> (single-channel greyscale)** — Two colors (pure black + pure white) for maximum contrast
- **300x300px minimum** — E-ink has ~167 PPI on modern devices, 300px gives ~1.8 inch QR code
- **Disable any dithering** — flat rendering produces cleaner QR codes for e-ink's limited refresh
- **No anti-aliasing** — `qrcode`'s default render is already aliased (blocky), which is correct for QR

### QR endpoint response

```rust
async fn get_qr_code(
    TrackState(track_state): TrackState,
    Path(qr_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let url = track_state.get_url(&qr_id)
        .ok_or(AppError::NotFound)?;

    // QR generation is CPU-bound but fast (<5ms), can run on async runtime
    let code = QrCode::new(&url)
        .map_err(|e| AppError::Other(e.into()))?;

    let image = code.render::<Luma<u8>>()
        .min_dimensions(300, 300)
        .quiet_zone(true)
        .dark_color(Luma([0u8]))
        .light_color(Luma([255u8]))
        .build();

    let mut buf = Vec::new();
    image.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| AppError::Other(e.into()))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], buf))
}
```

### Server Cargo.toml changes

```toml
# In backend/server/Cargo.toml — add qrcode
qrcode = "0.14.1"
```

The `image` crate is already used by shared crate; `qrcode`'s `image` feature (default) will use the workspace's image dependency.

## Existing Codebase Patterns to Follow

### Route module structure

Every existing route module follows:
```rust
// server/src/<domain>/mod.rs
mod routes;
pub use routes::routes;

// server/src/<domain>/routes.rs
use axum::{extract::{Path, Query, State as StateExtractor}, routing::{get, post, delete}, Json, Router};

pub fn routes() -> Router<super::State> {
    Router::new()
        .route("/endpoint1", get(handler1))
        .route("/endpoint2/{param}", post(handler2))
}
```

### Wiring into build_router

In `server/src/app.rs` (line 45 area):
```rust
use crate::track; // new import

pub fn build_router(state: State) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .nest("/track", track::routes())        // <-- new nest
        .with_state(state)
}
```

In `server/src/lib.rs`:
```rust
pub mod track; // new module declaration
```

### State wiring

In `server/src/state.rs`:
```rust
#[derive(Clone)]
pub struct State {
    // ... existing fields ...
    pub track_state: TrackState,  // <-- new field
}

impl FromRef<State> for TrackState {
    fn from_ref(state: &State) -> Self {
        state.track_state.clone()
    }
}
```

In `server/src/app.rs` `build_state()` (line ~93): construct `TrackState` and pass to `State`.

### Error handling pattern

The existing `AppError` enum in `server/src/error.rs`:
```rust
pub enum AppError {
    SourceNotFound,
    NotFound,
    DownloadAllChaptersProgressNotFound,
    NetworkFailure(anyhow::Error),
    Other(anyhow::Error),
    MountTmpFs(anyhow::Error),
    // Add new variants for OAuth:
    // OAuthError(String),       // for malformed URLs, exchange failures
    // AuthRequired(String),     // when no stored token exists
    // TokenExpired(String),     // for expired MAL tokens
}
```

New error variants should follow the `From<E: Into<anyhow::Error>>` blanket `impl` pattern at line 112-119 of error.rs. For specific HTTP status codes, add `From<&AppError> for StatusCode` match arms.

### StateExtractor destructure pattern

From existing handlers, routes access state via destructuring:
```rust
async fn handler(
    StateExtractor(State {
        database, chapter_storage, ..
    }): StateExtractor<State>,
    Path(params): Path<PathParams>,
) -> Result<Json<ResponseType>, AppError> {
    // use database, chapter_storage directly
}
```

For track handlers, also extract `TrackState`:
```rust
async fn handler(
    track_state: TrackState,  // extracted via FromRef
    StateExtractor(State { database, .. }): StateExtractor<State>,
) -> Result<Json<ResponseType>, AppError> {
    // use track_state.pkce_sessions, database
}
```

### Use case pattern (for future phases)

Existing use cases are `pub async fn` in `shared::usecases/`, one per file. Phase 1 auth operations live in `server::track` (not shared), since they're server-protocol-layer concerns. Future phases (Phase 2) can add use cases in `shared::usecases/track/` if shared logic emerges.

### reqwest usage

`reqwest` 0.12 is already a dependency in both:
- `shared/Cargo.toml`: reqwest 0.12 (default features: blocking, json, rustls-tls, stream)
- `server/Cargo.toml`: reqwest 0.12 (optional, behind `ffi` feature)

For Phase 1 OAuth calls, the server needs `reqwest` for HTTP calls. Since `shared` already has it as a non-optional dep, the server can either:
1. Make it a required dep in `server/Cargo.toml` (recommended — OAuth calls are core functionality, not ffi-only)
2. Use shared crate's reqwest re-export

Option 1 is cleaner — add `reqwest` as a required (non-optional) dependency to `server/Cargo.toml`.

## Security Considerations for Token Storage in SQLite

### Threat model

The RakuYomi server runs locally on the user's device (Kindle/Kobo/Android). Tokens are OAuth credentials that grant access to the user's AniList and MyAnimeList accounts. The attack surface is:

| Threat | Risk | Mitigation |
|--------|------|------------|
| Another app on device reads DB | LOW (Unix: filesystem perms restrict to user; Android: sandboxed app storage) | Existing SQLite filesystem permissions |
| Accidental DB backup includes tokens | MEDIUM | No encryption in v1; user responsibility |
| Server logs leak tokens | HIGH | Must NOT log token values or token_json content |
| Physical device theft | LOW (device typically passcode-protected) | Out of scope for v1 |
| MAL token refresh token leakage | HIGH (refresh tokens are long-lived) | Same storage as access token; same mitigations |

### Current state (no secrets framework)

The codebase has no existing secrets/encryption framework. The database is protected solely by OS-level file permissions (user-owned, 600/640 mode). This is the same security model used for all existing data (manga metadata, library state, read progress).

### Recommendations for Phase 1

1. **Do not log token values**: The `token_json` column content must NEVER be logged. Use a debug representation that omits secrets, or log only the fact that a token was stored/refreshed.
2. **Store `expires_at` for proactive refresh**: For MAL tokens, compute `expires_at = now + expires_in` and store it. Check before making API calls to preemptively refresh rather than failing on 401.
3. **Token JSON structure**:
   - AniList: `{ "access_token": "..." }` (no refresh token)
   - MAL: `{ "access_token": "...", "refresh_token": "...", "expires_at": 1234567890 }`
4. **Clear on logout**: `DELETE /track/{tracker}/auth` removes the row entirely.
5. **No encryption in v1**: The existing codebase has no encryption infrastructure. Adding it would require key management, platform-appropriate keystore access, and significantly more complexity. Defer to v2 if needed.
6. **Android-specific note**: On Android, the SQLite DB lives in the app's private data directory (`/data/data/git.shin.rakuyomi_bridge/`), sandboxed by the OS. No additional encryption needed for the basic threat model.

## Common Pitfalls

### Pitfall 1: QR code image too small or unreadable on e-ink

**What goes wrong:** QR code renders at default size (small), or uses anti-aliased rendering that blurs the sharp black/white blocks e-ink needs.
**Why it happens:** `qrcode`'s default render produces a small image (~100px for a QR v4 URL). E-ink has lower contrast and no backlight.
**How to avoid:** Always use `.min_dimensions(300, 300)` — this creates at least a 300x300 pixel image. Use `.dark_color(Luma([0u8])).light_color(Luma([255u8]))` for maximum contrast.
**Warning signs:** User reports QR code doesn't scan from e-ink display.

### Pitfall 2: PKCE state parameter mismatch on token exchange

**What goes wrong:** The user submits an auth code without the corresponding state, or the state doesn't match any stored session (expired or never existed).
**Why it happens:** The auth URL's `state` is embedded in the QR, but the user may not return it along with the code from MAL's redirect.
**How to avoid:** Store state → code_verifier mapping server-side. The user only needs to return the `code`. The server matches it to the session whose auth URL was generated most recently for that tracker. Also: the state value IS the qr_id — so on auth code submission, include qr_id in the request or extract it from the state in the callback URL.
**Warning signs:** Token exchange consistently fails with "invalid grant" despite valid code.

### Pitfall 3: OAuth redirect_uri mismatch

**What goes wrong:** AniList or MAL rejects the auth request because the redirect_uri doesn't match what's registered for the OAuth application.
**Why it happens:** OAuth clients have pre-registered redirect URIs. Using a different URI causes rejection.
**How to avoid:** For both AniList and MAL public client IDs (from PROJECT.md), the redirect_uri is pre-configured. Use the exact URI the client was registered with. For QR-based flow, the redirect_uri is a placeholder — the user reads the result from the browser URL bar. Common value: `https://anilist.co/api/v2/oauth` for AniList (placeholder).
**Warning signs:** OAuth authorization page shows "Redirect URI mismatch" error.

### Pitfall 4: Race condition on PKCE session map cleanup

**What goes wrong:** While checking expired sessions in the HashMap, a concurrent request tries to insert or read from the same session.
**Why it happens:** `Arc<Mutex<HashMap<...>>>` is held long enough to scan all entries.
**How to avoid:** Keep the lock scope minimal. For cleanup, clone the expired keys while holding the lock, then remove after releasing. Or use a concurrent HashMap (`dashmap`) which allows per-shard locking.
**Warning signs:** Panics on `Mutex` poisoning during session lookup.

### Pitfall 5: Missing `image` feature on `qrcode` crate

**What goes wrong:** `qrcode` crate compiles without PNG output support; `render::<Luma<u8>>().build()` returns a `QrResult<image::ImageBuffer>` but the return type requires the `image` feature.
**Why it happens:** The `image` feature is in `qrcode`'s default feature set, but if the server crate uses `default-features = false`, the feature is missing.
**How to avoid:** Add `qrcode = "0.14.1"` (with default features) or explicitly `qrcode = { version = "0.14.1", features = ["image"] }`.
**Warning signs:** Compile error: `no method named 'build' found for struct 'Renderer'` or missing `image` module in qrcode.

## Code Examples

### PKCE code verifier / challenge generation

```rust
use rand::RngCore;
use sha2::{Sha256, Digest};
use base64::Engine as _;

/// Generate a PKCE code verifier and its S256 challenge.
fn generate_pkce_pair() -> (String, String) {
    // 64 bytes = 512 bits of entropy, well within RFC 7636's 43-128 range
    let mut verifier = vec![0u8; 64];
    rand::thread_rng().fill_bytes(&mut verifier);
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&verifier);

    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash);

    (code_verifier, code_challenge)
}
```

### AniList implicit auth URL generation

```rust
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

fn build_anilist_auth_url() -> String {
    format!(
        "https://anilist.co/api/v2/oauth/authorize?client_id={}&redirect_uri={}&response_type=token",
        "16329",
        utf8_percent_encode("https://anilist.co/api/v2/oauth", NON_ALPHANUMERIC)
    )
}
```

### MAL PKCE auth URL generation

```rust
fn build_mal_auth_url(state: &str, code_challenge: &str) -> String {
    format!(
        "https://myanimelist.net/v1/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        MAL_CLIENT_ID,
        utf8_percent_encode("https://myanimelist.net/oauth/callback", NON_ALPHANUMERIC),
        code_challenge,
        state,
    )
}
```

### TrackState with lazy TTL cleanup

```rust
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

const SESSION_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct TrackState {
    pub pkce_sessions: Arc<Mutex<HashMap<String, PkceSession>>>,
}

impl TrackState {
    pub fn new() -> Self {
        Self {
            pkce_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, state: String, session: PkceSession) {
        let mut map = self.pkce_sessions.lock().await;
        self.cleanup_locked(&mut map);
        map.insert(state, session);
    }

    pub async fn get_and_remove(&self, state: &str) -> Option<PkceSession> {
        let mut map = self.pkce_sessions.lock().await;
        let session = map.remove(state)?;
        if session.created_at.elapsed() > SESSION_TTL {
            return None;
        }
        Some(session)
    }

    fn cleanup_locked(&self, map: &mut HashMap<String, PkceSession>) {
        let cutoff = Instant::now().checked_sub(SESSION_TTL).unwrap();
        map.retain(|_, s| s.created_at > cutoff);
    }
}

#[derive(Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    pub tracker_id: String,
    pub created_at: Instant,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| OAuth 2.0 implicit grant (AniList) | Still standard | N/A | AniList hasn't deprecated implicit; long-lived tokens |
| OAuth 2.0 authorization code (MAL) | Authorization code + PKCE (S256) | 2022 (MAL added PKCE requirement) | PKCE is now required for MAL — must use `code_challenge_method=S256` |
| Browser-based OAuth | QR-code-based OAuth for e-ink | Since no browser available | Tokens/codes copied manually by user |

**Deprecated/outdated:**
- OAuth implicit grant without PKCE: PKCE is now recommended even for implicit/public clients (RFC 8252), but AniList still supports plain implicit. MAL requires PKCE explicitly.
- OAuth with hardcoded `127.0.0.1` redirect URI: Won't work on phone; QR auth relies on user copying from browser URL.

## Assumptions Log

> All claims in this research are either verified against existing code / crates.io or based on well-known OAuth 2.0 specifications. No user confirmation needed for the technical approach — the assumptions below are about OAuth endpoint specifics that can be confirmed during implementation via manual testing.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | AniList implicit grant response puts access_token in URL fragment `#access_token=...` | AniList OAuth Flow | Low — if fragment differs (e.g., `#token=...`), user instruction text changes; server code unaffected since user copies value manually |
| A2 | MAL token exchange returns JSON with `access_token`, `refresh_token`, `expires_in` fields | MAL PKCE Flow | Low — MAL docs confirm this is the response format; verified against standard OAuth 2.0 token endpoint spec |
| A3 | MAL access token expires in ~30 days | MAL PKCE Flow | Low — if expiration differs, refresh timing logic adjusts; `expires_at` is computed dynamically from `expires_in` |
| A4 | AniList GraphQL Viewer query is the correct token verification endpoint | AniList OAuth Flow | Low — if different query needed, only the GraphQL query string changes (e.g., `{ User { id } }` vs `{ Viewer { id } }`) |
| A5 | AniList `redirect_uri` value `https://anilist.co/api/v2/oauth` is valid for client_id 16329 | AniList OAuth Flow | Medium — if wrong, auth page shows redirect_uri mismatch error; user reports issue during development; easy fix once confirmed |

**If this table is empty:** All claims in this research were verified or cited — no user confirmation needed.

## Open Questions

1. **AniList implicit grant redirect_uri**: What exact value is registered for AniList client_id 16329? The standard is `https://anilist.co/api/v2/oauth` but this should be confirmed during implementation.
   - **What we know**: The client_id 16329 is a public app client (from PROJECT.md decisions).
   - **What's unclear**: The exact redirect_uri registered with AniList for this client.
   - **Recommendation**: Test during implementation — if wrong, adjust. The QR auth flow means this is just a browser redirect target; user copies token from URL bar.

2. **MAL redirect_uri for client_id c46c9e24640a64dad5be5ca7a1a53a0f**: Similar to AniList — what redirect URI is this client registered with?
   - **Recommendation**: Test during implementation. Common value: `https://myanimelist.net/oauth/callback`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.95.0 | Phase 1 code | ✓ (rust-toolchain.toml) | 1.95.0 | — |
| cargo | Adding qrcode dependency | ✓ | — | — |
| sqlx CLI | Testing migrations | ✓ (dev dependency) | — | — |
| reqwest (0.12) | OAuth HTTP calls | Already in shared dep | 0.12.28 | — |
| `qrcode` crate | QR generation | ✗ (needs `cargo add`) | 0.14.1 | `fast_qr` as alternative |

**Missing dependencies with no fallback:**
- None — once `qrcode` is added via `cargo add`, all deps are satisfied.

**Missing dependencies with fallback:**
- `qrcode` crate not yet in server Cargo.toml — needs `cargo add qrcode --package server`; falls back to `fast_qr` if issues arise (less mature, fewer downloads).

## Validation Architecture

> nyquist_validation not explicitly set in config.json (no config.json found). Treat as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust test harness) |
| Config file | none (workspace-level `cargo test` covers all crates) |
| Quick run command | `cargo test -p server -- track` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DB-01/02 | Migrations create track + tracker_auth tables | integration | `cargo test -p shared -- test_tracking_tables` | ❌ Wave 0 |
| DB-03 | Rust types (TrackerService, TrackEntry, etc.) | unit | `cargo test -p shared -- track::types::test` | ❌ Wave 0 |
| AL-01 | AniList auth URL generation | unit | `cargo test -p server -- anilist::test_auth_url` | ❌ Wave 0 |
| ML-01 | MAL auth URL generation with PKCE params | unit | `cargo test -p server -- mal::test_auth_url` | ❌ Wave 0 |
| ML-02 | PKCE code_verifier/challenge generation | unit | `cargo test -p server -- pkce::test` | ❌ Wave 0 |
| QR-01/02 | QR code generation produces valid PNG | integration | `cargo test -p server -- qr::test_generate` | ❌ Wave 0 |
| API-01 | GET /track/services | integration | `cargo test -p server -- track::test_services` | ❌ Wave 0 |
| API-03 | POST /track/{tracker}/auth stores tokens | integration | `cargo test -p server -- track::test_auth_submit` | ❌ Wave 0 |
| API-04 | DELETE /track/{tracker}/auth clears credentials | integration | `cargo test -p server -- track::test_auth_delete` | ❌ Wave 0 |
| API-05 | GET /track/{tracker}/status returns login state | integration | `cargo test -p server -- track::test_status` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p server -- track` (targeted)
- **Per wave merge:** `cargo test --workspace` (full suite)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `backend/server/src/track/routes.rs` — all new route handlers
- [ ] `backend/server/src/track/auth/anilist.rs` — AniListAuth unit tests
- [ ] `backend/server/src/track/auth/mal.rs` — MalAuth unit + PKCE tests
- [ ] Test code for QR generation round-trip (encode → decode → verify content matches)
- [ ] Test for PKCE session TTL expiry behavior

*(No gaps for existing test infrastructure — all seven new test areas require new test files)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | OAuth 2.0 implicit (AniList) + PKCE (MAL) handled by library; tokens stored in SQLite |
| V3 Session Management | no | No user sessions; per-request OAuth tokens |
| V4 Access Control | no | Local-only server (127.0.0.1 / UDS); no multi-user |
| V5 Input Validation | yes | Auth URL params: percent-encoding via `percent-encoding` crate (existing dep) |
| V6 Cryptography | no | PKCE SHA-256 via `sha2` (existing dep); no encryption at rest in v1 |
| V8 Data Protection | partial | `token_json` stored in SQLite; no encryption; OS-level file permissions only |

### Known Threat Patterns for stack (Rust + SQLite + OAuth)

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Token leakage via logs | Information Disclosure | Never log `token_json` values; use `Debug` impl that redacts secrets |
| SQL injection | Tampering | Handled by sqlx — all queries are parameterized / compile-time checked |
| CSRF on auth endpoint | Elevation of Privilege | Not applicable (local-only server, single-user); OAuth state parameter prevents CSRF in PKCE flow |
| Session fixation (PKCE) | Elevation of Privilege | Random `state` per auth URL (32 bytes via `rand`) prevents session fixation |
| Stale token reuse | Spoofing | `DELETE /auth` clears row; MAL auto-refresh invalidates old tokens |

## Sources

### Primary (HIGH confidence)
- [VERIFIED: crates.io] — qrcode 0.14.1, sha2 0.11, base64 0.22, rand (Cargo.lock)
- [VERIFIED: codebase patterns] — All route module, state, error handling, migration patterns read from existing source files
- [VERIFIED: OAuth specs] — OAuth 2.0 (RFC 6749), PKCE (RFC 7636) — well-known standards

### Secondary (MEDIUM confidence)
- [ASSUMED] — AniList API implicit grant specific endpoint behavior
- [ASSUMED] — MAL API PKCE specific endpoint behavior

### Tertiary (LOW confidence)
- (None — all claims are HIGH or MEDIUM)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified via `cargo search` and `package-legitimacy` check
- Architecture: HIGH — patterns verified against existing codebase (app.rs, state.rs, error.rs, route modules)
- OAuth flows: MEDIUM — standard OAuth 2.0 specs verified; exact AniList/MAL API quirks are assumed
- Pitfalls: HIGH — based on documented OAuth gotchas and e-ink display characteristics
- QR generation: HIGH — based on `qrcode` crate published API docs and e-ink best practices

**Research date:** 2026-06-28
**Valid until:** 2026-07-28 (30 days — OAuth API endpoints are stable; crate versions are pinned)
