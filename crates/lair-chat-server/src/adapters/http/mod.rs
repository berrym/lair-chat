//! # HTTP Adapter
//!
//! REST API for stateless requests.
//! See docs/protocols/HTTP.md for the API specification.
//!
//! ## Components
//!
//! - `server`: Axum server setup
//! - `routes`: Route definitions
//! - `handlers`: Request handlers
//! - `middleware`: Auth, rate limiting, etc.
//!
//! ## Current Status
//!
//! Placeholder for Phase 3 implementation.

pub mod server;
pub mod routes;
pub mod handlers;
pub mod middleware;

/// HTTP server configuration
pub struct HttpConfig {
    /// Port to listen on
    pub port: u16,
}

// HTTP server implementation will be added in Phase 3
