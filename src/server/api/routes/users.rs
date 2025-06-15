//! User management routes
//!
//! This module defines all user-related HTTP routes including
//! profile management, settings, user search, and account operations.

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::server::api::{handlers::users, ApiState};

/// Create user management routes
///
/// These routes handle user profile operations, settings management,
/// and user discovery features. All routes require authentication.
pub fn create_user_routes() -> Router<ApiState> {
    Router::new()
        // Get current user profile
        .route("/profile", get(users::get_profile))
        // Update current user profile
        .route("/profile", put(users::update_profile))
        // Get current user settings
        .route("/settings", get(users::get_settings))
        // Update current user settings
        .route("/settings", put(users::update_settings))
        // Search for users
        .route("/search", post(users::search_users))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_routes_creation() {
        // Test that user routes can be created without panicking
        let _router = create_user_routes();
    }
}
