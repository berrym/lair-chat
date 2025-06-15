//! Configuration loading utilities for lair-chat server
//!
//! This module provides utilities for loading configuration from various sources
//! including files, environment variables, and command line arguments, with
//! support for different formats and validation.

use super::*;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

/// Configuration file formats supported by the loader
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    /// TOML format (.toml)
    Toml,
    /// JSON format (.json)
    Json,
    /// YAML format (.yaml, .yml)
    Yaml,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "toml" => ConfigFormat::Toml,
                "json" => ConfigFormat::Json,
                "yaml" | "yml" => ConfigFormat::Yaml,
                _ => ConfigFormat::Toml, // Default to TOML
            })
    }
}

/// Configuration loader with support for multiple sources and formats
pub struct ConfigLoader {
    search_paths: Vec<PathBuf>,
    environment_prefix: String,
    format_preference: Vec<ConfigFormat>,
}

impl ConfigLoader {
    /// Create a new configuration loader with default settings
    pub fn new() -> Self {
        Self {
            search_paths: Self::default_search_paths(),
            environment_prefix: "LAIR_CHAT".to_string(),
            format_preference: vec![ConfigFormat::Toml, ConfigFormat::Json, ConfigFormat::Yaml],
        }
    }

    /// Set custom search paths for configuration files
    pub fn with_search_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.search_paths = paths;
        self
    }

    /// Add a search path to the existing list
    pub fn add_search_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Set the environment variable prefix (default: LAIR_CHAT)
    pub fn with_environment_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.environment_prefix = prefix.into();
        self
    }

    /// Set format preference order
    pub fn with_format_preference(mut self, formats: Vec<ConfigFormat>) -> Self {
        self.format_preference = formats;
        self
    }

    /// Load configuration from all available sources
    pub fn load(&self) -> Result<ServerConfig, ConfigError> {
        let mut builder = ConfigBuilder::new().with_defaults();

        // Try to load from configuration files
        if let Ok(config) = self.load_from_files() {
            builder = builder.with_file(&config)?;
        }

        // Load from environment variables
        builder = builder.with_environment()?;

        // Build final configuration
        builder.build()
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<ServerConfig, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.display().to_string(),
            });
        }

        let format = ConfigFormat::from_path(path).unwrap_or(ConfigFormat::Toml);
        let contents =
            fs::read_to_string(path).map_err(|e| ConfigError::ReadError { source: e })?;

        self.parse_config(&contents, format)
    }

    /// Load configuration from the first available file in search paths
    pub fn load_from_files(&self) -> Result<PathBuf, ConfigError> {
        for search_path in &self.search_paths {
            for format in &self.format_preference {
                let filename = match format {
                    ConfigFormat::Toml => "server.toml",
                    ConfigFormat::Json => "server.json",
                    ConfigFormat::Yaml => "server.yaml",
                };

                let config_path = search_path.join(filename);
                if config_path.exists() {
                    info!("Found configuration file: {}", config_path.display());
                    return Ok(config_path);
                }
            }
        }

        Err(ConfigError::FileNotFound {
            path: "No configuration file found in search paths".to_string(),
        })
    }

    /// Load configuration from environment variables
    pub fn load_from_environment(&self) -> Result<ServerConfig, ConfigError> {
        let mut config = ServerConfig::default();
        let prefix = format!("{}_", self.environment_prefix);

        // Collect all environment variables with our prefix
        let env_vars: HashMap<String, String> = env::vars()
            .filter(|(key, _)| key.starts_with(&prefix))
            .collect();

        if env_vars.is_empty() {
            debug!("No environment variables found with prefix {}", prefix);
            return Ok(config);
        }

        info!(
            "Loading configuration from {} environment variables",
            env_vars.len()
        );

        // Parse environment variables into configuration
        self.apply_environment_variables(&mut config, &env_vars)?;

        Ok(config)
    }

    /// Parse configuration string with the specified format
    fn parse_config(
        &self,
        contents: &str,
        format: ConfigFormat,
    ) -> Result<ServerConfig, ConfigError> {
        match format {
            ConfigFormat::Toml => {
                toml::from_str(contents).map_err(|e| ConfigError::ParseError { source: e })
            }
            ConfigFormat::Json => {
                serde_json::from_str(contents).map_err(|e| ConfigError::ValidationError {
                    message: format!("JSON parse error: {}", e),
                })
            }
            ConfigFormat::Yaml => {
                // Note: In a real implementation, you'd use serde_yaml
                // For now, we'll return an error as YAML support isn't implemented
                Err(ConfigError::ValidationError {
                    message: "YAML format not yet implemented".to_string(),
                })
            }
        }
    }

    /// Apply environment variables to configuration
    fn apply_environment_variables(
        &self,
        config: &mut ServerConfig,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), ConfigError> {
        let prefix = format!("{}_", self.environment_prefix);

        for (key, value) in env_vars {
            let config_key = key.strip_prefix(&prefix).unwrap().to_lowercase();
            self.apply_environment_variable(config, &config_key, value)?;
        }

        Ok(())
    }

    /// Apply a single environment variable to configuration
    fn apply_environment_variable(
        &self,
        config: &mut ServerConfig,
        key: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        let parts: Vec<&str> = key.split('_').collect();

        if parts.len() < 2 {
            warn!("Invalid environment variable format: {}", key);
            return Ok(());
        }

        let section = parts[0];
        let field = parts[1..].join("_");

        match section {
            "server" => self.apply_server_env(config, &field, value)?,
            "database" => self.apply_database_env(config, &field, value)?,
            "security" => self.apply_security_env(config, &field, value)?,
            "logging" => self.apply_logging_env(config, &field, value)?,
            "features" => self.apply_features_env(config, &field, value)?,
            "limits" => self.apply_limits_env(config, &field, value)?,
            "admin" => self.apply_admin_env(config, &field, value)?,
            _ => {
                warn!("Unknown configuration section: {}", section);
            }
        }

        Ok(())
    }

    /// Apply server environment variables
    fn apply_server_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "host" => config.server.host = value.to_string(),
            "port" => {
                config.server.port = value.parse().map_err(|_| ConfigError::EnvironmentError {
                    name: "SERVER_PORT".to_string(),
                    message: "Invalid port number".to_string(),
                })?;
            }
            "max_connections" => {
                config.server.max_connections =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "SERVER_MAX_CONNECTIONS".to_string(),
                        message: "Invalid connection count".to_string(),
                    })?;
            }
            "connection_timeout" => {
                config.server.connection_timeout =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "SERVER_CONNECTION_TIMEOUT".to_string(),
                        message: "Invalid timeout value".to_string(),
                    })?;
            }
            "enable_tls" => {
                config.server.enable_tls = self.parse_bool(value)?;
            }
            "tls_cert_path" => {
                config.server.tls_cert_path = Some(PathBuf::from(value));
            }
            "tls_key_path" => {
                config.server.tls_key_path = Some(PathBuf::from(value));
            }
            _ => warn!("Unknown server configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply database environment variables
    fn apply_database_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "type" => config.database.database_type = value.to_string(),
            "url" => config.database.url = value.to_string(),
            "max_connections" => {
                config.database.max_connections =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "DATABASE_MAX_CONNECTIONS".to_string(),
                        message: "Invalid connection count".to_string(),
                    })?;
            }
            "connection_timeout" => {
                config.database.connection_timeout =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "DATABASE_CONNECTION_TIMEOUT".to_string(),
                        message: "Invalid timeout value".to_string(),
                    })?;
            }
            "auto_migrate" => {
                config.database.auto_migrate = self.parse_bool(value)?;
            }
            _ => warn!("Unknown database configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply security environment variables
    fn apply_security_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "enable_encryption" => {
                config.security.enable_encryption = self.parse_bool(value)?;
            }
            "session_timeout" => {
                config.security.session_timeout =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "SECURITY_SESSION_TIMEOUT".to_string(),
                        message: "Invalid timeout value".to_string(),
                    })?;
            }
            "max_login_attempts" => {
                config.security.max_login_attempts =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "SECURITY_MAX_LOGIN_ATTEMPTS".to_string(),
                        message: "Invalid attempt count".to_string(),
                    })?;
            }
            "password_min_length" => {
                config.security.password_min_length =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "SECURITY_PASSWORD_MIN_LENGTH".to_string(),
                        message: "Invalid length value".to_string(),
                    })?;
            }
            "jwt_secret" => {
                config.security.jwt_secret = Some(value.to_string());
            }
            _ => warn!("Unknown security configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply logging environment variables
    fn apply_logging_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "level" => config.logging.level = value.to_string(),
            "format" => config.logging.format = value.to_string(),
            "enable_file_logging" => {
                config.logging.enable_file_logging = self.parse_bool(value)?;
            }
            "file_path" => {
                config.logging.file_path = Some(PathBuf::from(value));
            }
            "enable_stdout" => {
                config.logging.enable_stdout = self.parse_bool(value)?;
            }
            _ => warn!("Unknown logging configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply features environment variables
    fn apply_features_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "enable_direct_messages" => {
                config.features.enable_direct_messages = self.parse_bool(value)?;
            }
            "enable_file_uploads" => {
                config.features.enable_file_uploads = self.parse_bool(value)?;
            }
            "enable_message_history" => {
                config.features.enable_message_history = self.parse_bool(value)?;
            }
            "max_file_size" => {
                config.features.max_file_size =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "FEATURES_MAX_FILE_SIZE".to_string(),
                        message: "Invalid file size".to_string(),
                    })?;
            }
            _ => warn!("Unknown features configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply limits environment variables
    fn apply_limits_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "messages_per_minute" => {
                config.limits.messages_per_minute =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "LIMITS_MESSAGES_PER_MINUTE".to_string(),
                        message: "Invalid rate limit".to_string(),
                    })?;
            }
            "max_message_length" => {
                config.limits.max_message_length =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "LIMITS_MAX_MESSAGE_LENGTH".to_string(),
                        message: "Invalid message length".to_string(),
                    })?;
            }
            "max_connections_per_ip" => {
                config.limits.max_connections_per_ip =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "LIMITS_MAX_CONNECTIONS_PER_IP".to_string(),
                        message: "Invalid connection limit".to_string(),
                    })?;
            }
            _ => warn!("Unknown limits configuration field: {}", field),
        }
        Ok(())
    }

    /// Apply admin environment variables
    fn apply_admin_env(
        &self,
        config: &mut ServerConfig,
        field: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match field {
            "enable_admin_api" => {
                config.admin.enable_admin_api = self.parse_bool(value)?;
            }
            "admin_host" => config.admin.admin_host = value.to_string(),
            "admin_port" => {
                config.admin.admin_port =
                    value.parse().map_err(|_| ConfigError::EnvironmentError {
                        name: "ADMIN_ADMIN_PORT".to_string(),
                        message: "Invalid port number".to_string(),
                    })?;
            }
            "admin_token" => {
                config.admin.admin_token = Some(value.to_string());
            }
            "enable_metrics" => {
                config.admin.enable_metrics = self.parse_bool(value)?;
            }
            _ => warn!("Unknown admin configuration field: {}", field),
        }
        Ok(())
    }

    /// Parse boolean value from string
    fn parse_bool(&self, value: &str) -> Result<bool, ConfigError> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
            "false" | "0" | "no" | "off" | "disabled" => Ok(false),
            _ => Err(ConfigError::EnvironmentError {
                name: "BOOLEAN_VALUE".to_string(),
                message: format!("Invalid boolean value: {}", value),
            }),
        }
    }

    /// Get default search paths for configuration files
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Current directory
        paths.push(PathBuf::from("."));

        // Config subdirectory
        paths.push(PathBuf::from("config"));

        // User config directory
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "lair-chat") {
            paths.push(config_dir.config_dir().to_path_buf());
        }

        // System config directories
        paths.push(PathBuf::from("/etc/lair-chat"));
        paths.push(PathBuf::from("/usr/local/etc/lair-chat"));

        paths
    }

    /// Create a sample configuration file
    pub fn create_sample_config<P: AsRef<Path>>(
        &self,
        path: P,
        format: ConfigFormat,
    ) -> Result<(), ConfigError> {
        let config = ServerConfig::default();
        let contents = match format {
            ConfigFormat::Toml => {
                toml::to_string_pretty(&config).map_err(|e| ConfigError::ValidationError {
                    message: format!("TOML serialization error: {}", e),
                })?
            }
            ConfigFormat::Json => {
                serde_json::to_string_pretty(&config).map_err(|e| ConfigError::ValidationError {
                    message: format!("JSON serialization error: {}", e),
                })?
            }
            ConfigFormat::Yaml => {
                return Err(ConfigError::ValidationError {
                    message: "YAML format not yet implemented".to_string(),
                });
            }
        };

        fs::write(path.as_ref(), contents).map_err(|e| ConfigError::ReadError { source: e })?;

        info!("Sample configuration created: {}", path.as_ref().display());
        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility function to load configuration with default settings
pub fn load_config() -> Result<ServerConfig, ConfigError> {
    ConfigLoader::new().load()
}

/// Utility function to load configuration from a specific file
pub fn load_config_from_file<P: AsRef<Path>>(path: P) -> Result<ServerConfig, ConfigError> {
    ConfigLoader::new().load_from_file(path)
}

/// Utility function to create a sample configuration file
pub fn create_sample_config<P: AsRef<Path>>(path: P) -> Result<(), ConfigError> {
    let format = ConfigFormat::from_path(&path).unwrap_or(ConfigFormat::Toml);
    ConfigLoader::new().create_sample_config(path, format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_format_detection() {
        assert_eq!(
            ConfigFormat::from_path("config.toml"),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_path("config.json"),
            Some(ConfigFormat::Json)
        );
        assert_eq!(
            ConfigFormat::from_path("config.yaml"),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path("config.yml"),
            Some(ConfigFormat::Yaml)
        );
    }

    #[test]
    fn test_config_loader_creation() {
        let loader = ConfigLoader::new();
        assert!(!loader.search_paths.is_empty());
        assert_eq!(loader.environment_prefix, "LAIR_CHAT");
    }

    #[test]
    fn test_boolean_parsing() {
        let loader = ConfigLoader::new();

        assert_eq!(loader.parse_bool("true").unwrap(), true);
        assert_eq!(loader.parse_bool("1").unwrap(), true);
        assert_eq!(loader.parse_bool("yes").unwrap(), true);
        assert_eq!(loader.parse_bool("false").unwrap(), false);
        assert_eq!(loader.parse_bool("0").unwrap(), false);
        assert_eq!(loader.parse_bool("no").unwrap(), false);

        assert!(loader.parse_bool("invalid").is_err());
    }

    #[test]
    fn test_sample_config_creation() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.toml");

        let loader = ConfigLoader::new();
        loader
            .create_sample_config(&config_path, ConfigFormat::Toml)
            .unwrap();

        assert!(config_path.exists());

        // Verify we can load the created config
        let loaded_config = loader.load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_environment_variable_loading() {
        // Set test environment variables
        env::set_var("LAIR_CHAT_SERVER_PORT", "9999");
        env::set_var("LAIR_CHAT_DATABASE_URL", "test.db");
        env::set_var("LAIR_CHAT_SECURITY_ENABLE_ENCRYPTION", "false");

        let loader = ConfigLoader::new();
        let config = loader.load_from_environment().unwrap();

        assert_eq!(config.server.port, 9999);
        assert_eq!(config.database.url, "test.db");
        assert_eq!(config.security.enable_encryption, false);

        // Clean up
        env::remove_var("LAIR_CHAT_SERVER_PORT");
        env::remove_var("LAIR_CHAT_DATABASE_URL");
        env::remove_var("LAIR_CHAT_SECURITY_ENABLE_ENCRYPTION");
    }

    #[test]
    fn test_file_loading() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.toml");

        let test_config = r#"
[server]
host = "0.0.0.0"
port = 9000

[database]
database_type = "postgresql"
url = "postgresql://localhost/test"
"#;

        fs::write(&config_path, test_config).unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load_from_file(&config_path).unwrap();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.database_type, "postgresql");
    }

    #[test]
    fn test_nonexistent_file() {
        let loader = ConfigLoader::new();
        let result = loader.load_from_file("/nonexistent/config.toml");
        assert!(result.is_err());

        if let Err(ConfigError::FileNotFound { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected FileNotFound error");
        }
    }
}
