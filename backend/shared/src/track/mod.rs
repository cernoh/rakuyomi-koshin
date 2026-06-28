//! Tracking domain types shared between the server and other consumers.
//!
//! This module is intentionally types-only: it does not depend on the
//! database, the HTTP server, or the use case layer. Any code that needs
//! to talk about AniList/MyAnimeList tracking (model serialization, DB
//! row mapping, OAuth protocol messages) imports from here.

pub mod types;

pub use types::*;
