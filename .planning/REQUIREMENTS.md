# Requirements: RakuYomi

**Defined:** 2026-06-28
**Core Value:** Reliably browse, search, download, and read manga from any source on any device KOReader runs on.

## v2.0 Requirements — External Tracker Sync

Requirements for milestone v2.0. Each maps to roadmap phases.

### Authentication & Connections

- [ ] **AUTH-01**: User can initiate OAuth2 flow for AniList from settings
- [ ] **AUTH-02**: User can initiate OAuth2 flow for MyAnimeList from settings
- [ ] **AUTH-03**: AniList OAuth2 completes via KOReader webview + backend localhost redirect server
- [ ] **AUTH-04**: MyAnimeList OAuth2 completes via KOReader webview + backend localhost redirect server (with PKCE plain)
- [ ] **AUTH-05**: OAuth tokens are persisted in SQLite across app restarts
- [ ] **AUTH-06**: Expired tokens (MAL refresh) are automatically refreshed
- [ ] **AUTH-07**: User can disconnect/remove a tracker connection from settings
- [ ] **AUTH-08**: User can view connection status for each tracker in settings

### Manga Linking

- [ ] **LINK-01**: User can search AniList by manga title to link a library manga to a tracker ID
- [ ] **LINK-02**: User can search MyAnimeList by manga title to link a library manga to a tracker ID
- [ ] **LINK-03**: Linked tracker media ID is stored per manga in SQLite
- [ ] **LINK-04**: User can unlink a manga from its tracker entry
- [ ] **LINK-05**: Tracker-linked status is visible in manga info panel

### Progress Sync

- [ ] **SYNC-01**: Reading progress (latest read chapter number) is pushed to all connected trackers when a chapter is marked read
- [ ] **SYNC-02**: User can manually trigger a sync for a single manga from the manga info panel
- [ ] **SYNC-03**: User can manually trigger a bulk sync for all linked manga
- [ ] **SYNC-04**: Sync operations are non-blocking — shown as background jobs with progress
- [ ] **SYNC-05**: Rate limits are respected (AniList: 90 req/min, MAL: ~1 req/s) with retry/backoff

### Status & Scores

- [ ] **STAT-01**: User can set reading status per manga (Reading/Completed/Dropped/On Hold/Plan to Read)
- [ ] **STAT-02**: Reading status syncs to all connected trackers on save
- [ ] **STAT-03**: User can score a manga (integer 0-10) from the manga info panel
- [ ] **STAT-04**: Score syncs to all connected trackers (with scale conversion: RakuYomi 0-10 → AniList 0-100, MAL 0-10)
- [ ] **STAT-05**: Score display in UI reflects RakuYomi's internal scale (0-10)

## v2.1 Requirements (Deferred)

- **PULL-01**: User can import their tracker library into RakuYomi ('reading' entries appear in library)
- **PULL-02**: Two-way sync conflict resolution (tracker vs. local state)
- **PULL-03**: Scheduled/periodic sync (auto-push on chapter mark, periodic full sync)
- **PULL-04**: Tracker-specific settings per manga (e.g., sync AniList but not MAL for a specific title)
- **AUTO-01**: Automatic manga-to-tracker matching by title (no manual search needed when title matches)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Kitsu/MALgraph/other trackers | Only AniList and MAL have significant manga-reader user base. Adding more would triple the OAuth/API surface without proportional value. |
| Pull from trackers (library import) | Conflict resolution and UI for merging is significant scope. Deferred to v2.1. |
| Tachiyomi/TachiSYNC import | Different data model, would require bespoke format parsing. Only if user demand emerges. |
| Silent auto-sync without user confirmation | Must be explicit per-manga opt-in. No silent sync. |
| Native Android OAuth (Custom Tabs) | KOReader cross-platform model means one solution for all: webview + localhost redirect server. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUTH-01 | Phase 1 | Pending |
| AUTH-02 | Phase 1 | Pending |
| AUTH-03 | Phase 1 | Pending |
| AUTH-04 | Phase 1 | Pending |
| AUTH-05 | Phase 2 | Pending |
| AUTH-06 | Phase 2 | Pending |
| AUTH-07 | Phase 3 | Pending |
| AUTH-08 | Phase 3 | Pending |
| LINK-01 | Phase 2 | Pending |
| LINK-02 | Phase 2 | Pending |
| LINK-03 | Phase 2 | Pending |
| LINK-04 | Phase 3 | Pending |
| LINK-05 | Phase 3 | Pending |
| SYNC-01 | Phase 3 | Pending |
| SYNC-02 | Phase 3 | Pending |
| SYNC-03 | Phase 3 | Pending |
| SYNC-04 | Phase 3 | Pending |
| SYNC-05 | Phase 3 | Pending |
| STAT-01 | Phase 3 | Pending |
| STAT-02 | Phase 3 | Pending |
| STAT-03 | Phase 3 | Pending |
| STAT-04 | Phase 3 | Pending |
| STAT-05 | Phase 3 | Pending |

**Coverage:**
- v2.0 requirements: 24 total
- Mapped to phases: 24
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-28*
*Last updated: 2026-06-28 after initial definition*
