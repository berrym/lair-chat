//! Room management routes
//!
//! This module defines all room-related HTTP routes including
//! room creation, membership management, and room discovery.

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::server::api::{handlers::rooms, ApiState};

/// Create room management routes
///
/// These routes handle room operations, membership management,
/// and room discovery features. All routes require authentication.
pub fn create_room_routes() -> Router<ApiState> {
    Router::new()
        // Create a new room
        .route("/", post(rooms::create_room))
        // Search for rooms
        .route("/search", post(rooms::search_rooms))
        // Get room information
        .route("/:room_id", get(rooms::get_room))
        // Update room information
        .route("/:room_id", put(rooms::update_room))
        // Join a room
        .route("/:room_id/join", post(rooms::join_room))
        // Leave a room
        .route("/:room_id/leave", post(rooms::leave_room))
        // Get room members
        .route("/:room_id/members", get(rooms::get_room_members))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_routes_creation() {
        // Test that room routes can be created without panicking
        let _router = create_room_routes();
    }
}
