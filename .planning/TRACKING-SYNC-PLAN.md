# Tracker Sync Implementation Plan

> MyAnimeList + AniList progress syncing for RakuYomi.
> Pushes on chapter completion, queues offline, flushes on reconnect.

## Current State

Already built:
- OAuth flows: AniList implicit-grant, MAL PKCE (with QR codes for e-ink auth)
- DB tables: `track` (per-manga tracking rows) + `tracker_auth` (tokens)
- Types: `TrackerService`, `TrackEntry`, `TrackStatus`, `AuthToken`
- Routes: `/track/services`, `/track/{tracker}/auth-url`, `/track/qr/{qr_id}`, `/track/{tracker}/auth`, `/track/{tracker}/status`
- Frontend: zero tracker UI yet

Missing: tracker API clients, manga search/link, progress push, offline queue, auto-sync hook, frontend.

---

## Architecture

```
┌──────────────┐       ┌──────────────────┐       ┌─────────────────┐
│ Lua frontend │──HTTP──│ axum /track/*    │──SQL───│ SQLite (track,  │
│              │       │                  │       │  tracker_auth,  │
│              │       │  tracker_client  │──HTTPS─│  sync_queue)    │
│              │       │  (AniList/MAL)   │       │                 │
└──────────────┘       └──────────────────┘       └─────────────────┘
                                │
                    mark_chapter_as_read()
                    update_last_read_chapter()
                                │
                    spawn sync_task (fire-and-forget)
                                │
                    ┌───────────▼───────────┐
                    │ online? push progress  │
                    │   ├── yes: API call    │
                    │   └── no:  INSERT into │
                    │            sync_queue  │
                    └───────────────────────┘
                                │
                    on next success (any API call)
                    ┌───────────▼───────────┐
                    │ drain sync_queue      │
                    │ (background, batched) │
                    └───────────────────────┘
```

---

## Phase 1: Tracker API Clients

### 1.1 — `shared/src/track/client.rs` (new module)

A trait + two implementations:

```rust
#[async_trait]
pub trait TrackerClient: Send + Sync {
    /// Search the tracker's catalog for manga matching `query`.
    async fn search_manga(&self, token: &str, query: &str) -> Result<Vec<TrackerMangaSearchResult>>;

    /// Create or update a manga entry on the tracker. Returns the
    /// remote ID and library entry ID (if applicable).
    async fn update_progress(
        &self,
        token: &str,
        remote_id: &str,
        library_id: Option<&str>,
        progress: &ProgressUpdate,
    ) -> Result<TrackerUpdateResult>;

    /// Fetch current progress for a manga from the tracker.
    async fn get_progress(
        &self,
        token: &str,
        remote_id: &str,
    ) -> Result<Option<TrackerProgress>>;
}

pub struct ProgressUpdate {
    pub chapters_read: i32,
    pub status: Option<TrackStatus>,
    pub score: Option<i32>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
}
```

**AniList implementation** — GraphQL only:
- `search_manga`: `query { Page { media(search: $q, type: MANGA) { id title { romaji english } chapters } } }`
- `update_progress`: `mutation { SaveMediaListEntry(mediaId: $id, progress: $ch, status: $st) { id } }`
- `get_progress`: `query { Media(id: $id, type: MANGA) { mediaListEntry { progress status score } } }`

**MAL implementation** — REST v2:
- `search_manga`: `GET /v2/manga?q={query}&fields=id,title,num_chapters`
- `update_progress`: `PATCH /v2/manga/{id}/my_list_status` with form body `status=reading&num_chapters_read={n}`
- `get_progress`: `GET /v2/manga/{id}?fields=my_list_status`

Both live in `shared/src/track/client/anilist.rs` and `shared/src/track/client/mal.rs`.

### 1.2 — Token refresh middleware

Before every API call, check `tracker_auth.expires_at`. If expired (or within 5min of expiry):
- MAL: call `MalAuth::refresh_token()`, persist new token, use it.
- AniList: can't refresh — mark the entry as needing re-auth, surface to user.

Factor into a helper `get_valid_token(db, tracker) -> Result<String>`.

---

## Phase 2: Manga Link/Unlink API

### 2.1 — New routes

```
POST   /track/{tracker}/search          { "query": "..." } → Vec<TrackerMangaSearchResult>
POST   /track/{tracker}/link            { "manga_source_id", "manga_id", "remote_id", "title", "total_chapters" } → TrackEntry
DELETE /track/{tracker}/unlink           { "manga_source_id", "manga_id" } → ()
GET    /track/{tracker}/entries          → Vec<TrackEntry>  (all linked manga for this tracker)
```

### 2.2 — Link flow

1. User searches tracker by manga title in frontend.
2. Backend calls `TrackerClient::search_manga()` with the stored token.
3. Frontend shows results, user picks one.
4. Frontend POSTs `/track/{tracker}/link` with the selected `remote_id`.
5. Backend:
   - Computes current `last_chapter_read` from `chapter_state` for this manga.
   - INSERTs into `track` table (UPSERT on `UNIQUE(manga_source_id, manga_id, tracker_id)`).
   - Optionally pushes initial progress (chapters already read locally).

### 2.3 — Unlink flow

DELETE the `track` row. Optionally call the tracker API to remove the list entry (configurable — default: don't, just unlink locally).

---

## Phase 3: Progress Push — The Core Hook

### 3.1 — Where to hook

Two code paths mark chapters as read:

1. **`usecases::mark_chapter_as_read`** — single chapter marked read.
2. **`usecases::mark_chapters_as_read`** — batch mark (range syntax like "1-5,7").
3. **`usecases::update_last_read_chapter`** — called when the reader closes/finishes a chapter (from `ChapterListing.lua`).

All three live in `backend/shared/src/usecases/`. The push should fire from the **route handlers** in `backend/server/src/manga/routes.rs`, not from inside the usecases, to keep `shared` free of network I/O and HTTP client deps.

### 3.2 — Implementation

In `backend/server/src/manga/routes.rs`, after each successful mark/update:

```rust
// Fire-and-forget sync. Errors are logged, never returned to the client.
let db_clone = database.clone();
let settings_clone = settings.clone();
let chapter_id_clone = chapter_id.clone();
tokio::spawn(async move {
    if let Err(e) = sync_tracker_progress(&db_clone, &settings_clone, &chapter_id_clone).await {
        log::warn!("tracker sync failed: {e}");
    }
});
```

### 3.3 — `sync_tracker_progress` logic

```
fn sync_tracker_progress(db, settings, chapter_id):
    1. Look up which trackers have a `track` row for this manga.
    2. For each tracker:
       a. Load token from `tracker_auth`.
       b. Compute chapters_read = COUNT of read chapters for this manga
          (or MAX(chapter_number) if the source uses numeric chapters).
       c. Check if `last_chapter_read` already equals this value → skip.
       d. Call TrackerClient::update_progress().
       e. On success: UPDATE track SET last_chapter_read = new_value,
          updated_at = now.
       f. On network/auth error: INSERT into sync_queue (Phase 4).
```

### 3.4 — Chapter number mapping

Sources use different chapter numbering. The `track` table stores `last_chapter_read` as an integer, but sources may have `chapter_number: Option<f64>`. Strategy:
- Use `MAX(chapter_number)` cast to integer for the "progress" value.
- For sources without chapter numbers (e.g. some webtoon sources), fall back to `COUNT(read chapters)` — this matches how most trackers model it.

---

## Phase 4: Offline Queue

### 4.1 — New migration

```sql
CREATE TABLE track_sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tracker_id TEXT NOT NULL,
    manga_source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    chapters_read INTEGER NOT NULL,
    status TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    UNIQUE(tracker_id, manga_source_id, manga_id)
) STRICT;
```

UNIQUE on (tracker, manga) — upsert on conflict: if a newer sync is queued for the same manga, replace with the higher chapter count (coalesce: always push the latest state).

### 4.2 — Queue insert (on push failure)

When `TrackerClient::update_progress()` fails with a network/timeout error:
- INSERT/UPSERT into `track_sync_queue` with the intended chapter count.
- Don't queue on auth errors (token invalid) — those need user re-login.

### 4.3 — Queue drain (on connectivity restored)

**Trigger:** After any successful tracker API call (push, search, link), attempt to drain the queue. This is lazy — no background polling, no connectivity probing.

```
fn drain_sync_queue(db, tracker_client, tracker):
    rows = SELECT * FROM track_sync_queue WHERE tracker_id = ? ORDER BY created_at
    for row in rows:
        match tracker_client.update_progress(token, row.remote_id, ...):
            Ok => DELETE FROM track_sync_queue WHERE id = row.id
            Err(network) => UPDATE attempts += 1; if attempts > 10: DELETE
            Err(auth) => DELETE (user needs to re-login; don't retry)
```

**Why no background polling?** KOReader runs on low-power e-ink readers. A background sync daemon would waste battery and complicate the process model. The lazy-drain-on-success pattern is simpler and matches the actual usage pattern: user reads offline → goes online → opens app → next API call drains the queue.

### 4.4 — Queue visibility

New route: `GET /track/sync-queue` → returns pending sync items with counts. Frontend can show "N updates pending" badge.

---

## Phase 5: Pull Sync (Optional, Lower Priority)

### 5.1 — Use case

User reads on another device (phone MAL app), then opens RakuYomi. Pull fetches the remote progress and reconciles.

### 5.2 — When

- Manual: user taps "Sync with AniList/MAL" in settings.
- On app start: background task pulls all tracked manga.
- NOT automatic on every chapter read — that would be a round-trip per chapter, too expensive.

### 5.3 — Conflict resolution

If local `last_chapter_read` differs from remote:
- Take the **higher** value (user read more than the tracker knows about → they read elsewhere too, or the tracker is behind).
- Exception: if `sync_queue` has a pending push, the local value is authoritative — skip the pull for that entry.

---

## Phase 6: Frontend UI

### 6.1 — Tracking section in manga detail view

In the manga detail screen (where chapters are listed), add a "Tracking" section:
- Shows tracker status icons (AniList / MAL) with current progress.
- If not linked: "Link to AniList" / "Link to MAL" buttons.
- If linked: shows `last_chapter_read / total_chapters`, status, score. Tap to edit.
- "Unlink" option in a menu.

### 6.2 — Link dialog

1. User taps "Link to AniList".
2. Input dialog: search query (pre-filled with manga title).
3. Backend searches tracker, returns results.
4. List dialog: show results with title + chapter count.
5. User picks one → POST `/track/{tracker}/link`.
6. Success: show confirmation, refresh the tracking section.

### 6.3 — Backend.lua additions

```lua
function Backend.searchTrackerManga(tracker, query) ... end
function Backend.linkMangaToTracker(tracker, source_id, manga_id, remote_id, title, total_chapters) ... end
function Backend.unlinkMangaFromTracker(tracker, source_id, manga_id) ... end
function Backend.getTrackerEntries(tracker) ... end
function Backend.getSyncQueue() ... end
```

### 6.4 — Settings toggle

Add to `UpdateableSettings`:
- `track_anilist_enabled: bool` (default: true if logged in)
- `track_mal_enabled: bool` (default: true if logged in)
- `track_auto_sync: bool` (default: true — push on chapter read)

Users who don't want auto-sync can disable it and manually sync.

---

## Phase 7: File Layout

```
backend/shared/src/track/
    mod.rs              -- already exists
    types.rs            -- already exists (extend with search result types)
    client.rs           -- NEW: TrackerClient trait + helpers
    client/anilist.rs   -- NEW: AniList GraphQL client
    client/mal.rs       -- NEW: MAL REST client

backend/server/src/track/
    mod.rs              -- already exists (add client submodule)
    routes.rs           -- already exists (extend with search/link/unlink/entries/sync-queue)
    sync.rs             -- NEW: sync_tracker_progress + drain_sync_queue
    state.rs            -- already exists (add reqwest::Client for tracker API calls)

backend/server/src/manga/
    routes.rs           -- add tokio::spawn sync after mark_chapter_as_read

backend/shared/migrations/
    20260712000001_create_track_sync_queue.sql  -- NEW

frontend/rakuyomi.koplugin/
    Backend.lua         -- add tracker API methods
    TrackingMenu.lua    -- NEW: tracking section in manga detail
    TrackerSearchDialog.lua -- NEW: search + pick manga on tracker
```

---

## Implementation Order

1. **Phase 1** (tracker clients) — no UI dependency, can be unit tested against mocked HTTP.
2. **Phase 4** (sync queue migration + DB methods) — pure data layer.
3. **Phase 2** (search/link/unlink routes) — needs Phase 1 clients.
4. **Phase 3** (push hook in manga routes + sync.rs) — needs Phases 1, 2, 4.
5. **Phase 6** (frontend) — needs Phases 2, 3.
6. **Phase 5** (pull sync) — last, lowest priority.

---

## Key Design Decisions

### Fire-and-forget sync, not blocking
The sync runs in a `tokio::spawn` after the mark-read response is already sent. Network hiccups never slow down the reader UX.

### Lazy drain, no background daemon
Sync queue drains on the next successful tracker API call. No polling, no connectivity probes. Matches e-ink reader constraints (battery, no background processes).

### Upsert queue, not append
If you read chapters 5, 6, 7 offline, the queue has ONE entry per manga with the highest chapter count. No point pushing "I read chapter 5" after "I read chapter 7".

### Don't sync on `update_last_read_chapter` alone
That route is called mid-read (it just records which chapter was last touched). The sync fires from `mark_chapter_as_read` — when the user explicitly finishes a chapter. This avoids pushing "I'm on chapter 3" when they're only halfway through.

Actually, reconsider: `update_last_read_chapter` IS called when the reader closes/finishes. Check the call sites in `ChapterListing.lua` (lines 966-970, 981-985). If it fires on chapter completion, hook there. If it fires mid-read (e.g. on page turn), hook on `mark_chapter_as_read` instead.

### AniList token expiry
AniList implicit-grant tokens don't expire (or expire very slowly — AniList docs are vague). If a call returns 401, surface "Re-login to AniList" in the UI. Don't auto-retry.

### MAL token refresh
MAL tokens expire in ~1 hour. The `get_valid_token()` helper handles refresh transparently. If refresh fails (refresh token expired after ~1 month), surface "Re-login to MAL".

### Chapter number vs count
Use `MAX(chapter_number)` when available, fall back to `COUNT(read)`. Store both in `TrackEntry` so the UI can show "ch. 42 / 150" clearly.

---

## Testing Strategy

### Unit tests (Rust)
- Tracker client parsing: feed mock HTTP responses, verify struct deserialization.
- Sync queue upsert: insert twice for same manga, verify only latest chapter count survives.
- Progress computation: given a set of `chapter_state` rows, verify `MAX(chapter_number)` logic.

### Integration tests
- Link a manga, mark chapters read, verify `track.last_chapter_read` updates.
- Simulate network failure (mock client returns Err), verify sync_queue row created.
- Simulate recovery (mock client succeeds), verify queue drains.

### E2E tests (future)
- Full flow: auth via QR, search, link, read chapter, verify progress pushed to real AniList/MAL sandbox account.

---

## Risk / Open Questions

1. **AniList rate limits** — 90 requests/minute per user. Unlikely to hit with per-chapter pushes, but batch operations (mark 50 chapters read) could spike. Add a simple rate limiter if needed.

2. **MAL chapter numbering** — MAL uses `num_chapters_read`, an integer. Sources with decimal chapter numbers (e.g. chapter 12.5) need rounding. Truncate to floor.

3. **Source manga ≠ tracker manga** — A manga on MangaDex might be listed under a different title on AniList. The search + pick UI handles this — we don't auto-match.

4. **Private lists** — AniList supports private list entries. Expose the `private` field (already in `TrackEntry`) in the link dialog as a toggle.

5. **Score format** — MAL uses 0-10 integer. AniList uses 0-100 integer. Normalize in the client layer: store as the tracker's native format, convert in the UI if needed.

6. **Duplicate syncs** — If the user has both AniList and MAL linked for the same manga, both get pushed. Independent, no coordination needed.

---

## Estimated Effort

| Phase | Files | Complexity |
|-------|-------|------------|
| 1. Tracker clients | 3 new Rust files | Medium — GraphQL/REST parsing |
| 2. Link/unlink API | 1 route file extended | Low — CRUD |
| 3. Push hook | 1 new Rust file + 1 route edit | Medium — the core logic |
| 4. Offline queue | 1 migration + 1 Rust file | Low — INSERT/SELECT/DELETE |
| 5. Pull sync | 1 Rust file | Low — fetch + compare |
| 6. Frontend | 3-4 new Lua files | Medium — dialogs, state |
| **Total** | ~10 new files, ~5 modified | ~2-3 days focused work |
