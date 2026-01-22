//! Invitation handlers.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::routes::AppState;
use crate::domain::{Invitation, InvitationId, RoomId, RoomMembership, UserId};
use crate::storage::Storage;
use crate::Error;

use super::auth::extract_session_id;
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
    pub invitation: Invitation,
}

#[derive(Serialize)]
pub struct InvitationsListResponse {
    pub invitations: Vec<Invitation>,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct AcceptInvitationResponse {
    pub membership: RoomMembership,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create an invitation.
pub async fn create_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<InvitationResponse>), Error> {
    let session_id = extract_session_id(&headers)?;
    let room_id = RoomId::parse(&req.room_id).map_err(|_| Error::RoomNotFound)?;
    let user_id = UserId::parse(&req.user_id).map_err(|_| Error::UserNotFound)?;

    let invitation = state
        .engine
        .invite_to_room(session_id, room_id, user_id)
        .await?;

    Ok((StatusCode::CREATED, Json(InvitationResponse { invitation })))
}

/// List pending invitations for current user.
pub async fn list_invitations<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
) -> Result<Json<InvitationsListResponse>, Error> {
    let session_id = extract_session_id(&headers)?;

    let invitations = state.engine.list_invitations(session_id).await?;

    Ok(Json(InvitationsListResponse {
        invitations,
        has_more: false,
    }))
}

/// Accept an invitation.
pub async fn accept_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Path(invitation_id): Path<String>,
) -> Result<Json<AcceptInvitationResponse>, Error> {
    let session_id = extract_session_id(&headers)?;
    let invitation_id =
        InvitationId::parse(&invitation_id).map_err(|_| Error::InvitationNotFound)?;

    let membership = state
        .engine
        .accept_invitation(session_id, invitation_id)
        .await?;

    Ok(Json(AcceptInvitationResponse { membership }))
}

/// Decline an invitation.
pub async fn decline_invitation<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Path(invitation_id): Path<String>,
) -> Result<Json<SuccessResponse>, Error> {
    let session_id = extract_session_id(&headers)?;
    let invitation_id =
        InvitationId::parse(&invitation_id).map_err(|_| Error::InvitationNotFound)?;

    state
        .engine
        .decline_invitation(session_id, invitation_id)
        .await?;

    Ok(Json(SuccessResponse::ok()))
}
