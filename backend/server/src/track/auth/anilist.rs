//! AniList OAuth via the implicit-grant flow.
//!
//! AniList only supports implicit grant for public OAuth clients: the
//! authorization endpoint returns the access token directly in the
//! URL fragment, so there's no token exchange step and no refresh
//! tokens. The user is redirected back to a callback page that hands
//! the token to the caller via JavaScript — in our flow the Lua
//! frontend reads the fragment and POSTs the token to
//! `/track/anilist/auth`.

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use shared::track::types::AuthToken;
use url::Url;

use crate::AppError;

use super::TrackerAuth;

const ANILIST_AUTH_URL: &str = "https://anilist.co/api/v2/oauth/authorize";
const ANILIST_GRAPHQL_URL: &str = "https://graphql.anilist.co";
/// Public OAuth client id assigned to RakuYomi (see PROJECT.md).
const ANILIST_CLIENT_ID: &str = "16329";

/// AniList OAuth client.
#[derive(Clone)]
pub struct AniListAuth {
    client_id: String,
    client: Client,
}

impl AniListAuth {
    pub fn new() -> Self {
        Self {
            client_id: ANILIST_CLIENT_ID.to_string(),
            client: Client::new(),
        }
    }
}

impl Default for AniListAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrackerAuth for AniListAuth {
    async fn generate_auth_url(&self) -> Result<(Url, String, String), AppError> {
        // Implicit grant: `response_type=token`. The user is redirected
        // back to our callback with `#access_token=...` in the URL
        // fragment. No `state` is needed because implicit grant doesn't
        // issue an authorization code.
        let mut url = Url::parse(ANILIST_AUTH_URL).map_err(|e| AppError::Other(e.into()))?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "token");
        // (url, state, code_verifier) — both empty for implicit grant.
        Ok((url, String::new(), String::new()))
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
    ) -> Result<AuthToken, AppError> {
        // In the implicit flow, `code` IS the access token from the URL
        // fragment. We trust the caller to have extracted it; the
        // caller will then call `verify_token` to confirm it works
        // before we persist it.
        Ok(AuthToken {
            access_token: code.to_string(),
            token_type: Some("bearer".to_string()),
            refresh_token: None,
            expires_in: None,
            scope: None,
            created_at: Some(unix_now()),
        })
    }

    async fn refresh_token(&self, _token: &AuthToken) -> Result<AuthToken, AppError> {
        // AniList implicit-grant tokens cannot be refreshed. The user
        // must re-authenticate through the QR code flow.
        Err(AppError::OAuth(
            "AniList implicit-grant tokens cannot be refreshed — please re-authenticate"
                .to_string(),
        ))
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<String, AppError> {
        // Hit the AniList GraphQL endpoint with a minimal Viewer query.
        // A valid token returns `{ "data": { "Viewer": { "id": ..., "name": ... } } }`.
        // We extract the inner Viewer object so the caller can store a
        // human-friendly username.
        let query = r#"{ Viewer { id name } }"#;

        let response = self
            .client
            .post(ANILIST_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .map_err(|e| AppError::OAuth(format!("AniList verify request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::OAuth(format!(
                "AniList verify returned status {} — token may be invalid",
                status.as_u16()
            )));
        }

        #[derive(Deserialize)]
        struct ViewerResponse {
            data: ViewerData,
        }
        #[derive(Deserialize)]
        struct ViewerData {
            viewer: ViewerFields,
        }
        #[derive(Deserialize)]
        struct ViewerFields {
            id: i64,
            name: String,
        }

        let body: ViewerResponse = response
            .json()
            .await
            .map_err(|e| AppError::OAuth(format!("AniList verify parse failed: {e}")))?;

        Ok(serde_json::json!({
            "id": body.data.viewer.id,
            "name": body.data.viewer.name,
        })
        .to_string())
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
