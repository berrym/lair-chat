//! REST API module for Lair Chat Server
//!
//! This module provides HTTP REST API endpoints for the chat server,
//! built on top of the Axum web framework. It exposes all core functionality
//! through standardized REST endpoints with JSON serialization.
//!
//! # Architecture
//!
//! The API is organized into several layers:
//! - **Routes**: HTTP route definitions and endpoint grouping
//! - **Handlers**: Request processing logic and business operations
//! - **Middleware**: Cross-cutting concerns (auth, logging, rate limiting)
//! - **Models**: Request/response data structures and validation
//!
//! # Authentication
//!
//! The API uses JWT (JSON Web Tokens) for authentication with the following flow:
//! 1. User registers/logs in via `/api/v1/auth/login`
//! 2. Server returns JWT token with user claims
//! 3. Client includes token in `Authorization: Bearer <token>` header
//! 4. Middleware validates token on protected endpoints
//!
//! # API Versioning
//!
//! All endpoints are versioned under `/api/v1/` to support future API evolution
//! while maintaining backward compatibility.

use axum::{extract::State, http::StatusCode, response::Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

use crate::server::storage::StorageManager;

pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;

// Re-export commonly used types
pub use handlers::*;
pub use middleware::*;
pub use models::*;
pub use routes::*;

/// Shared application state passed to all handlers
#[derive(Clone)]
pub struct ApiState {
    /// Database storage layer
    pub storage: Arc<StorageManager>,
    /// JWT signing keys
    pub jwt_secret: String,
    /// Server configuration
    pub config: Arc<crate::server::config::ServerConfig>,
}

impl ApiState {
    /// Create new API state
    pub fn new(
        storage: Arc<StorageManager>,
        jwt_secret: String,
        config: Arc<crate::server::config::ServerConfig>,
    ) -> Self {
        Self {
            storage,
            jwt_secret,
            config,
        }
    }
}

/// Create the main API router with all routes and middleware
pub fn create_api_router(state: ApiState) -> Router {
    info!("Creating API router with middleware stack");

    // Create the main API router
    let api_router = Router::new()
        // Health check endpoint (no auth required)
        .route("/health", axum::routing::get(health_check))
        // Authentication routes (no auth required)
        .nest("/auth", routes::auth::create_auth_routes())
        // User management routes (auth required)
        .nest("/users", routes::users::create_user_routes())
        // Room management routes (auth required)
        .nest("/rooms", routes::rooms::create_room_routes())
        // Message routes (auth required)
        .nest("/messages", routes::messages::create_message_routes())
        // Session management routes (auth required)
        .nest("/sessions", routes::sessions::create_session_routes())
        // Admin routes (admin auth required)
        .nest("/admin", routes::admin::create_admin_routes())
        // Add state to all routes
        .with_state(state);

    // Wrap API routes under /api/v1 prefix and apply simplified middleware
    Router::new()
        .nest("/api/v1", api_router)
        // Add Swagger UI documentation
        .merge(create_docs_router())
        // Apply CORS configuration
        .layer(create_cors_layer())
        // Request tracing
        .layer(TraceLayer::new_for_http())
}

/// Health check endpoint - returns server status
async fn health_check(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Test database connectivity
    let db_status = match state.storage.health_check().await {
        Ok(_) => "healthy",
        Err(e) => {
            warn!("Database health check failed: {}", e);
            "degraded"
        }
    };

    Ok(Json(json!({
        "status": "ok",
        "service": "lair-chat-api",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Create CORS layer with appropriate settings
fn create_cors_layer() -> CorsLayer {
    use http::Method;
    use tower_http::cors::{Any, CorsLayer};

    // In production, this should be more restrictive
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_origin(Any) // TODO: Restrict in production
}

/// Create documentation router with Swagger UI
fn create_docs_router() -> Router {
    use axum::routing::get;

    // Simple docs router without OpenAPI for now
    Router::new().route("/docs", get(|| async { "API Documentation - Coming Soon" }))
}

/// Start the API server on the specified address
pub async fn start_api_server(
    bind_addr: std::net::SocketAddr,
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting API server on {}", bind_addr);

    let app = create_api_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("API server listening on http://{}", bind_addr);
    info!("API documentation available at http://{}/docs", bind_addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use std::sync::Arc;

    async fn create_test_state() -> ApiState {
        // This would need proper test setup
        todo!("Implement test state creation")
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = create_test_state().await;
        let app = create_api_router(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/v1/health").await;

        assert_eq!(response.status_code(), 200);

        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "lair-chat-api");
    }

    #[tokio::test]
    async fn test_docs_endpoint() {
        let state = create_test_state().await;
        let app = create_api_router(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/docs").await;
        assert_eq!(response.status_code(), 200);
    }
}
