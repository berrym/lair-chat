//! # Lair Chat Server
//!
//! Unified server binary that runs TCP and HTTP protocol adapters
//! with a shared core engine.
//!
//! ## Usage
//!
//! ```bash
//! # Run with defaults
//! lair-chat-server
//!
//! # With environment variables
//! LAIR_TCP_PORT=8080 LAIR_HTTP_PORT=8082 LAIR_DATABASE_URL=sqlite:data.db lair-chat-server
//! ```
//!
//! ## Environment Variables
//!
//! - `LAIR_TCP_PORT`: TCP server port (default: 8080)
//! - `LAIR_HTTP_PORT`: HTTP server port (default: 8082)
//! - `LAIR_DATABASE_URL`: SQLite database URL (default: sqlite:lair-chat.db?mode=rwc)
//! - `RUST_LOG`: Log level (default: info)

use std::sync::Arc;

use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lair_chat_server::adapters::{http::HttpServer, tcp::TcpServer};
use lair_chat_server::config::Config;
use lair_chat_server::core::engine::ChatEngine;
use lair_chat_server::storage::sqlite::SqliteStorage;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,lair_chat_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        "Lair Chat Server v{} starting...",
        env!("CARGO_PKG_VERSION")
    );

    if let Err(e) = run().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded");
    info!("  TCP port: {}", config.tcp.port);
    info!("  HTTP port: {}", config.http.port);
    info!("  Database: {}", config.database.url);

    // Log TLS status
    if let Some(ref tls) = config.http.tls {
        info!("  TLS: enabled");
        info!("    Certificate: {}", tls.cert_path.display());
        info!("    Private key: {}", tls.key_path.display());
    } else {
        info!("  TLS: disabled (set LAIR_TLS_ENABLED=true to enable)");
    }

    // Initialize storage
    info!("Initializing storage...");
    let storage = SqliteStorage::with_config(config.database).await?;

    // Initialize core engine
    info!("Initializing chat engine...");
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), &config.jwt_secret));

    // Start protocol adapters
    info!("Starting protocol adapters...");

    let tcp_server = TcpServer::start(config.tcp.clone(), engine.clone()).await?;
    let http_server = HttpServer::start(config.http.clone(), engine.clone()).await?;

    info!("Server ready!");
    info!("  TCP: telnet localhost {}", config.tcp.port);
    if config.http.tls.is_some() {
        info!(
            "  HTTPS: curl -k https://localhost:{}/health",
            config.http.port
        );
    } else {
        info!("  HTTP: curl http://localhost:{}/health", config.http.port);
    }

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutdown signal received, stopping servers...");

    // Graceful shutdown
    tcp_server.shutdown().await;
    http_server.shutdown().await;

    info!("Servers stopped. Goodbye!");

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
