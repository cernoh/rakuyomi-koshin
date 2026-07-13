-- Queue for tracker sync operations that failed due to network issues.
-- Upserted on conflict: if a newer sync is queued for the same manga,
-- replace with the higher chapter count (coalesce: always push the latest state).
CREATE TABLE track_sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tracker_id TEXT NOT NULL,
    manga_source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    chapters_read INTEGER NOT NULL,
    status TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    UNIQUE(tracker_id, manga_source_id, manga_id)
) STRICT;

-- Index for draining by tracker
CREATE INDEX idx_track_sync_queue_tracker ON track_sync_queue (tracker_id, created_at);
