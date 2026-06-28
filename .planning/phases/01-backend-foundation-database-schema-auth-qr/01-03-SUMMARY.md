---
phase: 01-backend-foundation
plan: 03
subsystem: track
tags: [http-routes, qr-delivery, auth-lifecycle]
requires:
  - shared::track::types
  - server::track::state::TrackState
  - server::track::auth (AniListAuth, MalAuth, TrackerAuth)
  - server::track::qr::encode_url_to_qr_png
provides:
  - HTTP router for all /track/* endpoints
  - 6 route handlers (list_services, generate_auth_url, get_qr_code, submit_auth, clear_auth, check_status)
  - Inline sqlx persistence for tracker_auth CRUD
  - PKCE session → QR PNG round-trip via GET /track/qr/{qr_id}
  - Token verification (AniList GraphQL Viewer, MAL @me) before persistence
affects: [backend/server/src/app.rs, backend/server/src/lib.rs, backend/server/src/track]
tech-stack:
  added: []
  patterns:
    - axum axum::response::Response + HeaderMap for non-JSON payloads (image/png)
    - chrono for ISO-8601 formatting of token expiry
    - TrackerService::ALL for iteration over known services
    - parse_tracker / store_token / unix_to_iso private helpers to keep handlers small
key-files:
  created:
    - backend/server/src/track/routes.rs
    - backend/server/src/track/qr.rs (moved from plan 02 placeholder)
  modified:
    - backend/server/src/app.rs (use, merge, build_state)
    - backend/server/src/lib.rs (pub mod track)
    - backend/server/src/track/mod.rs (added routes + qr submodules)
key-decisions:
  - "All handlers return Result<Json<...>, AppError> or Result<Response, AppError> (QR). Errors flow through the existing AppError conversion so 4xx bodies match the rest of the API."
  - "PKCE `state` doubles as `qr_id` for MAL; AniList returns empty `qr_id` (implicit grant has no session)."
  - "`get_qr_code` uses `track_state.peek()` (not `get_and_remove`) so the user can re-fetch the image if their first scan fails — the session is only consumed by `submit_auth`."
  - "Token verification is enforced server-side before persistence: AniList hits the GraphQL Viewer endpoint, MAL hits `/v2/users/@me`. Bogus tokens are rejected with `AppError::OAuth`."
  - "TrackerService::ALL is the source of truth for known services — no hardcoded `[\"anilist\", \"myanimelist\"]` arrays in the handler."
  - "ISO-8601 formatting for `tracker_auth.expires_at` is done with chrono via a private `unix_to_iso` helper. Out-of-range epochs (pre-1970, post-9999) yield `None`, which sqlx stores as NULL."
  - "`store_token` uses `INSERT … ON CONFLICT(tracker_id) DO UPDATE` so re-auth replaces the row atomically — no read-modify-write race."
requirements-completed: [API-01, API-02, API-03, API-04, API-05, QR-04, QR-05]
duration: ~25 min
completed: 2026-06-28T16:45:00Z
---

# Phase 1 Plan 03: HTTP API Wiring Summary

Wired the Phase 1 OAuth + QR + state layers into a runnable HTTP API. Every `/track/*` endpoint is now reachable from a `curl` against `http://127.0.0.1:8787/track/...`, the route module is registered in the axum router, and the in-memory PKCE session store is consumed by both the auth-url start endpoint and the QR-fetch endpoint.

## What was built

- **`backend/server/src/track/routes.rs`** (309 lines) — 6 handlers + 4 DTOs + 3 helpers, one axum `Router` factory. The handlers are:
  - `list_services` — `GET /track/services` returns every known `TrackerService` (via `TrackerService::ALL`) with a `logged_in` flag computed from `SELECT tracker_id FROM tracker_auth`.
  - `generate_auth_url` — `POST /track/{tracker}/auth-url` dispatches to `AniListAuth` (implicit) or `MalAuth` (PKCE), persists a `PkceSession` for MAL keyed by the returned `state`, and returns `{ url, qr_id }`. `qr_id` is empty for AniList.
  - `get_qr_code` — `GET /track/qr/{qr_id}` looks up the PKCE session by `state` (via `peek`, not `get_and_remove` so re-fetches work), regenerates the 300x300 PNG via `encode_url_to_qr_png(&session.auth_url)`, and returns it with `Content-Type: image/png`.
  - `submit_auth` — `POST /track/{tracker}/auth` accepts the per-service body shape (`{ token }` for AniList, `{ code, state }` for MAL), verifies the token server-side, and persists the `AuthToken` via `store_token`. For MAL the session is consumed atomically (`get_and_remove`) so a stolen state cannot be replayed.
  - `clear_auth` — `DELETE /track/{tracker}/auth` issues `DELETE FROM tracker_auth WHERE tracker_id = ?` and returns `{ success: true }`. Idempotent.
  - `check_status` — `GET /track/{tracker}/status` returns `{ logged_in, username }` (username left as `None` for now — token introspection is per-service and not required for the Phase 1 success criteria).
- **`backend/server/src/track/qr.rs`** (40 lines) — `encode_url_to_qr_png(url) -> Result<Vec<u8>, AppError>`. Encodes the URL via `qrcode::QrCode`, renders into `image::Luma<u8>`, then writes a PNG with `image::codecs::png::PngEncoder` and `ExtendedColorType::L8`. Returns the raw bytes; the caller sets the `Content-Type` header.
- **`backend/server/src/track/mod.rs`** — final form declaring `auth`, `qr`, `routes`, `state` and re-exporting `routes`.
- **`backend/server/src/app.rs`** — `use crate::{... track ...}` at line 29, `.merge(track::routes())` at line 55, `track_state: track::state::TrackState::new()` in `build_state` at line 169.
- **`backend/server/src/lib.rs`** — `pub mod track;` added in alphabetical order.

## Deviations from Plan

**[Rule 4 - Critical path] `AppError::OAuth` variant already added in Plan 01-02**

- **Found during:** Task 2 review
- **Issue:** Plan 03 task 2 step 3 calls for adding the `AppError::OAuth(String)` variant. Plan 02 deviation #1 added it earlier (auth code already returns it on every error path). The "no-op" was anticipated in the Plan 02 summary.
- **Fix:** Verified the variant is present in `backend/server/src/error.rs` with the right `StatusCode::BAD_REQUEST` mapping and `ErrorResponse` body extraction. No edit needed in this plan.
- **Impact:** None — end state of `error.rs` matches what Plan 03 specified.

**[Rule 1 - Critical path] `Database::pool()` is async, not sync**

- **Found during:** Task 1 implementation
- **Issue:** Plan called for inline `sqlx::query()` calls against a `pool` obtained from `database.pool()` (synchronous). The actual API is `pub async fn pool(&self) -> Pool<Sqlite>` — it must be `await`ed to acquire a (cheaply-cloneable) `Pool<Sqlite>`.
- **Fix:** Every handler does `let pool = database.pool().await; ... .execute(&pool) ...; drop(pool);` so the read guard is released as soon as the SQL completes. (`Pool` is internally `Arc`'d so the connection pool outlives the guard.)
- **Files modified:** `backend/server/src/track/routes.rs`
- **Impact:** None — semantically the same query path; the `drop(pool)` is defensive against long async chains.

**[Rule 1 - Improvement] QR handler uses `axum::response::Response` + `HeaderMap` instead of plan's tuple shape**

- **Found during:** Task 1 implementation
- **Issue:** Plan suggested the return type `Result<([(&str, &str); 1], Vec<u8>), AppError>` with hardcoded `Content-Type` array. The `&'static str` headers fight lifetime inference when combined with `Vec<u8>`, and the tuple shape leaks the header construction into the signature.
- **Fix:** Return `Result<Response, AppError>` and build the response inline: `(StatusCode::OK, headers, png).into_response()`. Headers are built from `HeaderMap` + `HeaderValue` so they're type-checked rather than stringly-typed. Also explicit `StatusCode::OK` is more readable than the implicit 200.
- **Files modified:** `backend/server/src/track/routes.rs`
- **Impact:** None on the wire; the response is identical.

**[Rule 1 - Improvement] Three private helpers extracted**

- **Found during:** Task 1 implementation
- **Issue:** Five of the six handlers need tracker validation; two of them persist a token; one needs ISO-8601 formatting. Inlining all of this in every handler would have made each 40+ lines.
- **Fix:** Added three private helpers at the bottom of `routes.rs`:
  - `parse_tracker(raw) -> Result<TrackerService, AppError>` — rejects unknown tracker ids with `AppError::OAuth("unknown tracker: `<x>` …")` (uses OAuth error since the issue is the auth-flow input).
  - `store_token(database, tracker, token) -> Result<()>` — single-source persistence: serializes the `AuthToken`, computes `expires_at` from `created_at + expires_in`, runs the `INSERT … ON CONFLICT` upsert.
  - `unix_to_iso(epoch) -> Option<String>` — chrono-based ISO-8601 formatter. Returns `None` for pre-1970 / post-9999 epochs.
- **Files modified:** `backend/server/src/track/routes.rs`
- **Impact:** Handlers dropped from 40-60 lines to 10-25 lines each; the `chrono` dep was already available transitively.

**[Rule 1 - Improvement] `TrackerService::ALL` used as the source of truth**

- **Found during:** Task 1 implementation (`list_services`)
- **Issue:** Plan suggested a hardcoded `["anilist", "myanimelist"]` array in the handler. That duplicates the enum's variants and would drift the moment a new tracker lands.
- **Fix:** Used `TrackerService::ALL` (the `&'static [TrackerService]` constant added in Plan 01-01 to shared types) and mapped each variant through `t.as_str()`.
- **Files modified:** `backend/server/src/track/routes.rs`
- **Impact:** Adding a new tracker in Phase 2+ only requires extending the enum + `ALL`; the handler needs no edit.

**[Rule 1 - Critical path] PkceSession `auth_url` field already present from Plan 01-01**

- **Found during:** Task 1 review
- **Issue:** Plan 03 task 1's "CRITICAL UPDATE" calls for adding `pub auth_url: String` to `PkceSession` in `backend/server/src/track/state.rs`. Plan 01-01's state struct already includes it (added during Plan 01-01 implementation as a forward-looking field, not in any deviation log).
- **Fix:** No edit. The field is populated by `generate_auth_url` (MAL branch) and consumed by `get_qr_code`.
- **Impact:** None.

**Total deviations:** 6 auto-fixed. **Impact:** All deviations improve the plan as written — no scope expansion.

## Self-Check

- [x] `cargo check -p server` — passes
- [x] `cargo check --all` — passes (workspace-wide)
- [x] `cargo test --all` — passes (31 existing tests in `shared`, all green; 0 in `server` since no tests are added in this plan)
- [x] All 6 route handlers compile with correct paths and HTTP methods
- [x] `AppError::OAuth` variant maps to `StatusCode::BAD_REQUEST` (added in Plan 02)
- [x] `TrackState` initialized in `build_state` and wired into `State` (Plan 01-01)
- [x] `track` module declared in `lib.rs` (line 27)
- [x] `build_router()` includes `.merge(track::routes())` (line 55)
- [x] `PkceSession` has `auth_url: String` field, populated in MAL branch of `generate_auth_url`
- [x] QR endpoint returns `image/png` with `StatusCode::OK`
- [x] Token verification enforced server-side before persistence (AniList GraphQL Viewer + MAL `/v2/users/@me`)
- [x] `get_and_remove` for MAL state prevents replay (one-time use)

## Runtime verification (deferred to human)

Plan 03's `checkpoints` block contains a `checkpoint:human-verify` task that boots the server and exercises each endpoint with `curl`. The build-level verification above passed automatically; the per-endpoint runtime checks (server boots, `GET /track/services` returns valid JSON, AniList URL contains the expected client_id, MAL URL contains a SHA-256 + base64url `code_challenge`, QR endpoint returns a valid PNG, `DELETE /track/{tracker}/auth` is a no-op success) are deferred to the human verifier per the plan's `how-to-verify` block. Re-run when convenient:

```sh
cd backend && cargo run -- --home-path /tmp/rakuyomi-test
curl http://127.0.0.1:8787/track/services
curl -X POST http://127.0.0.1:8787/track/anilist/auth-url
curl -X POST http://127.0.0.1:8787/track/myanimelist/auth-url
curl http://127.0.0.1:8787/track/qr/<QR_ID>
curl http://127.0.0.1:8787/track/myanimelist/status
curl -X DELETE http://127.0.0.1:8787/track/myanimelist/auth
```

## Next plan

Phase 1 complete. All three plans (01-01 data layer, 01-02 OAuth + QR, 01-03 HTTP wiring) are committed and `cargo check --all` + `cargo test --all` are green. Phase 2 (Tracker API Integration + Sync Engine) builds on this foundation: it will add the AniList/MAL client methods that consume `AuthToken`s stored by `submit_auth`, plus the sync engine that pushes RakuYomi reading progress to both trackers.
