//! HTTP route handlers for the `/track/*` API surface.
//!
//! Endpoints:
//!
//! - `GET    /track/services`           — list known trackers + login state
//! - `POST   /track/{tracker}/auth-url` — start an auth flow, get URL + qr_id
//! - `GET    /track/qr/{qr_id}`         — fetch the 300x300 QR PNG
//! - `POST   /track/{tracker}/auth`     — submit token (AniList) or code+state (MAL)
//! - `DELETE /track/{tracker}/auth`     — clear stored credentials
//! - `GET    /track/{tracker}/status`   — current login state

use axum::{
    extract::{Path, State as StateExtractor},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::track::types::{AuthToken, TrackerService};

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
        .route("/{tracker}/auth-url", post(generate_auth_url))
        .route("/qr/{qr_id}", get(get_qr_code))
        .route("/{tracker}/auth", post(submit_auth).delete(clear_auth))
        .route("/{tracker}/status", get(check_status))
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
            // Implicit grant — no session to track; the returned
            // `state` and `code_verifier` are both empty.
            let (url, _state, _verifier) = AniListAuth::new().generate_auth_url().await?;
            (url, String::new())
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

    Ok(Json(AuthUrlResponse {
        url: url.as_str().to_string(),
        qr_id: state,
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
