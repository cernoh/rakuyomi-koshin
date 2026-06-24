use anyhow::{Context, Result};
use shared::database::MangaTrackerIds;

/// AniList GraphQL API endpoint.
const ANILIST_API: &str = "https://graphql.anilist.co";

/// Sync read progress to AniList.
///
/// Uses the AniList GraphQL API to update the manga's read progress.
/// Requires an AniList access token and a manga-to-anilist ID mapping.
/// Falls back to searching by title if no AniList ID is stored.
pub async fn sync(
    token: &str,
    tracked_ids: &MangaTrackerIds,
    manga_title: &str,
    chapter_number: Option<f32>,
) -> Result<()> {
    let anilist_id = match tracked_ids.anilist_id {
        Some(id) => id,
        None => {
            // Try to look up the AniList ID by manga title
            match search_anilist_id(token, manga_title).await? {
                Some(id) => id,
                None => {
                    log::warn!(
                        "No AniList ID found for manga '{manga_title}', skipping sync"
                    );
                    return Ok(());
                }
            }
        }
    };

    let progress = chapter_number.map(|n| n as i32).unwrap_or(1);

    let query = serde_json::json!({
        "query": r#"
            mutation ($mediaId: Int, $progress: Int) {
                SaveMediaListEntry(mediaId: $mediaId, progress: $progress) {
                    id
                    progress
                }
            }
        "#,
        "variables": {
            "mediaId": anilist_id,
            "progress": progress,
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(ANILIST_API)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&query)
        .send()
        .await
        .context("AniList API request failed")?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse AniList response")?;

    if !status.is_success() {
        let msg = body["errors"]
            .as_array()
            .and_then(|e| e.first())
            .and_then(|e| e["message"].as_str())
            .unwrap_or("unknown error");
        anyhow::bail!("AniList API error (HTTP {status}): {msg}");
    }

    log::info!(
        "AniList sync complete: media={anilist_id} progress={progress}"
    );

    Ok(())
}

/// Search AniList for a manga by title and return the first match's ID.
async fn search_anilist_id(token: &str, title: &str) -> Result<Option<i64>> {
    let query = serde_json::json!({
        "query": r#"
            query ($search: String) {
                Page(page: 1, perPage: 1) {
                    media(search: $search, type: MANGA) {
                        id
                        title {
                            romaji
                            english
                        }
                    }
                }
            }
        "#,
        "variables": {
            "search": title,
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(ANILIST_API)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&query)
        .send()
        .await
        .context("AniList search request failed")?;

    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse AniList search response")?;

    let media = body["data"]["Page"]["media"]
        .as_array()
        .and_then(|arr| arr.first());

    match media {
        Some(entry) => {
            let id = entry["id"]
                .as_i64()
                .context("AniList search returned invalid ID")?;
            log::info!("Found AniList ID {id} for manga '{title}'");
            Ok(Some(id))
        }
        None => {
            log::warn!("No AniList entry found for manga '{title}'");
            Ok(None)
        }
    }
}
