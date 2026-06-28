## Project Reference

See: .planning/PROJECT.md (updated 2026-06-28)

**Core value:** Users can browse, download, and read manga from any supported source directly on their e-ink device with a KOReader-native interface.

**Current focus:** Milestone — Tracking Integration (AniList + MyAnimeList)

## Active Milestone

### Tracking Integration: v1

Add AniList and MyAnimeList tracking support to RakuYomi, enabling two-way sync of reading progress and status. QR-code-based OAuth authentication for e-ink devices.

**Phase plan:** ROADMAP.md

### Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | Backend Foundation — DB + Auth + QR | Planned |
| 2 | Tracker API Integration + Sync Engine | Planned |
| 3 | Frontend — Lua UI Integration | Planned |

## Session: Phase 1 Context Discussion
**2026-06-28** — Discussed DB schema, OAuth architecture, PKCE state management, QR delivery.
Decisions captured in `.planning/phases/01-backend-foundation-database-schema-auth-qr/01-CONTEXT.md`.

### Current Phase

Phase 1 — Context gathered, ready for planning.

### Next Action

Run `/gsd-plan-phase 1` to create execution plan for Phase 1.
