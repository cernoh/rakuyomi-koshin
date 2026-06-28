//! MyAnimeList OAuth via the PKCE authorization-code flow.
//!
//! MAL issues short-lived access tokens (1 hour by default) and
//! long-lived refresh tokens (~1 month). The flow:
//!
//! 1. [`TrackerAuth::generate_auth_url`] builds the authorization URL
//!    with a `code_challenge` (SHA-256 of `code_verifier`) and a
//!    random 32-byte hex `state`. The caller stores the `code_verifier`
//!    in [`crate::track::state::PkceSession`] keyed by `state`.
//! 2. The user authorizes on MAL and is redirected back with a `code`.
//! 3. [`TrackerAuth::exchange_code`] POSTs the `code` plus the
//!    previously-stored `code_verifier` to the token endpoint to
//!    obtain the access + refresh token bundle.
//! 4. [`TrackerAuth::refresh_token`] trades the refresh token for a
//!    fresh access token once the old one expires.

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::Engine;
use rand::Rng;
use reqwest::Client;
use shared::track::types::AuthToken;
use sha2::{Digest, Sha256};
use url::Url;

use crate::AppError;

use super::TrackerAuth;

const MAL_AUTH_URL: &str = "https://myanimelist.net/v1/oauth2/authorize";
const MAL_TOKEN_URL: &str = "https://myanimelist.net/v1/oauth2/token";
const MAL_USER_URL: &str = "https://api.myanimelist.net/v2/users/@me";
/// Public OAuth client id assigned to RakuYomi (see PROJECT.md).
const MAL_CLIENT_ID: &str = "c46c9e24640a64dad5be5ca7a1a53a0f";

/// MyAnimeList OAuth client.
#[derive(Clone)]
pub struct MalAuth {
    client_id: String,
    client: Client,
}

impl MalAuth {
    pub fn new() -> Self {
        Self {
            client_id: MAL_CLIENT_ID.to_string(),
            client: Client::new(),
        }
    }
}

impl Default for MalAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrackerAuth for MalAuth {
    async fn generate_auth_url(&self) -> Result<(Url, String, String), AppError> {
        // 1. Generate PKCE pair: `code_verifier` (random 32 bytes
        //    base64url-no-pad) and `code_challenge` (SHA-256 of the
        //    verifier, base64url-no-pad).
        let code_verifier = random_base64url(32);
        let code_challenge = pkce_challenge(&code_verifier);

        // 2. Generate a 32-byte hex `state` — random, used as the
        //    PKCE session key in `TrackState` and as the `qr_id` in
        //    `/track/qr/{qr_id}`.
        let state = random_hex(32);

        // 3. Build the authorization URL. We don't send a
        //    `redirect_uri` — MAL doesn't require it for PKCE (the
        //    token endpoint validates via `code_verifier`), and
        //    keeping it out avoids one more thing the Lua frontend
        //    has to handle in the callback.
        let mut url = Url::parse(MAL_AUTH_URL).map_err(|e| AppError::Other(e.into()))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);

        // Caller stores `code_verifier` in `PkceSession` keyed by
        // `state`, and reuses `state` as `qr_id` for QR image
        // lookups. The verifier is a secret — it never leaves the
        // server process.
        Ok((url, state, code_verifier))
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<AuthToken, AppError> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", code_verifier),
        ];

        let response = self
            .client
            .post(MAL_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL token exchange failed: {e}")))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL token response read failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::OAuth(format!(
                "MAL token exchange returned status {}: {}",
                status.as_u16(),
                body_text
            )));
        }

        let mut token: AuthToken = serde_json::from_str(&body_text).map_err(|e| {
            AppError::OAuth(format!(
                "MAL token response parse failed: {e}; body={body_text}"
            ))
        })?;
        token.created_at = Some(unix_now());
        Ok(token)
    }

    async fn refresh_token(&self, token: &AuthToken) -> Result<AuthToken, AppError> {
        let refresh = token.refresh_token.as_deref().ok_or_else(|| {
            AppError::OAuth("MAL token has no refresh_token — cannot refresh".to_string())
        })?;

        let params = [
            ("client_id", self.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
        ];

        let response = self
            .client
            .post(MAL_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL refresh request failed: {e}")))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL refresh response read failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::OAuth(format!(
                "MAL refresh returned status {}: {}",
                status.as_u16(),
                body_text
            )));
        }

        let mut new_token: AuthToken = serde_json::from_str(&body_text).map_err(|e| {
            AppError::OAuth(format!(
                "MAL refresh response parse failed: {e}; body={body_text}"
            ))
        })?;
        new_token.created_at = Some(unix_now());
        Ok(new_token)
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<String, AppError> {
        // MAL requires both `Authorization: Bearer` AND the
        // `X-MAL-CLIENT-ID` header. Without either the call returns 401.
        let response = self
            .client
            .get(MAL_USER_URL)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .header("X-MAL-CLIENT-ID", &self.client_id)
            .send()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL verify request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::OAuth(format!(
                "MAL verify returned status {} — token may be invalid",
                status.as_u16()
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::OAuth(format!("MAL verify parse failed: {e}")))?;

        Ok(body.to_string())
    }
}

/// Generate a random N-byte base64url-no-pad string suitable for use
/// as a PKCE `code_verifier` (RFC 7636 §4.1 requires 43–128 chars of
/// unreserved set; 32 bytes base64url is 43 chars — exactly the floor).
fn random_base64url(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill(&mut bytes[..]);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate a random N-byte hex string used as the OAuth `state`
/// parameter and the PKCE session lookup key. 32 bytes = 64 hex chars,
/// which is comfortably above OAuth's "unguessable" requirement.
fn random_hex(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill(&mut bytes[..]);
    hex::encode(bytes)
}

/// Derive the PKCE `code_challenge` from a `code_verifier`:
/// `BASE64URL(SHA256(verifier))` with no padding.
fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
