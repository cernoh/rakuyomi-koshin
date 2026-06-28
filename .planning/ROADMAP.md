# Roadmap: RakuYomi Tracking Integration

## Phase 1: Backend Foundation — Database Schema + Auth + QR

**Goal**: SQLite schema for tracking, OAuth auth flow for both services, QR code generation, HTTP API for auth and service discovery.

### Tasks

1. **Database migration** — Create `track` and `tracker_auth` SQLite tables via sqlx migration
2. **Rust types** — Define `TrackerService` enum, `TrackEntry`, `TrackStatus`, `SyncDirection` in `shared::track` module
3. **Auth token storage** — Implement token CRUD inline in route handlers (per CONTEXT.md: auth operations live in server::track, not shared). Track entry CRUD deferred to Phase 2.
4. **Tracker HTTP client module** — Create `shared::track::client` with base HTTP client, rate limiting, error types
5. **AniList OAuth** — Implement auth URL generation, token submission, credential storage
6. **MAL OAuth** — Implement auth URL generation (with PKCE), token exchange, credential storage, refresh
7. **QR code generation** — Add `qrcode` crate dependency, implement PNG rendering endpoint
8. **Auth API endpoints** — `GET /track/services`, `POST /track/{tracker}/auth-url`, `POST /track/{tracker}/auth`, `DELETE /track/{tracker}/auth`, `GET /track/{tracker}/status`
9. **Register route module** — Wire `crate::track::routes()` into `build_router`

### Dependencies

- shared crate gains `track` module
- `qrcode` + `image` crates added to server
- New sqlx migration files

### Success Criteria

- `GET /track/services` returns list with correct login status
- AniList OAuth URL generation returns valid `anilist.co/oauth/authorize` URL
- MAL OAuth URL generation returns valid PKCE URL with code challenge
- QR code endpoint returns valid PNG image bytes
- Auth submission stores tokens in DB and verifies login
- All existing tests pass

---

## Phase 2: Tracker API Integration + Sync Engine

**Goal**: Full API integration with AniList (GraphQL) and MyAnimeList (REST), search, bind/unbind, two-way sync logic.

### Tasks

1. **AniList GraphQL client** — Implement search, add/update/delete list entry, get user list, get current user, status/score mapping
2. **AniList rate limiting** — Implement 85 req/min throttle with token bucket
3. **MAL REST client** — Implement search, add/update/delete list entry, get user list, get current user, status/score mapping
4. **MAL token refreshing** — Auto-refresh expired tokens via stored refresh_token
5. **Sync engine: bind flow** — On first link, fetch remote entry (if exists) or create new; merge local state
6. **Sync engine: push sync** — Send local `last_chapter_read`, `status`, `score` → remote
7. **Sync engine: pull sync** — Fetch remote progress → update local DB entry
8. **Sync engine: merge logic** — Take MAX of last chapter read; auto-status transitions
9. **Tracker API endpoints** — Search, bind, unbind, get manga tracking, trigger sync, library summary
10. **Error handling** — Network errors, auth expiry, rate limits, invalid manga ID, typed error responses

### Dependencies

- Phase 1 (DB + auth) must be complete
- New reqwest/ureq based HTTP clients for AniList GraphQL + MAL REST

### Success Criteria

- Can search AniList for manga by title
- Can search MAL for manga by title
- Bind flow works: link local manga → remote entry, auto-creates if not existing
- Push sync: reading a chapter updates remote progress
- Pull sync: remote progress changes reflected locally
- Two-way sync merges correctly (max wins)
- Auto-complete: reading last chapter sets COMPLETED status
- MAL token auto-refresh works after expiry
- All endpoints handle errors gracefully

---

## Phase 3: Frontend — Lua UI Integration

**Goal**: Full KOReader UI for tracker management, QR display, manga tracking status, sync triggers.

### Tasks

1. **Backend.lua API methods** — Add HTTP wrapper methods for all tracker endpoints
2. **Tracker settings UI** — New tracking section in Settings showing services with login status, connect/disconnect buttons
3. **QR code display widget** — Center dialog showing QR image from server, with text URL fallback
4. **Auth completion widget** — Text input for OAuth token/code entry after phone scan
5. **Manga info tracking section** — Show bound tracker entries (status, progress, score) in MangaInfoWidget
6. **Track search dialog** — Search tracker for manga, select, bind to current manga
7. **Manual sync button** — "Sync Now" on bound manga to trigger push+pull
8. **Auto-sync on chapter read** — After marking chapter as read, trigger sync for bound mangas
9. **Library tracking badges** — Optional icons in LibraryView showing tracked manga
10. **Error handling UI** — Display sync errors with actionable messages (re-auth, retry)
11. **Localization strings** — Add l10n entries for all new UI text

### Dependencies

- Phase 1 + Phase 2 must be complete
- QR image endpoints available

### Success Criteria

- Can authenticate with AniList via QR code → scan → token entry
- Can authenticate with MAL via QR code → scan → code entry
- Tracking settings show accurate login state
- Can search and bind manga to tracker from manga info
- Bound manga shows status, progress, score
- Reading a chapter auto-syncs to bound tracker
- Manual sync works (push + pull)
- Error states displayed clearly
- All existing UI functionality preserved

---

## Future Phases (v2+)

| Phase | Content |
|-------|---------|
| Phase 4 | Additional trackers (Kitsu, Bangumi) |
| Phase 5 | Batch sync + Delayed retry queue |
| Phase 6 | Auto-track on manga add |
| Phase 7 | EnhancedTracker-style auto-matching by source |

---

## Notes

- Phase numbering continues from existing ROADMAP.md (if any) or starts at 1.
- Each phase produces a shippable increment.
- Phases are executed in order; no phase-skipping.
- After each phase: `cargo test` + `luacheck` must pass.
