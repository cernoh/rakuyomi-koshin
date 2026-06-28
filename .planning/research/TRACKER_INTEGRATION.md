# Research: External Tracker Integration

**Date:** 2026-06-28
**Milestone:** v2.0 External Tracker Sync
**Targets:** AniList, MyAnimeList (MAL)

---

## 1. Executive Summary

Two-thirds of manga readers use at least one tracking service (AniList or MyAnimeList). This research covers both APIs for integrating read-progress, reading-status, and score sync into RakuYomi.

**Key constraint:** KOReader's webview can load and display a page but cannot run JavaScript or intercept URL redirects. The OAuth2 flow must use an embedded localhost HTTP server (in the Rust backend) to catch the redirect callback.

---

## 2. AniList API

### Overview

- **API format:** GraphQL — single endpoint `https://graphql.anilist.co`
- **API version:** v2 (current)
- **Documentation:** https://docs.anilist.co

### Authentication

- **Protocol:** OAuth2 Authorization Code Grant
- **App registration:** https://anilist.co/settings/developer
- **No scopes** — full access after auth
- **PKCE:** Not required
- **Auth URL:** `https://anilist.co/api/v2/oauth/authorize?client_id={id}&redirect_uri={uri}&response_type=code`
- **Token URL:** `POST https://anilist.co/api/v2/oauth/token` (form-encoded body: `grant_type, client_id, client_secret, redirect_uri, code`)
- **Token lifetime:** 1 year
- **Refresh tokens:** NOT SUPPORTED — user must re-authenticate after expiry
- **Fallback:** Auth PIN flow available

### Authorization Header

```
Authorization: Bearer {access_token}
Content-Type: application/json
Accept: application/json
```

### Rate Limits

| Tier | Limit | Source |
|------|-------|--------|
| Standard | 90 req/min | Official docs |
| Degraded | 30 req/min | Current state (2026) |
| Burst | N/A | Not specified |

Headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
429 response includes `Retry-After`.

### Key Queries

**Get current user:**
```graphql
query {
  Viewer { id name }
}
```

**Search manga:**
```graphql
query ($search: String!) {
  Page(page: 1, perPage: 50) {
    pageInfo { hasNextPage }
    media(search: $search, type: MANGA) {
      id
      title { romaji english }
      chapters
      volumes
      status
      coverImage { large }
      format
    }
  }
}
```

**Get manga details (with user's list entry):**
```graphql
query ($id: Int!) {
  Media(id: $id, type: MANGA) {
    id
    title { romaji english }
    chapters
    volumes
    status
    mediaListEntry {
      id status progress score
    }
  }
}
```

**Get user's complete manga list:**
```graphql
query ($userId: Int!, $chunk: Int, $perChunk: Int) {
  MediaListCollection(userId: $userId, type: MANGA, chunk: $chunk, perChunk: $perChunk) {
    lists {
      name
      isCustomList
      entries {
        id
        mediaId
        status
        score
        progress
        progressVolumes
        media { id title { romaji } chapters }
      }
    }
  }
}
```

### Key Mutation

```graphql
# Create or update a media list entry
mutation ($mediaId: Int!, $status: MediaListStatus, $progress: Int, $scoreRaw: Int) {
  SaveMediaListEntry(mediaId: $mediaId, status: $status, progress: $progress, scoreRaw: $scoreRaw) {
    id
    status
    progress
    score
  }
}
```

### Status Mapping

| AniList Status | Meaning |
|----------------|---------|
| `CURRENT` | Reading |
| `COMPLETED` | Completed |
| `DROPPED` | Dropped |
| `PAUSED` | On Hold |
| `PLANNING` | Plan to Read |
| `REPEATING` | Rereading |

### Score System

- `scoreRaw`: 0-100 integer (submitted via API)
- Displayed as 0-10 or 0-100 depending on user preference
- Unscored = null or 0

### Manga-Only Filtering

AniList uses unified IDs across anime and manga. **Always** include `type: MANGA` in queries, otherwise anime results may shadow manga.

### Rust Strategy

Use `reqwest` + `serde_json` directly — both already in workspace dependencies. No GraphQL crate needed. Module structure:

```
backend/shared/src/tracker/
├── mod.rs
├── anilist/
│   ├── mod.rs          # AnilistClient struct, constructor
│   ├── auth.rs         # OAuth2 flow (auth URL, token exchange)
│   ├── queries.graphql # GraphQL query/mutation strings (const)
│   ├── model.rs        # Response types (serde::Deserialize)
│   └── api.rs          # API methods (search, get_list, update_entry)
├── mal/
│   └── ...
├── oauth.rs            # Shared OAuth2 redirect HTTP server
└── storage.rs          # Token persistence (SQLite)
```

### Pitfalls

1. No refresh tokens — re-auth after 1 year
2. Anime/manga ID collision — always `type: MANGA`
3. `PageInfo.total` unreliable — use `hasNextPage`
4. Custom lists may hide entries from default status lists
5. API errors may appear on HTTP 200 — always check `errors` array
6. 11K entry limit on collection

---

## 3. MyAnimeList API

### Overview

- **API format:** REST v2
- **Base URL:** `https://api.myanimelist.net/v2`
- **API version:** v2 (beta)
- **Documentation:** https://myanimelist.net/apiconfig/references/api/v2
- **v1:** Fully decommissioned

### Authentication

- **Protocol:** OAuth2 Authorization Code Grant **with PKCE**
- **PKCE method:** `plain` ONLY (no S256 support) — `code_challenge` = `code_verifier`
- **Scope:** `write:users` (the only scope available)
- **App registration:** https://myanimelist.net/apiconfig
- **Auth URL:** `https://myanimelist.net/v1/oauth2/authorize?response_type=code&client_id={id}&state={state}&redirect_uri={uri}&code_challenge={challenge}&code_challenge_method=plain`
- **Token URL:** `POST https://myanimelist.net/v1/oauth2/token`
  - Two auth modes: HTTP Basic (client_id:client_secret) OR body-only
- **Access token lifetime:** 1 hour
- **Refresh token lifetime:** ~1 month
- **Token size:** ~1KB each

### Token Exchange

```http
POST https://myanimelist.net/v1/oauth2/token
Content-Type: application/x-www-form-urlencoded

client_id={id}&client_secret={secret}&grant_type=authorization_code&code={code}&redirect_uri={uri}&code_verifier={verifier}
```

### Token Refresh

```http
POST https://myanimelist.net/v1/oauth2/token
Content-Type: application/x-www-form-urlencoded
Authorization: Basic {base64(client_id:client_secret)}

grant_type=refresh_token&refresh_token={token}
```

### Authorization Header

```
X-MAL-CLIENT-ID: {client_id}    # Read-only (search, details)
Authorization: Bearer {token}    # Read/write (user list operations)
```

### Rate Limits

- **Unpublished** — MAL does not document rate limits
- **Community practice:** ~1 request/second
- **Throttle signal:** 403 response (not 429)
- **Strategy:** Exponential backoff on 403

### Key Endpoints

**Search manga:**
```
GET /v2/manga?q={query}&limit={1-100}&offset={0}&fields={...}
```
Auth: client_auth (X-MAL-CLIENT-ID) or main_auth (Bearer)

**Get manga details:**
```
GET /v2/manga/{manga_id}?fields={...}
```
Auth: client_auth or main_auth

**Get user's manga list:**
```
GET /v2/users/@me/mangalist?status={...}&sort={...}&limit={1-1000}&offset={0}&fields={...}
```
Auth: client_auth or main_auth
Fields: `list_status{status,score,num_chapters_read,num_volumes_read}`

**Update manga list entry:**
```
PATCH /v2/manga/{manga_id}/my_list_status
Content-Type: application/x-www-form-urlencoded

status=reading&score=8&num_chapters_read=15
```
Auth: main_auth ONLY
Returns: `{ status, score, num_chapters_read, num_volumes_read, updated_at }`

**Delete from list:**
```
DELETE /v2/manga/{manga_id}/my_list_status
```
Returns 404 if not in list — this is normal, NOT an error condition.

### Status Mapping

| MAL Status | Meaning |
|------------|---------|
| `reading` | Reading |
| `completed` | Completed |
| `on_hold` | On Hold |
| `dropped` | Dropped |
| `plan_to_read` | Plan to Read |

### Score System

- Integer 0-10 (0 = not scored)
- Unlike AniList's 0-100 scale

### Fields System

MAL v2 does NOT return all fields by default. Use the `fields` parameter:

```
?fields=id,title{romaji,english},main_picture,chapters,volumes,status,my_list_status{status,score,num_chapters_read}
```

Nested fields work with `{}` syntax.

### Rust Strategy

**Option A:** Use the `mal-api` crate (dobecad/mal-rs v2.0.3) — type-safe builders, OAuth2/PKCE support, token persistence.
**Option B:** Implement manually with `reqwest` + `oauth2` crate for PKCE — lighter dependency, more control.

Given the PKCE `plain` requirement and the need to integrate with existing `reqwest` infrastructure, a manual approach (Option B) using `reqwest` + the `oauth2` crate (v5) for PKCE challenge generation is recommended.

### Pitfalls

1. PKCE `plain` only — `code_challenge` = `code_verifier` verbatim (do NOT hash)
2. `fields` parameter REQUIRED — no default fields
3. DELETE returns 404 if not in list — non-error
4. Rate limits unpublished — use conservative pacing (~1 req/s)
5. Score 0-10 (not 0-100 like AniList) — needs mapping
6. NSFW content excluded by default — add `?nsfw=true` to include
7. Token refresh revokes OLD access token immediately, old refresh token valid until expiry
8. Official examples show PUT but PATCH is semantically correct for partial updates

---

## 4. Cross-Cutting Concerns

### OAuth2 Design (Both Services)

KOReader webview limitation: no JS execution, no URL interception.

**Proposed flow:**
1. User selects "Connect to [Tracker]" in settings
2. RakuYomi Lua frontend requests `/tracker/{name}/auth-url` from backend
3. Rust backend generates auth URL (with PKCE challenge for MAL) and starts a localhost HTTP server on a random port
4. Lua opens KOReader webview to the auth URL
5. User authorizes (logs in on tracker's site)
6. Tracker redirects to `http://127.0.0.1:{port}/callback?code=...`
7. Rust backend's local server receives the callback, exchanges the code for tokens
8. Lua polls `/tracker/{name}/status` until authenticated (or timeout)
9. Backend stores tokens in SQLite

**Alternative (fallback):** Auth PIN flow — AniList supports this. MAL does not.

### Token Storage

SQLite in `tracker_tokens` table:
```sql
CREATE TABLE tracker_tokens (
    service TEXT PRIMARY KEY,     -- 'anilist' | 'myanimelist'
    access_token TEXT NOT NULL,
    refresh_token TEXT,           -- NULL for AniList (no refresh)
    token_type TEXT,
    expires_at INTEGER,           -- Unix timestamp
    scope TEXT,
    created_at INTEGER,
    updated_at INTEGER
);
```

### Manga Title Matching

A manga in RakuYomi's library may not have the same title on the tracker. Options:
1. **Manual matching**: User searches and selects the manga on the tracker
2. **Automatic matching**: Match by title (romaji/english), confirm if ambiguous
3. **Source-provided tracker ID**: Some Aidoku sources include MAL/AniList IDs in metadata

Recommended: Start with automatic matching (best-effort by title), fall back to manual search dialog.

### Score Mapping

| System | Range | Notes |
|--------|-------|-------|
| AniList | 0-100 (scoreRaw) | Submit as 0-100 integer |
| MAL | 0-10 | Submit as 0-10 integer |
| RakuYomi | None yet | Need to decide: 0-10 (simple) or 0-100 (AniList native) |

---

## 5. Recommended Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Lua Frontend (KOReader Plugin)                              │
│                                                              │
│  TrackerManager.lua ─── TrackerAuthDialog.lua                │
│       │                                                      │
│       │ HTTP/JSON (Backend.requestJson)                      │
│       ▼                                                      │
│  Rust Backend (axum server)                                  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  tracker/  (new module)                              │    │
│  │                                                      │    │
│  │  ├── mod.rs          ── TrackerService (state)       │    │
│  │  ├── oauth.rs        ── OAuth2 redirect server      │    │
│  │  ├── storage.rs      ── SQLite token persistence    │    │
│  │  ├── anilist/                                        │    │
│  │  │   ├── mod.rs      ── AnilistClient               │    │
│  │  │   ├── auth.rs     ── Auth URL + token exchange   │    │
│  │  │   ├── api.rs      ── Query/mutation methods      │    │
│  │  │   └── model.rs    ── GraphQL response types      │    │
│  │  ├── mal/                                           │    │
│  │  │   ├── mod.rs      ── MalClient                   │    │
│  │  │   ├── auth.rs     ── Auth URL + PKCE exchange    │    │
│  │  │   ├── api.rs      ── REST API methods            │    │
│  │  │   └── model.rs    ── JSON response types         │    │
│  │  └── routes.rs       ── axum route definitions      │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### Route Design

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/tracker/{service}/auth-url` | Get OAuth2 URL + start redirect listener |
| GET | `/tracker/{service}/status` | Check if auth completed |
| POST | `/tracker/{service}/disconnect` | Remove stored tokens |
| POST | `/manga/{id}/tracker-sync` | Push progress for a single manga |
| POST | `/tracker/sync-all` | Bulk push all tracked manga |
| GET | `/tracker/{service}/search?q=` | Search manga on tracker for matching |
| POST | `/manga/{id}/tracker-link` | Link RakuYomi manga to tracker media |
| GET | `/tracker/status` | Get auth status for all services |

---

*Research compiled: 2026-06-28*
*Sources: AniList API docs (docs.anilist.co), MAL API docs (myanimelist.net/apiconfig), mal-rs (dobecad), mal-api-rs (MolotovCherry), mal-cli-rs (L4z3x)*
