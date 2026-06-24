use anyhow::{Context, Result};
use shared::model::ChapterId;

/// MangaDex API base URL.
const MANGADEX_API: &str = "https://api.mangadex.org";

/// Sync read progress to MangaDex using the manga UUID and chapter UUID.
///
/// Used when the source is a MangaDex source. The `manga_id` from the
/// chapter references the MangaDex manga UUID, and `chapter_id.value()`
/// is the MangaDex chapter UUID.
pub async fn sync(
    token: &str,
    chapter_id: &ChapterId,
    chapter_number: Option<f32>,
) -> Result<()> {
    let mangadex_manga_id = chapter_id.manga_id().value();
    let md_chapter_uuid = chapter_id.value();

    let body = serde_json::json!({
        "chapterIdsRead": [md_chapter_uuid],
    });

    let client = reqwest::Client::new();
    let response = client
        .put(format!(
            "{MANGADEX_API}/manga/{mangadex_manga_id}/read"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("MangaDex API request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        anyhow::bail!("MangaDex API error (HTTP {status}): {body_text}");
    }

    let chapter_display = chapter_number
        .map(|n| format!("Ch. {n}"))
        .unwrap_or_else(|| {
            let truncated: String = md_chapter_uuid.chars().take(8).collect();
            truncated
        });

    log::info!(
        "MangaDex sync complete: {chapter_display} for manga {mangadex_manga_id}"
    );

    Ok(())
}

/// Sync read progress to MangaDex using a mapped manga ID.
///
/// Used when the source is NOT MangaDex but the user has provided a
/// MangaDex ID mapping. Sends the chapter ID as read under the mapped
/// manga.
pub async fn sync_by_manga_id(
    token: &str,
    mangadex_manga_id: &str,
    chapter_id: &str,
    _chapter_number: Option<f32>,
) -> Result<()> {
    let body = serde_json::json!({
        "chapterIdsRead": [chapter_id],
    });

    let client = reqwest::Client::new();
    let response = client
        .put(format!(
            "{MANGADEX_API}/manga/{mangadex_manga_id}/read"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("MangaDex API (mapped) request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        anyhow::bail!("MangaDex API error (HTTP {status}): {body_text}");
    }

    log::info!(
        "MangaDex sync complete (mapped manga {mangadex_manga_id}, chapter {chapter_id})"
    );

    Ok(())
}
