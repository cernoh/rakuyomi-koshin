---
phase: 01-backend-foundation
plan: 01
subsystem: track
tags: [db, oauth-foundation, state]
requires: []
provides:
  - SQLite migration creating `track` and `tracker_auth` tables
  - shared::track::types domain types (TrackerService, TrackEntry, TrackStatus, SyncDirection, AuthToken, PkceSessionData)
  - server::track::state::TrackState (Arc<Mutex<HashMap<String, PkceSession>>>) with 15-min TTL
  - FromRef<State> for TrackState
affects: [backend/server/src/track, backend/shared/src/track]
tech-stack:
  added: []
  patterns: [axum FromRef, Arc<Mutex<HashMap>> for short-lived server state]
key-files:
  created:
    - backend/shared/migrations/20260628000001_create_tracking_tables.sql
    - backend/shared/src/track/mod.rs
    - backend/shared/src/track/types.rs
    - backend/server/src/track/mod.rs
    - backend/server/src/track/state.rs
  modified:
    - backend/shared/src/lib.rs
    - backend/server/src/lib.rs
    - backend/server/src/state.rs
    - backend/server/src/app.rs
key-decisions:
  - "STRICT mode on both tables following the existing migration pattern in `20260305005317_create_playlists_table.sql`."
  - "PKCE session store is in-memory (Arc<Mutex<HashMap>>) keyed by the 32-byte hex `state` string. Persistent storage was rejected to keep the schema minimal; flows are short-lived (~minutes) and a server restart mid-flow is an edge case."
  - "Expired entries are reaped lazily on `insert` / `get_and_remove` / `peek` — no background reaper task, no allocations when the map is empty."
  - "TrackerService enum serializes to lower-snake-case strings (`\"anilist\"`, `\"myanimelist\"`) that match the TEXT `tracker_id` columns. Display + FromStr implemented manually instead of pulling in `strum`."
  - "TrackStatus serializes to Mihon-compatible SCREAMING_SNAKE_CASE TEXT for direct DB round-trip via `as_mihon_status` / `from_mihon_status`."
requirements-completed: [DB-01, DB-02, DB-03, DB-04, QR-06]
duration: ~15 min
completed: 2026-06-28T16:30:00Z
---

# Phase 1 Plan 01: Data Layer Summary

Created the foundational data layer for Phase 1: SQLite schema for `track` + `tracker_auth`, shared Rust domain types, and in-memory PKCE state infrastructure. Every subsequent plan (01-02 OAuth, 01-03 routes) depends on these artifacts.

## What was built

- **`backend/shared/migrations/20260628000001_create_tracking_tables.sql`** — STRICT-mode SQLite tables matching the schema in `01-CONTEXT.md` D-04. `track` has 16 columns with `UNIQUE(manga_source_id, manga_id, tracker_id)`. `tracker_auth` has `UNIQUE(tracker_id)` for one-account-per-service. Two indices: `(tracker_id, manga_id)` and `(tracker_id)`.
- **`backend/shared/src/track/types.rs`** — Domain types: `TrackerService` (AniList + MyAnimeList, `as_str`/`FromStr`), `TrackStatus` (Mihon-compatible SCREAMING_SNAKE_CASE, `as_mihon_status`/`from_mihon_status`), `SyncDirection` (Push/Pull/TwoWay), `TrackEntry` (matches `track` table minus the auto-increment `id`, `#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]`), `AuthToken`, `PkceSessionData`.
- **`backend/server/src/track/state.rs`** — `TrackState` wrapping `Arc<Mutex<HashMap<String, PkceSession>>>`. `PkceSession` carries `code_verifier`, `tracker_id`, `auth_url`, and `created_at: Instant`. `insert` / `get_and_remove` / `peek` reaps expired entries lazily. 15-minute TTL constant `PKCE_SESSION_TTL`.
- **`backend/server/src/state.rs`** — Added `track_state: TrackState` field and `FromRef<State> for TrackState` impl.
- **`backend/server/src/app.rs`** — `build_state` now initializes `track_state: track::state::TrackState::new()` in the `State` literal.
- **`backend/shared/src/lib.rs` / `backend/server/src/lib.rs`** — Wired `pub mod track;` into the library roots.

## Deviations from Plan

**[Rule 1 - Critical path] Plan 1 Task 3 — module graph created early**

- **Found during:** Task 3 implementation
- **Issue:** Plan said to defer `backend/server/src/track/mod.rs` to Plan 3, but Task 3 also adds `use crate::track::state::TrackState;` in `state.rs`. Without `pub mod track;` in `lib.rs` and `track/mod.rs` declaring `pub mod state;`, the import path doesn't resolve and `cargo check -p server` fails.
- **Fix:** Created a minimal `backend/server/src/track/mod.rs` containing only `pub mod state;` in this plan. Plan 3's `track/mod.rs` (which adds `pub mod auth;`, `pub mod routes;`, `mod qr;`) is a strict superset — the existing `pub mod state;` is fully compatible. Added `pub mod track;` to `lib.rs` in alphabetical position.
- **Files modified:** `backend/server/src/track/mod.rs` (new), `backend/server/src/lib.rs`
- **Verification:** `cargo check -p server` passes after the change.
- **Impact:** None — Plan 3's `track/mod.rs` still works as written. The early `pub mod state;` declaration just means Plan 3 doesn't need to re-declare it.

**[Rule 1 - Dead code] Removed unused `cleanup_expired` async method**

- **Found during:** Task 3 verification
- **Issue:** `TrackState::cleanup_expired` was a private `async fn` not called by anything. The public methods (`insert`, `get_and_remove`, `peek`) call the synchronous `cleanup_locked` helper directly while already holding the lock.
- **Fix:** Removed the unused method.
- **Files modified:** `backend/server/src/track/state.rs`
- **Impact:** None — behavior unchanged.

**Total deviations:** 2 auto-fixed. **Impact:** Both deviations improve the plan as written; no semantic change to the produced artifacts.

## Self-Check

- [x] `cargo check -p shared` — passes
- [x] `cargo check -p server` — passes
- [x] Migration file parses (validated by sqlx at `Database::new()` build time)
- [x] All required types derive Serialize/Deserialize/Clone/Debug (where appropriate)
- [x] `FromRef<State> for TrackState` compiles and is registered
- [x] `TrackState::new()` is called once in `build_state`; field present on `State`

## Next plan

`01-02-PLAN.md` — OAuth protocol support (AniListAuth implicit grant, MalAuth PKCE flow) and QR code image generation. Depends on the types and `TrackState` defined here.
