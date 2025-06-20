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

use axum::{
    extract::State,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{Html, Json, Redirect},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::{info, warn};

use crate::server::storage::StorageManager;
use crate::shared_types::SharedTcpState;

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
    /// TCP server state for monitoring (optional for integrated mode)
    pub tcp_state: Option<SharedTcpState>,
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
            tcp_state: None,
        }
    }

    /// Create new API state with TCP server state for integrated mode
    pub fn new_with_tcp_state(
        storage: Arc<StorageManager>,
        jwt_secret: String,
        config: Arc<crate::server::config::ServerConfig>,
        tcp_state: SharedTcpState,
    ) -> Self {
        Self {
            storage,
            jwt_secret,
            config,
            tcp_state: Some(tcp_state),
        }
    }

    /// Check if running in integrated mode with TCP server
    pub fn is_integrated_mode(&self) -> bool {
        self.tcp_state.is_some()
    }

    /// Get TCP server state if available
    pub fn tcp_state(&self) -> Option<&SharedTcpState> {
        self.tcp_state.as_ref()
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
        .nest(
            "/users",
            routes::users::create_user_routes()
                .layer(from_fn_with_state(state.clone(), jwt_auth_middleware)),
        )
        // Room management routes (auth required)
        .nest(
            "/rooms",
            routes::rooms::create_room_routes()
                .layer(from_fn_with_state(state.clone(), jwt_auth_middleware)),
        )
        // Message routes (auth required)
        .nest(
            "/messages",
            routes::messages::create_message_routes()
                .layer(from_fn_with_state(state.clone(), jwt_auth_middleware)),
        )
        // Session management routes (auth required)
        .nest(
            "/sessions",
            routes::sessions::create_session_routes()
                .layer(from_fn_with_state(state.clone(), jwt_auth_middleware)),
        )
        // Admin routes (admin auth required)
        .nest("/admin", routes::admin::create_admin_routes())
        // Add state to all routes
        .with_state(state);

    // Wrap API routes under /api/v1 prefix and apply simplified middleware
    Router::new()
        .nest("/api/v1", api_router)
        // Add Swagger UI documentation
        .merge(create_docs_router())
        // Serve admin dashboard as static files
        .merge(create_admin_dashboard_router())
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
    // Simple docs router without OpenAPI for now
    Router::new().route("/docs", get(|| async { "API Documentation - Coming Soon" }))
}

/// Create admin dashboard router to serve static files
fn create_admin_dashboard_router() -> Router {
    // Check if admin dashboard directory exists
    let dashboard_path = std::path::Path::new("admin-dashboard");

    if dashboard_path.exists() && dashboard_path.is_dir() {
        info!("Admin dashboard found at admin-dashboard/, serving static files");

        Router::new()
            // Serve the main dashboard at /admin
            .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
            // Serve static files from admin-dashboard directory
            .nest_service("/admin/", ServeDir::new("admin-dashboard"))
            // Also serve at root path for convenience
            .route("/", get(serve_root_redirect))
    } else {
        warn!("Admin dashboard not found at admin-dashboard/");
        Router::new()
            .route("/admin", get(|| async {
                Html("<h1>Admin Dashboard Not Found</h1><p>Please ensure admin-dashboard/ directory exists</p>")
            }))
            .route("/", get(serve_root_info))
    }
}

/// Serve root path with info about available services
async fn serve_root_info() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Lair Chat Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f7fa; }
        .container { max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #34495e; }
        .service { margin: 20px 0; padding: 15px; background: #ecf0f1; border-radius: 5px; }
        .service h3 { margin: 0 0 10px 0; color: #2c3e50; }
        a { color: #3498db; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .status { display: inline-block; width: 12px; height: 12px; background: #27ae60; border-radius: 50%; margin-right: 8px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Lair Chat Server</h1>
        <p>Welcome to the Lair Chat administration server. Your server is running and ready!</p>

        <div class="service">
            <h3><span class="status"></span>Admin Dashboard</h3>
            <p>Web-based administration interface</p>
            <a href="/admin/">→ Open Admin Dashboard</a>
        </div>

        <div class="service">
            <h3><span class="status"></span>REST API</h3>
            <p>RESTful API for chat operations</p>
            <a href="/api/v1/health">→ API Health Check</a>
        </div>

        <div class="service">
            <h3><span class="status"></span>API Documentation</h3>
            <p>Interactive API documentation</p>
            <a href="/docs">→ View API Docs</a>
        </div>

        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ecf0f1;">

        <p><strong>Default Admin Credentials:</strong></p>
        <ul>
            <li>Username: <code>admin</code></li>
            <li>Password: <code>AdminPassword123!</code></li>
        </ul>
    </div>
</body>
</html>
    "#,
    )
}

/// Redirect root to admin dashboard if available
async fn serve_root_redirect() -> Result<Redirect, Html<&'static str>> {
    let dashboard_path = std::path::Path::new("admin-dashboard/index.html");

    if dashboard_path.exists() {
        Ok(Redirect::permanent("/admin/"))
    } else {
        Err(serve_root_info().await)
    }
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
    info!("Admin dashboard available at http://{}/admin/", bind_addr);
    info!("Server info page available at http://{}/", bind_addr);

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
