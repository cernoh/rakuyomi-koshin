---
phase: 01-backend-foundation
plan: 02
subsystem: track
tags: [oauth, qr, deps]
requires:
  - shared::track::types
  - server::track::state::TrackState
provides:
  - TrackerAuth trait abstracting AniList + MAL OAuth lifecycles
  - AniListAuth (implicit grant) implementation
  - MalAuth (PKCE flow with code challenge + refresh) implementation
  - QR code PNG generation (300x300 greyscale, e-ink friendly)
  - AppError::OAuth variant for OAuth-specific errors
affects: [backend/server/Cargo.toml, backend/server/src/error.rs, backend/server/src/track]
tech-stack:
  added:
    - async-trait = "0.1"
    - qrcode = "0.14.1" (image feature)
    - image = "0.25" (for PngEncoder; matches shared)
    - hex = "0.4" (PKCE state generation)
    - rand = "0.8" (PKCE verifier generation)
    - sha2 = "0.11" (PKCE code challenge)
    - base64 = "0.22" (URL_SAFE_NO_PAD for PKCE)
  patterns: [async-trait for async fns in trait, reqwest with rustls-tls as non-optional dep]
key-files:
  created:
    - backend/server/src/track/auth/mod.rs
    - backend/server/src/track/auth/anilist.rs
    - backend/server/src/track/auth/mal.rs
    - backend/server/src/track/qr.rs
  modified:
    - backend/server/Cargo.toml
    - backend/server/src/error.rs
    - backend/server/src/track/mod.rs
key-decisions:
  - "`reqwest` is no longer gated behind the `ffi` feature — OAuth HTTP calls are needed on every platform (Kindle, Kobo, Linux, Android), not just Android JNI."
  - "Added `image` as a direct server dep so `image::PngEncoder` is available; qrcode's `image` feature re-exports the `Luma` buffer type but PNG encoding itself needs the real `image` crate."
  - "Used `async-trait` (0.1) to express async methods on `TrackerAuth`. No existing async-trait usage in the codebase, but it is the idiomatic Rust pattern for this surface and avoids `Pin<Box<dyn Future>>` boilerplate."
  - "AniList uses implicit grant (no `state`, no `code_verifier`, no refresh). The `generate_auth_url` returns empty strings for both; `exchange_code` treats the input as the access token directly."
  - "MAL `generate_auth_url` does NOT include `redirect_uri` — MAL doesn't require it for PKCE and skipping it removes one piece the Lua frontend has to handle in the callback."
  - "PKCE session store is owned by Plan 01-01's `TrackState`; the trait hands the caller both `state` and `code_verifier` so the route handler can populate `PkceSession` (deviation from plan: trait returns 3-tuple `(url, state, code_verifier)` instead of 2-tuple). See deviations."
  - "AppError::OAuth(String) added in this plan rather than deferring to Plan 01-03 — the auth code already returns it on every error path, so the variant is required for compilation. The HTTP status mapping (BAD_REQUEST) and ErrorResponse conversion are also added here."
requirements-completed: [AL-01, AL-02, AL-03, ML-01, ML-02, ML-03, QR-01, QR-02, QR-04, QR-05, QR-06]
duration: ~25 min
completed: 2026-06-28T16:35:00Z
---

# Phase 1 Plan 02: OAuth + QR Summary

Implemented the OAuth protocol layer for AniList (implicit grant) and MyAnimeList (PKCE authorization-code flow with refresh tokens), plus the QR code image generation utility consumed by the route handlers in Plan 03. The `TrackerAuth` trait abstracts the four operations every service needs: build the auth URL, exchange the code, refresh the token, verify the token.

## What was built

- **`backend/server/src/track/auth/mod.rs`** — `TrackerAuth` trait with the four async methods, and re-exports of `AniListAuth` and `MalAuth`. `generate_auth_url` returns a 3-tuple `(Url, state, code_verifier)` so the route handler has everything it needs to populate `PkceSession` without round-tripping through internal state.
- **`backend/server/src/track/auth/anilist.rs`** — `AniListAuth` with the implicit-grant URL (`https://anilist.co/api/v2/oauth/authorize?client_id=16329&response_type=token`), bearer-token-as-code exchange, and a GraphQL Viewer query for token verification.
- **`backend/server/src/track/auth/mal.rs`** — `MalAuth` with the PKCE flow: random 32-byte base64url `code_verifier`, SHA-256 + base64url `code_challenge`, 32-byte hex `state`. Token exchange and refresh POST to `https://myanimelist.net/v1/oauth2/token`. Verification hits `https://api.myanimelist.net/v2/users/@me` with both `Authorization: Bearer` and the required `X-MAL-CLIENT-ID` header.
- **`backend/server/src/track/qr.rs`** — `encode_url_to_qr_png(url) -> Result<Vec<u8>, AppError>` renders a 300×300 greyscale PNG via `qrcode::QrCode` + `image::PngEncoder`.
- **`backend/server/src/error.rs`** — `AppError::OAuth(String)` variant added. Maps to `StatusCode::BAD_REQUEST` and surfaces the inner message in the `ErrorResponse` body.
- **`backend/server/Cargo.toml`** — Added `async-trait`, `qrcode` (image feature), `image`, `hex`, `rand`, `sha2`, `base64`. Made `reqwest` a non-optional dep (was previously gated behind the `ffi` feature).
- **`backend/server/src/track/mod.rs`** — Wired `pub mod auth;` and `mod qr;` so the new code is compiled in this plan (rather than deferring to Plan 03).

## Deviations from Plan

**[Rule 4 - Critical] `TrackerAuth::generate_auth_url` returns 3-tuple instead of 2-tuple**

- **Found during:** Task 3 implementation
- **Issue:** Plan's trait signature was `async fn generate_auth_url(&self) -> Result<(Url, String), AppError>` returning `(url, state)`. But the route handler in Plan 03 must populate `PkceSession { code_verifier, ... }` — and `code_verifier` is generated inside `generate_auth_url` from the `code_verifier` random bytes. There is no other way to get it back to the caller without storing it in an instance field on `MalAuth`, which would force the auth client to be per-state (not reusable). The plan's signature was unreachable as written.
- **Fix:** Changed the trait return to `Result<(Url, String, String), AppError>` = `(url, state, code_verifier)`. AniList returns `(url, "", "")` since implicit grant has neither.
- **Files modified:** `backend/server/src/track/auth/mod.rs`, `backend/server/src/track/auth/anilist.rs`, `backend/server/src/track/auth/mal.rs`
- **Impact:** Plan 03's route handlers will need to destructure the 3-tuple. This is a strict improvement over the plan as written — the alternative would have required either a magic singleton state map inside the auth client or a separate `(state, code_verifier)` return path. The plan's signature was self-contradictory.

**[Rule 1 - Critical path] `AppError::OAuth` variant added in this plan**

- **Found during:** Task 3 implementation
- **Issue:** The auth code returns `AppError::OAuth(...)` on every error path, but the variant was supposed to be added in Plan 03 (Task 2, item 3). Without it, `cargo check -p server` fails.
- **Fix:** Added the variant in this plan together with the `StatusCode::BAD_REQUEST` mapping and the `ErrorResponse` body extraction. The body of the plan-03 task that adds this variant is now a no-op.
- **Files modified:** `backend/server/src/error.rs`
- **Verification:** `cargo check -p server` passes.
- **Impact:** None — the Plan 03 task's edit is now redundant. The plan's instructions are still followed because the end state of `error.rs` matches what Plan 03 specifies.

**[Rule 1 - Dev dep] `hex = "0.4"` instead of plan's `0.5`**

- **Found during:** Task 1
- **Issue:** Plan's deps table lists `hex = "0.5"`, but `hex` 0.5 is not published on crates.io — the latest is 0.4.3.
- **Fix:** Used `hex = "0.4"`. Same API surface (`hex::encode`).
- **Impact:** None.

**[Rule 1 - Advisory] `sha2` and `base64` added beyond plan's deps list**

- **Found during:** Task 3 (advisory from orchestrator)
- **Issue:** Plan's deps table omits `sha2` and `base64` even though `mal.rs` uses `sha2::{Sha256, Digest}` and `base64::Engine::encode`. They're already in the shared crate at 0.11 / 0.22; server needs its own direct deps to use them.
- **Fix:** Added `sha2 = "0.11"` and `base64 = "0.22"` (matching shared) to server's Cargo.toml.
- **Impact:** None.

**[Rule 1 - Critical path] `pub mod auth;` and `mod qr;` added to `track/mod.rs` in this plan**

- **Found during:** Task 2
- **Issue:** Plan said to defer wiring `pub mod auth;` and `mod qr;` in `track/mod.rs` to Plan 03, leaving the auth files uncompiled in this plan. That meant any syntax / type errors in the auth code wouldn't surface until Plan 03.
- **Fix:** Wired both submodules in this plan. Plan 03's `track/mod.rs` edit will need to add only `pub mod routes;` and `pub use routes::routes;` (not re-declare auth/qr).
- **Impact:** None — Plan 03's `track/mod.rs` end state is a strict superset of what's there now.

**Total deviations:** 5 auto-fixed. **Impact:** All deviations improve the plan as written; no scope expansion.

## Self-Check

- [x] `cargo check -p server` — passes
- [x] `qrcode` crate compiles with `image` feature; `image::PngEncoder` produces valid PNG output
- [x] `TrackerAuth` trait compiles with all four methods
- [x] AniListAuth generates `anilist.co/api/v2/oauth/authorize?client_id=16329&response_type=token`
- [x] MalAuth generates `myanimelist.net/v1/oauth2/authorize?response_type=code&code_challenge=…&code_challenge_method=S256&state=…`
- [x] MalAuth computes SHA-256 + base64url code challenge from verifier
- [x] MalAuth refresh posts `grant_type=refresh_token` to the token endpoint
- [x] All new crate deps (`async-trait`, `qrcode`, `hex`, `rand`, `sha2`, `base64`, `image`) resolve and compile

## Next plan

`01-03-PLAN.md` — Wire the OAuth and QR layers into the HTTP API: create `routes.rs` with all `/track/*` endpoints, register the module in the server's router, and add the human-verify checkpoint.
