## Project Reference

See: .planning/PROJECT.md (updated 2026-06-28)

**Core value:** Reliably browse, search, download, and read manga from any source on any device KOReader runs on.
**Current focus:** v2.0 External Tracker Sync — milestone planning

## Active Milestone

**Name:** v2.0 External Tracker Sync
**Goal:** Integrate AniList and MyAnimeList tracking — OAuth2 authentication, push reading progress, reading status, and scores to connected tracker accounts.
**Phase:** Planning (pre-Phase 1)
**Started:** 2026-06-28

## Phase Progress

| Phase | Status |
|-------|--------|
| Planning | In Progress |
| (phases defined in ROADMAP.md) | — |

## Key Context

- Push-first: reading progress on chapter completion, status/scores on user action
- AniList: GraphQL, OAuth2, no PKCE
- MyAnimeList: REST v2, OAuth2 with PKCE
- OAuth2 flow: Rust localhost redirect server catches callback
- Tokens stored in SQLite

---
*Last updated: 2026-06-28 after milestone init*
