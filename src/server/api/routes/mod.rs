//! API Routes Module
//!
//! This module organizes all HTTP routes for the REST API, providing
//! a structured way to define endpoint groups and their corresponding
//! handlers with appropriate middleware.

use axum::{middleware, routing::get, Router};

use crate::server::api::{middleware::jwt_auth_middleware, ApiState};

pub mod admin;
pub mod auth;
pub mod messages;
pub mod rooms;
pub mod sessions;
pub mod users;

// Re-export route creation functions
pub use admin::create_admin_routes;
pub use auth::create_auth_routes;
pub use messages::create_message_routes;
pub use rooms::create_room_routes;
pub use sessions::create_session_routes;
pub use users::create_user_routes;

/// Create the main API router with all route groups
pub fn create_api_routes() -> Router<ApiState> {
    Router::new()
        // Health check endpoint (no auth required)
        .route("/health", get(crate::server::api::handlers::health_check))
        // Authentication routes (no auth required)
        .nest("/auth", create_auth_routes())
        // User management routes (auth required)
        .nest(
            "/users",
            create_user_routes().layer(middleware::from_fn_with_state(
                (), // ApiState will be passed by the parent router
                jwt_auth_middleware,
            )),
        )
        // Room management routes (auth required)
        .nest(
            "/rooms",
            create_room_routes().layer(middleware::from_fn_with_state((), jwt_auth_middleware)),
        )
        // Message routes (auth required)
        .nest(
            "/messages",
            create_message_routes().layer(middleware::from_fn_with_state((), jwt_auth_middleware)),
        )
        // Session management routes (auth required)
        .nest(
            "/sessions",
            create_session_routes().layer(middleware::from_fn_with_state((), jwt_auth_middleware)),
        )
        // Admin routes (admin auth required)
        .nest(
            "/admin",
            create_admin_routes().layer(middleware::from_fn_with_state((), jwt_auth_middleware)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_routes_creation() {
        // Test that routes can be created without panicking
        let _router = create_api_routes();
    }
}
