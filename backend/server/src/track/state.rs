//! In-memory state for the tracking subsystem.
//!
//! Right now this only holds the PKCE session map for the MAL OAuth
//! flow. Each entry is keyed by a 32-byte random hex `state` string
//! returned in the MAL authorization URL; the entry's `code_verifier`
//! is the PKCE secret sent to the token endpoint during code exchange.
//! Entries are one-time-use (`get_and_remove`) and expire after
//! [`PKCE_SESSION_TTL`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// How long a PKCE session is kept before it can be reaped. Picked to
/// comfortably cover a phone-scan → approve → return flow without
/// keeping stale entries around indefinitely.
pub const PKCE_SESSION_TTL: Duration = Duration::from_secs(15 * 60);

/// A single in-flight MAL PKCE session.
#[derive(Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    /// Lower-snake-case tracker id (`"anilist"` or `"myanimelist"`).
    pub tracker_id: String,
    /// The OAuth `state` parameter — also used as the `qr_id` for
    /// `/track/qr/{qr_id}` so the Lua frontend can re-fetch the QR
    /// image without re-running the auth-url step.
    pub auth_url: String,
    pub created_at: Instant,
}

/// All tracking-related state held by the server. Currently just the
/// PKCE session map; future fields (HTTP clients, rate-limit buckets)
/// would land here.
#[derive(Clone)]
pub struct TrackState {
    pkce_sessions: Arc<Mutex<HashMap<String, PkceSession>>>,
    /// Shared HTTP client for tracker API calls (AniList GraphQL, MAL REST).
    pub http_client: reqwest::Client,
}

impl TrackState {
    pub fn new() -> Self {
        Self {
            pkce_sessions: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        }
    }

    /// Insert a session, evicting any expired entries first. The `state`
    /// string is the random 32-byte hex returned in the auth URL and
    /// used as the QR id.
    pub async fn insert(&self, state: String, session: PkceSession) {
        let mut map = self.pkce_sessions.lock().await;
        Self::cleanup_locked(&mut map);
        map.insert(state, session);
    }

    /// Atomically read and remove a session. PKCE codes are one-time use,
    /// so successful lookups must not leave the entry behind.
    pub async fn get_and_remove(&self, state: &str) -> Option<PkceSession> {
        let mut map = self.pkce_sessions.lock().await;
        Self::cleanup_locked(&mut map);
        map.remove(state)
    }

    /// Look up a session without removing it. Used by the QR endpoint
    /// to render a code for an in-flight auth flow that may still be
    /// pending in the user's phone browser.
    pub async fn peek(&self, state: &str) -> Option<PkceSession> {
        let mut map = self.pkce_sessions.lock().await;
        Self::cleanup_locked(&mut map);
        map.get(state).cloned()
    }

    fn cleanup_locked(map: &mut HashMap<String, PkceSession>) {
        let now = Instant::now();
        map.retain(|_, session| now.duration_since(session.created_at) < PKCE_SESSION_TTL);
    }
}

impl Default for TrackState {
    fn default() -> Self {
        Self::new()
    }
}
