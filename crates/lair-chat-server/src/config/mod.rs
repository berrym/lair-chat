//! Configuration management.
//!
//! Handles loading configuration from environment variables and files.

pub mod settings;

use std::env;

use crate::adapters::{http::HttpConfig, tcp::TcpConfig};
use crate::storage::sqlite::SqliteConfig;
use crate::Result;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// TCP server configuration.
    pub tcp: TcpConfig,
    /// HTTP server configuration.
    pub http: HttpConfig,
    /// Database configuration.
    pub database: SqliteConfig,
    /// JWT secret for token signing.
    pub jwt_secret: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tcp: TcpConfig::default(),
            http: HttpConfig::default(),
            database: SqliteConfig::default(),
            jwt_secret: generate_default_jwt_secret(),
        }
    }
}

/// Generate a default JWT secret (for development only).
fn generate_default_jwt_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &secret)
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `LAIR_TCP_PORT`: TCP server port (default: 8080)
    /// - `LAIR_HTTP_PORT`: HTTP server port (default: 8082)
    /// - `LAIR_DATABASE_URL`: SQLite database URL (default: sqlite:lair-chat.db?mode=rwc)
    /// - `LAIR_JWT_SECRET`: JWT signing secret (auto-generated if not set)
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

        let jwt_secret = env::var("LAIR_JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "LAIR_JWT_SECRET not set, generating random secret. \
                 Set LAIR_JWT_SECRET in production for persistent sessions."
            );
            generate_default_jwt_secret()
        });

        Ok(Self {
            tcp: TcpConfig { port: tcp_port },
            http: HttpConfig { port: http_port },
            database: SqliteConfig {
                url: database_url,
                ..Default::default()
            },
            jwt_secret,
        })
    }

    /// Load configuration (currently from environment, could be extended to file).
    pub fn load() -> Result<Self> {
        Self::from_env()
    }
}
