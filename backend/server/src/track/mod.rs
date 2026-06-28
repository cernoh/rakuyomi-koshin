//! Tracking server module: PKCE session state, OAuth implementations,
//! QR generation, and the `/track/*` HTTP routes.
//!
//! Submodules are wired in incrementally — Plan 01-01 adds `state`,
//! Plan 01-02 adds `auth`, Plan 01-03 adds `routes` and `qr`. The
//! `state` submodule is referenced by the others once they exist, so
//! it must be declared first.

pub mod state;
