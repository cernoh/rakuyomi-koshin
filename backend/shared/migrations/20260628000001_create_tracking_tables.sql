-- Create track table for tracking reading progress on external services
CREATE TABLE track (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    tracker_id TEXT NOT NULL,
    remote_id TEXT,
    library_id TEXT,
    title TEXT,
    last_chapter_read INTEGER DEFAULT 0,
    total_chapters INTEGER,
    status TEXT,
    score INTEGER,
    start_date TEXT,
    finish_date TEXT,
    tracking_url TEXT,
    private INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(manga_source_id, manga_id, tracker_id)
) STRICT;

-- Create tracker_auth table for OAuth token storage per tracker service
CREATE TABLE tracker_auth (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tracker_id TEXT NOT NULL UNIQUE,
    token_json TEXT NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Index for efficient lookups by tracker + manga
CREATE INDEX idx_track_tracker_manga ON track (tracker_id, manga_id);

-- Index for auth token lookups by tracker_id
CREATE INDEX idx_tracker_auth_tracker ON tracker_auth (tracker_id);
