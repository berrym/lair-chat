//! Room handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::middleware::AuthUser;
use crate::adapters::http::routes::AppState;
use crate::domain::{Pagination, Room, RoomId, RoomMembership, RoomSettings, User};
use crate::storage::Storage;
use crate::Error;

use super::SuccessResponse;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub description: Option<String>,
    pub settings: Option<RoomSettingsRequest>,
}

#[derive(Deserialize)]
pub struct RoomSettingsRequest {
    #[serde(default)]
    pub public: Option<bool>,
    pub max_members: Option<u32>,
}

impl From<RoomSettingsRequest> for RoomSettings {
    fn from(req: RoomSettingsRequest) -> Self {
        RoomSettings {
            description: None,
            is_private: req.public.map(|p| !p).unwrap_or(false),
            max_members: req.max_members,
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateRoomRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<RoomSettingsRequest>,
}

#[derive(Deserialize)]
pub struct ListRoomsQuery {
    pub search: Option<String>,
    #[serde(default)]
    pub joined_only: bool,
    #[serde(default)]
    pub public_only: bool,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    50
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub room: Room,
}

#[derive(Serialize)]
pub struct RoomWithMembershipResponse {
    pub room: Room,
    pub membership: Option<RoomMembership>,
    pub member_count: u32,
}

#[derive(Serialize)]
pub struct JoinRoomResponse {
    pub room: Room,
    pub membership: RoomMembership,
}

#[derive(Serialize)]
pub struct RoomsListResponse {
    pub rooms: Vec<RoomListItem>,
    pub has_more: bool,
    pub total_count: u32,
}

#[derive(Serialize)]
pub struct RoomListItem {
    pub room: Room,
    pub member_count: u32,
    pub is_member: bool,
}

#[derive(Serialize)]
pub struct MembersListResponse {
    pub members: Vec<MemberWithUser>,
    pub has_more: bool,
    pub total_count: u32,
}

#[derive(Serialize)]
pub struct MemberWithUser {
    pub user: User,
    pub membership: RoomMembership,
    pub online: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new room.
pub async fn create_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Json(req): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<RoomResponse>), Error> {
    let settings = req.settings.map(|s| s.into());

    let room = state
        .engine
        .create_room(auth.session_id, &req.name, req.description, settings)
        .await?;

    Ok((StatusCode::CREATED, Json(RoomResponse { room })))
}

/// Get a room by ID.
pub async fn get_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<RoomWithMembershipResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    let room = state
        .engine
        .get_room(room_id)
        .await?
        .ok_or(Error::RoomNotFound)?;

    let members = state.engine.get_room_members(room_id).await.ok();
    let member_count = members.as_ref().map(|m| m.len() as u32).unwrap_or(0);

    // Get user from session to check membership
    let membership = if let Ok((_, user)) = state.engine.validate_session(auth.session_id).await {
        state
            .engine
            .storage_clone()
            .get_membership(room_id, user.id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    Ok(Json(RoomWithMembershipResponse {
        room,
        membership,
        member_count,
    }))
}

/// List rooms with filtering.
pub async fn list_rooms<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Query(query): Query<ListRoomsQuery>,
) -> Result<Json<RoomsListResponse>, Error> {
    let pagination = Pagination {
        limit: query.limit.min(100),
        offset: query.offset,
    };

    let rooms = if query.joined_only {
        state.engine.list_user_rooms(auth.session_id).await?
    } else {
        state.engine.list_public_rooms(pagination).await?
    };

    let has_more = rooms.len() == query.limit as usize;
    let total_count = rooms.len() as u32;

    let rooms: Vec<RoomListItem> = rooms
        .into_iter()
        .map(|room| RoomListItem {
            room,
            member_count: 0,
            is_member: query.joined_only,
        })
        .collect();

    Ok(Json(RoomsListResponse {
        rooms,
        has_more,
        total_count,
    }))
}

/// Update a room.
pub async fn update_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
    Json(req): Json<UpdateRoomRequest>,
) -> Result<Json<RoomResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    let settings = req.settings.map(|s| s.into());

    let room = state
        .engine
        .update_room(
            auth.session_id,
            room_id,
            req.name.as_deref(),
            req.description,
            settings,
        )
        .await?;

    Ok(Json(RoomResponse { room }))
}

/// Delete a room.
pub async fn delete_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<SuccessResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    state.engine.delete_room(auth.session_id, room_id).await?;
    Ok(Json(SuccessResponse::ok()))
}

/// Join a room.
pub async fn join_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<JoinRoomResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    let membership = state.engine.join_room(auth.session_id, room_id).await?;
    let room = state
        .engine
        .get_room(room_id)
        .await?
        .ok_or(Error::RoomNotFound)?;

    Ok(Json(JoinRoomResponse { room, membership }))
}

/// Leave a room.
pub async fn leave_room<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<SuccessResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    state.engine.leave_room(auth.session_id, room_id).await?;
    Ok(Json(SuccessResponse::ok()))
}

/// Get room members.
pub async fn get_members<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    _auth: AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<MembersListResponse>, Error> {
    let room_id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;

    let members = state.engine.get_room_members(room_id).await?;
    let total_count = members.len() as u32;

    let members: Vec<MemberWithUser> = members
        .into_iter()
        .map(|(user, membership)| MemberWithUser {
            user,
            membership,
            online: false,
        })
        .collect();

    Ok(Json(MembersListResponse {
        members,
        has_more: false,
        total_count,
    }))
}
