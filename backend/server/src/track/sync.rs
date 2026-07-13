//! Tracker progress sync logic.
//!
//! Two entry points:
//!
//! - [`sync_tracker_progress`] — fire-and-forget push after a chapter
//!   is marked read. Looks up all trackers linked to the manga,
//!   computes the current progress, and pushes to each one. On network
//!   failure, enqueues into `track_sync_queue` for later drain.
//!
//! - [`drain_sync_queue`] — called opportunistically after any
//!   successful tracker API call. Walks the queue and retries each
//!   pending entry.

use anyhow::Result;
use log::{info, warn};
use shared::track::client::anilist::AniListClient;
use shared::track::client::mal::MalClient;
use shared::track::client::TrackerClient;
use shared::track::types::{ProgressUpdate, TrackerService};
use std::sync::Arc;

/// Fire-and-forget sync after a chapter is marked read.
///
/// Called from a `tokio::spawn` in the manga route handlers. Errors
/// are logged, never returned to the client.
pub async fn sync_tracker_progress(
    database: &Arc<shared::database::Database>,
    http_client: &reqwest::Client,
    manga_source_id: &str,
    manga_id: &str,
) -> Result<()> {
    // Find all trackers linked to this manga.
    let pool = database.pool().await;
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT tracker_id, remote_id, last_chapter_read \
         FROM track WHERE manga_source_id = ? AND manga_id = ? AND remote_id IS NOT NULL",
    )
    .bind(manga_source_id)
    .bind(manga_id)
    .fetch_all(&pool)
    .await?;
    drop(pool);

    if rows.is_empty() {
        return Ok(());
    }

    // Compute current progress: MAX(chapter_number) among read chapters,
    // or COUNT(read chapters) if no chapter numbers are available.
    let pool = database.pool().await;
    let max_ch: Option<(Option<f64>,)> = sqlx::query_as(
        "SELECT MAX(CAST(chapter_number AS REAL)) FROM chapter_state \
         WHERE manga_id = ? AND source_id = ? AND read_state = 1 \
         AND chapter_number IS NOT NULL AND chapter_number != ''",
    )
    .bind(manga_id)
    .bind(manga_source_id)
    .fetch_optional(&pool)
    .await?;
    drop(pool);

    let current_progress = match max_ch.and_then(|r| r.0) {
        Some(n) => n.floor() as i32,
        None => {
            let pool = database.pool().await;
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM chapter_state \
                 WHERE manga_id = ? AND source_id = ? AND read_state = 1",
            )
            .bind(manga_id)
            .bind(manga_source_id)
            .fetch_one(&pool)
            .await?;
            drop(pool);
            count.0 as i32
        }
    };

    for (tracker_id_str, remote_id, last_pushed) in &rows {
        let tracker: TrackerService = match tracker_id_str.parse() {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Skip if progress hasn't changed since last push.
        if current_progress <= *last_pushed {
            continue;
        }

        // Get a valid token.
        let token = match crate::track::routes::get_valid_token(database, tracker).await {
            Ok(t) => t,
            Err(e) => {
                warn!("tracker sync: cannot get token for {tracker}: {e:?}");
                continue;
            }
        };

        let progress = ProgressUpdate {
            chapters_read: current_progress,
            status: None,
            score: None,
            start_date: None,
            finish_date: None,
        };

        let result = match tracker {
            TrackerService::AniList => {
                AniListClient::new()
                    .update_progress(http_client, &token, &remote_id, None, &progress)
                    .await
            }
            TrackerService::MyAnimeList => {
                MalClient::new()
                    .update_progress(http_client, &token, &remote_id, None, &progress)
                    .await
            }
        };

        match result {
            Ok(update_result) => {
                // Update the track row with new progress.
                let pool = database.pool().await;
                let library_id = update_result.library_id.as_deref();
                sqlx::query(
                    "UPDATE track SET last_chapter_read = ?, library_id = COALESCE(?, library_id), \
                     updated_at = datetime('now') \
                     WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
                )
                .bind(current_progress)
                .bind(library_id)
                .bind(tracker.as_str())
                .bind(manga_source_id)
                .bind(manga_id)
                .execute(&pool)
                .await?;
                drop(pool);

                // Remove from sync queue if present.
                let pool = database.pool().await;
                sqlx::query(
                    "DELETE FROM track_sync_queue WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
                )
                .bind(tracker.as_str())
                .bind(manga_source_id)
                .bind(manga_id)
                .execute(&pool)
                .await?;
                drop(pool);

                info!(
                    "tracker sync: pushed {} ch to {} for {}/{}",
                    current_progress, tracker, manga_source_id, manga_id
                );
            }
            Err(e) => {
                let err_str = format!("{e:#}");
                let is_auth = err_str.contains("authentication") || err_str.contains("401") || err_str.contains("token");

                if is_auth {
                    // Auth errors need user re-login; don't queue.
                    warn!("tracker sync: auth error for {tracker}, not queuing: {e}");
                    continue;
                }

                // Network/timeout error: enqueue for later drain.
                let pool = database.pool().await;
                sqlx::query(
                    "INSERT INTO track_sync_queue (tracker_id, manga_source_id, manga_id, remote_id, chapters_read, last_error) \
                     VALUES (?, ?, ?, ?, ?, ?) \
                     ON CONFLICT(tracker_id, manga_source_id, manga_id) DO UPDATE SET \
                       chapters_read = MAX(excluded.chapters_read, track_sync_queue.chapters_read), \
                       last_error = excluded.last_error, \
                       created_at = datetime('now')",
                )
                .bind(tracker.as_str())
                .bind(manga_source_id)
                .bind(manga_id)
                .bind(&remote_id)
                .bind(current_progress)
                .bind(&err_str)
                .execute(&pool)
                .await?;
                drop(pool);

                warn!("tracker sync: failed for {tracker}, queued: {e}");
            }
        }
    }

    Ok(())
}

/// Drain the sync queue for a given tracker. Called opportunistically
/// after any successful tracker API call.
pub async fn drain_sync_queue(
    database: &Arc<shared::database::Database>,
    http_client: &reqwest::Client,
    tracker: TrackerService,
    token: &str,
) -> Result<()> {
    let pool = database.pool().await;
    let rows: Vec<(i64, String, String, String, String, i32, i32)> = sqlx::query_as(
        "SELECT id, manga_source_id, manga_id, remote_id, COALESCE(status, ''), chapters_read, attempts \
         FROM track_sync_queue WHERE tracker_id = ? ORDER BY created_at",
    )
    .bind(tracker.as_str())
    .fetch_all(&pool)
    .await?;
    drop(pool);

    if rows.is_empty() {
        return Ok(());
    }

    for (id, source_id, manga_id, remote_id, _status, chapters_read, attempts) in rows {
        if attempts > 10 {
            // Give up after 10 attempts.
            let pool = database.pool().await;
            sqlx::query("DELETE FROM track_sync_queue WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await?;
            drop(pool);
            warn!("tracker sync: gave up on queue entry {id} after {attempts} attempts");
            continue;
        }

        let progress = ProgressUpdate {
            chapters_read,
            status: None,
            score: None,
            start_date: None,
            finish_date: None,
        };

        let result = match tracker {
            TrackerService::AniList => {
                AniListClient::new()
                    .update_progress(http_client, token, &remote_id, None, &progress)
                    .await
            }
            TrackerService::MyAnimeList => {
                MalClient::new()
                    .update_progress(http_client, token, &remote_id, None, &progress)
                    .await
            }
        };

        let pool = database.pool().await;
        match result {
            Ok(update_result) => {
                // Success: remove from queue, update track row.
                sqlx::query("DELETE FROM track_sync_queue WHERE id = ?")
                    .bind(id)
                    .execute(&pool)
                    .await?;

                let library_id = update_result.library_id.as_deref();
                sqlx::query(
                    "UPDATE track SET last_chapter_read = ?, library_id = COALESCE(?, library_id), \
                     updated_at = datetime('now') \
                     WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
                )
                .bind(chapters_read)
                .bind(library_id)
                .bind(tracker.as_str())
                .bind(&source_id)
                .bind(&manga_id)
                .execute(&pool)
                .await?;

                info!("tracker sync: drained queue entry {id} ({chapters_read} ch)");
            }
            Err(e) => {
                let err_str = format!("{e:#}");
                let is_auth = err_str.contains("authentication") || err_str.contains("401") || err_str.contains("token");

                if is_auth {
                    // Auth error: remove from queue (user needs to re-login).
                    sqlx::query("DELETE FROM track_sync_queue WHERE id = ?")
                        .bind(id)
                        .execute(&pool)
                        .await?;
                    warn!("tracker sync: auth error draining {id}, removed: {e}");
                } else {
                    // Network error: increment attempts.
                    sqlx::query(
                        "UPDATE track_sync_queue SET attempts = attempts + 1, last_error = ? WHERE id = ?",
                    )
                    .bind(&err_str)
                    .bind(id)
                    .execute(&pool)
                    .await?;
                    warn!("tracker sync: drain {id} failed (attempt {}): {e}", attempts + 1);
                }
            }
        }
        drop(pool);
    }

    Ok(())
}

/// Pull progress from all trackers for all tracked manga and reconcile.
///
/// Conflict resolution: take the higher value. If `track_sync_queue`
/// has a pending push for a manga, the local value is authoritative
/// (skip the pull for that entry).
pub async fn pull_sync_all(
    database: &Arc<shared::database::Database>,
    http_client: &reqwest::Client,
) -> Result<Vec<String>> {
    let pool = database.pool().await;
    let rows: Vec<(String, String, String, String, i32)> = sqlx::query_as(
        "SELECT tracker_id, manga_source_id, manga_id, remote_id, last_chapter_read \
         FROM track WHERE remote_id IS NOT NULL",
    )
    .fetch_all(&pool)
    .await?;
    drop(pool);

    // Get set of manga with pending pushes (local is authoritative).
    let pool = database.pool().await;
    let queued: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT tracker_id, manga_source_id, manga_id FROM track_sync_queue",
    )
    .fetch_all(&pool)
    .await?;
    drop(pool);

    let queued_set: std::collections::HashSet<(String, String, String)> =
        queued.into_iter().collect();

    let mut messages = Vec::new();

    for (tracker_id_str, source_id, manga_id, remote_id, local_progress) in rows {
        // Skip if there's a pending push for this entry.
        if queued_set.contains(&(tracker_id_str.clone(), source_id.clone(), manga_id.clone())) {
            continue;
        }

        let tracker: TrackerService = match tracker_id_str.parse() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let token = match crate::track::routes::get_valid_token(database, tracker).await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let remote = match tracker {
            TrackerService::AniList => {
                AniListClient::new()
                    .get_progress(http_client, &token, &remote_id)
                    .await
            }
            TrackerService::MyAnimeList => {
                MalClient::new()
                    .get_progress(http_client, &token, &remote_id)
                    .await
            }
        };

        match remote {
            Ok(Some(progress)) => {
                let remote_ch = progress.chapters_read;
                if remote_ch > local_progress {
                    // Remote is ahead — update local.
                    let pool = database.pool().await;
                    sqlx::query(
                        "UPDATE track SET last_chapter_read = ?, updated_at = datetime('now') \
                         WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
                    )
                    .bind(remote_ch)
                    .bind(tracker.as_str())
                    .bind(&source_id)
                    .bind(&manga_id)
                    .execute(&pool)
                    .await?;
                    drop(pool);

                    messages.push(format!(
                        "{} {}/{}: pulled {} (was {})",
                        tracker, source_id, manga_id, remote_ch, local_progress
                    ));
                }
            }
            Ok(None) => {} // Not on tracker's list yet.
            Err(e) => {
                warn!("pull sync: failed for {} {}/{}: {e:?}", tracker, source_id, manga_id);
            }
        }
    }

    Ok(messages)
}
