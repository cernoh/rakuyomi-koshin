//! HTTP route handlers for the `/track/*` API surface.
//!
//! Endpoints:
//!
//! - `GET    /track/services`             — list known trackers + login state
//! - `POST   /track/{tracker}/auth-url`   — start an auth flow, get URL + qr_id
//! - `GET    /track/qr/{qr_id}`           — fetch the 300x300 QR PNG
//! - `POST   /track/{tracker}/auth`       — submit token (AniList) or code+state (MAL)
//! - `DELETE /track/{tracker}/auth`       — clear stored credentials
//! - `GET    /track/{tracker}/status`     — current login state
//! - `POST   /track/{tracker}/search`     — search tracker catalog
//! - `POST   /track/{tracker}/link`       — link a manga to a tracker
//! - `DELETE /track/{tracker}/unlink`     — unlink a manga from a tracker
//! - `GET    /track/{tracker}/entries`    — all linked manga for this tracker
//! - `GET    /track/sync-queue`           — pending sync items

use axum::{
    extract::{Path, State as StateExtractor},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::track::types::{AuthToken, TrackerService};
use shared::track::client::{anilist::AniListClient, mal::MalClient, TrackerClient};
use std::sync::Arc;

use crate::{
    state::State,
    track::{
        auth::{AniListAuth, MalAuth, TrackerAuth},
        qr::encode_url_to_qr_png,
    },
    AppError,
};

/// Build the axum router for the `/track/*` endpoints.
pub fn routes() -> Router<State> {
    Router::new()
        .route("/services", get(list_services))
        .route("/sync-queue", get(get_sync_queue))
        .route("/pull-sync", post(pull_sync))
        .route("/{tracker}/auth-url", post(generate_auth_url))
        .route("/qr/{qr_id}", get(get_qr_code))
        .route("/{tracker}/auth", post(submit_auth).delete(clear_auth))
        .route("/{tracker}/status", get(check_status))
        .route("/{tracker}/search", post(search_manga))
        .route("/{tracker}/link", post(link_manga))
        .route("/{tracker}/unlink", delete(unlink_manga))
        .route("/{tracker}/entries", get(get_tracker_entries))
}

// --- Response / request DTOs -------------------------------------------------

#[derive(Serialize)]
struct ServiceStatus {
    tracker: String,
    logged_in: bool,
}

#[derive(Serialize)]
struct AuthUrlResponse {
    url: String,
    qr_id: String,
    /// Base64-encoded PNG of the QR code for the auth URL.
    qr_image_base64: String,
}

#[derive(Deserialize)]
struct SubmitAuthBody {
    /// AniList implicit-grant access token (extracted from the URL
    /// fragment by the Lua callback page).
    token: Option<String>,
    /// MAL authorization code from the redirect.
    code: Option<String>,
    /// MAL PKCE `state` from the redirect — used to look up the
    /// `code_verifier` in `TrackState`.
    state: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    logged_in: bool,
    username: Option<String>,
}

// --- Handlers ----------------------------------------------------------------

/// `GET /track/services` — list known trackers and whether each one
/// is currently logged in.
async fn list_services(
    StateExtractor(State { database, .. }): StateExtractor<State>,
) -> Result<Json<Vec<ServiceStatus>>, AppError> {
    let pool = database.pool().await;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT tracker_id FROM tracker_auth")
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);

    let logged_in: std::collections::HashSet<String> = rows.into_iter().map(|(id,)| id).collect();

    let services: Vec<ServiceStatus> = TrackerService::ALL
        .iter()
        .map(|t| ServiceStatus {
            tracker: t.as_str().to_string(),
            logged_in: logged_in.contains(t.as_str()),
        })
        .collect();

    Ok(Json(services))
}

/// `POST /track/{tracker}/auth-url` — start an auth flow for `tracker`.
async fn generate_auth_url(
    StateExtractor(State { track_state, .. }): StateExtractor<State>,
    Path(tracker): Path<String>,
) -> Result<Json<AuthUrlResponse>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let (url, state) = match tracker_id {
        TrackerService::AniList => {
            // Implicit grant — no PKCE verifier needed, but we still
            // store the session so the QR endpoint can render the URL.
            let (url, _state, _verifier) = AniListAuth::new().generate_auth_url().await?;
            let qr_id = uuid::Uuid::new_v4().to_string();
            track_state
                .insert(
                    qr_id.clone(),
                    crate::track::state::PkceSession {
                        code_verifier: String::new(),
                        tracker_id: tracker_id.as_str().to_string(),
                        auth_url: url.as_str().to_string(),
                        created_at: std::time::Instant::now(),
                    },
                )
                .await;
            (url, qr_id)
        }
        TrackerService::MyAnimeList => {
            let (url, state, verifier) = MalAuth::new().generate_auth_url().await?;
            // Persist the PKCE session so `/track/{tracker}/auth` can
            // look up the verifier when the user returns from MAL.
            track_state
                .insert(
                    state.clone(),
                    crate::track::state::PkceSession {
                        code_verifier: verifier,
                        tracker_id: tracker_id.as_str().to_string(),
                        auth_url: url.as_str().to_string(),
                        created_at: std::time::Instant::now(),
                    },
                )
                .await;
            (url, state)
        }
    };

    // Generate QR code as base64-encoded PNG
    let qr_png = encode_url_to_qr_png(url.as_str())?;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let qr_image_base64 = STANDARD.encode(&qr_png);

    Ok(Json(AuthUrlResponse {
        url: url.as_str().to_string(),
        qr_id: state,
        qr_image_base64,
    }))
}

/// `GET /track/qr/{qr_id}` — render the QR PNG for an in-flight
/// MAL PKCE session. Returns 404 if the session has expired or never
/// existed.
async fn get_qr_code(
    StateExtractor(State { track_state, .. }): StateExtractor<State>,
    Path(qr_id): Path<String>,
) -> Result<Response, AppError> {
    let session = track_state.peek(&qr_id).await.ok_or(AppError::NotFound)?;
    let png = encode_url_to_qr_png(&session.auth_url)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );
    Ok((StatusCode::OK, headers, png).into_response())
}

/// `POST /track/{tracker}/auth` — finalize the auth flow.
///
/// - **AniList:** body is `{ "token": "<access_token>" }` (extracted
///   from the implicit-grant URL fragment by the Lua callback page).
///   We verify the token against the AniList GraphQL Viewer endpoint
///   before persisting, so we never store a bogus token.
/// - **MAL:** body is `{ "code": "<auth_code>", "state": "<state>" }`.
///   We look up the matching PKCE session by `state`, exchange the
///   code for tokens, and persist the bundle.
async fn submit_auth(
    StateExtractor(State {
        database,
        track_state,
        ..
    }): StateExtractor<State>,
    Path(tracker): Path<String>,
    Json(body): Json<SubmitAuthBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;

    match tracker_id {
        TrackerService::AniList => {
            let token = body.token.as_deref().ok_or_else(|| {
                AppError::OAuth("missing `token` for AniList auth".to_string())
            })?;
            let auth = AniListAuth::new();
            let auth_token = auth.exchange_code(token, "").await?;
            auth.verify_token(&auth_token).await?;
            store_token(&database, tracker_id, &auth_token).await?;
        }
        TrackerService::MyAnimeList => {
            let code = body.code.as_deref().ok_or_else(|| {
                AppError::OAuth("missing `code` for MAL auth".to_string())
            })?;
            let state = body.state.as_deref().ok_or_else(|| {
                AppError::OAuth("missing `state` for MAL auth".to_string())
            })?;
            // One-time-use: the session is removed even if exchange
            // fails, so a stolen state can't be replayed.
            let session = track_state
                .get_and_remove(state)
                .await
                .ok_or(AppError::NotFound)?;
            let auth = MalAuth::new();
            let auth_token = auth.exchange_code(code, &session.code_verifier).await?;
            auth.verify_token(&auth_token).await?;
            store_token(&database, tracker_id, &auth_token).await?;
        }
    }

    Ok(Json(json!({ "success": true })))
}

/// `DELETE /track/{tracker}/auth` — wipe stored credentials for a
/// tracker. Idempotent: deleting a non-existent row is a no-op.
async fn clear_auth(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(tracker): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let pool = database.pool().await;
    sqlx::query("DELETE FROM tracker_auth WHERE tracker_id = ?")
        .bind(tracker_id.as_str())
        .execute(&pool)
        .await
        .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);
    Ok(Json(json!({ "success": true })))
}

/// `GET /track/{tracker}/status` — return login state for `tracker`.
async fn check_status(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(tracker): Path<String>,
) -> Result<Json<StatusResponse>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let pool = database.pool().await;
    let row: Option<(String,)> =
        sqlx::query_as("SELECT token_json FROM tracker_auth WHERE tracker_id = ?")
            .bind(tracker_id.as_str())
            .fetch_optional(&pool)
            .await
            .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);

    if row.is_none() {
        return Ok(Json(StatusResponse {
            logged_in: false,
            username: None,
        }));
    }

    Ok(Json(StatusResponse {
        logged_in: true,
        username: None,
    }))
}

// --- New route handlers (Phase 2 + 4) --------------------------------------

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

#[derive(Deserialize)]
struct LinkBody {
    manga_source_id: String,
    manga_id: String,
    remote_id: String,
    title: Option<String>,
    total_chapters: Option<i32>,
}

#[derive(Deserialize)]
struct UnlinkBody {
    manga_source_id: String,
    manga_id: String,
}

/// `POST /track/{tracker}/search` — search the tracker's catalog.
async fn search_manga(
    StateExtractor(State {
        database,
        track_state,
        ..
    }): StateExtractor<State>,
    Path(tracker): Path<String>,
    Json(body): Json<SearchQuery>,
) -> Result<Json<Vec<shared::track::types::TrackerMangaSearchResult>>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let token = get_valid_token(&database, tracker_id).await?;
    let client = &track_state.http_client;

    let results = match tracker_id {
        TrackerService::AniList => {
            AniListClient::new()
                .search_manga(client, &token, &body.query)
                .await?
        }
        TrackerService::MyAnimeList => {
            MalClient::new()
                .search_manga(client, &token, &body.query)
                .await?
        }
    };

    // Drain queue opportunistically after any successful API call.
    let _ = crate::track::sync::drain_sync_queue(&database, client, tracker_id, &token).await;

    Ok(Json(results))
}

/// `POST /track/{tracker}/link` — link a local manga to a tracker entry.
async fn link_manga(
    StateExtractor(State {
        database,
        ..
    }): StateExtractor<State>,
    Path(tracker): Path<String>,
    Json(body): Json<LinkBody>,
) -> Result<Json<shared::track::types::TrackEntry>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let _token = get_valid_token(&database, tracker_id).await?;
    let pool = database.pool().await;

    // Compute current progress: MAX chapter_number among read chapters,
    // falling back to COUNT of read chapters.
    let max_ch: Option<(Option<f64>,)> = sqlx::query_as(
        "SELECT MAX(CAST(chapter_number AS REAL)) FROM chapter_state \
         WHERE manga_id = ? AND source_id = ? AND read_state = 1 \
         AND chapter_number IS NOT NULL AND chapter_number != ''",
    )
    .bind(&body.manga_id)
    .bind(&body.manga_source_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;

    let last_chapter_read = match max_ch.and_then(|r| r.0) {
        Some(n) => n.floor() as i32,
        None => {
            // Fallback: count read chapters
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM chapter_state \
                 WHERE manga_id = ? AND source_id = ? AND read_state = 1",
            )
            .bind(&body.manga_id)
            .bind(&body.manga_source_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::Other(e.into()))?;
            count.0 as i32
        }
    };

    // UPSERT into track table
    sqlx::query(
        "INSERT INTO track (manga_source_id, manga_id, tracker_id, remote_id, title, \
         last_chapter_read, total_chapters, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now')) \
         ON CONFLICT(manga_source_id, manga_id, tracker_id) DO UPDATE SET \
           remote_id = excluded.remote_id, \
           title = excluded.title, \
           last_chapter_read = excluded.last_chapter_read, \
           total_chapters = excluded.total_chapters, \
           updated_at = excluded.updated_at",
    )
    .bind(&body.manga_source_id)
    .bind(&body.manga_id)
    .bind(tracker_id.as_str())
    .bind(&body.remote_id)
    .bind(&body.title)
    .bind(last_chapter_read)
    .bind(body.total_chapters)
    .execute(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);

    // Read back the row we just wrote
    let entry = get_track_entry(&database, &body.manga_source_id, &body.manga_id, tracker_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(entry))
}

/// `DELETE /track/{tracker}/unlink` — unlink a manga from a tracker.
async fn unlink_manga(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(tracker): Path<String>,
    Json(body): Json<UnlinkBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let pool = database.pool().await;
    sqlx::query(
        "DELETE FROM track WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
    )
    .bind(tracker_id.as_str())
    .bind(&body.manga_source_id)
    .bind(&body.manga_id)
    .execute(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);
    Ok(Json(json!({ "success": true })))
}

/// `GET /track/{tracker}/entries` — all linked manga for this tracker.
async fn get_tracker_entries(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(tracker): Path<String>,
) -> Result<Json<Vec<shared::track::types::TrackEntry>>, AppError> {
    let tracker_id = parse_tracker(&tracker)?;
    let pool = database.pool().await;
    let rows: Vec<TrackRow> = sqlx::query_as(
        "SELECT manga_source_id, manga_id, tracker_id, remote_id, library_id, title, \
         last_chapter_read, total_chapters, status, score, start_date, finish_date, \
         tracking_url, private, updated_at \
         FROM track WHERE tracker_id = ? ORDER BY updated_at DESC",
    )
    .bind(tracker_id.as_str())
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);
    Ok(Json(rows.into_iter().map(|r| r.into_entry()).collect()))
}

/// `GET /track/sync-queue` — pending sync items.
async fn get_sync_queue(
    StateExtractor(State { database, .. }): StateExtractor<State>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let pool = database.pool().await;
    let rows: Vec<(String, String, String, String, i32, String, i32)> = sqlx::query_as(
        "SELECT tracker_id, manga_source_id, manga_id, remote_id, chapters_read, \
         COALESCE(status, ''), created_at \
         FROM track_sync_queue ORDER BY created_at",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(tracker_id, source_id, manga_id, remote_id, chapters_read, status, _at)| {
            json!({
                "tracker_id": tracker_id,
                "manga_source_id": source_id,
                "manga_id": manga_id,
                "remote_id": remote_id,
                "chapters_read": chapters_read,
                "status": status,
            })
        })
        .collect();
    Ok(Json(items))
}

/// `POST /track/pull-sync` — pull progress from all trackers for all tracked manga.
async fn pull_sync(
    StateExtractor(State {
        database,
        track_state,
        ..
    }): StateExtractor<State>,
) -> Result<Json<Vec<String>>, AppError> {
    let messages = crate::track::sync::pull_sync_all(&database, &track_state.http_client)
        .await
        .map_err(|e| AppError::Other(e))?;
    Ok(Json(messages))
}

/// DB row mirror for `track` — uses `Option<String>` for `status` since
/// `TrackStatus` doesn't implement `sqlx::Type`. Converted to `TrackEntry`
/// via `into_entry()`.
#[derive(sqlx::FromRow)]
struct TrackRow {
    manga_source_id: String,
    manga_id: String,
    tracker_id: String,
    remote_id: Option<String>,
    library_id: Option<String>,
    title: Option<String>,
    last_chapter_read: i32,
    total_chapters: Option<i32>,
    status: Option<String>,
    score: Option<i32>,
    start_date: Option<String>,
    finish_date: Option<String>,
    tracking_url: Option<String>,
    private: bool,
    updated_at: String,
}

impl TrackRow {
    fn into_entry(self) -> shared::track::types::TrackEntry {
        shared::track::types::TrackEntry {
            manga_source_id: self.manga_source_id,
            manga_id: self.manga_id,
            tracker_id: self.tracker_id,
            remote_id: self.remote_id,
            library_id: self.library_id,
            title: self.title,
            last_chapter_read: self.last_chapter_read,
            total_chapters: self.total_chapters,
            status: self.status.as_deref().and_then(shared::track::types::TrackStatus::from_mihon_status),
            score: self.score,
            start_date: self.start_date,
            finish_date: self.finish_date,
            tracking_url: self.tracking_url,
            private: self.private,
            updated_at: self.updated_at,
        }
    }
}

// --- helpers -----------------------------------------------------------------

/// Parse a path-supplied tracker id and reject anything we don't know.
fn parse_tracker(raw: &str) -> Result<TrackerService, AppError> {
    match raw {
        "anilist" => Ok(TrackerService::AniList),
        "myanimelist" => Ok(TrackerService::MyAnimeList),
        other => Err(AppError::OAuth(format!(
            "unknown tracker: `{other}` (expected anilist or myanimelist)"
        ))),
    }
}

/// Persist the `AuthToken` for a tracker, replacing any existing row.
async fn store_token(
    database: &std::sync::Arc<shared::database::Database>,
    tracker: TrackerService,
    token: &AuthToken,
) -> Result<(), AppError> {
    let token_json = serde_json::to_string(token)
        .map_err(|e| AppError::Other(anyhow::anyhow!("serialize AuthToken: {e}")))?;
    let expires_at = token
        .created_at
        .zip(token.expires_in)
        .map(|(created, ttl)| created.saturating_add(ttl))
        .and_then(unix_to_iso);
    let pool = database.pool().await;
    sqlx::query(
        "INSERT INTO tracker_auth (tracker_id, token_json, expires_at, created_at) \
         VALUES (?, ?, ?, datetime('now')) \
         ON CONFLICT(tracker_id) DO UPDATE SET \
           token_json = excluded.token_json, \
           expires_at = excluded.expires_at, \
           created_at = excluded.created_at",
    )
    .bind(tracker.as_str())
    .bind(&token_json)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    Ok(())
}

/// Format a Unix epoch (seconds) as an ISO-8601 UTC string for the
/// `tracker_auth.expires_at` column. Returns `None` for an out-of-range
/// timestamp (before 1970 or after year 9999).
fn unix_to_iso(epoch: i64) -> Option<String> {
    if epoch < 0 {
        return None;
    }
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0)?;
    Some(datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Load the access token for a tracker, refreshing it if expired (MAL only).
/// Returns the raw access_token string.
pub(crate) async fn get_valid_token(
    database: &Arc<shared::database::Database>,
    tracker: TrackerService,
) -> Result<String, AppError> {
    let pool = database.pool().await;
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT token_json, expires_at FROM tracker_auth WHERE tracker_id = ?",
    )
    .bind(tracker.as_str())
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);

    let (token_json, expires_at) = row.ok_or_else(|| {
        AppError::OAuth(format!("not logged in to {}", tracker.as_str()))
    })?;

    let mut token: AuthToken = serde_json::from_str(&token_json)
        .map_err(|e| AppError::Other(anyhow::anyhow!("corrupt token: {e}")))?;

    // Check if token is expired (or within 5 min of expiry).
    let needs_refresh = expires_at
        .as_ref()
        .map(|ea| {
            chrono::DateTime::parse_from_rfc3339(ea)
                .map(|dt| {
                    let now = chrono::Utc::now();
                    dt < now + chrono::Duration::minutes(5)
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if needs_refresh {
        match tracker {
            TrackerService::MyAnimeList => {
                let auth = MalAuth::new();
                let new_token = auth.refresh_token(&token).await?;
                store_token(database, tracker, &new_token).await?;
                token = new_token;
            }
            TrackerService::AniList => {
                // AniList implicit-grant tokens don't refresh.
                // If it's truly expired, the API call will fail and we'll
                // surface "Re-login to AniList" to the user.
            }
        }
    }

    Ok(token.access_token)
}

/// Read a single track entry from the database.
async fn get_track_entry(
    database: &Arc<shared::database::Database>,
    manga_source_id: &str,
    manga_id: &str,
    tracker: TrackerService,
) -> Result<Option<shared::track::types::TrackEntry>, AppError> {
    let pool = database.pool().await;
    let row: Option<TrackRow> = sqlx::query_as(
        "SELECT manga_source_id, manga_id, tracker_id, remote_id, library_id, title, \
         last_chapter_read, total_chapters, status, score, start_date, finish_date, \
         tracking_url, private, updated_at \
         FROM track WHERE tracker_id = ? AND manga_source_id = ? AND manga_id = ?",
    )
    .bind(tracker.as_str())
    .bind(manga_source_id)
    .bind(manga_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Other(e.into()))?;
    drop(pool);
    Ok(row.map(|r| r.into_entry()))
}
