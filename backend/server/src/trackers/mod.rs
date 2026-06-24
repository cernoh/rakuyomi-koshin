mod anilist;
mod mangadex;

use shared::database::MangaTrackerIds;
use shared::database::Database;
use shared::model::ChapterInformation;

/// Synchronize read progress to all enabled trackers.
///
/// Called after a chapter is marked as read. Checks settings, extracts
/// chapter metadata, and dispatches to each tracker that is configured
/// and enabled.
pub async fn sync_read_progress(
    database: &Database,
    settings: &shared::settings::Settings,
    chapter: &ChapterInformation,
    manga_source_id: &str,
    manga_title: &str,
) {
    let tracked_ids = match database.find_manga_tracker_ids(&chapter.id.manga_id()).await {
        Ok(Some(ids)) => ids,
        Ok(None) => MangaTrackerIds::default(),
        Err(e) => {
            log::error!("Failed to fetch tracker IDs: {e}");
            MangaTrackerIds::default()
        }
    };

    let chapter_number = chapter.chapter_number;

    if settings.sync_to_anilist {
        if let Some(token) = &settings.anilist_token {
            if !token.is_empty() {
                if let Err(e) = anilist::sync(
                    token,
                    &tracked_ids,
                    manga_title,
                    chapter_number,
                )
                .await
                {
                    log::error!("AniList sync failed: {e}");
                }
            }
        }
    }

    // For MangaDex, we only sync if the source is MangaDex (source_id starts
    // with "mangadex") or if we have an explicit mangadex_id mapping.
    if settings.sync_to_mangadex {
        if let Some(token) = &settings.mangadex_token {
            if !token.is_empty() {
                let is_mangadex_source = manga_source_id
                    .to_lowercase()
                    .contains("mangadex");

                // If this is a MangaDex source, the chapter_id IS the MangaDex
                // chapter UUID, so we can sync directly.
                if is_mangadex_source {
                    if let Err(e) = mangadex::sync(
                        token,
                        &chapter.id,
                        chapter_number,
                    )
                    .await
                    {
                        log::error!("MangaDex sync failed: {e}");
                    }
                } else if let Some(md_id) = &tracked_ids.mangadex_id {
                    if let Err(e) = mangadex::sync_by_manga_id(
                        token,
                        md_id,
                        &chapter.id.value(),
                        chapter_number,
                    )
                    .await
                    {
                        log::error!("MangaDex sync (mapped) failed: {e}");
                    }
                }
            }
        }
    }
}
