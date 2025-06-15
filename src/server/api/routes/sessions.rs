//! Session management routes
//!
//! This module defines all session-related HTTP routes including
//! session listing, termination, and device management.

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::server::api::{handlers::sessions, ApiState};

/// Create session management routes
///
/// These routes handle session operations, device management,
/// and session security features. All routes require authentication.
pub fn create_session_routes() -> Router<ApiState> {
    Router::new()
        // Get current user's active sessions
        .route("/", get(sessions::get_sessions))
        // Get current session information
        .route("/current", get(sessions::get_current_session))
        // Update current session metadata
        .route("/current", put(sessions::update_current_session))
        // Terminate a specific session
        .route("/:session_id", delete(sessions::terminate_session))
        // Terminate all sessions except current
        .route("/terminate-all", post(sessions::terminate_all_sessions))
        // Get session statistics
        .route("/stats", get(sessions::get_session_statistics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_routes_creation() {
        // Test that session routes can be created without panicking
        let _router = create_session_routes();
    }
}
