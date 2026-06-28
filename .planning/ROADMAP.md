# Roadmap: v2.0 External Tracker Sync

**Defined:** 2026-06-28
**Goal:** Integrate AniList and MyAnimeList tracking — OAuth2 authentication, push reading progress, reading status, and scores.

---

## Phase 1 — OAuth2 Foundation

**Focus:** Backend OAuth2 infrastructure for both services. Auth URL generation, localhost redirect server, token exchange.

**Requirements:** AUTH-01, AUTH-02, AUTH-03, AUTH-04

### Task Breakdown

#### Backend: Tracker Module Scaffold
- [ ] Create `backend/shared/src/tracker/` module structure
- [ ] Define `TrackerService` trait + `TrackerKind` enum (`Anilist`, `MyAnimeList`)
- [ ] Wire up in server state (`src/state.rs`) and router (`src/app.rs`)

#### Backend: OAuth2 Redirect Server
- [ ] Implement localhost HTTP listener on random port in `tracker/oauth.rs`
- [ ] Handle redirect callback (`/callback?code=...&state=...`)
- [ ] Return a "close this page" response for KOReader webview
- [ ] Support timeout/cleanup (close server after N seconds or on first callback)

#### Backend: AniList Auth
- [ ] `AnilistClient::auth_url()` — generates authorization URL (`client_id`, `redirect_uri`, `response_type=code`)
- [ ] `AnilistClient::exchange_code()` — POSTs to token endpoint, returns access token
- [ ] Model types for auth response (serde)

#### Backend: MyAnimeList Auth
- [ ] `MalClient::auth_url()` — generates authorization URL with PKCE challenge (plain method)
- [ ] `MalClient::exchange_code()` — POSTs token endpoint with code_verifier
- [ ] `MalClient::refresh_token()` — POSTs refresh endpoint with Basic auth
- [ ] Model types for auth + refresh responses (serde)

#### Backend: Tracker Routes
- [ ] `GET /tracker/{service}/auth-url` — returns auth URL; starts redirect server
- [ ] `GET /tracker/{service}/status` — returns auth state (pending/done/error)

#### Lua Frontend: TrackerAuthDialog
- [ ] `TrackerAuthDialog.lua` — dialog that opens KOReader webview to auth URL
- [ ] Polls `/tracker/{service}/status` until authenticated or timeout
- [ ] Error handling: network error, timeout, auth denied

#### Lua Frontend: Settings Integration
- [ ] Add "Tracker Accounts" section to `Settings.lua`
- [ ] Per-service "Connect" button → launches `TrackerAuthDialog`

### Acceptance Criteria
- User can tap "Connect to AniList" in settings, authenticate in webview, and see "Connected" status
- User can tap "Connect to MyAnimeList", complete PKCE flow, and see "Connected" status
- Failed/interrupted auth shows error state and allows retry

### Dependencies
- New SQLite table: `tracker_tokens` (schema defined in requirements)
- `rand` crate likely already in deps for PKCE challenge generation

---

## Phase 2 — Token Persistence & Manga Linking

**Focus:** Store tokens in SQLite, handle MAL token refresh, build the manga→tracker linking system.

**Requirements:** AUTH-05, AUTH-06, LINK-01, LINK-02, LINK-03, LINK-04, LINK-05

### Task Breakdown

#### Backend: Token Storage
- [ ] Add `tracker_tokens` table to SQLite migrations
- [ ] `TrackerStorage` in `tracker/storage.rs` — CRUD for token rows
- [ ] On successful auth (Phase 1), persist tokens
- [ ] On server startup, load tokens from DB into state
- [ ] Token expiry check: AniList (1yr — show warning near expiry), MAL (1hr — auto-refresh on 401)

#### Backend: MAL Token Auto-Refresh
- [ ] Intercept 401 from MAL API calls → trigger refresh flow
- [ ] `MalClient::refresh_token()` implementation
- [ ] Update stored tokens after refresh
- [ ] Handle refresh failure (invalid grant) → mark as disconnected

#### Backend: Manga Linking — Data Model
- [ ] Add `tracker_links` table: `manga_id TEXT, service TEXT, tracker_media_id TEXT, tracker_slug TEXT, linked_at INTEGER, UNIQUE(manga_id, service)`
- [ ] Migration to add tracker_links table
- [ ] Model types for link operations
- [ ] Expose linked tracker IDs in manga detail response for Lua UI

#### Backend: Manga Linking — Search
- [ ] `GET /tracker/{service}/search?q=...` — search manga on tracker by title
- [ ] AniList search: GraphQL `Page.media(search, type: MANGA)` returning id + title + chapters
- [ ] MAL search: `GET /v2/manga?q=...` returning id + title + chapters
- [ ] Search results cached briefly (TTL 5 min) to reduce API calls

#### Backend: Manga Linking — Link/Unlink
- [ ] `POST /manga/{id}/tracker-link` — body: `{ service, tracker_media_id, tracker_slug }`, upserts into tracker_links
- [ ] `POST /manga/{id}/tracker-unlink` — body: `{ service }`, removes link
- [ ] `GET /manga/{id}/tracker-links` — returns linked services for a manga

#### Lua Frontend: TrackerLinkDialog
- [ ] Dialog in manga info panel — "Link to Tracker" section
- [ ] Shows current linked services (e.g., "Linked to AniList: One Piece ✓")
- [ ] "Search" button → opens search field, queries backend, shows results
- [ ] Select result → links manga to tracker entry
- [ ] "Unlink" button for each connected service

#### Lua Frontend: UI Updates to MangaInfoWidget
- [ ] Show per-manga tracker status in manga info panel (linked/unlinked, last synced)
- [ ] Add tracker section to `MangaInfoWidget.lua`

### Acceptance Criteria
- Tokens survive server restart (verified: check DB, test auth check after restart)
- MAL token auto-refresh works (start with stored token, wait 1+ hr OR manually expire, verify refresh succeeds)
- User can search AniList for a manga title and link it
- Linked status shows in manga details
- User can unlink a manga

### Dependencies
- Phase 1 (OAuth2 infrastructure must exist)

---

## Phase 3 — Sync Engine & Full UI

**Focus:** The sync pipeline, progress push on read, status & score management, connection management UI.

**Requirements:** AUTH-07, AUTH-08, SYNC-01, SYNC-02, SYNC-03, SYNC-04, SYNC-05, STAT-01, STAT-02, STAT-03, STAT-04, STAT-05

### Task Breakdown

#### Backend: Sync Service
- [ ] `TrackerSyncService` in `tracker/mod.rs` — orchestrates sync operations
- [ ] Single-manga sync: read linked entries → push to each service
- [ ] Bulk sync: iterate all linked mangas → push to each service
- [ ] Rate-limiting wrapper (AniList: token bucket 90/min, MAL: leaky bucket 1/s)
- [ ] Retry with exponential backoff on 429/403/network errors
- [ ] Sync runs as background tokio task (no blocking)

#### Backend: Progress Push
- [ ] Hook into existing `mark_chapter_as_read` / `mark_chapters_as_read` use cases
- [ ] After successful mark-read, emit sync event for linked tracker entries
- [ ] Sync: `POST /manga/{id}/tracker-sync` → single push
- [ ] Sync: `POST /tracker/sync-all` → bulk push all

#### Backend: Status & Score API
- [ ] `POST /manga/{id}/tracker-status` — body: `{ status }` → sync reading status
- [ ] `POST /manga/{id}/tracker-score` — body: `{ score }` → sync score
- [ ] AniList status/score map: `SaveMediaListEntry(mediaId, status, progress, scoreRaw)` 
- [ ] MAL status/score map: `PATCH /v2/manga/{id}/my_list_status` with status + score + num_chapters_read
- [ ] Score conversion: RakuYomi 0-10 → AniList 0-100 (*10), MAL 0-10 (direct)
- [ ] Score display: RakuYomi internal 0-10

#### Backend: Connection Management
- [ ] `POST /tracker/{service}/disconnect` — remove tokens, clear links for that service
- [ ] `GET /tracker/status` — aggregated status of all tracker connections (connected/disconnected, username, expiry_hint)

#### Lua Frontend: Sync Triggers
- [ ] Hook into chapter-read marking in `ChapterListing.lua` — after successful mark-read, trigger sync
- [ ] Sync runs as background job (via existing `Job.lua` infrastructure)
- [ ] Show sync progress/status in job dialog

#### Lua Frontend: Status & Score UI
- [ ] Add status selector to manga info panel (Reading/Completed/Dropped/On Hold/Plan to Read)
- [ ] Add score input (0-10 slider or number input) to manga info panel
- [ ] Show current tracker status and score if linked

#### Lua Frontend: Manual Sync Actions
- [ ] "Sync Now" button in manga info panel for single-manga sync
- [ ] "Sync All Tracked" button in settings or library menu
- [ ] `TrackedMangaSync` job dialog showing progress per tracked manga

#### Lua Frontend: Connection Management in Settings
- [ ] Show connected services with status (username, token expiry for AniList)
- [ ] "Disconnect" button per service with confirmation dialog
- [ ] Re-auth option when token is expired/revoked

### Acceptance Criteria
- Reading a chapter pushes progress to all linked trackers
- User can set manga status (Reading/Completed) and it syncs
- User can score a manga (0-10) and it syncs (with scale conversion)
- Bulk sync processes all linked manga with visible progress
- Rate limits respected — no 429s during normal operation
- Disconnecting a tracker removes tokens and unlinks all mangas for that service
- All sync operations work as background jobs (non-blocking UI)

### Dependencies
- Phase 1 (auth infrastructure) + Phase 2 (token persistence + linking)
- `reqwest` already in deps, but may need `tokio::sync::Semaphore` or similar for rate limiting

---

## Phase 4 — Polish & Verification

**Focus:** Error handling, edge cases, documentation, testing.

### Task Breakdown

#### Error Handling
- [ ] Handle token expiry gracefully (AniList: show re-auth prompt, MAL: auto-refresh)
- [ ] Handle tracker service down/unreachable
- [ ] Handle manga deleted from tracker (404 on sync → unlink and notify)
- [ ] Handle network timeout during sync
- [ ] Show sync error state per manga in UI

#### Testing
- [ ] Unit tests for token storage (CRUD, expiry detection)
- [ ] Unit tests for score conversion (0-10 ↔ 0-100)
- [ ] Unit tests for rate limiter
- [ ] Unit tests for linked manga model
- [ ] Integration tests for OAuth2 redirect server (mock callback)
- [ ] Rust tests: validate GraphQL query strings + response deserialization
- [ ] E2E tests: Playwright tests for OAuth2 flow (mock tracker endpoints)

#### Documentation
- [ ] Update user guide with tracker setup instructions
- [ ] Document OAuth2 app registration process (AniList: developer settings, MAL: API config)
- [ ] Architecture docs for tracker module in `docs/`

#### i18n
- [ ] New translatable strings for tracker UI elements
- [ ] Run `make update-trans` to generate translation templates

#### Edge Cases
- [ ] Manga with no chapters → skip tracker sync
- [ ] Manga title changes → re-search and re-link flow
- [ ] Same manga linked to multiple trackers → progress pushed to all
- [ ] Chapter reread (reading a chapter that's already marked read) → re-push?
- [ ] Sync while offline → queue and retry on next sync trigger

### Acceptance Criteria
- Sync errors (offline, auth failed, manga deleted) show in UI without crashing
- Tests pass: `cargo test`
- User guide updated with tracker integration instructions
- All translatable strings available via gettext

---

## Phase Dependencies

```
Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4
(Auth)      (Persist +   (Sync +      (Polish)
             Linking)     Status/UI)
```

Phase 4 can overlap with Phase 3 (write tests for Phase 1/2 components while building Phase 3).

---

## Requirements → Phase Mapping Summary

| Phase | Requirements | Theme |
|-------|-------------|-------|
| 1 | AUTH-01–04 | OAuth2 auth flow (both services) |
| 2 | AUTH-05–06, LINK-01–05 | Token persistence + manga linking |
| 3 | AUTH-07–08, SYNC-01–05, STAT-01–05 | Sync engine + full UI |
| 4 | (polish) | Error handling, tests, docs, i18n |

---
*Roadmap defined: 2026-06-28*
