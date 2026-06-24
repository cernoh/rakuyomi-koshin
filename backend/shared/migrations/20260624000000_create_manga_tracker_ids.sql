-- Create manga_tracker_ids table
CREATE TABLE manga_tracker_ids (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    anilist_id INTEGER NULL,
    mangadex_id TEXT NULL,
    PRIMARY KEY (source_id, manga_id)
) STRICT;
