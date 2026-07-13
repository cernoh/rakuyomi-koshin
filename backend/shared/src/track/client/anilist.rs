//! AniList GraphQL API client for manga search and progress sync.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::track::types::{
    ProgressUpdate, TrackerMangaSearchResult, TrackerProgress, TrackerUpdateResult,
};

use super::TrackerClient;

const ANILIST_GRAPHQL_URL: &str = "https://graphql.anilist.co";

/// AniList tracker API client.
#[derive(Clone, Default)]
pub struct AniListClient;

impl AniListClient {
    pub fn new() -> Self {
        Self
    }
}

// --- GraphQL request/response types ----------------------------------------

#[derive(Serialize)]
struct GraphQLRequest<Q: Serialize> {
    query: &'static str,
    variables: Q,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
}

// --- Search ----------------------------------------------------------------

#[derive(Serialize)]
struct SearchVariables {
    search: String,
}

#[derive(Deserialize)]
struct SearchData {
    page: Option<SearchPage>,
}

#[derive(Deserialize)]
struct SearchPage {
    media: Option<Vec<SearchMedia>>,
}

#[derive(Deserialize)]
struct SearchMedia {
    id: i64,
    title: Option<MediaTitle>,
    chapters: Option<i32>,
    coverimage: Option<MediaCoverImage>,
    description: Option<Option<String>>,
}

#[derive(Deserialize)]
struct MediaTitle {
    romaji: Option<String>,
    english: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaCoverImage {
    extra_large: Option<String>,
    large: Option<String>,
}

// --- Update progress -------------------------------------------------------

#[derive(Serialize)]
struct UpdateVariables {
    media_id: i64,
    progress: i32,
    status: Option<String>,
    score: Option<i32>,
}

#[derive(Deserialize)]
struct UpdateData {
    #[serde(rename = "SaveMediaListEntry")]
    save_medialist_entry: Option<UpdateEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateEntry {
    id: i64,
    progress: Option<i32>,
    status: Option<String>,
}

// --- Get progress ----------------------------------------------------------

#[derive(Serialize)]
struct ProgressVariables {
    media_id: i64,
}

#[derive(Deserialize)]
struct ProgressData {
    media: Option<ProgressMedia>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressMedia {
    id: i64,
    chapters: Option<i32>,
    media_list_entry: Option<ProgressEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEntry {
    id: i64,
    progress: Option<i32>,
    status: Option<String>,
    score_raw: Option<i32>,
    started_at: Option<DateField>,
    completed_at: Option<DateField>,
}

#[derive(Deserialize)]
struct DateField {
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
}

// --- Helper ----------------------------------------------------------------

fn anilist_status_to_track(s: &str) -> Option<crate::track::types::TrackStatus> {
    match s {
        "CURRENT" => Some(crate::track::types::TrackStatus::CurrentlyReading),
        "COMPLETED" => Some(crate::track::types::TrackStatus::Completed),
        "PAUSED" => Some(crate::track::types::TrackStatus::OnHold),
        "DROPPED" => Some(crate::track::types::TrackStatus::Dropped),
        "PLANNING" => Some(crate::track::types::TrackStatus::PlanToRead),
        "REPEATING" => Some(crate::track::types::TrackStatus::Repeating),
        _ => None,
    }
}

fn track_status_to_anilist(s: &crate::track::types::TrackStatus) -> &'static str {
    match s {
        crate::track::types::TrackStatus::CurrentlyReading => "CURRENT",
        crate::track::types::TrackStatus::Completed => "COMPLETED",
        crate::track::types::TrackStatus::OnHold => "PAUSED",
        crate::track::types::TrackStatus::Dropped => "DROPPED",
        crate::track::types::TrackStatus::PlanToRead => "PLANNING",
        crate::track::types::TrackStatus::Repeating => "REPEATING",
    }
}

fn date_field_to_string(d: &DateField) -> Option<String> {
    match (d.year, d.month, d.day) {
        (Some(y), Some(m), Some(d)) => Some(format!("{y:04}-{m:02}-{d:02}")),
        _ => None,
    }
}

fn extract_graphql_errors<T>(resp: &GraphQLResponse<T>) -> String {
    resp.errors
        .as_ref()
        .map(|errs| {
            errs.iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

#[async_trait::async_trait]
impl TrackerClient for AniListClient {
    async fn search_manga(
        &self,
        client: &Client,
        token: &str,
        query: &str,
    ) -> Result<Vec<TrackerMangaSearchResult>> {
        const SEARCH_QUERY: &str = r#"
            query ($search: String) {
                Page(page: 1, perPage: 20) {
                    media(search: $search, type: MANGA, format_not: NOVEL) {
                        id
                        title { romaji english }
                        chapters
                        coverImage { extraLarge large }
                        description(asHtml: false)
                    }
                }
            }
        "#;

        let req = GraphQLRequest {
            query: SEARCH_QUERY,
            variables: SearchVariables {
                search: query.to_string(),
            },
        };

        let resp: GraphQLResponse<SearchData> = client
            .post(ANILIST_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&req)
            .send()
            .await
            .context("AniList search request failed")?
            .json()
            .await
            .context("AniList search response parse failed")?;

        let data = match resp.data {
            Some(d) => d,
            None => bail!("AniList search returned errors: {}", extract_graphql_errors(&resp)),
        };

        let results = data
            .page
            .and_then(|p| p.media)
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let title = m
                    .title
                    .and_then(|t| t.romaji.or(t.english))
                    .unwrap_or_else(|| "Unknown".to_string());
                let cover_url = m.coverimage.and_then(|c| c.extra_large.or(c.large));
                let description = m.description.flatten();
                TrackerMangaSearchResult {
                    remote_id: m.id.to_string(),
                    title,
                    total_chapters: m.chapters,
                    cover_url,
                    description,
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
        const UPDATE_MUTATION: &str = r#"
            mutation ($mediaId: Int, $progress: Int, $status: MediaListStatus, $score: Int) {
                SaveMediaListEntry(mediaId: $mediaId, progress: $progress, status: $status, scoreRaw: $score) {
                    id
                    progress
                    status
                }
            }
        "#;

        let media_id: i64 = remote_id.parse().context("invalid AniList media id")?;
        let status = progress
            .status
            .as_ref()
            .map(track_status_to_anilist)
            .map(str::to_string);

        let req = GraphQLRequest {
            query: UPDATE_MUTATION,
            variables: UpdateVariables {
                media_id,
                progress: progress.chapters_read,
                status,
                score: progress.score,
            },
        };

        let resp: GraphQLResponse<UpdateData> = client
            .post(ANILIST_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&req)
            .send()
            .await
            .context("AniList update request failed")?
            .json()
            .await
            .context("AniList update response parse failed")?;

        let data = match resp.data {
            Some(d) => d,
            None => bail!(
                "AniList update returned errors: {}",
                extract_graphql_errors(&resp)
            ),
        };
        let entry = data
            .save_medialist_entry
            .context("AniList returned null SaveMediaListEntry")?;

        Ok(TrackerUpdateResult {
            remote_id: remote_id.to_string(),
            library_id: Some(entry.id.to_string()),
            chapters_read: entry.progress.unwrap_or(progress.chapters_read),
            status: entry.status.as_deref().and_then(anilist_status_to_track),
        })
    }

    async fn get_progress(
        &self,
        client: &Client,
        token: &str,
        remote_id: &str,
    ) -> Result<Option<TrackerProgress>> {
        const PROGRESS_QUERY: &str = r#"
            query ($mediaId: Int) {
                Media(id: $mediaId, type: MANGA) {
                    id
                    chapters
                    mediaListEntry {
                        id
                        progress
                        status
                        scoreRaw: score(format: POINT_100_INT)
                        startedAt { year month day }
                        completedAt { year month day }
                    }
                }
            }
        "#;

        let media_id: i64 = remote_id.parse().context("invalid AniList media id")?;

        let req = GraphQLRequest {
            query: PROGRESS_QUERY,
            variables: ProgressVariables { media_id },
        };

        let resp: GraphQLResponse<ProgressData> = client
            .post(ANILIST_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&req)
            .send()
            .await
            .context("AniList progress request failed")?
            .json()
            .await
            .context("AniList progress response parse failed")?;

        let data = match resp.data {
            Some(d) => d,
            None => {
                let errs = extract_graphql_errors(&resp);
                if errs.contains("Not Found") || errs.contains("not found") {
                    return Ok(None);
                }
                bail!("AniList progress returned errors: {errs}");
            }
        };

        let media = match data.media {
            Some(m) => m,
            None => return Ok(None),
        };

        let entry = match media.media_list_entry {
            Some(e) => e,
            None => return Ok(None),
        };

        Ok(Some(TrackerProgress {
            remote_id: media.id.to_string(),
            library_id: Some(entry.id.to_string()),
            chapters_read: entry.progress.unwrap_or(0),
            total_chapters: media.chapters,
            status: entry.status.as_deref().and_then(anilist_status_to_track),
            score: entry.score_raw,
            start_date: entry.started_at.as_ref().and_then(date_field_to_string),
            finish_date: entry.completed_at.as_ref().and_then(date_field_to_string),
        }))
    }
}
