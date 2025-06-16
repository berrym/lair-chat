//! Server configuration management for lair-chat
//!
//! This module provides comprehensive configuration management for the Lair Chat server,
//! supporting multiple configuration sources (files, environment variables, CLI args)
//! with validation, hot-reload capabilities, and structured error handling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::{debug, info, warn};

pub mod defaults;
pub mod loader;
pub mod validation;

pub use defaults::*;
pub use loader::*;
pub use validation::*;

/// Main server configuration structure containing all subsystem configurations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    /// Network and connection settings
    pub server: NetworkConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Security and authentication settings
    pub security: SecurityConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Feature flags and toggles
    pub features: FeatureConfig,

    /// Rate limiting and resource constraints
    pub limits: LimitsConfig,

    /// Administrative settings
    pub admin: AdminConfig,
}

/// Network and connection configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    /// Server bind address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Keep-alive timeout in seconds
    pub keep_alive_timeout: u64,

    /// Enable TLS encryption
    pub enable_tls: bool,

    /// Path to TLS certificate file
    pub tls_cert_path: Option<PathBuf>,

    /// Path to TLS private key file
    pub tls_key_path: Option<PathBuf>,

    /// TLS protocol versions to support
    pub tls_versions: Vec<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    /// Database type (sqlite, postgresql, mysql)
    pub database_type: String,

    /// Database connection URL or path
    pub url: String,

    /// Maximum number of database connections in pool
    pub max_connections: u32,

    /// Minimum number of database connections in pool
    pub min_connections: u32,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Idle connection timeout in seconds
    pub idle_timeout: u64,

    /// Enable automatic migrations
    pub auto_migrate: bool,

    /// Database-specific settings
    pub settings: HashMap<String, String>,
}

/// Security and authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConfig {
    /// Enable end-to-end encryption
    pub enable_encryption: bool,

    /// Session timeout in seconds
    pub session_timeout: u64,

    /// Maximum login attempts before lockout
    pub max_login_attempts: u32,

    /// Account lockout duration in seconds
    pub lockout_duration: u64,

    /// Password minimum length
    pub password_min_length: usize,

    /// Require special characters in passwords
    pub password_require_special: bool,

    /// Require numbers in passwords
    pub password_require_numbers: bool,

    /// Require uppercase letters in passwords
    pub password_require_uppercase: bool,

    /// Enable two-factor authentication
    pub enable_2fa: bool,

    /// JWT secret for session tokens
    pub jwt_secret: Option<String>,

    /// Argon2 configuration
    pub argon2: Argon2Config,

    /// Whether to rotate refresh tokens on each use
    pub rotate_refresh_tokens: bool,
}

/// Argon2 password hashing configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Argon2Config {
    /// Memory cost in KiB
    pub memory_cost: u32,

    /// Time cost (iterations)
    pub time_cost: u32,

    /// Parallelism factor
    pub parallelism: u32,

    /// Hash output length
    pub hash_length: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log format (json, pretty, compact)
    pub format: String,

    /// Log to file
    pub enable_file_logging: bool,

    /// Log file path
    pub file_path: Option<PathBuf>,

    /// Maximum log file size in MB
    pub max_file_size: u64,

    /// Number of log files to retain
    pub max_files: u32,

    /// Enable log compression
    pub compress_logs: bool,

    /// Log to stdout
    pub enable_stdout: bool,

    /// Enable structured logging
    pub enable_json: bool,

    /// Additional log targets
    pub targets: HashMap<String, String>,
}

/// Feature flags and toggles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureConfig {
    /// Enable direct messaging
    pub enable_direct_messages: bool,

    /// Enable room creation by users
    pub enable_user_room_creation: bool,

    /// Enable file uploads
    pub enable_file_uploads: bool,

    /// Maximum file upload size in MB
    pub max_file_size: u64,

    /// Allowed file types
    pub allowed_file_types: Vec<String>,

    /// Enable message history
    pub enable_message_history: bool,

    /// Message history retention in days (0 = unlimited)
    pub message_history_retention: u32,

    /// Enable message search
    pub enable_message_search: bool,

    /// Enable user profiles
    pub enable_user_profiles: bool,

    /// Enable presence indicators
    pub enable_presence: bool,

    /// Enable typing indicators
    pub enable_typing_indicators: bool,

    /// Enable message reactions
    pub enable_reactions: bool,

    /// Enable message threading
    pub enable_threading: bool,
}

/// Rate limiting and resource constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LimitsConfig {
    /// Messages per minute per user
    pub messages_per_minute: u32,

    /// Maximum message length in characters
    pub max_message_length: usize,

    /// Maximum username length
    pub max_username_length: usize,

    /// Maximum room name length
    pub max_room_name_length: usize,

    /// Maximum number of rooms per user
    pub max_rooms_per_user: u32,

    /// Maximum users per room
    pub max_users_per_room: u32,

    /// Maximum concurrent connections per IP
    pub max_connections_per_ip: u32,

    /// Rate limit window in seconds
    pub rate_limit_window: u64,

    /// Memory limit in MB
    pub memory_limit: u64,

    /// CPU usage limit percentage
    pub cpu_limit: f32,
}

/// Administrative configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminConfig {
    /// Enable admin API
    pub enable_admin_api: bool,

    /// Admin API bind address
    pub admin_host: String,

    /// Admin API port
    pub admin_port: u16,

    /// Admin API authentication token
    pub admin_token: Option<String>,

    /// Enable metrics endpoint
    pub enable_metrics: bool,

    /// Metrics endpoint path
    pub metrics_path: String,

    /// Enable health check endpoint
    pub enable_health_check: bool,

    /// Health check endpoint path
    pub health_check_path: String,

    /// Enable debug endpoints
    pub enable_debug_endpoints: bool,

    /// Audit log configuration
    pub audit_log: AuditLogConfig,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditLogConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Audit log file path
    pub file_path: Option<PathBuf>,

    /// Audit log format (json, csv)
    pub format: String,

    /// Log admin actions
    pub log_admin_actions: bool,

    /// Log user authentication
    pub log_auth_events: bool,

    /// Log security events
    pub log_security_events: bool,

    /// Log data access
    pub log_data_access: bool,
}

/// Configuration loading and management errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to read configuration file: {source}")]
    ReadError { source: std::io::Error },

    #[error("Failed to parse configuration: {source}")]
    ParseError { source: toml::de::Error },

    #[error("Configuration validation failed: {message}")]
    ValidationError { message: String },

    #[error("Environment variable error: {name} - {message}")]
    EnvironmentError { name: String, message: String },

    #[error("Invalid configuration value: {field} - {message}")]
    InvalidValue { field: String, message: String },

    #[error("Missing required configuration: {field}")]
    MissingRequired { field: String },

    #[error("Configuration conflict: {message}")]
    Conflict { message: String },
}

/// Configuration source priority
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    /// Default values
    Default,
    /// Configuration file
    File(PathBuf),
    /// Environment variables
    Environment,
    /// Command line arguments
    CommandLine,
}

/// Configuration builder for constructing configurations from multiple sources
pub struct ConfigBuilder {
    sources: Vec<(ConfigSource, ServerConfig)>,
    validation_enabled: bool,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            validation_enabled: true,
        }
    }

    /// Add default configuration
    pub fn with_defaults(mut self) -> Self {
        self.sources
            .push((ConfigSource::Default, ServerConfig::default()));
        self
    }

    /// Add configuration from file
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ConfigError> {
        let config = load_from_file(path.as_ref())?;
        self.sources
            .push((ConfigSource::File(path.as_ref().to_path_buf()), config));
        Ok(self)
    }

    /// Add configuration from environment variables
    pub fn with_environment(mut self) -> Result<Self, ConfigError> {
        let config = load_from_environment()?;
        self.sources.push((ConfigSource::Environment, config));
        Ok(self)
    }

    /// Disable validation (for testing)
    pub fn without_validation(mut self) -> Self {
        self.validation_enabled = false;
        self
    }

    /// Build the final configuration by merging all sources
    pub fn build(self) -> Result<ServerConfig, ConfigError> {
        if self.sources.is_empty() {
            return Ok(ServerConfig::default());
        }

        // Start with the first configuration
        let mut final_config = self.sources[0].1.clone();

        // Merge subsequent configurations
        for (source, config) in self.sources.iter().skip(1) {
            debug!("Merging configuration from source: {:?}", source);
            final_config = merge_configs(final_config, config.clone())?;
        }

        // Validate the final configuration
        if self.validation_enabled {
            validate_config(&final_config)?;
        }

        info!(
            "Configuration successfully built from {} sources",
            self.sources.len()
        );
        Ok(final_config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Merge two configurations, with the second taking precedence
fn merge_configs(_base: ServerConfig, overlay: ServerConfig) -> Result<ServerConfig, ConfigError> {
    // For now, we'll do a simple overlay merge
    // In a production implementation, you'd want more sophisticated merging logic
    Ok(overlay)
}

/// Load configuration from a TOML file
fn load_from_file(path: &Path) -> Result<ServerConfig, ConfigError> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError { source: e })?;

    let config: ServerConfig =
        toml::from_str(&contents).map_err(|e| ConfigError::ParseError { source: e })?;

    debug!("Loaded configuration from file: {}", path.display());
    Ok(config)
}

/// Load configuration from environment variables
fn load_from_environment() -> Result<ServerConfig, ConfigError> {
    // Create a partial configuration from environment variables
    // Environment variables use the pattern: LAIR_CHAT_SECTION_FIELD
    let mut config = ServerConfig::default();

    // Server configuration
    if let Ok(host) = std::env::var("LAIR_CHAT_SERVER_HOST") {
        config.server.host = host;
    }
    if let Ok(port) = std::env::var("LAIR_CHAT_SERVER_PORT") {
        config.server.port = port.parse().map_err(|_| ConfigError::EnvironmentError {
            name: "LAIR_CHAT_SERVER_PORT".to_string(),
            message: "Invalid port number".to_string(),
        })?;
    }

    // Database configuration
    if let Ok(url) = std::env::var("LAIR_CHAT_DATABASE_URL") {
        config.database.url = url;
    }
    if let Ok(db_type) = std::env::var("LAIR_CHAT_DATABASE_TYPE") {
        config.database.database_type = db_type;
    }

    // Security configuration
    if let Ok(session_timeout) = std::env::var("LAIR_CHAT_SECURITY_SESSION_TIMEOUT") {
        config.security.session_timeout =
            session_timeout
                .parse()
                .map_err(|_| ConfigError::EnvironmentError {
                    name: "LAIR_CHAT_SECURITY_SESSION_TIMEOUT".to_string(),
                    message: "Invalid timeout value".to_string(),
                })?;
    }

    // Add more environment variable mappings as needed

    debug!("Loaded configuration from environment variables");
    Ok(config)
}

/// Save configuration to a TOML file
pub fn save_to_file(config: &ServerConfig, path: &Path) -> Result<(), ConfigError> {
    let contents = toml::to_string_pretty(config).map_err(|e| ConfigError::ValidationError {
        message: format!("TOML serialization error: {}", e),
    })?;

    std::fs::write(path, contents).map_err(|e| ConfigError::ReadError { source: e })?;

    info!("Configuration saved to file: {}", path.display());
    Ok(())
}

/// Watch configuration file for changes and reload
pub async fn watch_config_file<P: AsRef<Path>>(
    _path: P,
    _callback: impl Fn(ServerConfig) + Send + Sync,
) -> Result<(), ConfigError> {
    // Implementation would use a file watcher like `notify`
    // For now, we'll just log that watching is not implemented
    warn!("Configuration file watching not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(config.security.enable_encryption);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new().with_defaults().build().unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_config_file_loading() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.toml");

        let test_config = r#"
[server]
host = "0.0.0.0"
port = 9000
max_connections = 2000

[database]
database_type = "sqlite"
url = "test.db"
max_connections = 20
"#;

        fs::write(&file_path, test_config).unwrap();

        let config = ConfigBuilder::new()
            .with_file(&file_path)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.url, "test.db");
    }

    #[test]
    fn test_environment_override() {
        std::env::set_var("LAIR_CHAT_SERVER_PORT", "9999");

        let config = ConfigBuilder::new()
            .with_defaults()
            .with_environment()
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.server.port, 9999);

        std::env::remove_var("LAIR_CHAT_SERVER_PORT");
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        config.server.port = 0; // Invalid port

        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = ServerConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed_config: ServerConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, parsed_config);
    }
}
