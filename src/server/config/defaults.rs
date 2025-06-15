//! Default configuration values for lair-chat server
//!
//! This module provides sensible default values for all configuration options,
//! ensuring the server can start with minimal configuration while providing
//! secure and performant defaults for production use.

use super::*;
use std::time::Duration;

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: NetworkConfig::default(),
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
            limits: LimitsConfig::default(),
            admin: AdminConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            connection_timeout: 30, // 30 seconds
            keep_alive_timeout: 60, // 60 seconds
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_versions: vec!["1.2".to_string(), "1.3".to_string()],
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_type: "sqlite".to_string(),
            url: "data/lair-chat.db".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout: 30,
            idle_timeout: 600, // 10 minutes
            auto_migrate: true,
            settings: HashMap::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption: true,
            session_timeout: 86400, // 24 hours
            max_login_attempts: 5,
            lockout_duration: 900, // 15 minutes
            password_min_length: 8,
            password_require_special: true,
            password_require_numbers: true,
            password_require_uppercase: true,
            enable_2fa: false,
            jwt_secret: None,
            argon2: Argon2Config::default(),
            rotate_refresh_tokens: true,
        }
    }
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory_cost: 65536, // 64 MiB
            time_cost: 3,       // 3 iterations
            parallelism: 4,     // 4 parallel threads
            hash_length: 32,    // 32 bytes output
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            enable_file_logging: true,
            file_path: Some(PathBuf::from("logs/server.log")),
            max_file_size: 10, // 10 MB
            max_files: 5,
            compress_logs: true,
            enable_stdout: true,
            enable_json: false,
            targets: HashMap::new(),
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            enable_direct_messages: true,
            enable_user_room_creation: true,
            enable_file_uploads: false, // Disabled by default for security
            max_file_size: 10,          // 10 MB
            allowed_file_types: vec![
                "txt".to_string(),
                "md".to_string(),
                "pdf".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
            ],
            enable_message_history: true,
            message_history_retention: 0, // Unlimited
            enable_message_search: true,
            enable_user_profiles: true,
            enable_presence: true,
            enable_typing_indicators: true,
            enable_reactions: true,
            enable_threading: false, // Disabled by default for simplicity
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            messages_per_minute: 60,
            max_message_length: 4000,
            max_username_length: 32,
            max_room_name_length: 50,
            max_rooms_per_user: 50,
            max_users_per_room: 100,
            max_connections_per_ip: 10,
            rate_limit_window: 60, // 1 minute
            memory_limit: 512,     // 512 MB
            cpu_limit: 80.0,       // 80% CPU usage
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enable_admin_api: true,
            admin_host: "127.0.0.1".to_string(),
            admin_port: 8081,
            admin_token: None, // Should be generated or provided
            enable_metrics: true,
            metrics_path: "/metrics".to_string(),
            enable_health_check: true,
            health_check_path: "/health".to_string(),
            enable_debug_endpoints: false, // Disabled by default for security
            audit_log: AuditLogConfig::default(),
        }
    }
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_path: Some(PathBuf::from("logs/audit.log")),
            format: "json".to_string(),
            log_admin_actions: true,
            log_auth_events: true,
            log_security_events: true,
            log_data_access: false, // Disabled by default for performance
        }
    }
}

/// Production-ready configuration with enhanced security
pub fn production_config() -> ServerConfig {
    let mut config = ServerConfig::default();

    // Production network settings
    config.server.host = "0.0.0.0".to_string();
    config.server.max_connections = 5000;
    config.server.enable_tls = true;

    // Production database settings
    config.database.database_type = "postgresql".to_string();
    config.database.url = "postgresql://user:pass@localhost/lair_chat".to_string();
    config.database.max_connections = 50;
    config.database.min_connections = 5;

    // Enhanced security for production
    config.security.session_timeout = 3600; // 1 hour
    config.security.max_login_attempts = 3;
    config.security.lockout_duration = 1800; // 30 minutes
    config.security.enable_2fa = true;
    config.security.argon2.memory_cost = 131072; // 128 MiB
    config.security.argon2.time_cost = 4;

    // Production logging
    config.logging.level = "warn".to_string();
    config.logging.format = "json".to_string();
    config.logging.enable_json = true;
    config.logging.max_file_size = 100; // 100 MB
    config.logging.max_files = 30;

    // Conservative feature flags for production
    config.features.enable_file_uploads = false;
    config.features.enable_threading = false;
    config.features.message_history_retention = 365; // 1 year

    // Stricter limits for production
    config.limits.messages_per_minute = 30;
    config.limits.max_connections_per_ip = 5;
    config.limits.memory_limit = 2048; // 2 GB

    // Production admin settings
    config.admin.enable_debug_endpoints = false;
    config.admin.audit_log.log_data_access = true;

    config
}

/// Development configuration with relaxed settings
pub fn development_config() -> ServerConfig {
    let mut config = ServerConfig::default();

    // Development-friendly settings
    config.server.max_connections = 100;

    // In-memory database for development
    config.database.url = ":memory:".to_string();
    config.database.auto_migrate = true;

    // Relaxed security for development
    config.security.session_timeout = 86400 * 7; // 1 week
    config.security.max_login_attempts = 10;
    config.security.lockout_duration = 60; // 1 minute
    config.security.password_min_length = 4;
    config.security.password_require_special = false;
    config.security.password_require_numbers = false;
    config.security.password_require_uppercase = false;

    // Verbose logging for development
    config.logging.level = "debug".to_string();
    config.logging.format = "pretty".to_string();
    config.logging.enable_file_logging = false;

    // All features enabled for development
    config.features.enable_file_uploads = true;
    config.features.enable_threading = true;
    config.features.max_file_size = 100; // 100 MB

    // Relaxed limits for development
    config.limits.messages_per_minute = 1000;
    config.limits.max_connections_per_ip = 50;
    config.limits.memory_limit = 1024; // 1 GB

    // Debug endpoints enabled
    config.admin.enable_debug_endpoints = true;

    config
}

/// Testing configuration with minimal settings
pub fn test_config() -> ServerConfig {
    let mut config = ServerConfig::default();

    // Use random port for testing
    config.server.port = 0; // OS will assign a random port
    config.server.max_connections = 10;

    // In-memory database for tests
    config.database.url = ":memory:".to_string();
    config.database.max_connections = 1;
    config.database.auto_migrate = true;

    // Minimal security for fast tests
    config.security.session_timeout = 3600;
    config.security.password_min_length = 1;
    config.security.password_require_special = false;
    config.security.password_require_numbers = false;
    config.security.password_require_uppercase = false;
    config.security.argon2.memory_cost = 4096; // 4 MiB
    config.security.argon2.time_cost = 1;

    // Quiet logging for tests
    config.logging.level = "error".to_string();
    config.logging.enable_file_logging = false;
    config.logging.enable_stdout = false;

    // Minimal features for testing
    config.features.enable_file_uploads = false;
    config.features.enable_message_search = false;
    config.features.enable_presence = false;
    config.features.enable_typing_indicators = false;
    config.features.enable_reactions = false;

    // Relaxed limits for testing
    config.limits.messages_per_minute = 10000;
    config.limits.max_connections_per_ip = 100;

    // No admin features for testing
    config.admin.enable_admin_api = false;
    config.admin.enable_metrics = false;
    config.admin.enable_health_check = false;
    config.admin.audit_log.enabled = false;

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(config.security.enable_encryption);
        assert_eq!(config.database.database_type, "sqlite");
    }

    #[test]
    fn test_production_config() {
        let config = production_config();
        assert_eq!(config.server.host, "0.0.0.0");
        assert!(config.server.enable_tls);
        assert_eq!(config.database.database_type, "postgresql");
        assert!(config.security.enable_2fa);
        assert_eq!(config.logging.format, "json");
    }

    #[test]
    fn test_development_config() {
        let config = development_config();
        assert_eq!(config.database.url, ":memory:");
        assert_eq!(config.logging.level, "debug");
        assert!(config.features.enable_file_uploads);
        assert!(config.admin.enable_debug_endpoints);
    }

    #[test]
    fn test_test_config() {
        let config = test_config();
        assert_eq!(config.server.port, 0);
        assert_eq!(config.database.url, ":memory:");
        assert_eq!(config.logging.level, "error");
        assert!(!config.admin.enable_admin_api);
    }

    #[test]
    fn test_config_values_are_reasonable() {
        let config = ServerConfig::default();

        // Network settings
        assert!(config.server.port > 0 && config.server.port <= 65535);
        assert!(config.server.max_connections > 0);
        assert!(config.server.connection_timeout > 0);

        // Database settings
        assert!(config.database.max_connections > 0);
        assert!(config.database.connection_timeout > 0);

        // Security settings
        assert!(config.security.password_min_length >= 4);
        assert!(config.security.session_timeout > 0);
        assert!(config.security.max_login_attempts > 0);

        // Limits
        assert!(config.limits.max_message_length > 0);
        assert!(config.limits.max_username_length > 0);
        assert!(config.limits.messages_per_minute > 0);
    }
}
