//! Tracking server module: PKCE session state, OAuth implementations,
//! QR generation, and the `/track/*` HTTP routes.
//!
//! Submodules are wired in incrementally — Plan 01-01 added `state`,
//! Plan 01-02 adds `auth` and `qr`, Plan 01-03 will add `routes`.

pub mod auth;
pub mod qr;
pub mod state;
