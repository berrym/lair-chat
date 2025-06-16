//! Message handling API endpoints
//!
//! This module provides HTTP handlers for message-related operations including
//! sending messages, editing, reactions, and message search functionality.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::server::api::{
    handlers::{responses, validation},
    middleware::UserContext,
    models::{
        common::{ApiError, ApiResult, EmptyResponse, SuccessResponse},
        messages::*,
        PaginationParams,
    },
    ApiState,
};

/// Send a message to a room
///
/// Sends a new message to the specified room. The user must be a member
/// of the room to send messages.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message sent successfully", body = Message),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn send_message(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
    Json(request): Json<SendMessageRequest>,
) -> ApiResult<(StatusCode, Json<SuccessResponse<Message>>)> {
    info!("Send message request to room: {}", room_id);

    // Validate request data
    validation::validate_request(&request)?;

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Verify room exists and is active
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user is a member of the room
    let membership = state
        .storage
        .rooms()
        .get_room_membership(&room_id_str, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
        .ok_or_else(|| ApiError::forbidden_error("You are not a member of this room"))?;

    // Check if user has permission to send messages
    if matches!(membership.role, crate::server::storage::RoomRole::Guest)
        && storage_room.settings.moderation.require_approval
    {
        return Err(ApiError::forbidden_error(
            "Guests cannot send messages in this room",
        ));
    }

    // Create the message storage model
    let message_id = crate::server::storage::generate_id();
    let now = crate::server::storage::current_timestamp();

    let storage_message = crate::server::storage::Message {
        id: message_id.clone(),
        room_id: room_id_str.clone(),
        user_id: user_id_str.clone(),
        content: request.content.clone(),
        message_type: match request.message_type.clone() {
            crate::server::api::models::messages::MessageType::Text => {
                crate::server::storage::MessageType::Text
            }
            crate::server::api::models::messages::MessageType::Image => {
                crate::server::storage::MessageType::Image
            }
            crate::server::api::models::messages::MessageType::File => {
                crate::server::storage::MessageType::File
            }
            crate::server::api::models::messages::MessageType::System => {
                crate::server::storage::MessageType::System
            }
            crate::server::api::models::messages::MessageType::Audio => {
                crate::server::storage::MessageType::Voice
            }
            crate::server::api::models::messages::MessageType::Video => {
                crate::server::storage::MessageType::Video
            }
        },
        timestamp: now,
        edited_at: None,
        parent_message_id: request.parent_id.map(|id| id.to_string()),
        metadata: crate::server::storage::MessageMetadata::default(),
        is_deleted: false,
        deleted_at: None,
    };

    // Store message
    let created_message = state
        .storage
        .messages()
        .store_message(storage_message)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to send message: {}", e)))?;

    // Convert back to API model
    let api_message = Message {
        id: Uuid::parse_str(&created_message.id)
            .map_err(|_| ApiError::internal_error("Invalid message ID"))?,
        room_id,
        user_id: user_context.user_id,
        content: created_message.content,
        message_type: request.message_type,
        parent_id: request.parent_id,
        thread_id: request.thread_id,
        created_at: chrono::DateTime::from_timestamp(created_message.timestamp as i64, 0)
            .unwrap_or_default(),
        updated_at: None,
        is_edited: false,
        is_deleted: false,
        metadata: serde_json::json!({}),
    };

    info!("Message sent successfully to room: {}", room_id);
    Ok((StatusCode::CREATED, responses::success(api_message)))
}

/// Get messages from a room
///
/// Retrieves messages from the specified room with pagination support.
/// Only members of the room can access its messages.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    responses(
        (status = 200, description = "Messages retrieved successfully", body = Vec<Message>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Access denied", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_messages(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<Message>>>> {
    debug!("Get messages request for room: {}", room_id);

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Verify room exists and is active
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user is a member of the room
    if !state
        .storage
        .rooms()
        .is_room_member(&room_id_str, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
    {
        return Err(ApiError::forbidden_error(
            "You are not a member of this room",
        ));
    }

    // Set up pagination
    let pagination = crate::server::storage::Pagination {
        offset: (params.page * params.page_size) as u64,
        limit: params.page_size.min(100) as u64,
    };

    // Get messages from storage
    let storage_messages = state
        .storage
        .messages()
        .get_room_messages(&room_id_str, pagination, None)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to retrieve messages: {}", e)))?;

    // Convert to API models
    let mut messages = Vec::new();
    for msg in storage_messages {
        if !msg.is_deleted {
            let api_message = Message {
                id: Uuid::parse_str(&msg.id)
                    .map_err(|_| ApiError::internal_server_error("Invalid message ID"))?,
                room_id,
                user_id: Uuid::parse_str(&msg.user_id)
                    .map_err(|_| ApiError::internal_server_error("Invalid user ID"))?,
                content: msg.content,
                message_type: match msg.message_type {
                    crate::server::storage::MessageType::Text => {
                        crate::server::api::models::messages::MessageType::Text
                    }
                    crate::server::storage::MessageType::Image => {
                        crate::server::api::models::messages::MessageType::Image
                    }
                    crate::server::storage::MessageType::File => {
                        crate::server::api::models::messages::MessageType::File
                    }
                    crate::server::storage::MessageType::System => {
                        crate::server::api::models::messages::MessageType::System
                    }
                    crate::server::storage::MessageType::Voice => {
                        crate::server::api::models::messages::MessageType::Audio
                    }
                    crate::server::storage::MessageType::Video => {
                        crate::server::api::models::messages::MessageType::Video
                    }
                    _ => crate::server::api::models::messages::MessageType::Text,
                },
                parent_id: msg
                    .parent_message_id
                    .and_then(|id| Uuid::parse_str(&id).ok()),
                thread_id: None, // TODO: Implement thread support
                created_at: chrono::DateTime::from_timestamp(msg.timestamp as i64, 0)
                    .unwrap_or_default(),
                updated_at: msg
                    .edited_at
                    .map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or_default()),
                is_edited: msg.edited_at.is_some(),
                is_deleted: false,
                metadata: serde_json::json!({}),
            };
            messages.push(api_message);
        }
    }

    debug!(
        "Retrieved {} messages for room: {}",
        messages.len(),
        room_id
    );
    Ok(responses::success(messages))
}

/// Edit a message
///
/// Updates the content of an existing message. Only the message author
/// can edit their own messages, and only within a certain time window.
#[utoipa::path(
    put,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    request_body = EditMessageRequest,
    responses(
        (status = 200, description = "Message edited successfully", body = Message),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied or edit window expired", body = ApiError),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn edit_message(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(message_id): Path<Uuid>,
    Json(request): Json<EditMessageRequest>,
) -> ApiResult<Json<SuccessResponse<Message>>> {
    info!("Edit message request for message: {}", message_id);

    // Validate request data
    validation::validate_request(&request)?;

    let message_id_str = message_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get existing message
    let mut storage_message = state
        .storage
        .messages()
        .get_message_by_id(&message_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to retrieve message: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found_error("Message"))?;

    // Check if message is deleted
    if storage_message.is_deleted {
        return Err(ApiError::not_found_error("Message"));
    }

    // Check if user owns the message
    if storage_message.user_id != user_id_str {
        return Err(ApiError::forbidden_error(
            "You can only edit your own messages",
        ));
    }

    // Check if message is too old to edit (15 minutes limit)
    let now = crate::server::storage::current_timestamp();
    let edit_window = 15 * 60; // 15 minutes in seconds
    if now - storage_message.timestamp > edit_window {
        return Err(ApiError::forbidden_error("Message edit window has expired"));
    }

    // Update message content
    storage_message.content = request.content.clone();
    storage_message.edited_at = Some(now);

    // Save updated message
    let updated_message = state
        .storage
        .messages()
        .update_message(storage_message)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to update message: {}", e)))?;

    // Convert to API model
    let api_message = Message {
        id: message_id,
        room_id: Uuid::parse_str(&updated_message.room_id)
            .map_err(|_| ApiError::internal_error("Invalid room ID"))?,
        user_id: user_context.user_id,
        content: updated_message.content,
        message_type: match updated_message.message_type {
            crate::server::storage::MessageType::Text => {
                crate::server::api::models::messages::MessageType::Text
            }
            crate::server::storage::MessageType::Image => {
                crate::server::api::models::messages::MessageType::Image
            }
            crate::server::storage::MessageType::File => {
                crate::server::api::models::messages::MessageType::File
            }
            crate::server::storage::MessageType::System => {
                crate::server::api::models::messages::MessageType::System
            }
            crate::server::storage::MessageType::Voice => {
                crate::server::api::models::messages::MessageType::Audio
            }
            crate::server::storage::MessageType::Video => {
                crate::server::api::models::messages::MessageType::Video
            }
            _ => crate::server::api::models::messages::MessageType::Text,
        },
        parent_id: updated_message
            .parent_message_id
            .and_then(|id| Uuid::parse_str(&id).ok()),
        thread_id: None,
        created_at: chrono::DateTime::from_timestamp(updated_message.timestamp as i64, 0)
            .unwrap_or_default(),
        updated_at: updated_message
            .edited_at
            .map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or_default()),
        is_edited: true,
        is_deleted: false,
        metadata: serde_json::json!({}),
    };

    info!("Message edited successfully: {}", message_id);
    Ok(responses::success(api_message))
}

/// Delete a message
///
/// Marks a message as deleted. Only the message author or room moderators
/// can delete messages.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 200, description = "Message deleted successfully", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied", body = ApiError),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn delete_message(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(message_id): Path<Uuid>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Delete message request for message: {}", message_id);

    let message_id_str = message_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get existing message
    let storage_message = state
        .storage
        .messages()
        .get_message_by_id(&message_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to retrieve message: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found_error("Message"))?;

    // Check if message is already deleted
    if storage_message.is_deleted {
        return Err(ApiError::not_found_error("Message"));
    }

    // Check permissions - user owns message OR user is moderator/admin in the room
    let can_delete = if storage_message.user_id == user_id_str {
        true
    } else {
        // Check if user has moderation privileges in the room
        let membership = state
            .storage
            .rooms()
            .get_room_membership(&storage_message.room_id, &user_id_str)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to check membership: {}", e)))?;

        match membership {
            Some(m) => matches!(
                m.role,
                crate::server::storage::RoomRole::Owner
                    | crate::server::storage::RoomRole::Admin
                    | crate::server::storage::RoomRole::Moderator
            ),
            None => false,
        }
    };

    if !can_delete {
        return Err(ApiError::forbidden_error(
            "You don't have permission to delete this message",
        ));
    }

    // Soft delete the message
    let now = crate::server::storage::current_timestamp();
    state
        .storage
        .messages()
        .delete_message(&message_id_str, now)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete message: {}", e)))?;

    info!("Message deleted successfully: {}", message_id);
    Ok(responses::empty_success(
        "Message deleted successfully".to_string(),
    ))
}

/// Add reaction to a message
///
/// Adds an emoji reaction to the specified message. Users can only
/// have one reaction per emoji per message.
#[utoipa::path(
    post,
    path = "/api/v1/messages/{message_id}/reactions",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    request_body = AddReactionRequest,
    responses(
        (status = 201, description = "Reaction added successfully", body = MessageReaction),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Access denied", body = ApiError),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 409, description = "Reaction already exists", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn add_reaction(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(message_id): Path<Uuid>,
    Json(request): Json<AddReactionRequest>,
) -> ApiResult<(StatusCode, Json<SuccessResponse<MessageReaction>>)> {
    info!("Add reaction request for message: {}", message_id);

    // Validate request data
    validation::validate_request(&request)?;

    let message_id_str = message_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get existing message
    let storage_message = state
        .storage
        .messages()
        .get_message_by_id(&message_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to retrieve message: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found_error("Message"))?;

    // Check if message is deleted
    if storage_message.is_deleted {
        return Err(ApiError::not_found_error("Message"));
    }

    // Check if user is a member of the room where the message was sent
    if !state
        .storage
        .rooms()
        .is_room_member(&storage_message.room_id, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
    {
        return Err(ApiError::forbidden_error(
            "You are not a member of this room",
        ));
    }

    // Create reaction
    let now = crate::server::storage::current_timestamp();
    let storage_reaction = crate::server::storage::MessageReaction {
        user_id: user_id_str.clone(),
        reaction: request.emoji.clone(),
        timestamp: now,
    };

    // Add reaction to message
    state
        .storage
        .messages()
        .add_reaction(&message_id_str, storage_reaction)
        .await
        .map_err(|e| match e {
            crate::server::storage::StorageError::DuplicateError { .. } => {
                ApiError::validation_error("You have already reacted with this emoji")
            }
            _ => ApiError::internal_error(&format!("Failed to add reaction: {}", e)),
        })?;

    let reaction = MessageReaction {
        message_id,
        user_id: user_context.user_id,
        emoji: request.emoji,
        created_at: chrono::DateTime::from_timestamp(now as i64, 0).unwrap_or_default(),
    };

    info!("Reaction added successfully to message: {}", message_id);
    Ok((StatusCode::CREATED, responses::success(reaction)))
}

/// Remove reaction from a message
///
/// Removes the user's reaction with the specified emoji from the message.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/{message_id}/reactions/{emoji}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID"),
        ("emoji" = String, Path, description = "Emoji reaction to remove")
    ),
    responses(
        (status = 200, description = "Reaction removed successfully", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Message or reaction not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn remove_reaction(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path((message_id, emoji)): Path<(Uuid, String)>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Remove reaction request for message: {}", message_id);

    let message_id_str = message_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get existing message
    let storage_message = state
        .storage
        .messages()
        .get_message_by_id(&message_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to retrieve message: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found_error("Message"))?;

    // Check if message is deleted
    if storage_message.is_deleted {
        return Err(ApiError::not_found_error("Message"));
    }

    // Check if user is a member of the room where the message was sent
    if !state
        .storage
        .rooms()
        .is_room_member(&storage_message.room_id, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
    {
        return Err(ApiError::forbidden_error(
            "You are not a member of this room",
        ));
    }

    // Remove reaction
    state
        .storage
        .messages()
        .remove_reaction(&message_id_str, &user_id_str, &emoji)
        .await
        .map_err(|e| match e {
            crate::server::storage::StorageError::NotFound { .. } => {
                ApiError::not_found_error("Reaction")
            }
            _ => ApiError::internal_error(&format!("Failed to remove reaction: {}", e)),
        })?;

    info!("Reaction removed successfully from message: {}", message_id);
    Ok(responses::empty_success(
        "Reaction removed successfully".to_string(),
    ))
}

/// Search messages
///
/// Performs full-text search across messages the user has access to.
/// Supports various filters and sorting options.
#[utoipa::path(
    post,
    path = "/api/v1/messages/search",
    request_body = MessageSearchRequest,
    responses(
        (status = 200, description = "Message search completed", body = Vec<Message>),
        (status = 400, description = "Invalid search request", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "messages",
    security(
        ("Bearer" = [])
    )
)]
pub async fn search_messages(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<MessageSearchRequest>,
) -> ApiResult<Json<SuccessResponse<Vec<Message>>>> {
    debug!("Message search request: {}", request.query);

    // Validate request data
    validation::validate_request(&request)?;

    let user_id_str = user_context.user_id.to_string();

    // Create search query
    let search_query = crate::server::storage::SearchQuery {
        query: request.query.clone(),
        room_id: request.room_id.map(|id| id.to_string()),
        user_id: request.user_id.map(|id| id.to_string()),
        message_type: request.message_type.as_ref().map(|mt| match mt {
            crate::server::api::models::messages::MessageType::Text => {
                crate::server::storage::MessageType::Text
            }
            crate::server::api::models::messages::MessageType::Image => {
                crate::server::storage::MessageType::Image
            }
            crate::server::api::models::messages::MessageType::File => {
                crate::server::storage::MessageType::File
            }
            crate::server::api::models::messages::MessageType::System => {
                crate::server::storage::MessageType::System
            }
            crate::server::api::models::messages::MessageType::Audio => {
                crate::server::storage::MessageType::Voice
            }
            crate::server::api::models::messages::MessageType::Video => {
                crate::server::storage::MessageType::Video
            }
        }),
        date_from: request.date_from.map(|dt| dt.timestamp() as u64),
        date_to: request.date_to.map(|dt| dt.timestamp() as u64),
        limit: Some(request.limit.unwrap_or(20).min(100) as u64),
        offset: Some(0),
    };

    // Perform search
    let search_results = state
        .storage
        .messages()
        .search_messages(search_query)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to search messages: {}", e)))?;

    // Filter results to only include messages from rooms the user is a member of
    let mut results = Vec::new();
    for msg in search_results.messages {
        // Check if user has access to the room
        if state
            .storage
            .rooms()
            .is_room_member(&msg.room_id, &user_id_str)
            .await
            .unwrap_or(false)
        {
            // Skip deleted messages
            if !msg.is_deleted {
                let api_message = Message {
                    id: Uuid::parse_str(&msg.id)
                        .map_err(|_| ApiError::internal_error("Invalid message ID"))?,
                    room_id: Uuid::parse_str(&msg.room_id)
                        .map_err(|_| ApiError::internal_error("Invalid room ID"))?,
                    user_id: Uuid::parse_str(&msg.user_id)
                        .map_err(|_| ApiError::internal_error("Invalid user ID"))?,
                    content: msg.content,
                    message_type: match msg.message_type {
                        crate::server::storage::MessageType::Text => {
                            crate::server::api::models::messages::MessageType::Text
                        }
                        crate::server::storage::MessageType::Image => {
                            crate::server::api::models::messages::MessageType::Image
                        }
                        crate::server::storage::MessageType::File => {
                            crate::server::api::models::messages::MessageType::File
                        }
                        crate::server::storage::MessageType::System => {
                            crate::server::api::models::messages::MessageType::System
                        }
                        crate::server::storage::MessageType::Voice => {
                            crate::server::api::models::messages::MessageType::Audio
                        }
                        crate::server::storage::MessageType::Video => {
                            crate::server::api::models::messages::MessageType::Video
                        }
                        _ => crate::server::api::models::messages::MessageType::Text,
                    },
                    parent_id: msg
                        .parent_message_id
                        .and_then(|id| Uuid::parse_str(&id).ok()),
                    thread_id: None,
                    created_at: chrono::DateTime::from_timestamp(msg.timestamp as i64, 0)
                        .unwrap_or_default(),
                    updated_at: msg.edited_at.map(|ts| {
                        chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or_default()
                    }),
                    is_edited: msg.edited_at.is_some(),
                    is_deleted: false,
                    metadata: serde_json::json!({}),
                };
                results.push(api_message);
            }
        }
    }

    debug!("Found {} messages matching search criteria", results.len());
    Ok(responses::success(results))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test to satisfy module structure
        assert!(true);
    }
}
