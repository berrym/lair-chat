//! Lair-Chat Server Binary
//!
//! This is the main entry point for the lair-chat server application.
//! It initializes logging, parses command line arguments, and starts the server.

use clap::Parser;

use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;

use lair_chat::server::app::{ChatServer, ServerConfig};

/// Command line arguments for the server
#[derive(Parser, Debug)]
#[command(name = "lair-chat-server")]
#[command(about = "A secure, terminal-based chat server built with Rust")]
#[command(version)]
struct Args {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Maximum number of concurrent connections
    #[arg(short, long, default_value_t = 1000)]
    max_connections: usize,

    /// Disable encryption (not recommended for production)
    #[arg(long)]
    no_encryption: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    // Create server configuration
    let config = ServerConfig {
        host: args.host.clone(),
        port: args.port,
        max_connections: args.max_connections,
        enable_encryption: !args.no_encryption,
    };

    info!("Starting Lair-Chat Server");
    info!("Configuration: {:?}", config);
    info!("Press Ctrl+C to shut down");

    // Create and start the server
    let server = ChatServer::new(config);

    // Handle shutdown signals
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("Server error: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = server_handle => {
            info!("Server stopped");
        }
    }

    info!("Server shutdown complete");
    Ok(())
}

/// Initialize logging based on command line arguments
fn init_logging(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("Invalid log level: {}", args.log_level);
            std::process::exit(1);
        }
    };

    // Human-readable logging
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    Ok(())
}
