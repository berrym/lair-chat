//! # TCP Adapter
//!
//! TCP socket protocol for persistent connections.
//! See docs/protocols/TCP.md for the wire protocol specification.
//!
//! ## Components
//!
//! - `server`: TCP listener that accepts connections
//! - `connection`: Individual connection handler
//! - `protocol`: Wire format parsing/serialization
//! - `commands`: Maps protocol messages to core commands
//!
//! ## Usage
//!
//! ```ignore
//! use lair_chat_server::adapters::tcp::{TcpConfig, TcpServer};
//!
//! let config = TcpConfig {
//!     port: 8080,
//!     max_connections: 10000, // 0 = unlimited
//! };
//! let server = TcpServer::start(config, engine).await?;
//!
//! // ... run server ...
//!
//! server.shutdown().await;
//! ```

pub mod commands;
pub mod connection;
pub mod protocol;
pub mod server;

pub use server::TcpServer;

/// TCP server configuration.
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Port to listen on.
    pub port: u16,
    /// Maximum number of concurrent connections.
    /// 0 means unlimited (not recommended for production).
    pub max_connections: u32,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            max_connections: 10000,
        }
    }
}
