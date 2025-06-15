//! Authentication routes
//!
//! This module defines all authentication-related HTTP routes including
//! user registration, login, logout, token refresh, and password management.

use axum::{
    routing::{get, post},
    Router,
};

use crate::server::api::{handlers::auth, ApiState};

/// Create authentication routes
///
/// These routes handle user authentication and account management.
/// Most routes don't require authentication since they are used to
/// establish authentication in the first place.
pub fn create_auth_routes() -> Router<ApiState> {
    Router::new()
        // User registration
        .route("/register", post(auth::register))
        // User login
        .route("/login", post(auth::login))
        // Token refresh
        .route("/refresh", post(auth::refresh))
        // User logout (requires auth)
        .route("/logout", post(auth::logout))
        // Change password (requires auth)
        .route("/change-password", post(auth::change_password))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_routes_creation() {
        // Test that auth routes can be created without panicking
        let _router = create_auth_routes();
    }
}
