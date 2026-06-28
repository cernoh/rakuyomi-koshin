//! OAuth protocol support for the supported tracking services.
//!
//! Each service implements [`TrackerAuth`]. The trait abstracts the
//! four operations that any service needs:
//!
//! 1. [`TrackerAuth::generate_auth_url`] — produce the URL the user
//!    opens on a phone browser. For PKCE services (MAL) it also
//!    generates a `state` and `code_verifier`; for implicit-grant
//!    services (AniList) both are empty.
//! 2. [`TrackerAuth::exchange_code`] — turn the authorization code
//!    (PKCE) or the implicit-grant access token into a stored
//!    [`AuthToken`].
//! 3. [`TrackerAuth::refresh_token`] — exchange a refresh token for a
//!    new access token. AniList returns an error here because its
//!    implicit-grant tokens cannot be refreshed.
//! 4. [`TrackerAuth::verify_token`] — call the service's "who am I?"
//!    endpoint with the access token and surface the result. Used
//!    before persisting a freshly-obtained token so we never store a
//!    bogus one.

use async_trait::async_trait;
use shared::track::types::AuthToken;
use url::Url;

use crate::AppError;

pub mod anilist;
pub mod mal;

pub use anilist::AniListAuth;
pub use mal::MalAuth;

/// Service-agnostic OAuth surface. See module docs for the lifecycle.
#[async_trait]
pub trait TrackerAuth: Send + Sync {
    /// Build the URL the user opens in a real browser to authorize the
    /// app. Returns:
    ///
    /// - the URL itself,
    /// - a `state` string the caller uses as the lookup key for the
    ///   PKCE session (also exposed to the client as `qr_id` for
    ///   `/track/qr/{qr_id}`), and
    /// - the PKCE `code_verifier` (empty for implicit-grant services
    ///   like AniList).
    ///
    /// For PKCE (MAL) the caller must store both `state` and
    /// `code_verifier` in [`crate::track::state::PkceSession`] so the
    /// later `exchange_code` call can complete the flow.
    async fn generate_auth_url(&self) -> Result<(Url, String, String), AppError>;

    /// Turn an authorization code (PKCE) or implicit-grant access
    /// token into a structured [`AuthToken`].
    ///
    /// - For MAL this performs the actual `POST /v1/oauth2/token`
    ///   exchange and returns the parsed token bundle.
    /// - For AniList the "code" is the access token from the URL
    ///   fragment, returned as-is.
    async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<AuthToken, AppError>;

    /// Refresh an expired access token. Only services that issue
    /// refresh tokens (MAL) implement this; AniList returns an error
    /// since implicit-grant tokens cannot be refreshed.
    async fn refresh_token(&self, token: &AuthToken) -> Result<AuthToken, AppError>;

    /// Confirm that `token` is currently valid by calling the service's
    /// "who am I?" endpoint. Returns a JSON-ish string with the user
    /// identity for storage / debugging. On 401 / network error
    /// returns `AppError::OAuth(...)`.
    async fn verify_token(&self, token: &AuthToken) -> Result<String, AppError>;
}
