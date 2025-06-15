//! Admin routes
//!
//! This module defines all administrative HTTP routes including
//! server management, user administration, system monitoring, and configuration.

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};

use crate::server::api::{handlers::admin, middleware::admin_auth_middleware, ApiState};

/// Create admin routes
///
/// These routes handle administrative operations including user management,
/// system monitoring, and server configuration. All routes require admin privileges.
pub fn create_admin_routes() -> Router<ApiState> {
    Router::new()
        // Server statistics
        .route("/stats", get(admin::get_server_statistics))
        // System health monitoring
        .route("/health", get(admin::get_system_health))
        // User management
        .route("/users", get(admin::get_admin_users))
        .route("/users/:user_id/status", put(admin::update_user_status))
        .route("/users/:user_id/role", put(admin::update_user_role))
        // Room management
        .route("/rooms", get(admin::get_admin_rooms))
        // Server configuration
        .route("/config", put(admin::update_server_config))
        // System maintenance
        .route("/maintenance", post(admin::perform_maintenance))
        // Apply admin authorization middleware to all routes
        .layer(middleware::from_fn(admin_auth_middleware))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_routes_creation() {
        // Test that admin routes can be created without panicking
        let _router = create_admin_routes();
    }
}
