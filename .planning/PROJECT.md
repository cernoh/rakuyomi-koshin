# RakuYomi

## What This Is

RakuYomi is a manga reader plugin for KOReader. A Rust HTTP backend (axum server) loads manga from WASM-based sources (Aidoku ecosystem), manages downloads, and serves a SQLite-backed library. A Lua plugin frontend provides the KOReader-native UI over HTTP/JSON. Runs on e-readers (Kindle, Kobo), Android, and desktop Linux.

## Core Value

Reliably browse, search, download, and read manga from any source on any device KOReader runs on.

## Business Context

<!-- Deleted — open-source community project, not monetized. -->

## Requirements

### Validated

- [x] **V01**: Browse and search manga from Aidoku-compatible WASM sources
- [x] **V02**: Install/uninstall sources from repository
- [x] **V03**: Add manga to personal library
- [x] **V04**: Read manga chapters with cached chapter data
- [x] **V05**: Download chapters for offline reading (with batch & scanlator options)
- [x] **V06**: Track read/unread chapter state per manga
- [x] **V07**: Manage manga playlists
- [x] **V08**: ARIMA-based light novel release forecasting
- [x] **V09**: Notifications for source, download, and update events
- [x] **V10**: Cross-platform (Kindle, Kobo, Android, desktop Linux) with platform-appropriate transport (UDS, TCP, JNI)
- [x] **V11**: EPUB export and CBZ reading
- [x] **V12**: Server auto-update checking

### Active

<!-- Current scope. Building toward these. -->

- [ ] **TRAK-01**: User can authenticate with AniList via OAuth2 in KOReader webview
- [ ] **TRAK-02**: User can authenticate with MyAnimeList via OAuth2 in KOReader webview
- [ ] **TRAK-03**: Reading progress (chapter number) is pushed to connected trackers on chapter completion
- [ ] **TRAK-04**: User can set manga reading status (Reading/Completed/Dropped/On Hold/Plan to Read) from RakuYomi
- [ ] **TRAK-05**: User can score manga from RakuYomi and sync to trackers
- [ ] **TRAK-06**: User can view tracker status per manga in manga info panel
- [ ] **TRAK-07**: User can manage tracker connections (add/remove/re-auth) in settings

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- [Kitsu/MALgraph/other trackers] — AniList and MyAnimeList only. Other trackers lack user base or stable APIs to warrant the complexity. Revisit if community demand is clear.
- [Tracker library import (pull)] — Phase 2 goal. v2.0 focuses on pushing read state outward. Pulling tracker lists to populate library requires conflict resolution design.
- [Auto-tracking without user confirmation] — Must be explicit per-manga opt-in. No silent sync.

## Context

RakuYomi runs as a standalone Rust HTTP server (the backend) that the Lua KOReader plugin talks to. The server manages all data — library, chapters, settings — in SQLite via sqlx. Sources are WASM modules (`.aix` files from the Aidoku ecosystem) executed by wasmi. The Lua frontend is purely a UI layer: it issues HTTP requests and renders responses.

The project already supports 40+ locale translations, caches chapters to RAM/tmpfs on e-ink devices, cross-compiles for 6 targets, and uses semantic-release for versioning.

Tracker integration is the first external API integration the backend has ever done. Currently, the server has no HTTP client integration of its own (sources manage their own networking via the WASM runtime). This milestone introduces persistent OAuth credential storage, HTTP client infrastructure at the server level, and an OAuth2 webview flow on the Lua side.

The two target APIs differ significantly:
- **AniList** uses GraphQL with a single endpoint. OAuth2 with client credentials, no PKCE. Generous rate limits.
- **MyAnimeList** uses REST v2. Requires PKCE in OAuth2 flow. Stricter rate limits (30 req/min per client).

## Constraints

- **Runtime**: Server must work on ARM e-ink devices with 256MB RAM — no heavy OAuth2 client library, keep binary size reasonable.
- **No browser engine**: KOReader webview is basic — can load a page and let user interact, but no JS. OAuth2 flow must handle the redirect via polling a local server or intercepting the redirect URL.
- **Network**: e-readers may have intermittent WiFi. OAuth tokens must persist across sessions.
- **No async in Lua frontend**: Lua plugin uses synchronous HTTP requests (blocking). The OAuth flow must be designed as a multi-step dialog sequence.
- **Credential storage**: OAuth secrets (tokens, refresh tokens) must be stored securely relative to the device — SQLite is acceptable for this app (no hardware-backed keystore on Kindles).

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Push-first sync direction | Pull requires conflict resolution and UI for library merging. Push is simple and immediate value. | — Pending |
| OAuth2 via localhost redirect server | KOReader webview can't intercept redirects or run JS. A tiny HTTP server on the Rust side catches the redirect and closes the flow. | — Pending |
| Server-level HTTP client (reqwest) for trackers | Source WASM modules already use reqwest internally, but tracker API calls are server-initiated, not source-initiated. Adds ~200KB to binary. | — Pending |
| SQLite for OAuth token storage | No secure keystore available on e-ink devices. SQLite is already the DB backend — no new infrastructure needed. Tokens encrypted at rest optionally. | — Pending |

---
*Last updated: 2026-06-28 after v2.0 External Tracker Sync milestone init*
