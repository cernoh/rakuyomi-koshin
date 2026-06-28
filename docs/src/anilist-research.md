# Anilist API Research — Manga Tracking Integration

**Date:** 2026-06-28
**Purpose:** Design requirements for v2.0 External Tracker Sync (RakuYomi)
**API Version:** v2 (GraphQL)
**Base URL:** `https://graphql.anilist.co`
**Official Docs:** https://docs.anilist.co/
**Schema Explorer:** https://studio.apollographql.com/sandbox/explorer?endpoint=https://graphql.anilist.co

---

## 1. Authentication (OAuth2)

### 1.1 Flow Type: Authorization Code Grant (recommended for server apps)

AniList supports two OAuth2 flows:
- **Authorization Code Grant** — for server-based apps where client secret can be secured
- **Implicit Grant** — for client-side apps (browser, mobile) where secret cannot be stored
- **Auth Pin** — fallback when HTTP/custom URI redirects aren't possible; redirect URL set to `https://anilist.co/api/v2/oauth/pin`, user copies token manually

**Important:**
- **No scopes** — access tokens provide (almost) full access to a user's data
- **Tokens are long-lived** — remain valid for 1 year from issuance
- **No refresh tokens** — once expired, user must re-authenticate
- **No refresh flow** — periodic re-auth is the only option

### 1.2 Step-by-step Flow

#### Step 1: Register application
Go to https://anilist.co/settings/developer → "Create New Application"
- Required: name, redirect URI (any valid URI, including custom schemes)
- Receives: `client_id`, `client_secret`
- Note: Applications **cannot be deleted once created**

#### Step 2: Redirect user to authorize
```
GET https://anilist.co/api/v2/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code
```
Parameters:
- `client_id` — your application's client ID
- `redirect_uri` — must exactly match the registered URI
- `response_type` — `code` for authorization code grant, `token` for implicit

User approves → redirect back with `?code={authorization_code}`

#### Step 3: Exchange code for token
```
POST https://anilist.co/api/v2/oauth/token
Content-Type: application/json
Accept: application/json

{
  "grant_type": "authorization_code",
  "client_id": "{client_id}",
  "client_secret": "{client_secret}",
  "redirect_uri": "{redirect_uri}",
  "code": "{code}"
}
```
Response:
```json
{ "access_token": "eyJhbGciOiJSUzI1NiIs..." }
```
The token is a **JWT**. Decode it to get:
- `sub` — user ID
- `exp` — expiration timestamp (1 year)
- `iat` — issued at
- `name` — username

#### Step 4: Authenticated requests
```
POST https://graphql.anilist.co
Authorization: Bearer {access_token}
Content-Type: application/json
Accept: application/json
```

### 1.3 Auth Pin Flow
Set redirect URL to `https://anilist.co/api/v2/oauth/pin`. After auth, user lands on an Anilist page showing the token. They copy-paste it into your app. Works for both authorization code and implicit grant.

### 1.4 Implicit Grant
```
<a href="https://anilist.co/api/v2/oauth/authorize?client_id={client_id}&response_type=token">
```
Token returned in **URL fragment** (`#access_token=...`). Used when client can't securely store secrets.

---

## 2. GraphQL API

### 2.1 Endpoint
All requests are `POST` to `https://graphql.anilist.co`
Request body: `{ "query": "...", "variables": { ... } }`

### 2.2 Rate Limiting

| Limit | Headers | Behavior |
|-------|---------|----------|
| **90 req/min** (degraded: 30) | `X-RateLimit-Limit: 90` | Standard limit |
| | `X-RateLimit-Remaining: 59` | Requests left this window |
| Burst limiting | — | Prevents hammering in short periods |
| 429 response | `Retry-After: 30`, `X-RateLimit-Reset: 1502035959` | 1 minute timeout |

Raise requests: email `contact@anilist.co` (not currently accepting requests).

**Strategy for RakuYomi:** Most tracking operations are user-driven (one chapter = one mutation). Burst limiting won't be an issue. Cache media metadata locally to avoid repeated queries.

### 2.3 Error Handling
- Errors in `errors` array of response, even on HTTP 200
- Validation errors include `validation` object with field-level messages
- Score validation: "The score may not be greater than 100"
- API unavailable returns HTTP 403 with error message
- IP blocking returns custom message

### 2.4 Pagination
- Wrap queries in `Page` object
- `perPage` max: 50 (25 for some connection types)
- `pageInfo` has `hasNextPage` (reliable), `total`/`lastPage` are **unreliable**
- Only one data field per `Page` query

---

## 3. Key Queries for Manga Tracking

### 3.1 Get Viewer (current authenticated user)
```graphql
query {
  Viewer {
    id
    name
  }
}
```

### 3.2 Search Manga by Title
```graphql
query ($search: String!, $page: Int = 1, $perPage: Int = 20) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      hasNextPage
    }
    media(search: $search, type: MANGA) {
      id
      idMal
      title {
        romaji
        english
        native
      }
      format        # MANGA, NOVEL, etc.
      status        # RELEASING, FINISHED, etc.
      chapters
      volumes
      coverImage {
        large
      }
      mediaListEntry {    # requires auth; null if not on user's list
        id
        status
        progress
        score
      }
    }
  }
}
```

### 3.3 Get Media Details by ID
```graphql
query ($id: Int!) {
  Media(id: $id, type: MANGA) {
    id
    idMal
    title {
      romaji
      english
      native
    }
    format
    status        # RELEASING, FINISHED, HIATUS, CANCELLED, NOT_YET_RELEASED
    chapters
    volumes
    description
    startDate { year month day }
    endDate { year month day }
    genres
    averageScore
    meanScore
    popularity
    coverImage { large medium }
    mediaListEntry {
      id
      status
      progress
      progressVolumes
      score
    }
  }
}
```

### 3.4 Get User's Manga List (Full Collection)
```graphql
query ($userId: Int!, $type: MediaType = MANGA) {
  MediaListCollection(userId: $userId, type: $type) {
    lists {
      name           # "Current", "Planning", "Completed", etc.
      isCustomList
      entries {
        id           # list entry ID
        mediaId
        status       # CURRENT, PLANNING, COMPLETED, DROPPED, PAUSED, REPEATING
        score
        progress    # chapter progress
        progressVolumes
        notes
        repeat
        private
        hiddenFromStatusLists
        startedAt { year month day }
        completedAt { year month day }
        updatedAt
        media {
          id
          title { romaji english }
          chapters
          volumes
          format
          status
          coverImage { large }
        }
      }
    }
  }
}
```
**Warning:** Limited to 11,000 most recently updated entries. Use `chunk`/`perChunk` params (max 500) for large lists. **Always include custom lists** — users can hide entries from default status lists.

### 3.5 Get User's Manga List (Paginated)
```graphql
query ($userId: Int!, $status: MediaListStatus, $page: Int = 1, $perPage: Int = 50) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { hasNextPage }
    mediaList(userId: $userId, type: MANGA, status: $status) {
      id
      mediaId
      status
      score
      progress
      progressVolumes
      media {
        id
        title { romaji english }
        chapters
      }
    }
  }
}
```
Filter options: `status_in`, `status_not_in`, `mediaId_in`, `startedAt_greater`, `completedAt_greater`, etc.

### 3.6 Get Single Media List Entry
```graphql
query ($mediaId: Int!, $userId: Int!) {
  MediaList(mediaId: $mediaId, userId: $userId) {
    id
    status
    score
    progress
    progressVolumes
    repeat
    notes
    startedAt { year month day }
    completedAt { year month day }
    updatedAt
  }
}
```
Or from Media object (authenticated only):
```graphql
query ($mediaId: Int!) {
  Media(id: $mediaId, type: MANGA) {
    mediaListEntry {
      id
      status
      progress
      score
    }
  }
}
```

---

## 4. Key Mutations

### 4.1 Create/Update Media List Entry (SaveMediaListEntry)
```graphql
mutation (
  $id: Int          # list entry ID (omit to create, include to update)
  $mediaId: Int     # required for create
  $status: MediaListStatus
  $score: Float     # in user's chosen scoring method
  $scoreRaw: Int    # 0-100 scoring
  $progress: Int    # chapters read
  $progressVolumes: Int
  $repeat: Int
  $private: Boolean
  $notes: String
  $hiddenFromStatusLists: Boolean
  $customLists: [String]
  $startedAt: FuzzyDateInput
  $completedAt: FuzzyDateInput
) {
  SaveMediaListEntry(
    id: $id, mediaId: $mediaId, status: $status,
    score: $score, scoreRaw: $scoreRaw,
    progress: $progress, progressVolumes: $progressVolumes,
    repeat: $repeat, private: $private, notes: $notes,
    hiddenFromStatusLists: $hiddenFromStatusLists,
    customLists: $customLists,
    startedAt: $startedAt, completedAt: $completedAt
  ) {
    id
    status
    progress
    score
  }
}
```
**Usage:**
- **Create:** Omit `id`, provide `mediaId`
- **Update:** Provide `id` (the list entry ID)
- Returns the updated/created `MediaList` object

### 4.2 Delete Media List Entry
```graphql
mutation ($id: Int!) {
  DeleteMediaListEntry(id: $id) {
    deleted
  }
}
```

### 4.3 Batch Update (UpdateMediaListEntries)
```graphql
mutation ($ids: [Int], $status: MediaListStatus, $progress: Int, $scoreRaw: Int) {
  UpdateMediaListEntries(
    ids: $ids, status: $status,
    progress: $progress, scoreRaw: $scoreRaw
  ) {
    id
    status
    progress
  }
}
```

### 4.4 FuzzyDateInput
```json
{ "year": 2026, "month": 6, "day": 28 }
```

---

## 5. Enums

### MediaListStatus
| Value | Meaning |
|-------|---------|
| `CURRENT` | Currently reading |
| `PLANNING` | Planning to read |
| `COMPLETED` | Finished reading |
| `DROPPED` | Stopped reading |
| `PAUSED` | Paused |
| `REPEATING` | Re-reading |

### MediaType
| Value | Description |
|-------|-------------|
| `ANIME` | Anime |
| `MANGA` | Manga (includes light novels, one-shots, doujinshi) |

### MediaFormat
| Value | Description |
|-------|-------------|
| `MANGA` | Standard manga |
| `NOVEL` | Light/Web novel |
| `ONE_SHOT` | One-shot |
| `DOUJINSHI` | Doujinshi |

### ScoreFormat
| Format | Range |
|--------|-------|
| `POINT_100` | Integer 0-100 |
| `POINT_10_DECIMAL` | Float 0-10 (1 decimal) |
| `POINT_10` | Integer 0-10 |
| `POINT_5` | Integer 0-5 (stars) |
| `POINT_3` | Integer 0-3 (smileys) |

The user's chosen format is in `User.mediaListOptions.scoreFormat`. When writing scores, you can use either `score` (in user's format) or `scoreRaw` (always 0-100).

---

## 6. Rust Implementation Recommendations

### 6.1 Approach: Direct reqwest + serde_json (no GraphQL client crate)

The project already has `reqwest` 0.12 (with TLS) and `serde_json` in dependencies. This is sufficient for the Anilist API. **Do not add a GraphQL client crate** — the API surface is small enough that manual queries with `serde_json::Value` or typed `#[derive(Deserialize)]` structs are cleaner and simpler.

The official docs include a Rust example using exactly this pattern (reqwest + serde_json).

### 6.2 Recommended Module Structure

```
backend/shared/src/tracker/anilist/
├── mod.rs            # Client struct, public API
├── auth.rs           # OAuth2 token exchange, token storage, JWT decode
├── queries.rs        # GraphQL query strings (const &str)
├── mutations.rs      # GraphQL mutation strings (const &str)
├── types.rs          # Serde types for responses
├── rate_limiter.rs   # Simple rate limiter respecting X-RateLimit headers
```

### 6.3 Client Design (Pseudocode)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

const ANILIST_API: &str = "https://graphql.anilist.co";
const AUTH_URL: &str = "https://anilist.co/api/v2/oauth/authorize";
const TOKEN_URL: &str = "https://anilist.co/api/v2/oauth/token";

pub struct AnilistClient {
    http: Client,
    access_token: Option<String>,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl AnilistClient {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self { ... }

    /// Build the authorization URL to redirect the user
    pub fn auth_url(&self) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code",
            AUTH_URL, self.client_id, self.redirect_uri
        )
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code(&mut self, code: &str) -> Result<(), Error> {
        let resp = self.http.post(TOKEN_URL)
            .json(&json!({
                "grant_type": "authorization_code",
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "redirect_uri": self.redirect_uri,
                "code": code
            }))
            .send().await?;
        let data: TokenResponse = resp.json().await?;
        self.access_token = Some(data.access_token);
        Ok(())
    }

    /// Execute a GraphQL query
    pub async fn query(&self, query: &str, variables: serde_json::Value) -> Result<GraphqlResponse> {
        let mut req = self.http.post(ANILIST_API)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");
        if let Some(token) = &self.access_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let resp = req.json(&json!({"query": query, "variables": variables})).send().await?;
        // Check X-RateLimit-Remaining header for rate limiting
        // Handle 429 with Retry-After
        let data: GraphqlResponse = resp.json().await?;
        Ok(data)
    }

    /// Check if token is still valid by decoding JWT `exp` claim
    pub fn is_token_expired(&self) -> bool { ... }
}
```

### 6.4 Token Storage
- Store `access_token` in the app settings JSON (already serialized via serde)
- Decode the JWT payload (base64 decode, no signature verification needed) to read `exp`
- On 401 response, clear token and notify user to re-authenticate
- No refresh tokens — user must go through OAuth again

### 6.5 Rate Limiting Strategy
The Anilist API has a rate limit of 90 req/min. For a manual tracker:
- Cache media metadata (chapters, title, ID) locally in SQLite
- Sync user list in background, 1 request at a time
- On 429, use `Retry-After` header and back off
- Track `X-RateLimit-Remaining` — when low, throttle

---

## 7. Key Field Mapping (RakuYomi → Anilist)

| RakuYomi Concept | Anilist Mutation Field | Notes |
|-----------------|----------------------|-------|
| Chapter progress | `progress` | Integer, chapters read |
| Reading status | `status` | `CURRENT`, `COMPLETED`, `DROPPED`, `PAUSED`, `PLANNING` |
| Score | `scoreRaw` | Integer 0-100, avoids format confusion |
| Start date | `startedAt` | `FuzzyDateInput { year, month, day }` |
| Finish date | `completedAt` | `FuzzyDateInput { year, month, day }` |

**Plan for push updates on chapter read:**
1. Track the current manga title + chapter number being read in KOReader
2. Look up Anilist media ID via `search` query (cache result in SQLite)
3. Call `SaveMediaListEntry` with `mediaId`, `progress=chapter`, `status=CURRENT`
4. If chapter ≥ total chapters, set `status=COMPLETED`

---

## 8. Common Pitfalls & Caveats

### 8.1 ID Collision
Anime and manga IDs are NOT unique across types. If media ID 1 is anime, no manga has ID 1. Always specify `type: MANGA` in queries.

### 8.2 404 on Wrong Type
Requesting manga by ID without `type: MANGA` (or with `type: ANIME`) when the ID belongs to the other type → HTTP 404.

### 8.3 auth endpoint naming confusion
The auth docs are at `docs.anilist.co/guide/auth/` not `docs.anilist.co/guide/authentication` (the latter 404s).

### 8.4 PageInfo Degradation
`total` and `lastPage` fields in `PageInfo` are unreliable due to performance issues. Only use `hasNextPage` for pagination.

### 8.5 Custom Lists
Always check custom lists when reading a user's full list. Users can hide entries from default status lists, making them only visible in custom lists via `MediaListCollection`.

### 8.6 Large List Limit
`MediaListCollection` is limited to 11,000 most recently updated entries. Use `chunk`/`perChunk` (max 500) for pagination through large collections.

### 8.7 Token Expiry
Tokens last exactly 1 year. No refresh mechanism. Plan for a re-auth flow.

### 8.8 User Agent
The docs don't mandate a specific User-Agent, but the project convention is to set one (see `check_update.rs` uses "rakuyomi"). **Set `User-Agent: rakuyomi`** as a best practice.

### 8.9 API Stability
API may be temporarily disabled (403 with error message) during severe outages. Monitor Anilist Discord for announcements.

### 8.10 Commercial Use
Free for non-commercial use. Projects over $150/month revenue need a commercial license. **RakuYomi is non-commercial, so this is fine.**

### 8.11 Prohibited Use Cases
- Using the API as a backup/data storage service
- Mass data collection/hoarding
- Competing anime/manga tracker services (RakuYomi as a reader syncing TO Anilist should be fine as it's complementary, not a competing tracker service)

---

## 9. References

### Official Documentation
- **Main docs:** https://docs.anilist.co/
- **Getting Started:** https://docs.anilist.co/guide/introduction
- **GraphQL guide:** https://docs.anilist.co/guide/graphql/
- **Auth guide:** https://docs.anilist.co/guide/auth/
- **Authorization Code Grant:** https://docs.anilist.co/guide/auth/authorization-code
- **Implicit Grant:** https://docs.anilist.co/guide/auth/implicit
- **Authenticated Requests:** https://docs.anilist.co/guide/auth/authenticated-requests
- **Rate Limiting:** https://docs.anilist.co/guide/rate-limiting
- **Query Reference:** https://docs.anilist.co/reference/query
- **Mutation Reference:** https://docs.anilist.co/reference/mutation
- **MediaList object:** https://docs.anilist.co/reference/object/medialist
- **MediaListStatus enum:** https://docs.anilist.co/reference/enum/medialiststatus
- **Media object:** https://docs.anilist.co/reference/object/media
- **MediaType enum:** https://docs.anilist.co/reference/enum/mediatype
- **ScoreFormat enum:** https://docs.anilist.co/reference/enum/scoreformat
- **Terms of Use:** https://docs.anilist.co/guide/terms-of-use
- **Considerations:** https://docs.anilist.co/guide/considerations
- **GraphQL Pagination:** https://docs.anilist.co/guide/graphql/pagination
- **GraphQL Mutations:** https://docs.anilist.co/guide/graphql/mutations
- **GraphQL Errors:** https://docs.anilist.co/guide/graphql/errors

### Developer Resources
- **Apollo Studio Explorer:** https://studio.apollographql.com/sandbox/explorer?endpoint=https://graphql.anilist.co
- **Developer settings:** https://anilist.co/settings/developer
- **Docs GitHub repo:** https://github.com/AniList/ApiV2-GraphQL-Docs

### Rust Crates (in project, available for use)
- `reqwest` 0.12 (with rustls-tls, json) — HTTP client
- `serde` / `serde_json` — serialization
- `url` 2.5 — URL parsing
- `chrono` — timestamps
- `tokio` — async runtime

### JWT decoding (no crate needed)
Base64-decode the token payload (2nd dot-separated segment) for user ID and expiry.
