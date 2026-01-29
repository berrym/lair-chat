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
//! use lair_chat_server::adapters::http::{HttpConfig, HttpServer, TlsConfig};
//!
//! // HTTP (no TLS)
//! let config = HttpConfig { port: 8082, tls: None };
//! let server = HttpServer::start(config, engine).await?;
//!
//! // HTTPS (with TLS)
//! let tls_config = TlsConfig {
//!     cert_path: PathBuf::from("cert.pem"),
//!     key_path: PathBuf::from("key.pem"),
//! };
//! let config = HttpConfig { port: 8082, tls: Some(tls_config) };
//! let server = HttpServer::start(config, engine).await?;
//!
//! // ... run server ...
//!
//! server.shutdown().await;
//! ```

use std::path::PathBuf;

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;

pub use server::HttpServer;

/// TLS configuration for HTTPS.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to the certificate PEM file.
    pub cert_path: PathBuf,
    /// Path to the private key PEM file.
    pub key_path: PathBuf,
}

/// HTTP server configuration.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Port to listen on.
    pub port: u16,
    /// TLS configuration (None for plain HTTP).
    pub tls: Option<TlsConfig>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 8082,
            tls: None,
        }
    }
}
