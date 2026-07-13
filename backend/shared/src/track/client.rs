//! Tracker API client trait and implementations.
//!
//! Each supported tracker (AniList, MyAnimeList) implements the
//! [`TrackerClient`] trait, which covers the three operations needed
//! for progress syncing: searching the tracker's catalog, pushing
//! progress updates, and pulling current progress.

use anyhow::Result;

use crate::track::types::{
    ProgressUpdate, TrackerMangaSearchResult, TrackerProgress, TrackerUpdateResult,
};

pub mod anilist;
pub mod mal;

/// Service-agnostic API surface for tracker catalog + progress operations.
#[async_trait::async_trait]
pub trait TrackerClient: Send + Sync {
    /// Search the tracker's catalog for manga matching `query`.
    async fn search_manga(
        &self,
        client: &reqwest::Client,
        token: &str,
        query: &str,
    ) -> Result<Vec<TrackerMangaSearchResult>>;

    /// Create or update a manga entry on the tracker. Returns the
    /// remote library entry state after the update.
    async fn update_progress(
        &self,
        client: &reqwest::Client,
        token: &str,
        remote_id: &str,
        library_id: Option<&str>,
        progress: &ProgressUpdate,
    ) -> Result<TrackerUpdateResult>;

    /// Fetch current progress for a manga from the tracker.
    /// Returns `None` if the manga is not on the user's list.
    async fn get_progress(
        &self,
        client: &reqwest::Client,
        token: &str,
        remote_id: &str,
    ) -> Result<Option<TrackerProgress>>;
}
