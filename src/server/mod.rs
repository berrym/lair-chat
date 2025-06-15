//! Server module for lair-chat
//!
//! This module contains all server-side functionality including:
//! - Application logic and server management
//! - Authentication and user management
//! - Chat message handling and room management
//! - Network connection management
//! - Configuration management
//! - Storage and persistence layer

pub mod api;
pub mod app;
pub mod auth;
pub mod chat;
pub mod config;
pub mod network;
pub mod storage;

// Re-export commonly used types and functions
pub use api::{create_api_router, start_api_server, ApiState};
pub use app::{ChatServer, ServerStats};
pub use config::{load_config, load_config_from_file, ConfigBuilder, ConfigError, ServerConfig};
pub use storage::{StorageError, StorageManager, StorageResult};

/// Server module result type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Server version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Default server configuration file name
pub const DEFAULT_CONFIG_FILE: &str = "server.toml";

/// Default data directory for server files
pub const DEFAULT_DATA_DIR: &str = "data";

/// Default log directory
pub const DEFAULT_LOG_DIR: &str = "logs";
