//! Room service - room creation, membership, and invitations.
//!
//! This service handles:
//! - Room creation and deletion
//! - Joining and leaving rooms
//! - Room membership management
//! - Room invitations

use std::sync::Arc;

use crate::domain::{
    events::{Event, EventPayload, RoomDeletedEvent, UserJoinedRoomEvent, UserLeftRoomEvent},
    Invitation, InvitationId, Pagination, Room, RoomId, RoomMembership, RoomName, RoomSettings,
    User, UserId,
};
use crate::storage::{
    InvitationRepository, MembershipRepository, RoomRepository, Storage, UserRepository,
};
use crate::{Error, Result};

use super::events::EventDispatcher;

// ============================================================================
// RoomService
// ============================================================================

/// Service for room operations.
pub struct RoomService<S: Storage> {
    storage: Arc<S>,
    events: EventDispatcher,
}

impl<S: Storage + 'static> RoomService<S> {
    /// Create a new room service.
    pub fn new(storage: Arc<S>, events: EventDispatcher) -> Self {
        Self { storage, events }
    }

    /// Create a new room.
    ///
    /// The creator automatically becomes the owner.
    pub async fn create(
        &self,
        owner_id: UserId,
        name: &str,
        description: Option<String>,
        settings: Option<RoomSettings>,
    ) -> Result<Room> {
        // Validate room name
        let room_name = RoomName::new(name).map_err(|e| Error::RoomNameInvalid {
            reason: e.to_string(),
        })?;

        // Check name availability
        if RoomRepository::name_exists(&*self.storage, room_name.as_str()).await? {
            return Err(Error::RoomNameTaken);
        }

        // Create room
        let mut room = Room::new(room_name, owner_id);
        room.description = description;
        if let Some(s) = settings {
            room.settings = s;
        }

        RoomRepository::create(&*self.storage, &room).await?;

        // Add owner as member
        let membership = RoomMembership::new_owner(room.id, owner_id);
        MembershipRepository::add_member(&*self.storage, &membership).await?;

        Ok(room)
    }

    /// Get a room by its ID.
    pub async fn get(&self, room_id: RoomId) -> Result<Option<Room>> {
        RoomRepository::find_by_id(&*self.storage, room_id).await
    }

    /// List public rooms.
    pub async fn list_public(&self, pagination: Pagination) -> Result<Vec<Room>> {
        RoomRepository::list_public(&*self.storage, pagination).await
    }

    /// List rooms a user is a member of.
    pub async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Room>> {
        RoomRepository::list_for_user(&*self.storage, user_id).await
    }

    /// Join a room.
    ///
    /// # Errors
    ///
    /// - `RoomNotFound` - Room doesn't exist
    /// - `RoomPrivate` - Room requires invitation
    /// - `AlreadyMember` - User is already in room
    /// - `RoomFull` - Room has reached max members
    pub async fn join(&self, user_id: UserId, room_id: RoomId) -> Result<RoomMembership> {
        // Get room
        let room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;

        // Check if room is public
        if !room.is_public() {
            // Check for invitation
            let invitation =
                InvitationRepository::find_pending(&*self.storage, room_id, user_id).await?;
            if invitation.is_none() {
                return Err(Error::RoomPrivate);
            }
        }

        // Check if already a member
        if MembershipRepository::is_member(&*self.storage, room_id, user_id).await? {
            return Err(Error::AlreadyMember);
        }

        // Check room capacity
        let member_count = MembershipRepository::count_members(&*self.storage, room_id).await?;
        if room.is_full(member_count) {
            return Err(Error::RoomFull);
        }

        // Add member
        let membership = RoomMembership::new(room_id, user_id);
        MembershipRepository::add_member(&*self.storage, &membership).await?;

        // Mark any pending invitation as accepted
        if let Some(mut invitation) =
            InvitationRepository::find_pending(&*self.storage, room_id, user_id).await?
        {
            let _ = invitation.accept();
            let _ = InvitationRepository::update(&*self.storage, &invitation).await;
        }

        // Emit event
        if let Ok(Some(user)) = UserRepository::find_by_id(&*self.storage, user_id).await {
            let event = Event::new(EventPayload::UserJoinedRoom(UserJoinedRoomEvent::new(
                room_id,
                user,
                membership.clone(),
            )));
            self.events.dispatch(event).await;
        }

        Ok(membership)
    }

    /// Leave a room.
    ///
    /// # Errors
    ///
    /// - `RoomNotFound` - Room doesn't exist
    /// - `NotRoomMember` - User isn't in this room
    /// - `LastOwner` - Can't leave as only owner
    pub async fn leave(&self, user_id: UserId, room_id: RoomId) -> Result<()> {
        // Verify room exists
        let _room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;

        // Get membership
        let membership = MembershipRepository::get_membership(&*self.storage, room_id, user_id)
            .await?
            .ok_or(Error::NotRoomMember)?;

        // Check if user is the only owner
        if membership.is_owner() {
            let members = MembershipRepository::list_members(&*self.storage, room_id).await?;
            let owner_count = members.iter().filter(|m| m.is_owner()).count();
            if owner_count <= 1 {
                return Err(Error::LastOwner);
            }
        }

        // Remove member
        MembershipRepository::remove_member(&*self.storage, room_id, user_id).await?;

        // Emit event
        let event = Event::new(EventPayload::UserLeftRoom(UserLeftRoomEvent::voluntary(
            room_id, user_id,
        )));
        self.events.dispatch(event).await;

        Ok(())
    }

    /// Update room settings.
    ///
    /// Only room owners and moderators can update rooms.
    pub async fn update(
        &self,
        user_id: UserId,
        room_id: RoomId,
        name: Option<&str>,
        description: Option<String>,
        settings: Option<RoomSettings>,
    ) -> Result<Room> {
        // Get room
        let mut room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;

        // Check permissions
        let membership = MembershipRepository::get_membership(&*self.storage, room_id, user_id)
            .await?
            .ok_or(Error::NotRoomMember)?;

        if !membership.is_moderator() {
            return Err(Error::PermissionDenied);
        }

        // Update name if provided
        if let Some(new_name) = name {
            let room_name = RoomName::new(new_name).map_err(|e| Error::RoomNameInvalid {
                reason: e.to_string(),
            })?;

            // Check name availability (if different from current)
            if room_name.as_str() != room.name.as_str()
                && RoomRepository::name_exists(&*self.storage, room_name.as_str()).await?
            {
                return Err(Error::RoomNameTaken);
            }
            room.name = room_name;
        }

        // Update description
        if let Some(desc) = description {
            room.description = Some(desc);
        }

        // Update settings
        if let Some(s) = settings {
            room.settings = s;
        }

        RoomRepository::update(&*self.storage, &room).await?;

        Ok(room)
    }

    /// Delete a room.
    ///
    /// Only room owners or system admins can delete rooms.
    pub async fn delete(&self, user_id: UserId, room_id: RoomId) -> Result<()> {
        // Get room
        let room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;

        // Check permissions (owner check)
        if room.owner != user_id {
            // Could also check for admin role here
            let membership =
                MembershipRepository::get_membership(&*self.storage, room_id, user_id).await?;
            if membership.map(|m| !m.is_owner()).unwrap_or(true) {
                return Err(Error::PermissionDenied);
            }
        }

        // Emit event before deletion (so members can be notified)
        let event = Event::new(EventPayload::RoomDeleted(RoomDeletedEvent::new(
            room_id,
            room.name.to_string(),
            user_id,
        )));
        self.events.dispatch(event).await;

        // Delete invitations
        let _ = InvitationRepository::delete_by_room(&*self.storage, room_id).await;

        // Delete room (cascades to memberships in DB)
        RoomRepository::delete(&*self.storage, room_id).await?;

        Ok(())
    }

    /// Get room members with user details.
    pub async fn get_members(&self, room_id: RoomId) -> Result<Vec<(User, RoomMembership)>> {
        MembershipRepository::list_members_with_users(&*self.storage, room_id).await
    }

    // ========================================================================
    // Invitation Operations
    // ========================================================================

    /// Invite a user to a room.
    pub async fn invite(
        &self,
        inviter_id: UserId,
        room_id: RoomId,
        invitee_id: UserId,
    ) -> Result<Invitation> {
        // Verify room exists
        let _room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;

        // Verify inviter is a member
        if !MembershipRepository::is_member(&*self.storage, room_id, inviter_id).await? {
            return Err(Error::NotRoomMember);
        }

        // Verify invitee exists
        let _invitee = UserRepository::find_by_id(&*self.storage, invitee_id)
            .await?
            .ok_or(Error::UserNotFound)?;

        // Check if invitee is already a member
        if MembershipRepository::is_member(&*self.storage, room_id, invitee_id).await? {
            return Err(Error::AlreadyMember);
        }

        // Check for existing pending invitation
        if InvitationRepository::find_pending(&*self.storage, room_id, invitee_id)
            .await?
            .is_some()
        {
            return Err(Error::AlreadyInvited);
        }

        // Create invitation
        let invitation = Invitation::new(room_id, inviter_id, invitee_id);
        InvitationRepository::create(&*self.storage, &invitation).await?;

        // TODO: Emit InvitationReceived event

        Ok(invitation)
    }

    /// Accept a room invitation.
    pub async fn accept_invitation(
        &self,
        user_id: UserId,
        invitation_id: InvitationId,
    ) -> Result<RoomMembership> {
        // Get invitation
        let mut invitation = InvitationRepository::find_by_id(&*self.storage, invitation_id)
            .await?
            .ok_or(Error::InvitationNotFound)?;

        // Verify user is the invitee
        if invitation.invitee != user_id {
            return Err(Error::NotInvitee);
        }

        // Accept invitation
        invitation.accept().map_err(|_| Error::InvitationExpired)?;
        InvitationRepository::update(&*self.storage, &invitation).await?;

        // Join the room
        self.join(user_id, invitation.room_id).await
    }

    /// Decline a room invitation.
    pub async fn decline_invitation(
        &self,
        user_id: UserId,
        invitation_id: InvitationId,
    ) -> Result<()> {
        // Get invitation
        let mut invitation = InvitationRepository::find_by_id(&*self.storage, invitation_id)
            .await?
            .ok_or(Error::InvitationNotFound)?;

        // Verify user is the invitee
        if invitation.invitee != user_id {
            return Err(Error::NotInvitee);
        }

        // Decline invitation
        invitation.decline().map_err(|_| Error::InvitationExpired)?;
        InvitationRepository::update(&*self.storage, &invitation).await?;

        Ok(())
    }

    /// List pending invitations for a user.
    pub async fn list_invitations(&self, user_id: UserId) -> Result<Vec<Invitation>> {
        InvitationRepository::list_pending_for_user(&*self.storage, user_id).await
    }
}
