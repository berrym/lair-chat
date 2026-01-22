//! New Lair Chat Server binary with REST API and configuration integration
//!
//! This is the main server binary that uses the new configuration management,
//! storage systems, and REST API. It provides a clean, production-ready server
//! implementation with proper error handling, logging, and graceful shutdown.

use base64::{engine::general_purpose, Engine as _};
use clap::{Arg, Command};
use color_eyre::eyre::{Context, Result};
use std::{path::PathBuf, sync::Arc};
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lair_chat::server::{
    api::{start_api_server, ApiState},
    config::{load_config, load_config_from_file, ConfigLoader, ServerConfig},
    storage::{DatabaseConfig, StorageManager},
};

/// Server application state
struct ServerApp {
    config: ServerConfig,
    storage: Arc<StorageManager>,
}

impl ServerApp {
    /// Create a new server application
    async fn new(config: ServerConfig) -> Result<Self> {
        info!("Initializing server with configuration");

        // Initialize storage
        let db_config = DatabaseConfig::from(config.database.clone());
        let storage = Arc::new(
            StorageManager::new(db_config)
                .await
                .context("Failed to initialize storage")?,
        );

        info!("Storage initialized successfully");

        // Migrations are handled automatically by the storage backend if auto_migrate is enabled
        info!("Storage initialization completed successfully");

        Ok(Self { config, storage })
    }

    /// Run the server
    async fn run(self) -> Result<()> {
        info!(
            "Starting Lair Chat REST API Server v{} on {}:{}",
            env!("CARGO_PKG_VERSION"),
            self.config.server.host,
            self.config.server.port
        );

        // Generate JWT secret (in production, this should be loaded from config)
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let secret: [u8; 32] = rng.gen();
            general_purpose::STANDARD.encode(secret)
        });

        // Create API state
        let api_state = ApiState::new(
            Arc::clone(&self.storage),
            jwt_secret,
            Arc::new(self.config.clone()),
        );

        // Create server bind address
        let bind_addr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse::<std::net::SocketAddr>()
            .context("Invalid server address")?;

        info!("Server components initialized successfully");

        // Display startup information
        self.display_startup_info(&bind_addr);

        // Start REST API server
        let api_state_clone = api_state.clone();
        let server_task = tokio::spawn(async move {
            if let Err(e) = start_api_server(bind_addr, api_state_clone).await {
                tracing::error!("API server error: {}", e);
            }
        });

        // Start cleanup task
        let cleanup_task = {
            let storage = Arc::clone(&self.storage);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Every hour
                loop {
                    interval.tick().await;
                    if let Err(e) = storage.cleanup().await {
                        warn!("Storage cleanup error: {}", e);
                    }
                }
            })
        };

        // Wait for shutdown signal
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, initiating graceful shutdown");
            }
            result = server_task => {
                match result {
                    Ok(()) => info!("API server completed successfully"),
                    Err(e) => warn!("API server task error: {}", e),
                }
            }
        }

        // Cancel cleanup task
        cleanup_task.abort();

        info!("Server shutdown complete");
        Ok(())
    }

    /// Display startup information and access URLs
    fn display_startup_info(&self, bind_addr: &std::net::SocketAddr) {
        println!("\n🎉 Lair Chat Server Started Successfully!");
        println!("==========================================");

        // Check if admin dashboard exists
        let dashboard_exists = std::path::Path::new("admin-dashboard/index.html").exists();

        println!("\n📊 Available Services:");
        println!("   • REST API:          http://{}/api/v1", bind_addr);
        println!("   • API Health Check:  http://{}/api/v1/health", bind_addr);
        println!("   • API Documentation: http://{}/docs", bind_addr);

        if dashboard_exists {
            println!("   • Admin Dashboard:    http://{}/admin/", bind_addr);
            println!("   • Server Info:        http://{}/", bind_addr);

            println!("\n🔐 Default Admin Credentials:");
            println!("   Username: admin");
            println!("   Password: AdminPassword123!");

            println!("\n🎯 Quick Start:");
            println!("   1. Open your browser");
            println!("   2. Navigate to: http://{}/admin/", bind_addr);
            println!("   3. Login with the credentials above");
            println!("   4. Start managing your chat system!");
        } else {
            println!(
                "   ⚠️  Admin Dashboard: Not available (admin-dashboard/ directory not found)"
            );
            println!("\n💡 To enable the admin dashboard:");
            println!("   1. Run: ./setup_admin_system.sh");
            println!("   2. Restart the server");
        }

        println!("\n🛠️  Management Commands:");
        println!("   Create admin user:    cargo run --bin create_admin_user");
        println!("   Debug authentication: cargo run --bin debug_jwt_auth");
        println!("   Test API endpoints:   ./test_api.sh");
        println!("   Verify system:        ./verify_system.sh");

        println!("\n📁 Important Files:");
        println!("   Configuration: .env");
        println!("   Database:      data/lair_chat.db");
        println!("   Logs:          logs/server.log");

        println!("\n🚀 Server is ready! Press Ctrl+C to stop.");
        println!("==========================================\n");
    }
}

/// Initialize logging based on configuration
fn init_logging(config: &ServerConfig) -> Result<()> {
    let log_level = config.logging.level.parse().unwrap_or(tracing::Level::INFO);

    let stdout_layer = if config.logging.enable_stdout {
        Some(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_level(true)
                .compact(),
        )
    } else {
        None
    };

    let file_layer = if config.logging.enable_file_logging {
        if let Some(log_path) = &config.logging.file_path {
            // Create log directory if it doesn't exist
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent).context("Failed to create log directory")?;
            }

            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .context("Failed to open log file")?;

            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(file)
                    .with_ansi(false),
            )
        } else {
            None
        }
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("lair_chat={}", log_level).into()),
        )
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(())
}

/// Create default configuration files
async fn create_default_config(path: &PathBuf) -> Result<()> {
    info!("Creating default configuration at: {}", path.display());

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let loader = ConfigLoader::new();
    loader
        .create_sample_config(path, lair_chat::server::config::loader::ConfigFormat::Toml)
        .context("Failed to create sample configuration")?;

    info!("Default configuration created successfully");
    Ok(())
}

/// Validate and display configuration
fn validate_config(config: &ServerConfig) -> Result<()> {
    // Run configuration validation
    lair_chat::server::config::validate_config(config)
        .context("Configuration validation failed")?;

    info!("Configuration validation passed");

    // Display key configuration settings
    info!("Server Configuration:");
    info!("  Host: {}", config.server.host);
    info!("  Port: {}", config.server.port);
    info!("  Max Connections: {}", config.server.max_connections);
    info!("  TLS Enabled: {}", config.server.enable_tls);
    info!("  Database: {}", config.database.url);
    info!("  Encryption: {}", config.security.enable_encryption);
    info!("  Admin API: {}", config.admin.enable_admin_api);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup panic handling
    color_eyre::install()?;

    let matches = Command::new("lair-chat-server")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Lair Chat Server - A secure, terminal-based chat server")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("create-config")
                .long("create-config")
                .value_name("FILE")
                .help("Create a default configuration file")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .help("Validate configuration and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("host")
                .short('H')
                .long("host")
                .value_name("HOST")
                .help("Override server host"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Override server port")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(
            Arg::new("database-url")
                .short('d')
                .long("database-url")
                .value_name("URL")
                .help("Override database URL"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Set log level (trace, debug, info, warn, error)"),
        )
        .get_matches();

    // Handle create-config command
    if let Some(config_path) = matches.get_one::<PathBuf>("create-config") {
        create_default_config(config_path).await?;
        return Ok(());
    }

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        info!("Loading configuration from: {}", config_path.display());
        load_config_from_file(config_path).context("Failed to load configuration file")?
    } else {
        info!("Loading configuration from default sources");
        match load_config() {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to load configuration: {}", e);
                info!("Using default configuration");
                ServerConfig::default()
            }
        }
    };

    // Apply command line overrides
    let mut config = config;
    if let Some(host) = matches.get_one::<String>("host") {
        config.server.host = host.clone();
    }
    if let Some(port) = matches.get_one::<u16>("port") {
        config.server.port = *port;
    }
    if let Some(database_url) = matches.get_one::<String>("database-url") {
        config.database.url = database_url.clone();
    }
    if let Some(log_level) = matches.get_one::<String>("log-level") {
        config.logging.level = log_level.clone();
    }

    // Initialize logging
    init_logging(&config)?;

    // Validate configuration
    validate_config(&config)?;

    // Handle validate-only command
    if matches.get_flag("validate") {
        info!("Configuration is valid");
        return Ok(());
    }

    // Create and run server
    let app = ServerApp::new(config).await?;
    app.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        create_default_config(&config_path).await.unwrap();
        assert!(config_path.exists());

        // Verify we can load the created config
        let config = load_config_from_file(&config_path).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_validate_default_config() {
        let config = ServerConfig::default();
        validate_config(&config).unwrap();
    }

    #[tokio::test]
    async fn test_server_app_creation() {
        let mut config = ServerConfig::default();
        config.database.url = "sqlite::memory:".to_string();
        let app = ServerApp::new(config).await.unwrap();

        // Basic verification that app was created successfully
        assert_eq!(app.config.server.host, "127.0.0.1");
    }
}
