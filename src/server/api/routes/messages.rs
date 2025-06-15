//! Message handling routes
//!
//! This module defines all message-related HTTP routes including
//! sending messages, editing, reactions, and message search functionality.

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::server::api::{handlers::messages, ApiState};

/// Create message handling routes
///
/// These routes handle message operations, reactions, and search.
/// All routes require authentication.
pub fn create_message_routes() -> Router<ApiState> {
    Router::new()
        // Send a message to a room
        .route("/rooms/:room_id/messages", post(messages::send_message))
        // Get messages from a room
        .route("/rooms/:room_id/messages", get(messages::get_messages))
        // Edit a specific message
        .route("/:message_id", put(messages::edit_message))
        // Delete a specific message
        .route("/:message_id", delete(messages::delete_message))
        // Add reaction to a message
        .route("/:message_id/reactions", post(messages::add_reaction))
        // Remove reaction from a message
        .route(
            "/:message_id/reactions/:emoji",
            delete(messages::remove_reaction),
        )
        // Search messages
        .route("/search", post(messages::search_messages))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_routes_creation() {
        // Test that message routes can be created without panicking
        let _router = create_message_routes();
    }
}
