# Research: Mihon Tracker Architecture

**Source**: https://github.com/mihonapp/mihon
**Date**: 2026-06-28
**Purpose**: Inform RakuYomi tracker feature design (AniList + MyAnimeList)

## Architecture Overview

Mihon implements trackers as a pluggable service layer with a common interface (`Tracker`), shared base class (`BaseTracker`), and per-service implementations. The system is designed for **two-way sync** of reading progress and manga list status between the local app and remote tracking services.

### Core Interface: `Tracker`

```
app/src/main/java/eu/kanade/tachiyomi/data/track/Tracker.kt
```

Key methods:

| Method | Purpose |
|--------|---------|
| `update(track, didReadChapter)` | Push local changes → remote (with auto-status logic) |
| `bind(track, hasReadChapters)` | First-time link: merge remote → local or create new entry |
| `search(query)` | Find manga on the tracker service |
| `refresh(track)` | Pull remote → local (overwrite local with remote state) |
| `login(username, password)` | Authenticate (OAuth token exchange) |
| `logout()` | Clear credentials |
| `getStatusList()` | Available reading statuses per service |
| `getScoreList()` | Available score formats per service |

### Track Data Model

```
tachiyomi/domain/track/model/Track.kt (domain)
eu.kanade.tachiyomi.data.database.models.Track (DB model)
```

| Field | Type | Purpose |
|-------|------|---------|
| `id` | Long | Local DB primary key |
| `manga_id` | Long | FK to local manga |
| `tracker_id` | Long | Which tracker service |
| `remote_id` | Long | Manga ID on the remote service |
| `library_id` | Long | User's list entry ID on remote |
| `title` | String | Manga title on the tracker |
| `last_chapter_read` | Double | Last read chapter number |
| `total_chapters` | Long | Total chapters from tracker |
| `status` | Long | Reading status (1=Reading, 2=Completed, etc.) |
| `score` | Double | User's score |
| `started_reading_date` | Long | Epoch millis |
| `finished_reading_date` | Long | Epoch millis |
| `tracking_url` | String | URL to tracker page |
| `private` | Boolean | Private tracking flag |

### Status Constants (shared across services)

```
READING = 1
COMPLETED = 2
ON_HOLD = 3
DROPPED = 4
PLAN_TO_READ = 5
REREADING = 6
```

### BaseTracker: Shared Logic

`BaseTracker` provides:
- Credential storage via preferences (trackUsername/trackPassword)
- `isLoggedIn` getter + Flow for reactive UI
- `updateRemote(track)` — generic PATCH to persist to DB after API calls
- Default implementations for `register`, `setRemoteStatus`, `setRemoteScore`, `setRemoteStartDate`, `setRemoteFinishDate`, `setRemotePrivate`, `setRemoteLastChapterRead`

### Extensions

- **EnhancedTracker** — For services that auto-match manga by source ID (no manual binding needed). Has `match(manga)` to find the entry without search.
- **DeletableTracker** — Services that support deleting entries from user list (AniList, MAL both implement this).

## AniList Implementation

| File | Purpose |
|------|---------|
| `Anilist.kt` | Main class: status/score lists, login/logout, OAuth persistence |
| `AnilistApi.kt` | GraphQL client: search, add/update/delete/list manga |
| `AnilistInterceptor.kt` | OkHttp interceptor: adds Bearer token, handles expiry |
| `AnilistUtils.kt` | Status/score mapping between local ↔ API formats |
| `dto/ALOAuth.kt` | OAuth token response DTO |
| `dto/ALManga.kt`, `ALUserList.kt`, etc. | GraphQL response DTOs |

### AniList OAuth Flow

- **Grant type**: Implicit (token in URL fragment)
- **Auth URL**: `https://anilist.co/api/v2/oauth/authorize?client_id=16329&response_type=token`
- **Client ID**: `16329` (public, hardcoded)
- **Token exchange**: Token arrives as URL fragment, stored directly
- **Token expiry**: Checked via `expires` field in ALOAuth (1 year), but no refresh token
- **API**: GraphQL at `https://graphql.anilist.co/`
- **Rate limit**: 85 requests/minute

### AniList Key APIs (GraphQL)

- **Search manga**: `Page.media(search:, type:MANGA)` — returns id, title, cover, status, etc.
- **Add to list**: `SaveMediaListEntry(mediaId:, progress:, status:, private:)`
- **Update list entry**: `SaveMediaListEntry(id:, progress:, status:, scoreRaw:, startedAt:, completedAt:, private:)`
- **Get user list**: `MediaListCollection(userId:, type:MANGA)` — returns all entries
- **Get current user**: `Viewer` query — returns id + name + media list options

### AniList Sync Logic (`update`)

1. Check if remote entry exists via `findLibManga`
2. If reading and `didReadChapter` is true:
   - Auto-set status to READING if not already
   - Set `started_reading_date` if first chapter
   - Auto-set COMPLETED + `finished_reading_date` if last chapter reached
3. Push update via GraphQL mutation

## MyAnimeList Implementation

| File | Purpose |
|------|---------|
| `MyAnimeList.kt` | Main class: status/score lists, OAuth persistence, auth expiry |
| `MyAnimeListApi.kt` | REST client: OAuth token exchange, CRUD for list entries |
| `MyAnimeListInterceptor.kt` | OkHttp interceptor: Bearer token, refresh token rotation |
| `MyAnimeListUtils.kt` | Status/score mapping |
| `dto/MALOAuth.kt` | OAuth token + refresh token DTO |
| `dto/MALManga.kt`, `MALList.kt` | REST response DTOs |

### MAL OAuth Flow

- **Grant type**: Authorization code with PKCE
- **Auth URL**: `https://myanimelist.net/v1/oauth2/authorize?client_id=c46c9e24640a64dad5be5ca7a1a53a0f&response_type=code&code_challenge=...`
- **Client ID**: `c46c9e24640a64dad5be5ca7a1a53a0f` (public, hardcoded)
- **PKCE**: Code verifier generated via `PkceUtil` (crypto-random base64url, 50 bytes)
- **Token exchange**: POST to `https://myanimelist.net/v1/oauth2/token` with `grant_type=authorization_code` + `code` + `code_verifier`
- **Refresh tokens**: Supported! `refresh_token` stored, auto-refresh via interceptor
- **Token expiry**: OAuth response includes `expires_in`, checked in interceptor
- **API**: REST at `https://api.myanimelist.net/v2`

### MAL Auth Management

- `saveOAuth(MALOAuth)` — persists OAuth DTO to preferences
- `loadOAuth()` — deserializes from preferences
- `getIfAuthExpired()` / `setAuthExpired()` — flag for expired refresh token
- Interceptor auto-refreshes on 401 via `refreshToken()` method
- `MALTokenExpired` / `MALTokenRefreshFailed` — typed exceptions

### MAL Key APIs (REST)

- **Search manga**: `GET /manga?q={query}&fields=id,title,...`
- **Get manga details**: `GET /manga/{id}?fields=...`
- **Update list entry**: `PATCH /manga/{id}/my_list_status` — status, score, num_chapters_read, priority, etc.
- **Delete list entry**: `DELETE /manga/{id}/my_list_status`
- **Get user list**: `GET /users/@me/mangalist?offset={offset}&limit={limit}&status={status}`
- **Token refresh**: `POST /v1/oauth2/token` with `grant_type=refresh_token`

### MAL Sync Logic

Same pattern as AniList but uses REST instead of GraphQL. Supports `start_date` and `finish_date` via ISO date conversion.

## Two-Way Sync Architecture

Mihon implements two-way sync through these components:

1. **`TrackChapter`** (write path): When user reads a chapter → tracked manga found → `refresh()` to get latest remote → `update()` to push new chapter count → `insertTrack()` to persist.

2. **`RefreshTracks`** (read path): Fetch latest from all trackers → `refresh()` each → `syncChapterProgressWithTrack()` to backfill local chapters.

3. **`SyncChapterProgressWithTrack`** (conflict resolution):
   - Takes the MAX of remote and local last chapter read
   - Marks local chapters as read if the remote has them ahead
   - Only for `EnhancedTracker` implementations (auto-matched series)

4. **`DelayedTrackingStore`** + **`DelayedTrackingUpdateJob`**: Failed syncs are retried later via WorkManager.

## UI Flow (Android)

1. **Track settings**: List of tracker services with login/logout
2. **Manga info dialog**: "Tracking" section shows linked entries per tracker
3. **Track search dialog**: Search remote service → select manga → bind
4. **Auto-tracking**: Can be configured to auto-track on manga add
5. **Delayed updates**: Background job retries failed syncs

## Key Differences: AniList vs MyAnimeList

| Aspect | AniList | MyAnimeList |
|--------|---------|-------------|
| API | GraphQL | REST |
| Auth | Implicit grant (token in URL fragment) | Auth code + PKCE |
| Refresh token | No | Yes (auto-rotating) |
| Rate limit | 85 req/min | Undocumented |
| Score types | 5pt, 10pt, 100pt decimal, smiley | 10pt integer |
| Reading dates | Yes | Yes (via ISO dates) |

## Implications for RakuYomi

Since RakuYomi uses Rust async (tokio/axum) + Lua frontend, the implementation differs architecturally:

- **Backend (Rust)**: HTTP endpoints following existing route pattern (`/manga/`, `/settings/`, etc.)
- **OAuth state**: Stored in SQLite (like other settings), not Android preferences
- **Sync trigger**: From Lua via HTTP requests (not Android broadcast receivers) — user-initiated or on chapter read completion
- **QR auth**: Since Kindles/e-readers lack browser, generate OAuth URL → render as QR code → user scans with phone → completes auth
- **No reactive streams**: Mihon uses Kotlin Flow for reactive auth state; RakuYomi uses request-based pattern

## QR Code Auth Strategy for e-Ink

The QR flow works as an alternative to browser-based OAuth:
1. Backend generates the OAuth authorization URL (AniList implicit or MAL PKCE)
2. Server returns URL and a short-lived session token
3. URL is encoded as QR on the e-ink screen
4. User scans QR with phone → opens browser → authorizes
5. **For AniList (implicit)**: Token arrives in redirect fragment — phone can't send back directly. Solution: Server provides a pairing code; user enters the displayed token returned after auth.
6. **For MAL (PKCE)**: Similar — auth code in redirect. Solution: Use a cloud callback or manual code entry.
7. **Preferred approach**: Device-code style: server creates a session, QR encodes a URL to a middleman page that collects the token/code; server polls for completion.
