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
//! ## Usage
//!
//! ```ignore
//! use lair_chat_server::adapters::http::{HttpConfig, HttpServer};
//!
//! let config = HttpConfig { port: 8082 };
//! let server = HttpServer::start(config, engine).await?;
//!
//! // ... run server ...
//!
//! server.shutdown().await;
//! ```

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;

pub use server::HttpServer;

/// HTTP server configuration.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Port to listen on.
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self { port: 8082 }
    }
}
