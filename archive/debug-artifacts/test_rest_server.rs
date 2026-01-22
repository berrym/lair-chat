//! Simple test script to start the REST API server with SQLite storage

use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lair_chat::server::{
    api::{start_api_server, ApiState},
    config::ServerConfig,
    storage::{DatabaseConfig, StorageManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lair_chat=info,test_rest_server=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🚀 Starting Lair Chat REST API Server Test");

    // Create directories
    tokio::fs::create_dir_all("data").await?;
    tokio::fs::create_dir_all("logs").await?;

    // Create a simple config
    let mut config = ServerConfig::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 8082;
    config.database.url = "sqlite:data/test-lair-chat.db".to_string();
    config.database.auto_migrate = true;

    println!("📊 Database: {}", config.database.url);

    // Initialize storage
    let db_config = DatabaseConfig::from(config.database.clone());
    let storage = Arc::new(StorageManager::new(db_config).await?);

    println!("✅ Storage initialized successfully");

    // Generate JWT secret
    let jwt_secret = "test-jwt-secret-change-in-production".to_string();

    // Create API state
    let api_state = ApiState::new(storage, jwt_secret, Arc::new(config.clone()));

    // Create server bind address
    let bind_addr =
        format!("{}:{}", config.server.host, config.server.port).parse::<std::net::SocketAddr>()?;

    println!("🌐 Starting REST API server on http://{}", bind_addr);
    println!(
        "📖 API documentation will be available at http://{}/docs",
        bind_addr
    );
    println!("🏥 Health check available at http://{}/health", bind_addr);
    println!(
        "🔐 Admin endpoints available at http://{}/api/v1/admin/*",
        bind_addr
    );
    println!("");
    println!("Press Ctrl+C to stop the server");

    // Start the server
    start_api_server(bind_addr, api_state).await?;

    Ok(())
}
