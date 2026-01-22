//! Configuration management.
//!
//! Handles loading configuration from environment variables and files.

pub mod settings;

use std::env;

use crate::adapters::{http::HttpConfig, tcp::TcpConfig};
use crate::storage::sqlite::SqliteConfig;
use crate::Result;

/// Server configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// TCP server configuration.
    pub tcp: TcpConfig,
    /// HTTP server configuration.
    pub http: HttpConfig,
    /// Database configuration.
    pub database: SqliteConfig,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `LAIR_TCP_PORT`: TCP server port (default: 8080)
    /// - `LAIR_HTTP_PORT`: HTTP server port (default: 8082)
    /// - `LAIR_DATABASE_URL`: SQLite database URL (default: sqlite:lair-chat.db?mode=rwc)
    pub fn from_env() -> Result<Self> {
        let tcp_port = env::var("LAIR_TCP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);

        let http_port = env::var("LAIR_HTTP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8082);

        let database_url = env::var("LAIR_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:lair-chat.db?mode=rwc".to_string());

        Ok(Self {
            tcp: TcpConfig { port: tcp_port },
            http: HttpConfig { port: http_port },
            database: SqliteConfig {
                url: database_url,
                ..Default::default()
            },
        })
    }

    /// Load configuration (currently from environment, could be extended to file).
    pub fn load() -> Result<Self> {
        Self::from_env()
    }
}
