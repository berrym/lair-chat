//! Room members handlers.
//!
//! Endpoints for managing room membership: listing members, changing roles, kicking members.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::middleware::AuthUser;
use crate::adapters::http::routes::AppState;
use crate::domain::events::{Event, EventPayload, MemberRoleChangedEvent, UserLeftRoomEvent};
use crate::domain::{RoomId, RoomMember, RoomRole, UserId};
use crate::storage::{MembershipRepository, RoomRepository, Storage, UserRepository};
use crate::Error;

use super::SuccessResponse;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Serialize)]
pub struct MembersListResponse {
    pub members: Vec<RoomMember>,
    pub total: u32,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

#[derive(Serialize)]
pub struct MemberResponse {
    pub member: RoomMember,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get room members with enriched data.
///
/// Requires the caller to be a member of the room.
pub async fn get_room_members<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<MembersListResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    // Verify room exists
    let _ = RoomRepository::find_by_id(state.engine.storage_clone().as_ref(), room_id)
        .await?
        .ok_or(Error::RoomNotFound)?;

    // Get the current user
    let (_, user) = state.engine.validate_session(auth.session_id).await?;

    // Check caller is a member
    let is_member =
        MembershipRepository::is_member(state.engine.storage_clone().as_ref(), room_id, user.id)
            .await?;
    if !is_member {
        return Err(Error::NotRoomMember);
    }

    // Get members with user details
    let members_with_users = MembershipRepository::list_members_with_users(
        state.engine.storage_clone().as_ref(),
        room_id,
    )
    .await?;

    // Get online user IDs
    let online_users = state.engine.online_user_ids().await;

    // Convert to RoomMember
    let members: Vec<RoomMember> = members_with_users
        .into_iter()
        .map(|(u, m)| RoomMember {
            user_id: u.id,
            username: u.username.to_string(),
            role: m.role,
            joined_at: m.joined_at,
            is_online: online_users.contains(&u.id),
        })
        .collect();

    let total = members.len() as u32;

    Ok(Json(MembersListResponse { members, total }))
}

/// Change a member's role.
///
/// Only room owners can change roles.
pub async fn update_member_role<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path((room_id, user_id)): Path<(String, String)>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<MemberResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;
    let target_user_id = UserId::parse(&user_id).map_err(|_| Error::UserNotFound)?;

    // Get the current user
    let (_, caller) = state.engine.validate_session(auth.session_id).await?;

    // Check caller's permissions (must be owner)
    let caller_membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        caller.id,
    )
    .await?
    .ok_or(Error::NotRoomMember)?;

    if !caller_membership.is_owner() {
        return Err(Error::PermissionDenied);
    }

    // Verify target is a member and get their current role
    let target_membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        target_user_id,
    )
    .await?
    .ok_or(Error::NotRoomMember)?;

    let old_role = target_membership.role;

    // Parse the new role
    let new_role = RoomRole::parse(&req.role);

    // Cannot change owner to something else if they're the only owner
    if target_membership.is_owner() && !new_role.is_owner() {
        let members =
            MembershipRepository::list_members(state.engine.storage_clone().as_ref(), room_id)
                .await?;
        let owner_count = members.iter().filter(|m| m.is_owner()).count();
        if owner_count <= 1 {
            return Err(Error::LastOwner);
        }
    }

    // Get user details (before update for event)
    let target_user =
        UserRepository::find_by_id(state.engine.storage_clone().as_ref(), target_user_id)
            .await?
            .ok_or(Error::UserNotFound)?;

    // Update the role
    MembershipRepository::update_role(
        state.engine.storage_clone().as_ref(),
        room_id,
        target_user_id,
        new_role,
    )
    .await?;

    // Emit role change event
    let event = Event::new(EventPayload::MemberRoleChanged(
        MemberRoleChangedEvent::new(
            room_id,
            target_user_id,
            target_user.username.to_string(),
            old_role,
            new_role,
            caller.id,
        ),
    ));
    state.engine.events().dispatch(event).await;

    // Get updated membership
    let updated_membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        target_user_id,
    )
    .await?
    .ok_or(Error::UserNotFound)?;

    // Check online status
    let online_users = state.engine.online_user_ids().await;

    let member = RoomMember {
        user_id: target_user.id,
        username: target_user.username.to_string(),
        role: updated_membership.role,
        joined_at: updated_membership.joined_at,
        is_online: online_users.contains(&target_user.id),
    };

    Ok(Json(MemberResponse { member }))
}

/// Remove a member from a room (kick).
///
/// Owners can kick anyone except themselves.
/// Moderators can kick regular members only.
pub async fn remove_member<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path((room_id, user_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;
    let target_user_id = UserId::parse(&user_id).map_err(|_| Error::UserNotFound)?;

    // Get the current user
    let (_, caller) = state.engine.validate_session(auth.session_id).await?;

    // Cannot kick yourself
    if caller.id == target_user_id {
        return Err(Error::PermissionDenied);
    }

    // Check caller's permissions
    let caller_membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        caller.id,
    )
    .await?
    .ok_or(Error::NotRoomMember)?;

    // Verify target is a member
    let target_membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        target_user_id,
    )
    .await?
    .ok_or(Error::NotRoomMember)?;

    // Permission check:
    // - Owners can kick anyone (except owners if they're the last one)
    // - Moderators can kick regular members only
    // - Members cannot kick anyone
    if caller_membership.is_owner() {
        // Owners can kick anyone except the last owner
        if target_membership.is_owner() {
            let members =
                MembershipRepository::list_members(state.engine.storage_clone().as_ref(), room_id)
                    .await?;
            let owner_count = members.iter().filter(|m| m.is_owner()).count();
            if owner_count <= 1 {
                return Err(Error::LastOwner);
            }
        }
    } else if caller_membership.is_moderator() {
        // Moderators can only kick regular members
        if target_membership.is_moderator() {
            return Err(Error::PermissionDenied);
        }
    } else {
        // Regular members cannot kick anyone
        return Err(Error::PermissionDenied);
    }

    // Remove the member
    MembershipRepository::remove_member(
        state.engine.storage_clone().as_ref(),
        room_id,
        target_user_id,
    )
    .await?;

    // Emit kicked event
    let event = Event::new(EventPayload::UserLeftRoom(UserLeftRoomEvent::kicked(
        room_id,
        target_user_id,
        caller.id,
    )));
    tracing::info!(
        "Dispatching UserLeftRoom (kicked) event: room={}, kicked_user={}, by={}",
        room_id,
        target_user_id,
        caller.id
    );
    state.engine.events().dispatch(event).await;

    Ok(Json(SuccessResponse::ok()))
}
