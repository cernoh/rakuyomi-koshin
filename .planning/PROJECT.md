# RakuYomi

## What This Is

RakuYomi is a manga reader plugin for KOReader on e-ink devices (Kindle, Kobo, Android). It provides a Rust HTTP backend with WASM/JS source support and a Lua-based frontend UI within KOReader, enabling manga browsing, downloading, and reading from community-maintained sources.

## Core Value

Users can browse, download, and read manga from any supported source directly on their e-ink device with a KOReader-native interface.

## Requirements

### Validated

- ✓ Manga browsing and search from Aidoku sources — initial release
- ✓ Chapter download and offline reading — initial release
- ✓ Last-read tracking per manga/chapter — initial release
- ✓ Source management (install/uninstall) — initial release
- ✓ Multi-platform support (Kindle, Kobo, Android, Linux) — initial release
- ✓ Library management (add/remove, sort by last read) — initial release
- ✓ Chapter read state management — initial release
- ✓ Notification system for updates — after initial release
- ✓ Playlist support — after initial release

### Active

- **[TRACK-01]** Integration with AniList for reading progress tracking and status syncing
- **[TRACK-02]** Integration with MyAnimeList for reading progress tracking and status syncing
- **[TRACK-03]** QR-code-based OAuth authentication suitable for e-ink devices
- **[TRACK-04]** Two-way sync of reading progress between local state and tracker services
- **[TRACK-05]** Visual tracking status in manga info and library views

### Out of Scope

- Kitsu/Bangumi/Shikimori trackers — future milestone, start with the two most popular
- Automatic periodic background sync — deferred; v1 uses manual sync + chapter-read trigger
- Batch sync across all mangas — v1 syncs per-manga; full library sync deferred
- Social features (comments, forums, friends) — outside RakuYomi's scope as a reader

## Context

RakuYomi currently has no tracking integration. Users on Kindles/Kobos have no way to sync their reading progress to AniList or MyAnimeList, which are the two most widely used manga tracking services. This feature is consistently requested by the community.

The architecture follows the existing pattern: Rust backend (axum HTTP server) handles API communication with tracker services, SQLite stores credentials and track mappings, Lua frontend provides the UI within KOReader.

E-ink devices are the primary target, which means OAuth authentication must work without a browser on the device itself. QR-code-based auth solves this: the device displays a QR code, the user scans it with their phone, completes auth on the phone, and the credentials are relayed back.

Mihon's tracker implementation serves as the reference for API contracts, status mappings, and sync logic. However, RakuYomi's implementation will be in Rust (not Kotlin) and will communicate with the Lua frontend via the existing JSON HTTP API.

## Constraints

- **Tech stack**: Rust backend (axum/tokio), Lua/LuaJIT frontend (KOReader plugin), SQLite for persistence
- **Platform**: e-ink primary target (Kindle, Kobo) — OOB browser is not available for OAuth
- **API compatibility**: Must use public OAuth clients for AniList (client_id=16329) and MAL (client_id=c46c9e24640a64dad5be5ca7a1a53a0f)
- **Rate limits**: AniList: 85 req/min; MAL: undocumented but must be respectful
- **Network**: Device must have internet access for sync to work; offline reading state is queued for next sync
- **Auth**: OAuth tokens stored in SQLite; tokens expire and must be refreshed (MAL supports refresh tokens, AniList does not)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust backend for tracker API calls | Existing axum pattern; no need for Lua HTTP client complexity | ✓ Good |
| SQLite for credential storage | Existing DB infrastructure, encrypted at filesystem level | ✓ Good |
| QR-code auth for e-ink | No browser available on Kindle/Kobo; phone-as-browser is standard pattern | — Pending |
| Per-manga sync on chapter read | Progressive sync avoids bursts; matches Mihon's pattern | — Pending |
| Mihon API contracts as reference | Well-established, community-tested status/score mappings | ✓ Good |
| No automatic background sync | e-ink devices have constrained battery; manual + trigger-based is appropriate | ✓ Good |

---
*Last updated: 2026-06-28 after milestone Tracking Integration kickoff*
