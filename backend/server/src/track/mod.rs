//! Tracking server module: PKCE session state, OAuth implementations,
//! QR generation, and the `/track/*` HTTP routes.

pub mod auth;
pub mod qr;
pub mod routes;
pub mod state;
pub mod sync;
pub use routes::routes;
