//! Core domain types for external service tracking (AniList + MyAnimeList).
//!
//! These mirror the columns of the `track` and `tracker_auth` SQLite
//! tables created in `20260628000001_create_tracking_tables.sql` and
//! follow the newtype / serde-transparent conventions used elsewhere in
//! the `shared` crate (see `model.rs` for `SourceId`, `MangaId`,
//! `ChapterId`).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Identifies a tracking service. Serializes to its lower-snake-case
/// tracker id, which is the value stored in the `tracker_id` TEXT
/// columns of `track` and `tracker_auth` and used as the URL path
/// segment in `/track/{tracker}/*` routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerService {
    AniList,
    MyAnimeList,
}

impl TrackerService {
    /// Stable lowercase identifier used in the database and URL paths.
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackerService::AniList => "anilist",
            TrackerService::MyAnimeList => "myanimelist",
        }
    }

    pub const ALL: &'static [TrackerService] =
        &[TrackerService::AniList, TrackerService::MyAnimeList];
}

impl fmt::Display for TrackerService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TrackerService {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anilist" => Ok(TrackerService::AniList),
            "myanimelist" => Ok(TrackerService::MyAnimeList),
            _ => Err(()),
        }
    }
}

/// Reading status of a tracked manga. Mirrors Mihon's status enum and is
/// stored as SCREAMING_SNAKE_CASE TEXT in the `track.status` column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackStatus {
    CurrentlyReading,
    Completed,
    OnHold,
    Dropped,
    PlanToRead,
    Repeating,
}

impl TrackStatus {
    /// Parse a Mihon-style SCREAMING_SNAKE_CASE status string.
    pub fn from_mihon_status(s: &str) -> Option<Self> {
        Some(match s {
            "CURRENTLY_READING" => Self::CurrentlyReading,
            "COMPLETED" => Self::Completed,
            "ON_HOLD" => Self::OnHold,
            "DROPPED" => Self::Dropped,
            "PLAN_TO_READ" => Self::PlanToRead,
            "REPEATING" => Self::Repeating,
            _ => return None,
        })
    }

    /// Render the status in Mihon's SCREAMING_SNAKE_CASE form.
    pub fn as_mihon_status(&self) -> &'static str {
        match self {
            Self::CurrentlyReading => "CURRENTLY_READING",
            Self::Completed => "COMPLETED",
            Self::OnHold => "ON_HOLD",
            Self::Dropped => "DROPPED",
            Self::PlanToRead => "PLAN_TO_READ",
            Self::Repeating => "REPEATING",
        }
    }
}

/// Direction in which tracking data is reconciled between the local DB
/// and the remote service. `TwoWay` means push then pull, sequentially.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SyncDirection {
    Push,
    Pull,
    TwoWay,
}

/// A row from the `track` table, minus the internal auto-increment `id`.
/// `tracker_id` is stored as the lower-snake-case service identifier
/// (e.g. `"anilist"`, `"myanimelist"`) so it round-trips through
/// `TrackerService::from_str`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct TrackEntry {
    pub manga_source_id: String,
    pub manga_id: String,
    pub tracker_id: String,
    pub remote_id: Option<String>,
    pub library_id: Option<String>,
    pub title: Option<String>,
    pub last_chapter_read: i32,
    pub total_chapters: Option<i32>,
    pub status: Option<TrackStatus>,
    pub score: Option<i32>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub tracking_url: Option<String>,
    pub private: bool,
    pub updated_at: String,
}

/// OAuth token payload, serialized to JSON and stored in the
/// `tracker_auth.token_json` column. Field names mirror what each
/// service returns from its token endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
    /// Epoch seconds at which the token was stored. Used together with
    /// `expires_in` to compute `expires_at` on the DB row.
    pub created_at: Option<i64>,
}

/// Wire format for a PKCE session — the in-memory map stores the same
/// shape plus an `Instant` (not serializable), so `PkceSessionData` is
/// what crosses JSON boundaries (e.g. tests, future IPC). The server
/// uses a richer `PkceSession` type that wraps this.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PkceSessionData {
    /// Random 32-byte hex string — the OAuth `state` parameter and the
    /// key used to look the session up in `TrackState`.
    pub state: String,
    /// PKCE `code_verifier`; hashed to derive the `code_challenge` sent
    /// to the auth endpoint, then sent in full to the token endpoint.
    pub code_verifier: String,
    /// Lower-snake-case tracker id (`"anilist"` or `"myanimelist"`).
    pub tracker_id: String,
}
