# Requirements: RakuYomi Tracking Integration

**Defined:** 2026-06-28
**Core Value:** Users can browse, download, and read manga from any supported source directly on their e-ink device with a KOReader-native interface.

## v1 Requirements

Requirements for the Tracking Integration milestone. Each maps to roadmap phases.

### Backend: Database & Core Models

- [ ] **DB-01**: Backend has a `track` SQLite table with fields: id, manga_id (FK), tracker_id, remote_id, library_id, title, last_chapter_read, total_chapters, status, score, start_date, finish_date, tracking_url, private, updated_at
- [ ] **DB-02**: Backend has a `tracker_auth` SQLite table for storing OAuth tokens per tracker service (tracker_id, token_json, expires_at, created_at)
- [ ] **DB-03**: Backend exposes Rust types for tracker operations (TrackerService enum, TrackEntry struct, TrackStatus, SyncDirection)
- [ ] **DB-04**: SQLx migration files for new tables with proper indices

### Backend: AniList Integration

- [ ] **AL-01**: Backend can generate AniList OAuth authorization URL for QR display
- [ ] **AL-02**: Backend can exchange an OAuth token with AniList API (implicit grant)
- [ ] **AL-03**: Backend stores AniList credentials and verifies login status
- [ ] **AL-04**: Backend can search manga on AniList via GraphQL API
- [ ] **AL-05**: Backend can create/update AniList list entry (add manga to tracking)
- [ ] **AL-06**: Backend can fetch remote tracking data from AniList for a manga
- [ ] **AL-07**: Backend can delete manga from AniList user list
- [ ] **AL-08**: Backend handles AniList status mapping (local ↔ API) and score conversion
- [ ] **AL-09**: Backend respects AniList rate limits (85 req/min)

### Backend: MyAnimeList Integration

- [ ] **ML-01**: Backend can generate MAL OAuth authorization URL with PKCE for QR display
- [ ] **ML-02**: Backend can exchange an authorization code with MAL API (PKCE flow)
- [ ] **ML-03**: Backend stores MAL OAuth tokens and can refresh them automatically
- [ ] **ML-04**: Backend can search manga on MAL via REST API
- [ ] **ML-05**: Backend can create/update MAL list entry
- [ ] **ML-06**: Backend can fetch remote tracking data from MAL for a manga
- [ ] **ML-07**: Backend can delete manga from MAL user list
- [ ] **ML-08**: Backend handles MAL status mapping and score conversion
- [ ] **ML-09**: Backend auto-refreshes MAL tokens when expired

### Backend: Sync Engine

- [ ] **SYNC-01**: Backend supports bind flow: on first link, fetch remote entry if exists, otherwise create new entry with local state
- [ ] **SYNC-02**: Backend supports push sync: send local chapter progress → remote service
- [ ] **SYNC-03**: Backend supports pull sync: fetch remote progress → update local DB
- [ ] **SYNC-04**: Backend applies sync conflict resolution: take max of local vs remote last chapter read
- [ ] **SYNC-05**: Backend auto-updates reading status on chapter progress (auto-complete when all chapters read)
- [ ] **SYNC-06**: Backend supports two-way sync that merges remote data into local without overwriting newer local reads

### Backend: HTTP API Routes

- [ ] **API-01**: `GET /track/services` — list available tracker services with login status
- [ ] **API-02**: `POST /track/{tracker}/auth-url` — generate OAuth URL for QR code display
- [ ] **API-03**: `POST /track/{tracker}/auth` — submit OAuth token/code to complete login
- [ ] **API-04**: `DELETE /track/{tracker}/auth` — logout (clear credentials)
- [ ] **API-05**: `GET /track/{tracker}/status` — check login status
- [ ] **API-06**: `GET /track/{tracker}/search?q={query}` — search manga on tracker
- [ ] **API-07**: `GET /track/{tracker}/manga/{remote_id}` — get remote manga details
- [ ] **API-08**: `POST /track/bind` — link a local manga to a tracker entry (request: manga_id, tracker_id, remote_id)
- [ ] **API-09**: `DELETE /track/unbind` — remove tracking link (request: manga_id, tracker_id)
- [ ] **API-10**: `GET /track/{manga_id}` — get all tracking statuses for a manga
- [ ] **API-11**: `POST /track/{manga_id}/sync` — trigger sync for a manga (push local, pull remote, merge)
- [ ] **API-12**: `GET /track/library` — get tracking status for all library mangas (summary view)

### Frontend: Lua UI

- [ ] **UI-01**: Settings page has a "Tracking" section listing available tracker services
- [ ] **UI-02**: Each tracker service shows login status with connect/disconnect action
- [ ] **UI-03**: Connect flow shows QR code (and text URL fallback) for OAuth authorization
- [ ] **UI-04**: Connect flow accepts OAuth token/code input after phone authorization
- [ ] **UI-05**: Manga info view shows tracking section per bound tracker (status, progress, score)
- [ ] **UI-06**: "Track" button on manga info opens search dialog to find manga on tracker
- [ ] **UI-07**: Track search dialog shows results from tracker, allows selection and binding
- [ ] **UI-08**: Bound manga shows sync status with manual sync button
- [ ] **UI-09**: Library view optionally shows tracking badges (manga bound to tracker)
- [ ] **UI-10**: Reading a chapter automatically triggers sync for bound mangas
- [ ] **UI-11**: Error states displayed when sync fails (network, auth expired, etc.)

### QR Authentication

- [ ] **QR-01**: Backend generates QR-code-compatible OAuth URLs (using `qrcode` crate or similar)
- [ ] **QR-02**: Backend encodes OAuth URL as PNG image for display on e-ink
- [ ] **QR-03**: Frontend displays QR code image in a centered dialog
- [ ] **QR-04**: Fallback: display plain text OAuth URL alongside QR code
- [ ] **QR-05**: Auth completion flow with manual token/code input
- [ ] **QR-06**: MAL PKCE flow: code verifier tied to session, auth URL encodes challenge

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Additional Trackers

- **Kitsu tracker** — Same pattern, Kitsu has public REST API
- **Bangumi tracker** — Chinese market tracker
- **Shikimori tracker** — Russian market tracker

### Enhanced Sync

- **Library-wide batch sync** — Sync all tracked mangas in one operation
- **Auto-sync on library refresh** — Sync tracking data when refreshing library
- **Delayed sync queue** — Queue failed syncs and retry on next app launch
- **Conflicts UI** — Show sync conflicts for user resolution

### Features

- **Auto-track on manga add** — Automatically bind manga to tracker if title matches
- **Sync on chapter download** — Trigger sync even without reading (for pre-downloaded chapters)
- **Multi-user** — Switch between multiple tracker accounts

## Out of Scope

| Feature | Reason |
|---------|--------|
| Kitsu/Bangumi/Shikimori/Komga trackers | v1 focuses on AniList + MAL (95%+ of users); others later |
| Automatic background periodic sync | e-ink battery constraints; v1 uses manual + on-read |
| Library-wide batch sync | Adds complexity; per-manga sync sufficient for v1 |
| EnhancedTracker (auto-match) | Requires source-specific mapping; v1 uses manual search+bind |
| OAuth via embedded WebView | No browser engine available on e-ink; QR auth is the correct solution |
| Sync conflict resolution UI | v1 uses max-take strategy (simplest merge); manual resolution deferred |
| Comments/social features | Outside scope of a reader plugin |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DB-01, DB-02, DB-03, DB-04 | Phase 1 | Pending |
| API-01, API-02, API-03, API-04, API-05 | Phase 1 | Pending |
| AL-01, AL-02, AL-03 | Phase 1 | Pending |
| ML-01, ML-02, ML-03 | Phase 1 | Pending |
| QR-01, QR-02, QR-04, QR-05, QR-06 | Phase 1 | Pending |
| QR-03 | Phase 3 | Pending |
| AL-04, AL-05, AL-06, AL-07, AL-08, AL-09 | Phase 2 | Pending |
| ML-04, ML-05, ML-06, ML-07, ML-08, ML-09 | Phase 2 | Pending |
| API-06, API-07, API-08, API-09, API-10, API-11, API-12 | Phase 2 | Pending |
| SYNC-01, SYNC-02, SYNC-03, SYNC-04, SYNC-05, SYNC-06 | Phase 2 | Pending |
| UI-01, UI-02, UI-03, UI-04 | Phase 3 | Pending |
| UI-05, UI-06, UI-07, UI-08, UI-09, UI-10, UI-11 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 39 total
- Mapped to phases: 39
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-28*
*Last updated: 2026-06-28 after initial definition*
