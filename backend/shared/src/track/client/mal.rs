//! MyAnimeList REST v2 API client for manga search and progress sync.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::track::types::{
    ProgressUpdate, TrackerMangaSearchResult, TrackerProgress, TrackerUpdateResult,
};

use super::TrackerClient;

const MAL_API_BASE: &str = "https://api.myanimelist.net/v2";

/// MAL tracker API client.
#[derive(Clone, Default)]
pub struct MalClient;

impl MalClient {
    pub fn new() -> Self {
        Self
    }
}

// --- Response types --------------------------------------------------------

#[derive(Deserialize)]
struct MalSearchResponse {
    data: Option<Vec<MalSearchNode>>,
}

#[derive(Deserialize)]
struct MalSearchNode {
    node: MalSearchNodeData,
}

#[derive(Deserialize)]
struct MalSearchNodeData {
    id: i64,
    title: Option<String>,
    num_chapters: Option<i32>,
    main_picture: Option<MalPicture>,
    synopsis: Option<String>,
}

#[derive(Deserialize)]
struct MalPicture {
    large: Option<String>,
    medium: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MalMangaDetail {
    id: i64,
    title: Option<String>,
    num_chapters: Option<i32>,
    my_list_status: Option<MalListStatus>,
}

#[derive(Deserialize)]
struct MalListStatus {
    status: Option<String>,
    num_chapters_read: Option<i32>,
    score: Option<i32>,
    start_date: Option<String>,
    finish_date: Option<String>,
}

// --- Helpers ---------------------------------------------------------------

fn mal_status_to_track(s: &str) -> Option<crate::track::types::TrackStatus> {
    match s {
        "reading" => Some(crate::track::types::TrackStatus::CurrentlyReading),
        "completed" => Some(crate::track::types::TrackStatus::Completed),
        "on_hold" => Some(crate::track::types::TrackStatus::OnHold),
        "dropped" => Some(crate::track::types::TrackStatus::Dropped),
        "plan_to_read" => Some(crate::track::types::TrackStatus::PlanToRead),
        "rereading" => Some(crate::track::types::TrackStatus::Repeating),
        _ => None,
    }
}

fn track_status_to_mal(s: &crate::track::types::TrackStatus) -> &'static str {
    match s {
        crate::track::types::TrackStatus::CurrentlyReading => "reading",
        crate::track::types::TrackStatus::Completed => "completed",
        crate::track::types::TrackStatus::OnHold => "on_hold",
        crate::track::types::TrackStatus::Dropped => "dropped",
        crate::track::types::TrackStatus::PlanToRead => "plan_to_read",
        crate::track::types::TrackStatus::Repeating => "rereading",
    }
}

fn is_auth_error(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
}

#[async_trait::async_trait]
impl TrackerClient for MalClient {
    async fn search_manga(
        &self,
        client: &Client,
        token: &str,
        query: &str,
    ) -> Result<Vec<TrackerMangaSearchResult>> {
        let resp = client
            .get(format!("{MAL_API_BASE}/manga"))
            .header("Authorization", format!("Bearer {token}"))
            .query(&[
                ("q", query),
                ("fields", "id,title,num_chapters,main_picture,synopsis"),
                ("limit", "20"),
                ("nsfw", "true"),
            ])
            .send()
            .await
            .context("MAL search request failed")?;

        if is_auth_error(resp.status()) {
            bail!("MAL authentication failed (token expired or invalid)");
        }
        if !resp.status().is_success() {
            bail!("MAL search returned status {}", resp.status());
        }

        let body: MalSearchResponse = resp
            .json()
            .await
            .context("MAL search response parse failed")?;

        let results = body
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|node| {
                let n = node.node;
                TrackerMangaSearchResult {
                    remote_id: n.id.to_string(),
                    title: n.title.unwrap_or_else(|| "Unknown".to_string()),
                    total_chapters: n.num_chapters,
                    cover_url: n
                        .main_picture
                        .and_then(|p| p.large.or(p.medium)),
                    description: n.synopsis,
                }
            })
            .collect();

        Ok(results)
    }

    async fn update_progress(
        &self,
        client: &Client,
        token: &str,
        remote_id: &str,
        _library_id: Option<&str>,
        progress: &ProgressUpdate,
    ) -> Result<TrackerUpdateResult> {
        let manga_id: i64 = remote_id.parse().context("invalid MAL manga id")?;

        let mut form_params: Vec<(&str, String)> = vec![
            (
                "num_chapters_read",
                progress.chapters_read.to_string(),
            ),
        ];

        if let Some(status) = &progress.status {
            form_params.push(("status", track_status_to_mal(status).to_string()));
        }
        if let Some(score) = progress.score {
            // MAL uses 0-10 scale
            form_params.push(("score", score.to_string()));
        }
        if let Some(ref date) = progress.start_date {
            form_params.push(("start_date", date.clone()));
        }
        if let Some(ref date) = progress.finish_date {
            form_params.push(("finish_date", date.clone()));
        }

        let resp = client
            .patch(format!("{MAL_API_BASE}/manga/{manga_id}/my_list_status"))
            .header("Authorization", format!("Bearer {token}"))
            .form(&form_params)
            .send()
            .await
            .context("MAL update request failed")?;

        if is_auth_error(resp.status()) {
            bail!("MAL authentication failed (token expired or invalid)");
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("MAL update returned {status}: {body}");
        }

        let detail: MalMangaDetail = resp
            .json()
            .await
            .context("MAL update response parse failed")?;

        let ls = detail.my_list_status;
        Ok(TrackerUpdateResult {
            remote_id: remote_id.to_string(),
            library_id: None,
            chapters_read: ls
                .as_ref()
                .and_then(|s| s.num_chapters_read)
                .unwrap_or(progress.chapters_read),
            status: ls
                .as_ref()
                .and_then(|s| s.status.as_deref())
                .and_then(mal_status_to_track),
        })
    }

    async fn get_progress(
        &self,
        client: &Client,
        token: &str,
        remote_id: &str,
    ) -> Result<Option<TrackerProgress>> {
        let manga_id: i64 = remote_id.parse().context("invalid MAL manga id")?;

        let resp = client
            .get(format!("{MAL_API_BASE}/manga/{manga_id}"))
            .header("Authorization", format!("Bearer {token}"))
            .query(&[("fields", "num_chapters,my_list_status")])
            .send()
            .await
            .context("MAL progress request failed")?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if is_auth_error(resp.status()) {
            bail!("MAL authentication failed (token expired or invalid)");
        }
        if !resp.status().is_success() {
            bail!("MAL progress returned status {}", resp.status());
        }

        let detail: MalMangaDetail = resp
            .json()
            .await
            .context("MAL progress response parse failed")?;

        let ls = match detail.my_list_status {
            Some(s) => s,
            None => return Ok(None),
        };

        Ok(Some(TrackerProgress {
            remote_id: detail.id.to_string(),
            library_id: None,
            chapters_read: ls.num_chapters_read.unwrap_or(0),
            total_chapters: detail.num_chapters,
            status: ls.status.as_deref().and_then(mal_status_to_track),
            score: ls.score,
            start_date: ls.start_date,
            finish_date: ls.finish_date,
        }))
    }
}
