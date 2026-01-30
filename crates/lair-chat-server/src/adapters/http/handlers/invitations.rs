//! Invitation handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::middleware::AuthUser;
use crate::adapters::http::routes::AppState;
use crate::domain::{
    EnrichedInvitation, Event, EventPayload, InvitationId, InvitationReceivedEvent, Room, RoomId,
    RoomMembership, UserId,
};
use crate::storage::{
    InvitationRepository, MembershipRepository, RoomRepository, Storage, UserRepository,
};
use crate::Error;

use super::SuccessResponse;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct CreateInvitationRequest {
    pub room_id: String,
    pub user_id: String,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct InvitationResponse {
    pub invitation: EnrichedInvitation,
}

#[derive(Serialize)]
pub struct InvitationsListResponse {
    pub invitations: Vec<EnrichedInvitation>,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct AcceptInvitationResponse {
    pub membership: RoomMembership,
    pub room: Room,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create an invitation.
///
/// Only room owners and moderators can create invitations.
pub async fn create_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<InvitationResponse>), Error> {
    let room_id = RoomId::parse(&req.room_id).map_err(|_| Error::RoomNotFound)?;
    let invitee_id = UserId::parse(&req.user_id).map_err(|_| Error::UserNotFound)?;

    // Get the current user
    let (_, user) = state.engine.validate_session(auth.session_id).await?;

    // Check permission: only owner/moderator can invite
    let membership = MembershipRepository::get_membership(
        state.engine.storage_clone().as_ref(),
        room_id,
        user.id,
    )
    .await?
    .ok_or(Error::NotRoomMember)?;

    if !membership.is_moderator() {
        return Err(Error::PermissionDenied);
    }

    // Create the invitation
    let invitation = state
        .engine
        .invite_to_room(auth.session_id, room_id, invitee_id)
        .await?;

    // Enrich the invitation with names
    let storage = state.engine.storage_clone();
    let enriched = enrich_invitation(&invitation, storage.as_ref()).await?;

    // Emit invitation received event to the invitee
    let event = Event::new(EventPayload::InvitationReceived(
        InvitationReceivedEvent::new(enriched.clone()),
    ));
    tracing::info!(
        "Dispatching InvitationReceived event to invitee_id={}, room={}",
        enriched.invitee_id,
        enriched.room_name
    );
    state.engine.events().dispatch(event).await;

    Ok((
        StatusCode::CREATED,
        Json(InvitationResponse {
            invitation: enriched,
        }),
    ))
}

/// List pending invitations for current user.
pub async fn list_invitations<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
) -> Result<Json<InvitationsListResponse>, Error> {
    let invitations = state.engine.list_invitations(auth.session_id).await?;

    // Enrich all invitations with names
    let storage = state.engine.storage_clone();
    let mut enriched = Vec::with_capacity(invitations.len());
    for inv in &invitations {
        enriched.push(enrich_invitation(inv, storage.as_ref()).await?);
    }

    Ok(Json(InvitationsListResponse {
        invitations: enriched,
        has_more: false,
    }))
}

/// Accept an invitation.
pub async fn accept_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(invitation_id): Path<String>,
) -> Result<Json<AcceptInvitationResponse>, Error> {
    let invitation_id =
        InvitationId::parse(&invitation_id).map_err(|_| Error::InvitationNotFound)?;

    // Get the invitation first to get the room_id
    let invitation =
        InvitationRepository::find_by_id(state.engine.storage_clone().as_ref(), invitation_id)
            .await?
            .ok_or(Error::InvitationNotFound)?;

    let room_id = invitation.room_id;

    let membership = state
        .engine
        .accept_invitation(auth.session_id, invitation_id)
        .await?;

    // Get the room details to return
    let room = RoomRepository::find_by_id(state.engine.storage_clone().as_ref(), room_id)
        .await?
        .ok_or(Error::RoomNotFound)?;

    Ok(Json(AcceptInvitationResponse { membership, room }))
}

/// Decline an invitation.
pub async fn decline_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Path(invitation_id): Path<String>,
) -> Result<Json<SuccessResponse>, Error> {
    let invitation_id =
        InvitationId::parse(&invitation_id).map_err(|_| Error::InvitationNotFound)?;

    state
        .engine
        .decline_invitation(auth.session_id, invitation_id)
        .await?;

    Ok(Json(SuccessResponse::ok()))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Enrich an invitation with room and user names.
async fn enrich_invitation<S: Storage>(
    invitation: &crate::domain::Invitation,
    storage: &S,
) -> Result<EnrichedInvitation, Error> {
    // Get room name
    let room = RoomRepository::find_by_id(storage, invitation.room_id)
        .await?
        .ok_or(Error::RoomNotFound)?;

    // Get inviter name
    let inviter = UserRepository::find_by_id(storage, invitation.inviter)
        .await?
        .ok_or(Error::UserNotFound)?;

    // Get invitee name
    let invitee = UserRepository::find_by_id(storage, invitation.invitee)
        .await?
        .ok_or(Error::UserNotFound)?;

    Ok(EnrichedInvitation::from_invitation(
        invitation,
        room.name.to_string(),
        inviter.username.to_string(),
        invitee.username.to_string(),
    ))
}
