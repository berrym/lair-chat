//! Room management handlers
//!
//! This module provides HTTP handlers for room-related operations including
//! room creation, membership management, and room discovery.

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
        rooms::*,
        PaginationParams,
    },
    ApiState,
};

/// Create a new room
///
/// Creates a new chat room with the specified configuration and adds
/// the creating user as the room owner.
#[utoipa::path(
    post,
    path = "/api/v1/rooms",
    request_body = CreateRoomRequest,
    responses(
        (status = 201, description = "Room created successfully", body = Room),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn create_room(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<CreateRoomRequest>,
) -> ApiResult<(StatusCode, Json<SuccessResponse<Room>>)> {
    info!("Room creation request from user: {}", user_context.username);

    // Validate request data
    validation::validate_request(&request)?;

    // Check if room name already exists
    if state
        .storage
        .rooms()
        .room_name_exists(&request.name)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to check room name: {}", e)))?
    {
        return Err(ApiError::validation_error("Room name already exists"));
    }

    // Create the room storage model
    let room_id = crate::server::storage::generate_id();
    let now = crate::server::storage::current_timestamp();

    let storage_room = crate::server::storage::Room {
        id: room_id.clone(),
        name: request.name.clone(),
        display_name: request.name.clone(),
        description: request.description.clone(),
        topic: None,
        room_type: match request.room_type.clone() {
            crate::server::api::models::rooms::RoomType::Channel => {
                crate::server::storage::RoomType::Channel
            }
            crate::server::api::models::rooms::RoomType::Group => {
                crate::server::storage::RoomType::Group
            }
            crate::server::api::models::rooms::RoomType::DirectMessage => {
                crate::server::storage::RoomType::DirectMessage
            }
            crate::server::api::models::rooms::RoomType::System => {
                crate::server::storage::RoomType::System
            }
            crate::server::api::models::rooms::RoomType::Temporary => {
                crate::server::storage::RoomType::Temporary
            }
        },
        privacy: match request.privacy.clone() {
            crate::server::api::models::rooms::PrivacyLevel::Public => {
                crate::server::storage::RoomPrivacy::Public
            }
            crate::server::api::models::rooms::PrivacyLevel::Private => {
                crate::server::storage::RoomPrivacy::Private
            }
            crate::server::api::models::rooms::PrivacyLevel::Protected => {
                crate::server::storage::RoomPrivacy::Protected
            }
            crate::server::api::models::rooms::PrivacyLevel::System => {
                crate::server::storage::RoomPrivacy::System
            }
        },
        settings: crate::server::storage::RoomSettings {
            max_users: request.max_members,
            ..Default::default()
        },
        created_by: user_context.user_id.to_string(),
        created_at: now,
        updated_at: now,
        is_active: true,
    };

    // Create room in storage
    let created_room = state
        .storage
        .rooms()
        .create_room(storage_room)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create room: {}", e)))?;

    // Add creator as room owner
    let membership = crate::server::storage::RoomMembership {
        id: crate::server::storage::generate_id(),
        room_id: created_room.id.clone(),
        user_id: user_context.user_id.to_string(),
        role: crate::server::storage::RoomRole::Owner,
        joined_at: now,
        last_activity: Some(now),
        is_active: true,
        settings: crate::server::storage::RoomMemberSettings::default(),
    };

    state
        .storage
        .rooms()
        .add_room_member(membership)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to add room owner: {}", e)))?;

    // Convert storage model back to API model
    let api_room = Room {
        id: Uuid::parse_str(&created_room.id)
            .map_err(|_| ApiError::internal_server_error("Invalid room ID"))?,
        name: created_room.name.clone(),
        description: created_room.description.clone(),
        room_type: request.room_type,
        privacy: request.privacy,
        owner_id: Uuid::parse_str(&created_room.created_by)
            .map_err(|_| ApiError::internal_server_error("Invalid owner ID"))?,
        max_members: created_room.settings.max_users,
        created_at: chrono::DateTime::from_timestamp(created_room.created_at as i64, 0)
            .unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(created_room.updated_at as i64, 0)
            .unwrap_or_default(),
        avatar_url: None,
        metadata: serde_json::json!({}),
    };

    info!(
        "Room created successfully: {} (ID: {})",
        created_room.name, created_room.id
    );
    Ok((StatusCode::CREATED, responses::success(api_room)))
}

/// Get room information
///
/// Retrieves detailed information about a specific room if the user
/// has permission to view it.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{room_id}",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    responses(
        (status = 200, description = "Room information retrieved", body = Room),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Access denied", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_room(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
) -> ApiResult<Json<SuccessResponse<Room>>> {
    debug!("Room info request for room: {}", room_id);

    let room_id_str = room_id.to_string();

    // Get room from storage
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    // Check if room is active
    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user has permission to view the room
    let user_id_str = user_context.user_id.to_string();

    // For private rooms, user must be a member
    if matches!(
        storage_room.privacy,
        crate::server::storage::RoomPrivacy::Private
    ) {
        let is_member = state
            .storage
            .rooms()
            .is_room_member(&room_id_str, &user_id_str)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to check membership: {}", e)))?;

        if !is_member {
            return Err(ApiError::forbidden_error(
                "You don't have permission to view this room",
            ));
        }
    }

    // Convert storage model to API model
    let api_room = Room {
        id: room_id,
        name: storage_room.name,
        description: storage_room.description,
        room_type: match storage_room.room_type {
            crate::server::storage::RoomType::Channel => {
                crate::server::api::models::rooms::RoomType::Channel
            }
            crate::server::storage::RoomType::Group => {
                crate::server::api::models::rooms::RoomType::Group
            }
            crate::server::storage::RoomType::DirectMessage => {
                crate::server::api::models::rooms::RoomType::DirectMessage
            }
            crate::server::storage::RoomType::System => {
                crate::server::api::models::rooms::RoomType::System
            }
            crate::server::storage::RoomType::Temporary => {
                crate::server::api::models::rooms::RoomType::Temporary
            }
        },
        privacy: match storage_room.privacy {
            crate::server::storage::RoomPrivacy::Public => {
                crate::server::api::models::rooms::PrivacyLevel::Public
            }
            crate::server::storage::RoomPrivacy::Private => {
                crate::server::api::models::rooms::PrivacyLevel::Private
            }
            crate::server::storage::RoomPrivacy::Protected => {
                crate::server::api::models::rooms::PrivacyLevel::Protected
            }
            crate::server::storage::RoomPrivacy::System => {
                crate::server::api::models::rooms::PrivacyLevel::System
            }
        },
        owner_id: Uuid::parse_str(&storage_room.created_by)
            .map_err(|_| ApiError::internal_error("Invalid owner ID"))?,
        max_members: storage_room.settings.max_users,
        created_at: chrono::DateTime::from_timestamp(storage_room.created_at as i64, 0)
            .unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(storage_room.updated_at as i64, 0)
            .unwrap_or_default(),
        avatar_url: None,                // TODO: Add avatar support
        metadata: serde_json::json!({}), // TODO: Convert metadata
    };

    debug!("Room info retrieved successfully: {}", storage_room.name);
    Ok(responses::success(api_room))
}

/// Update room information
///
/// Updates room settings such as name, description, and privacy level.
/// Only room owners and admins can update room information.
#[utoipa::path(
    put,
    path = "/api/v1/rooms/{room_id}",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    request_body = UpdateRoomRequest,
    responses(
        (status = 200, description = "Room updated successfully", body = Room),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_room(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
    Json(request): Json<UpdateRoomRequest>,
) -> ApiResult<Json<SuccessResponse<Room>>> {
    info!("Room update request for room: {}", room_id);

    // Validate request data
    validation::validate_request(&request)?;

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get existing room from storage
    let mut storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    // Check if room is active
    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user has permission to update the room
    let membership = state
        .storage
        .rooms()
        .get_room_membership(&room_id_str, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
        .ok_or_else(|| ApiError::forbidden_error("You are not a member of this room"))?;

    // Only owners and admins can update room settings
    if !matches!(
        membership.role,
        crate::server::storage::RoomRole::Owner | crate::server::storage::RoomRole::Admin
    ) {
        return Err(ApiError::forbidden_error(
            "You don't have permission to update this room",
        ));
    }

    // Update room fields
    if let Some(name) = &request.name {
        // Check if new name already exists (if it's different from current name)
        if name != &storage_room.name {
            if state
                .storage
                .rooms()
                .room_name_exists(name)
                .await
                .map_err(|e| {
                    ApiError::internal_error(&format!("Failed to check room name: {}", e))
                })?
            {
                return Err(ApiError::validation_error("Room name already exists"));
            }
        }
        storage_room.name = name.clone();
        storage_room.display_name = name.clone();
    }

    if let Some(description) = &request.description {
        storage_room.description = Some(description.clone());
    }

    if let Some(privacy) = &request.privacy {
        storage_room.privacy = match privacy {
            crate::server::api::models::rooms::PrivacyLevel::Public => {
                crate::server::storage::RoomPrivacy::Public
            }
            crate::server::api::models::rooms::PrivacyLevel::Private => {
                crate::server::storage::RoomPrivacy::Private
            }
            crate::server::api::models::rooms::PrivacyLevel::Protected => {
                crate::server::storage::RoomPrivacy::Protected
            }
            crate::server::api::models::rooms::PrivacyLevel::System => {
                crate::server::storage::RoomPrivacy::System
            }
        };
    }

    if let Some(max_members) = request.max_members {
        storage_room.settings.max_users = Some(max_members);
    }

    // Update timestamp
    storage_room.updated_at = crate::server::storage::current_timestamp();

    // Save updated room
    let updated_room = state
        .storage
        .rooms()
        .update_room(storage_room)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to update room: {}", e)))?;

    // Convert to API model
    let api_room = Room {
        id: room_id,
        name: updated_room.name.clone(),
        description: updated_room.description.clone(),
        room_type: match updated_room.room_type {
            crate::server::storage::RoomType::Channel => {
                crate::server::api::models::rooms::RoomType::Channel
            }
            crate::server::storage::RoomType::Group => {
                crate::server::api::models::rooms::RoomType::Group
            }
            crate::server::storage::RoomType::DirectMessage => {
                crate::server::api::models::rooms::RoomType::DirectMessage
            }
            crate::server::storage::RoomType::System => {
                crate::server::api::models::rooms::RoomType::System
            }
            crate::server::storage::RoomType::Temporary => {
                crate::server::api::models::rooms::RoomType::Temporary
            }
        },
        privacy: match updated_room.privacy {
            crate::server::storage::RoomPrivacy::Public => {
                crate::server::api::models::rooms::PrivacyLevel::Public
            }
            crate::server::storage::RoomPrivacy::Private => {
                crate::server::api::models::rooms::PrivacyLevel::Private
            }
            crate::server::storage::RoomPrivacy::Protected => {
                crate::server::api::models::rooms::PrivacyLevel::Protected
            }
            crate::server::storage::RoomPrivacy::System => {
                crate::server::api::models::rooms::PrivacyLevel::System
            }
        },
        owner_id: Uuid::parse_str(&updated_room.created_by)
            .map_err(|_| ApiError::internal_error("Invalid owner ID"))?,
        max_members: updated_room.settings.max_users,
        created_at: chrono::DateTime::from_timestamp(updated_room.created_at as i64, 0)
            .unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(updated_room.updated_at as i64, 0)
            .unwrap_or_default(),
        avatar_url: None,
        metadata: serde_json::json!({}),
    };

    info!(
        "Room updated successfully: {} (ID: {})",
        updated_room.name, updated_room.id
    );
    Ok(responses::success(api_room))
}

/// Join a room
///
/// Adds the authenticated user to the specified room if they have
/// permission to join and the room is not at capacity.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{room_id}/join",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    request_body = JoinRoomRequest,
    responses(
        (status = 200, description = "Successfully joined room", body = EmptyResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied or room full", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 409, description = "Already a member", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn join_room(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
    Json(request): Json<JoinRoomRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Join room request for room: {}", room_id);

    // Validate request data
    validation::validate_request(&request)?;

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get room from storage
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    // Check if room is active
    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user is already a member
    if state
        .storage
        .rooms()
        .is_room_member(&room_id_str, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
    {
        return Err(ApiError::validation_error(
            "You are already a member of this room",
        ));
    }

    // Check room capacity
    if let Some(max_users) = storage_room.settings.max_users {
        let member_count = state
            .storage
            .rooms()
            .count_room_members(&room_id_str)
            .await
            .map_err(|e| {
                ApiError::internal_server_error(&format!("Failed to count members: {}", e))
            })?;

        if member_count >= max_users as u64 {
            return Err(ApiError::forbidden_error("Room is at capacity"));
        }
    }

    // Check if room requires password for protected rooms
    if matches!(
        storage_room.privacy,
        crate::server::storage::RoomPrivacy::Protected
    ) {
        // TODO: Implement password verification when we have password hashing
        // For now, just check if password is provided
        if request.password.is_none() {
            return Err(ApiError::validation_error(
                "Password required for protected room",
            ));
        }
    }

    // Check privacy restrictions
    match storage_room.privacy {
        crate::server::storage::RoomPrivacy::Private => {
            return Err(ApiError::forbidden_error(
                "This is a private room. You need an invitation to join",
            ));
        }
        crate::server::storage::RoomPrivacy::System => {
            return Err(ApiError::forbidden_error(
                "System rooms cannot be joined directly",
            ));
        }
        _ => {}
    }

    // Create membership
    let now = crate::server::storage::current_timestamp();
    let membership = crate::server::storage::RoomMembership {
        id: crate::server::storage::generate_id(),
        room_id: room_id_str.clone(),
        user_id: user_id_str.clone(),
        role: crate::server::storage::RoomRole::Member,
        joined_at: now,
        last_activity: Some(now),
        is_active: true,
        settings: crate::server::storage::RoomMemberSettings::default(),
    };

    // Add user to room
    state
        .storage
        .rooms()
        .add_room_member(membership)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to join room: {}", e)))?;

    info!(
        "User {} joined room {} successfully",
        user_context.username, storage_room.name
    );
    Ok(responses::empty_success(
        "Successfully joined room".to_string(),
    ))
}

/// Leave a room
///
/// Removes the authenticated user from the specified room.
/// Room owners cannot leave unless they transfer ownership first.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{room_id}/leave",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    responses(
        (status = 200, description = "Successfully left room", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Cannot leave room (owner must transfer ownership)", body = ApiError),
        (status = 404, description = "Room not found or not a member", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn leave_room(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Leave room request for room: {}", room_id);

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get room from storage
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    // Check if user is a member
    let membership = state
        .storage
        .rooms()
        .get_room_membership(&room_id_str, &user_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to check membership: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found_error("You are not a member of this room"))?;

    // Check if user is the owner - owners cannot leave without transferring ownership
    if matches!(membership.role, crate::server::storage::RoomRole::Owner) {
        // Check if there are other admins who can take ownership
        let member_count = state
            .storage
            .rooms()
            .count_room_members(&room_id_str)
            .await
            .map_err(|e| {
                ApiError::internal_server_error(&format!("Failed to count members: {}", e))
            })?;

        if member_count > 1 {
            return Err(ApiError::forbidden_error(
                "Room owners must transfer ownership before leaving. Delete the room if you want to remove it entirely",
            ));
        }
        // If this is the only member and they're the owner, they can leave (room will become inactive)
    }

    // Remove user from room
    state
        .storage
        .rooms()
        .remove_room_member(&room_id_str, &user_id_str)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to leave room: {}", e)))?;

    // If the last member left, deactivate the room
    let remaining_members = state
        .storage
        .rooms()
        .count_room_members(&room_id_str)
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to count remaining members: {}", e))
        })?;

    if remaining_members == 0 {
        state
            .storage
            .rooms()
            .deactivate_room(&room_id_str)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to deactivate room: {}", e)))?;
        info!(
            "Room {} deactivated (no members remaining)",
            storage_room.name
        );
    }

    info!(
        "User {} left room {} successfully",
        user_context.username, storage_room.name
    );
    Ok(responses::empty_success(
        "Successfully left room".to_string(),
    ))
}

/// Get room members
///
/// Retrieves a list of all members in the specified room with their
/// roles and status information.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{room_id}/members",
    params(
        ("room_id" = Uuid, Path, description = "Room ID")
    ),
    responses(
        (status = 200, description = "Room members retrieved", body = Vec<RoomMember>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Access denied", body = ApiError),
        (status = 404, description = "Room not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_room_members(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(room_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<RoomMember>>>> {
    debug!("Room members request for room: {}", room_id);

    let room_id_str = room_id.to_string();
    let user_id_str = user_context.user_id.to_string();

    // Get room from storage
    let storage_room = state
        .storage
        .rooms()
        .get_room_by_id(&room_id_str)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to retrieve room: {}", e)))?
        .ok_or_else(|| ApiError::not_found_error("Room"))?;

    // Check if room is active
    if !storage_room.is_active {
        return Err(ApiError::not_found_error("Room"));
    }

    // Check if user has permission to view members
    match storage_room.privacy {
        crate::server::storage::RoomPrivacy::Private => {
            // For private rooms, user must be a member
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
                    "You don't have permission to view members of this room",
                ));
            }
        }
        _ => {} // Public and protected rooms allow viewing members
    }

    // Set up pagination
    let pagination = crate::server::storage::Pagination {
        offset: (params.page * params.page_size) as u64,
        limit: params.page_size.min(100) as u64, // Cap at 100
    };

    // Get room memberships
    let memberships = state
        .storage
        .rooms()
        .list_room_members(&room_id_str, pagination)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get room members: {}", e)))?;

    // Convert to API models
    let mut members = Vec::new();
    for membership in memberships {
        // Get user info
        if let Ok(Some(user)) = state
            .storage
            .users()
            .get_user_by_id(&membership.user_id)
            .await
        {
            let member = RoomMember {
                user_id: Uuid::parse_str(&membership.user_id)
                    .map_err(|_| ApiError::internal_error("Invalid user ID"))?,
                username: user.username,
                display_name: user
                    .profile
                    .display_name
                    .unwrap_or_else(|| user.username.clone()),
                avatar_url: user.profile.avatar,
                role: match membership.role {
                    crate::server::storage::RoomRole::Owner => {
                        crate::server::api::models::rooms::MemberRole::Owner
                    }
                    crate::server::storage::RoomRole::Admin => {
                        crate::server::api::models::rooms::MemberRole::Admin
                    }
                    crate::server::storage::RoomRole::Moderator => {
                        crate::server::api::models::rooms::MemberRole::Moderator
                    }
                    crate::server::storage::RoomRole::Member => {
                        crate::server::api::models::rooms::MemberRole::Member
                    }
                    crate::server::storage::RoomRole::Guest => {
                        crate::server::api::models::rooms::MemberRole::Guest
                    }
                },
                is_online: user.last_seen.map_or(false, |last_seen| {
                    let now = crate::server::storage::current_timestamp();
                    now - last_seen < 300 // Online if seen within last 5 minutes
                }),
                joined_at: chrono::DateTime::from_timestamp(membership.joined_at as i64, 0)
                    .unwrap_or_default(),
                last_activity: membership
                    .last_activity
                    .map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or_default()),
            };
            members.push(member);
        }
    }

    debug!("Retrieved {} members for room {}", members.len(), room_id);
    Ok(responses::success(members))
}

/// Search for rooms
///
/// Searches for rooms by name, description, or other criteria.
/// Only returns rooms that the user has permission to see.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/search",
    request_body = RoomSearchRequest,
    responses(
        (status = 200, description = "Room search completed", body = Vec<RoomSearchResult>),
        (status = 400, description = "Invalid search request", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "rooms",
    security(
        ("Bearer" = [])
    )
)]
pub async fn search_rooms(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<RoomSearchRequest>,
) -> ApiResult<Json<SuccessResponse<Vec<RoomSearchResult>>>> {
    debug!("Room search request: {}", request.query);

    // Validate request data
    validation::validate_request(&request)?;

    let user_id_str = user_context.user_id.to_string();

    // Set up pagination
    let pagination = crate::server::storage::Pagination {
        offset: 0,
        limit: request.limit.unwrap_or(20).min(50) as u64,
    };

    // Search rooms by name/description
    let rooms = state
        .storage
        .rooms()
        .search_rooms(&request.query, pagination)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to search rooms: {}", e)))?;

    // Filter and convert to search results
    let mut results = Vec::new();
    for room in rooms {
        // Skip inactive rooms
        if !room.is_active {
            continue;
        }

        // Filter by room type if specified
        if let Some(ref room_type_filter) = request.room_type {
            let matches_type = match (&room.room_type, room_type_filter) {
                (
                    crate::server::storage::RoomType::Channel,
                    crate::server::api::models::rooms::RoomType::Channel,
                ) => true,
                (
                    crate::server::storage::RoomType::Group,
                    crate::server::api::models::rooms::RoomType::Group,
                ) => true,
                (
                    crate::server::storage::RoomType::DirectMessage,
                    crate::server::api::models::rooms::RoomType::DirectMessage,
                ) => true,
                (
                    crate::server::storage::RoomType::System,
                    crate::server::api::models::rooms::RoomType::System,
                ) => true,
                (
                    crate::server::storage::RoomType::Temporary,
                    crate::server::api::models::rooms::RoomType::Temporary,
                ) => true,
                _ => false,
            };
            if !matches_type {
                continue;
            }
        }

        // Filter by privacy level if specified
        if let Some(ref privacy_filter) = request.privacy {
            let matches_privacy = match (&room.privacy, privacy_filter) {
                (
                    crate::server::storage::RoomPrivacy::Public,
                    crate::server::api::models::rooms::PrivacyLevel::Public,
                ) => true,
                (
                    crate::server::storage::RoomPrivacy::Private,
                    crate::server::api::models::rooms::PrivacyLevel::Private,
                ) => true,
                (
                    crate::server::storage::RoomPrivacy::Protected,
                    crate::server::api::models::rooms::PrivacyLevel::Protected,
                ) => true,
                (
                    crate::server::storage::RoomPrivacy::System,
                    crate::server::api::models::rooms::PrivacyLevel::System,
                ) => true,
                _ => false,
            };
            if !matches_privacy {
                continue;
            }
        }

        // Only include rooms the user can see
        let can_see_room = match room.privacy {
            crate::server::storage::RoomPrivacy::Public => true,
            crate::server::storage::RoomPrivacy::Protected => true,
            crate::server::storage::RoomPrivacy::Private => {
                // Check if user is a member
                state
                    .storage
                    .rooms()
                    .is_room_member(&room.id, &user_id_str)
                    .await
                    .unwrap_or(false)
            }
            crate::server::storage::RoomPrivacy::System => {
                // Only show system rooms to admins/moderators
                if let Ok(Some(user)) = state.storage.users().get_user_by_id(&user_id_str).await {
                    user.role.is_moderator()
                } else {
                    false
                }
            }
        };

        if !can_see_room {
            continue;
        }

        // Check if user is a member
        let is_member = state
            .storage
            .rooms()
            .is_room_member(&room.id, &user_id_str)
            .await
            .unwrap_or(false);

        // Get member count
        let member_count = state
            .storage
            .rooms()
            .count_room_members(&room.id)
            .await
            .unwrap_or(0) as u32;

        let search_result = RoomSearchResult {
            id: Uuid::parse_str(&room.id)
                .map_err(|_| ApiError::internal_error("Invalid room ID"))?,
            name: room.name,
            description: room.description,
            room_type: match room.room_type {
                crate::server::storage::RoomType::Channel => {
                    crate::server::api::models::rooms::RoomType::Channel
                }
                crate::server::storage::RoomType::Group => {
                    crate::server::api::models::rooms::RoomType::Group
                }
                crate::server::storage::RoomType::DirectMessage => {
                    crate::server::api::models::rooms::RoomType::DirectMessage
                }
                crate::server::storage::RoomType::System => {
                    crate::server::api::models::rooms::RoomType::System
                }
                crate::server::storage::RoomType::Temporary => {
                    crate::server::api::models::rooms::RoomType::Temporary
                }
            },
            privacy: match room.privacy {
                crate::server::storage::RoomPrivacy::Public => {
                    crate::server::api::models::rooms::PrivacyLevel::Public
                }
                crate::server::storage::RoomPrivacy::Private => {
                    crate::server::api::models::rooms::PrivacyLevel::Private
                }
                crate::server::storage::RoomPrivacy::Protected => {
                    crate::server::api::models::rooms::PrivacyLevel::Protected
                }
                crate::server::storage::RoomPrivacy::System => {
                    crate::server::api::models::rooms::PrivacyLevel::System
                }
            },
            member_count,
            max_members: room.settings.max_users,
            avatar_url: None, // TODO: Add avatar support
            is_member,
            requires_password: matches!(
                room.privacy,
                crate::server::storage::RoomPrivacy::Protected
            ),
        };

        results.push(search_result);
    }

    debug!("Found {} rooms matching search criteria", results.len());
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
