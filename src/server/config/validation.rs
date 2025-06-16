//! Configuration validation for lair-chat server
//!
//! This module provides comprehensive validation for all configuration options,
//! ensuring that the server starts with valid, secure, and consistent settings.
//! Validation includes range checks, format validation, dependency validation,
//! and security best practices enforcement.

use super::*;
use std::net::IpAddr;
use tracing::{debug, warn};

/// Validate a complete server configuration
pub fn validate_config(config: &ServerConfig) -> Result<(), ConfigError> {
    debug!("Starting configuration validation");

    // Validate each configuration section
    validate_network_config(&config.server)?;
    validate_database_config(&config.database)?;
    validate_security_config(&config.security)?;
    validate_logging_config(&config.logging)?;
    validate_feature_config(&config.features)?;
    validate_limits_config(&config.limits)?;
    validate_admin_config(&config.admin)?;

    // Cross-section validation
    validate_cross_dependencies(config)?;

    debug!("Configuration validation completed successfully");
    Ok(())
}

/// Validate network configuration
fn validate_network_config(config: &NetworkConfig) -> Result<(), ConfigError> {
    // Validate host address
    if config.host.is_empty() {
        return Err(ConfigError::InvalidValue {
            field: "server.host".to_string(),
            message: "Host cannot be empty".to_string(),
        });
    }

    // Try to parse as IP address or check if it's a valid hostname
    if config.host != "localhost" && config.host.parse::<IpAddr>().is_err() {
        // Basic hostname validation
        if !is_valid_hostname(&config.host) {
            return Err(ConfigError::InvalidValue {
                field: "server.host".to_string(),
                message: "Invalid hostname or IP address".to_string(),
            });
        }
    }

    // Validate port range
    if config.port == 0 {
        return Err(ConfigError::InvalidValue {
            field: "server.port".to_string(),
            message: "Port cannot be 0 (use random port assignment carefully)".to_string(),
        });
    }

    if config.port < 1024 && config.port != 0 {
        warn!(
            "Using privileged port {} - ensure proper permissions",
            config.port
        );
    }

    // Validate connection limits
    if config.max_connections == 0 {
        return Err(ConfigError::InvalidValue {
            field: "server.max_connections".to_string(),
            message: "Maximum connections must be greater than 0".to_string(),
        });
    }

    if config.max_connections > 100000 {
        warn!(
            "Very high connection limit ({}), ensure system resources are adequate",
            config.max_connections
        );
    }

    // Validate timeout values
    if config.connection_timeout == 0 {
        return Err(ConfigError::InvalidValue {
            field: "server.connection_timeout".to_string(),
            message: "Connection timeout must be greater than 0".to_string(),
        });
    }

    if config.keep_alive_timeout == 0 {
        return Err(ConfigError::InvalidValue {
            field: "server.keep_alive_timeout".to_string(),
            message: "Keep-alive timeout must be greater than 0".to_string(),
        });
    }

    // Validate TLS configuration
    if config.enable_tls {
        if config.tls_cert_path.is_none() {
            return Err(ConfigError::MissingRequired {
                field: "server.tls_cert_path".to_string(),
            });
        }

        if config.tls_key_path.is_none() {
            return Err(ConfigError::MissingRequired {
                field: "server.tls_key_path".to_string(),
            });
        }

        // Check if certificate files exist
        if let Some(cert_path) = &config.tls_cert_path {
            if !cert_path.exists() {
                return Err(ConfigError::InvalidValue {
                    field: "server.tls_cert_path".to_string(),
                    message: format!("Certificate file not found: {}", cert_path.display()),
                });
            }
        }

        if let Some(key_path) = &config.tls_key_path {
            if !key_path.exists() {
                return Err(ConfigError::InvalidValue {
                    field: "server.tls_key_path".to_string(),
                    message: format!("Private key file not found: {}", key_path.display()),
                });
            }
        }

        // Validate TLS versions
        for version in &config.tls_versions {
            if !matches!(version.as_str(), "1.0" | "1.1" | "1.2" | "1.3") {
                return Err(ConfigError::InvalidValue {
                    field: "server.tls_versions".to_string(),
                    message: format!("Invalid TLS version: {}", version),
                });
            }
        }

        // Warn about insecure TLS versions
        if config.tls_versions.contains(&"1.0".to_string())
            || config.tls_versions.contains(&"1.1".to_string())
        {
            warn!("Using deprecated TLS versions 1.0/1.1 - consider using only 1.2 and 1.3");
        }
    }

    Ok(())
}

/// Validate database configuration
fn validate_database_config(config: &DatabaseConfig) -> Result<(), ConfigError> {
    // Validate database type
    let valid_types = ["sqlite", "postgresql", "mysql"];
    if !valid_types.contains(&config.database_type.as_str()) {
        return Err(ConfigError::InvalidValue {
            field: "database.database_type".to_string(),
            message: format!(
                "Unsupported database type: {}. Supported types: {:?}",
                config.database_type, valid_types
            ),
        });
    }

    // Validate URL
    if config.url.is_empty() {
        return Err(ConfigError::InvalidValue {
            field: "database.url".to_string(),
            message: "Database URL cannot be empty".to_string(),
        });
    }

    // Basic URL validation for different database types
    match config.database_type.as_str() {
        "sqlite" => {
            if config.url != ":memory:" && !config.url.ends_with(".db") && !config.url.contains("/")
            {
                warn!(
                    "SQLite URL '{}' may not be a valid database path",
                    config.url
                );
            }
        }
        "postgresql" => {
            if !config.url.starts_with("postgresql://") && !config.url.starts_with("postgres://") {
                return Err(ConfigError::InvalidValue {
                    field: "database.url".to_string(),
                    message: "PostgreSQL URL must start with 'postgresql://' or 'postgres://'"
                        .to_string(),
                });
            }
        }
        "mysql" => {
            if !config.url.starts_with("mysql://") {
                return Err(ConfigError::InvalidValue {
                    field: "database.url".to_string(),
                    message: "MySQL URL must start with 'mysql://'".to_string(),
                });
            }
        }
        _ => {}
    }

    // Validate connection pool settings
    if config.max_connections == 0 {
        return Err(ConfigError::InvalidValue {
            field: "database.max_connections".to_string(),
            message: "Maximum connections must be greater than 0".to_string(),
        });
    }

    if config.min_connections > config.max_connections {
        return Err(ConfigError::InvalidValue {
            field: "database.min_connections".to_string(),
            message: "Minimum connections cannot be greater than maximum connections".to_string(),
        });
    }

    if config.connection_timeout == 0 {
        return Err(ConfigError::InvalidValue {
            field: "database.connection_timeout".to_string(),
            message: "Connection timeout must be greater than 0".to_string(),
        });
    }

    if config.idle_timeout == 0 {
        return Err(ConfigError::InvalidValue {
            field: "database.idle_timeout".to_string(),
            message: "Idle timeout must be greater than 0".to_string(),
        });
    }

    // Warn about potential performance issues
    if config.max_connections > 100 {
        warn!(
            "High database connection count ({}), ensure database server can handle the load",
            config.max_connections
        );
    }

    Ok(())
}

/// Validate security configuration
fn validate_security_config(config: &SecurityConfig) -> Result<(), ConfigError> {
    // Validate session timeout
    if config.session_timeout == 0 {
        return Err(ConfigError::InvalidValue {
            field: "security.session_timeout".to_string(),
            message: "Session timeout must be greater than 0".to_string(),
        });
    }

    // Warn about very long session timeouts
    if config.session_timeout > 86400 * 30 {
        warn!(
            "Very long session timeout ({}s), consider security implications",
            config.session_timeout
        );
    }

    // Validate login attempt limits
    if config.max_login_attempts == 0 {
        return Err(ConfigError::InvalidValue {
            field: "security.max_login_attempts".to_string(),
            message: "Maximum login attempts must be greater than 0".to_string(),
        });
    }

    if config.lockout_duration == 0 {
        return Err(ConfigError::InvalidValue {
            field: "security.lockout_duration".to_string(),
            message: "Lockout duration must be greater than 0".to_string(),
        });
    }

    // Validate password requirements
    if config.password_min_length < 4 {
        return Err(ConfigError::InvalidValue {
            field: "security.password_min_length".to_string(),
            message: "Password minimum length must be at least 4 characters".to_string(),
        });
    }

    if config.password_min_length < 8 {
        warn!("Password minimum length is {} - consider requiring at least 8 characters for better security", config.password_min_length);
    }

    // Validate JWT secret if provided
    if let Some(secret) = &config.jwt_secret {
        if secret.len() < 32 {
            return Err(ConfigError::InvalidValue {
                field: "security.jwt_secret".to_string(),
                message: "JWT secret must be at least 32 characters long".to_string(),
            });
        }
    } else {
        warn!("JWT secret not configured - sessions will not persist across server restarts");
    }

    // Validate Argon2 configuration
    validate_argon2_config(&config.argon2)?;

    Ok(())
}

/// Validate Argon2 configuration
fn validate_argon2_config(config: &Argon2Config) -> Result<(), ConfigError> {
    if config.memory_cost < 1024 {
        return Err(ConfigError::InvalidValue {
            field: "security.argon2.memory_cost".to_string(),
            message: "Argon2 memory cost must be at least 1024 KiB".to_string(),
        });
    }

    if config.time_cost == 0 {
        return Err(ConfigError::InvalidValue {
            field: "security.argon2.time_cost".to_string(),
            message: "Argon2 time cost must be greater than 0".to_string(),
        });
    }

    if config.parallelism == 0 {
        return Err(ConfigError::InvalidValue {
            field: "security.argon2.parallelism".to_string(),
            message: "Argon2 parallelism must be greater than 0".to_string(),
        });
    }

    if config.hash_length < 16 {
        return Err(ConfigError::InvalidValue {
            field: "security.argon2.hash_length".to_string(),
            message: "Argon2 hash length must be at least 16 bytes".to_string(),
        });
    }

    // Performance warnings
    if config.memory_cost > 1048576 {
        warn!(
            "Very high Argon2 memory cost ({}), may impact performance",
            config.memory_cost
        );
    }

    if config.time_cost > 10 {
        warn!(
            "Very high Argon2 time cost ({}), may impact performance",
            config.time_cost
        );
    }

    Ok(())
}

/// Validate logging configuration
fn validate_logging_config(config: &LoggingConfig) -> Result<(), ConfigError> {
    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error", "off"];
    if !valid_levels.contains(&config.level.as_str()) {
        return Err(ConfigError::InvalidValue {
            field: "logging.level".to_string(),
            message: format!(
                "Invalid log level: {}. Valid levels: {:?}",
                config.level, valid_levels
            ),
        });
    }

    // Validate log format
    let valid_formats = ["json", "pretty", "compact"];
    if !valid_formats.contains(&config.format.as_str()) {
        return Err(ConfigError::InvalidValue {
            field: "logging.format".to_string(),
            message: format!(
                "Invalid log format: {}. Valid formats: {:?}",
                config.format, valid_formats
            ),
        });
    }

    // Validate file logging settings
    if config.enable_file_logging {
        if config.file_path.is_none() {
            return Err(ConfigError::MissingRequired {
                field: "logging.file_path".to_string(),
            });
        }

        if let Some(file_path) = &config.file_path {
            // Check if parent directory exists or can be created
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    warn!("Log directory does not exist: {}", parent.display());
                }
            }
        }

        if config.max_file_size == 0 {
            return Err(ConfigError::InvalidValue {
                field: "logging.max_file_size".to_string(),
                message: "Maximum file size must be greater than 0".to_string(),
            });
        }

        if config.max_files == 0 {
            return Err(ConfigError::InvalidValue {
                field: "logging.max_files".to_string(),
                message: "Maximum files must be greater than 0".to_string(),
            });
        }
    }

    // Warn if both file and stdout logging are disabled
    if !config.enable_file_logging && !config.enable_stdout {
        warn!("Both file and stdout logging are disabled - logs will be lost");
    }

    Ok(())
}

/// Validate feature configuration
fn validate_feature_config(config: &FeatureConfig) -> Result<(), ConfigError> {
    // Validate file upload settings
    if config.enable_file_uploads {
        if config.max_file_size == 0 {
            return Err(ConfigError::InvalidValue {
                field: "features.max_file_size".to_string(),
                message: "Maximum file size must be greater than 0 when file uploads are enabled"
                    .to_string(),
            });
        }

        if config.allowed_file_types.is_empty() {
            warn!("File uploads enabled but no file types are allowed");
        }
    }

    // Validate message history settings
    if config.enable_message_history && config.message_history_retention > 0 {
        if config.message_history_retention < 1 {
            return Err(ConfigError::InvalidValue {
                field: "features.message_history_retention".to_string(),
                message: "Message history retention must be at least 1 day if not unlimited (0)"
                    .to_string(),
            });
        }
    }

    // Validate feature dependencies
    if config.enable_message_search && !config.enable_message_history {
        return Err(ConfigError::Conflict {
            message: "Message search requires message history to be enabled".to_string(),
        });
    }

    if config.enable_threading && !config.enable_message_history {
        warn!("Threading enabled without message history - threaded messages may not persist");
    }

    Ok(())
}

/// Validate limits configuration
fn validate_limits_config(config: &LimitsConfig) -> Result<(), ConfigError> {
    // Validate rate limiting
    if config.messages_per_minute == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.messages_per_minute".to_string(),
            message: "Messages per minute must be greater than 0".to_string(),
        });
    }

    if config.rate_limit_window == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.rate_limit_window".to_string(),
            message: "Rate limit window must be greater than 0".to_string(),
        });
    }

    // Validate length limits
    if config.max_message_length == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_message_length".to_string(),
            message: "Maximum message length must be greater than 0".to_string(),
        });
    }

    if config.max_username_length == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_username_length".to_string(),
            message: "Maximum username length must be greater than 0".to_string(),
        });
    }

    if config.max_room_name_length == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_room_name_length".to_string(),
            message: "Maximum room name length must be greater than 0".to_string(),
        });
    }

    // Validate count limits
    if config.max_rooms_per_user == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_rooms_per_user".to_string(),
            message: "Maximum rooms per user must be greater than 0".to_string(),
        });
    }

    if config.max_users_per_room == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_users_per_room".to_string(),
            message: "Maximum users per room must be greater than 0".to_string(),
        });
    }

    if config.max_connections_per_ip == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.max_connections_per_ip".to_string(),
            message: "Maximum connections per IP must be greater than 0".to_string(),
        });
    }

    // Validate resource limits
    if config.memory_limit == 0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.memory_limit".to_string(),
            message: "Memory limit must be greater than 0".to_string(),
        });
    }

    if config.cpu_limit <= 0.0 || config.cpu_limit > 100.0 {
        return Err(ConfigError::InvalidValue {
            field: "limits.cpu_limit".to_string(),
            message: "CPU limit must be between 0 and 100 percent".to_string(),
        });
    }

    // Performance warnings
    if config.messages_per_minute > 1000 {
        warn!(
            "Very high message rate limit ({}), ensure server can handle the load",
            config.messages_per_minute
        );
    }

    if config.max_message_length > 10000 {
        warn!(
            "Very long message limit ({}), may impact performance",
            config.max_message_length
        );
    }

    Ok(())
}

/// Validate admin configuration
fn validate_admin_config(config: &AdminConfig) -> Result<(), ConfigError> {
    // Validate admin API settings
    if config.enable_admin_api {
        // Validate admin host
        if config.admin_host.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "admin.admin_host".to_string(),
                message: "Admin host cannot be empty when admin API is enabled".to_string(),
            });
        }

        // Validate admin port
        if config.admin_port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "admin.admin_port".to_string(),
                message: "Admin port cannot be 0".to_string(),
            });
        }

        // Warn if admin token is not set
        if config.admin_token.is_none() {
            warn!("Admin API enabled but no admin token configured - API will be unsecured");
        } else if let Some(token) = &config.admin_token {
            if token.len() < 32 {
                return Err(ConfigError::InvalidValue {
                    field: "admin.admin_token".to_string(),
                    message: "Admin token must be at least 32 characters long".to_string(),
                });
            }
        }
    }

    // Validate endpoint paths
    if config.enable_metrics {
        if config.metrics_path.is_empty() || !config.metrics_path.starts_with('/') {
            return Err(ConfigError::InvalidValue {
                field: "admin.metrics_path".to_string(),
                message: "Metrics path must start with '/' and not be empty".to_string(),
            });
        }
    }

    if config.enable_health_check {
        if config.health_check_path.is_empty() || !config.health_check_path.starts_with('/') {
            return Err(ConfigError::InvalidValue {
                field: "admin.health_check_path".to_string(),
                message: "Health check path must start with '/' and not be empty".to_string(),
            });
        }
    }

    // Validate audit log configuration
    validate_audit_log_config(&config.audit_log)?;

    Ok(())
}

/// Validate audit log configuration
fn validate_audit_log_config(config: &AuditLogConfig) -> Result<(), ConfigError> {
    if config.enabled {
        if config.file_path.is_none() {
            return Err(ConfigError::MissingRequired {
                field: "admin.audit_log.file_path".to_string(),
            });
        }

        let valid_formats = ["json", "csv"];
        if !valid_formats.contains(&config.format.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "admin.audit_log.format".to_string(),
                message: format!(
                    "Invalid audit log format: {}. Valid formats: {:?}",
                    config.format, valid_formats
                ),
            });
        }

        // Check if audit log directory exists
        if let Some(file_path) = &config.file_path {
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    warn!("Audit log directory does not exist: {}", parent.display());
                }
            }
        }
    }

    Ok(())
}

/// Validate cross-section dependencies and conflicts
fn validate_cross_dependencies(config: &ServerConfig) -> Result<(), ConfigError> {
    // Check for port conflicts
    if config.admin.enable_admin_api && config.server.port == config.admin.admin_port {
        return Err(ConfigError::Conflict {
            message: "Server port and admin port cannot be the same".to_string(),
        });
    }

    // Validate TLS and security settings
    if config.server.enable_tls && !config.security.enable_encryption {
        warn!("TLS enabled but encryption disabled - consider enabling both for maximum security");
    }

    // Validate database and feature dependencies
    if config.features.enable_message_history && config.database.database_type == "memory" {
        warn!("Message history enabled with in-memory database - history will not persist");
    }

    // Validate admin security
    if config.admin.enable_admin_api
        && config.admin.admin_host == "0.0.0.0"
        && config.admin.admin_token.is_none()
    {
        return Err(ConfigError::ValidationError {
            message: "Admin API bound to all interfaces without authentication token - major security risk".to_string(),
        });
    }

    // Validate resource limits vs connection limits
    let estimated_memory_per_connection = 1; // 1 MB per connection estimate
    let estimated_total_memory = config.server.max_connections * estimated_memory_per_connection;
    if estimated_total_memory as u64 > config.limits.memory_limit {
        warn!(
            "Memory limit ({} MB) may be insufficient for max connections ({})",
            config.limits.memory_limit, config.server.max_connections
        );
    }

    Ok(())
}

/// Basic hostname validation
fn is_valid_hostname(hostname: &str) -> bool {
    if hostname.is_empty() || hostname.len() > 253 {
        return false;
    }

    // Check each label
    for label in hostname.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }

        // Labels must start and end with alphanumeric characters
        if !label.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }

        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_default_config() {
        let config = ServerConfig::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let mut config = ServerConfig::default();
        config.server.port = 0;
        assert!(validate_network_config(&config.server).is_err());
    }

    #[test]
    fn test_invalid_database_type() {
        let mut config = DatabaseConfig::default();
        config.database_type = "invalid".to_string();
        assert!(validate_database_config(&config).is_err());
    }

    #[test]
    fn test_weak_password_policy() {
        let mut config = SecurityConfig::default();
        config.password_min_length = 2;
        assert!(validate_security_config(&config).is_err());
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = LoggingConfig::default();
        config.level = "invalid".to_string();
        assert!(validate_logging_config(&config).is_err());
    }

    #[test]
    fn test_port_conflict() {
        let mut config = ServerConfig::default();
        config.admin.enable_admin_api = true;
        config.admin.admin_port = config.server.port;
        assert!(validate_cross_dependencies(&config).is_err());
    }

    #[test]
    fn test_hostname_validation() {
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("sub.example.com"));
        assert!(is_valid_hostname("localhost"));
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("-example.com"));
        assert!(!is_valid_hostname("example.com-"));
    }

    #[test]
    fn test_tls_validation() {
        let mut config = NetworkConfig::default();
        config.enable_tls = true;
        // Should fail without certificate paths
        assert!(validate_network_config(&config).is_err());

        config.tls_cert_path = Some(PathBuf::from("/nonexistent/cert.pem"));
        config.tls_key_path = Some(PathBuf::from("/nonexistent/key.pem"));
        // Should fail with nonexistent files
        assert!(validate_network_config(&config).is_err());
    }

    #[test]
    fn test_argon2_validation() {
        let mut config = Argon2Config::default();
        config.memory_cost = 512; // Too low
        assert!(validate_argon2_config(&config).is_err());

        config.memory_cost = 65536;
        config.time_cost = 0; // Invalid
        assert!(validate_argon2_config(&config).is_err());
    }
}
