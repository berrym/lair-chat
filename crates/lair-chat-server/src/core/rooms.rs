//! Room service - room creation, membership, and invitations.
//!
//! This service handles:
//! - Room creation and deletion
//! - Joining and leaving rooms
//! - Room membership management
//! - Room invitations

use std::sync::Arc;

use crate::domain::{
    events::{
        Event, EventPayload, InvitationReceivedEvent, RoomDeletedEvent, UserJoinedRoomEvent,
        UserLeftRoomEvent,
    },
    EnrichedInvitation, Invitation, InvitationId, Pagination, Room, RoomId, RoomMembership,
    RoomName, RoomSettings, User, UserId,
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

        // Build settings (merge description into settings)
        let room_settings = match settings {
            Some(mut s) => {
                if description.is_some() {
                    s.description = description;
                }
                s
            }
            None => RoomSettings {
                description,
                ..Default::default()
            },
        };

        // Create room
        let room = Room::new(room_name, owner_id, room_settings);
        RoomRepository::create(&*self.storage, &room).await?;

        // Add owner as member
        let membership = RoomMembership::as_owner(room.id, owner_id);
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
    pub async fn list_for_user(
        &self,
        user_id: UserId,
        pagination: Pagination,
    ) -> Result<Vec<Room>> {
        RoomRepository::list_for_user(&*self.storage, user_id, pagination).await
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
        let membership = RoomMembership::as_member(room_id, user_id);
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

        // Update description (stored in settings)
        if let Some(desc) = description {
            room.settings.description = Some(desc);
        }

        // Update other settings (preserving description if not overwritten above)
        if let Some(s) = settings {
            // Preserve description if it was just set above
            let current_desc = room.settings.description.clone();
            room.settings = s;
            if current_desc.is_some() && room.settings.description.is_none() {
                room.settings.description = current_desc;
            }
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

        // Emit InvitationReceived event
        let room = RoomRepository::find_by_id(&*self.storage, room_id)
            .await?
            .ok_or(Error::RoomNotFound)?;
        let inviter = UserRepository::find_by_id(&*self.storage, inviter_id)
            .await?
            .ok_or(Error::UserNotFound)?;
        let invitee = UserRepository::find_by_id(&*self.storage, invitee_id)
            .await?
            .ok_or(Error::UserNotFound)?;

        let enriched = EnrichedInvitation::from_invitation(
            &invitation,
            room.name.to_string(),
            inviter.username.to_string(),
            invitee.username.to_string(),
        );
        let event = Event::new(EventPayload::InvitationReceived(
            InvitationReceivedEvent::new(enriched),
        ));
        self.events.dispatch(event).await;

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

        // Verify invitation can be accepted (checks expiry/status)
        invitation.accept().map_err(|_| Error::InvitationExpired)?;

        // Join the room first — join() checks for a pending invitation on
        // private rooms, so we must not persist the "accepted" status until
        // after the join succeeds.
        let membership = self.join(user_id, invitation.room_id).await?;

        // Persist the accepted status now that membership is confirmed.
        // (join() already marks pending invitations as accepted for the
        // public-room path, but for private rooms the invitation was found
        // by join()'s find_pending check, so the update here is the
        // authoritative persistence.)
        InvitationRepository::update(&*self.storage, &invitation).await?;

        Ok(membership)
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::auth::AuthService;
    use crate::storage::sqlite::SqliteStorage;

    const TEST_JWT_SECRET: &str = "test-jwt-secret-for-room-tests";

    /// Create a test room service with in-memory storage.
    async fn create_test_service() -> (RoomService<SqliteStorage>, Arc<SqliteStorage>) {
        let storage = Arc::new(SqliteStorage::in_memory().await.unwrap());
        let events = EventDispatcher::new();
        let service = RoomService::new(storage.clone(), events);
        (service, storage)
    }

    /// Create a test user.
    async fn create_user(storage: &Arc<SqliteStorage>, username: &str, email: &str) -> UserId {
        let auth = AuthService::new(storage.clone(), TEST_JWT_SECRET);
        let result = auth.register(username, email, "password123").await.unwrap();
        result.0.id
    }

    // ========================================================================
    // Room Creation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_room_success() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        let room = service
            .create(owner_id, "general", Some("General chat".to_string()), None)
            .await;

        assert!(room.is_ok());
        let room = room.unwrap();
        assert_eq!(room.name.as_str(), "general");
        assert_eq!(room.owner, owner_id);
        assert_eq!(room.settings.description, Some("General chat".to_string()));
    }

    #[tokio::test]
    async fn test_create_room_invalid_name() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        // Empty name
        let result = service.create(owner_id, "", None, None).await;
        assert!(matches!(result, Err(Error::RoomNameInvalid { .. })));

        // Name with only spaces
        let result = service.create(owner_id, "   ", None, None).await;
        assert!(matches!(result, Err(Error::RoomNameInvalid { .. })));
    }

    #[tokio::test]
    async fn test_create_room_duplicate_name() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        // Create first room
        let _ = service
            .create(owner_id, "general", None, None)
            .await
            .unwrap();

        // Try to create room with same name
        let result = service.create(owner_id, "general", None, None).await;
        assert!(matches!(result, Err(Error::RoomNameTaken)));
    }

    #[tokio::test]
    async fn test_create_room_with_settings() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        let settings = RoomSettings {
            is_private: true,
            max_members: Some(10),
            ..Default::default()
        };

        let room = service
            .create(owner_id, "private-room", None, Some(settings))
            .await
            .unwrap();

        assert!(!room.is_public());
        assert_eq!(room.settings.max_members, Some(10));
    }

    // ========================================================================
    // Room Retrieval Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_room_success() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        let created = service
            .create(owner_id, "general", None, None)
            .await
            .unwrap();
        let found = service.get(created.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_get_room_not_found() {
        let (service, _) = create_test_service().await;

        let result = service.get(RoomId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_public_rooms() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        // Create public room
        let _ = service
            .create(owner_id, "public1", None, None)
            .await
            .unwrap();

        // Create private room
        let private_settings = RoomSettings {
            is_private: true,
            ..Default::default()
        };
        let _ = service
            .create(owner_id, "private1", None, Some(private_settings))
            .await
            .unwrap();

        let public_rooms = service.list_public(Pagination::default()).await.unwrap();
        assert_eq!(public_rooms.len(), 1);
        assert_eq!(public_rooms[0].name.as_str(), "public1");
    }

    // ========================================================================
    // Join/Leave Tests
    // ========================================================================

    #[tokio::test]
    async fn test_join_room_success() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let membership = service.join(bob_id, room.id).await;

        assert!(membership.is_ok());
        let membership = membership.unwrap();
        assert_eq!(membership.user_id, bob_id);
        assert_eq!(membership.room_id, room.id);
        assert!(!membership.is_owner()); // Bob is not owner
    }

    #[tokio::test]
    async fn test_join_room_already_member() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let _ = service.join(bob_id, room.id).await.unwrap();

        // Try to join again
        let result = service.join(bob_id, room.id).await;
        assert!(matches!(result, Err(Error::AlreadyMember)));
    }

    #[tokio::test]
    async fn test_join_room_not_found() {
        let (service, storage) = create_test_service().await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let result = service.join(bob_id, RoomId::new()).await;
        assert!(matches!(result, Err(Error::RoomNotFound)));
    }

    #[tokio::test]
    async fn test_join_private_room_without_invitation() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let settings = RoomSettings {
            is_private: true,
            ..Default::default()
        };
        let room = service
            .create(alice_id, "private", None, Some(settings))
            .await
            .unwrap();

        // Bob tries to join without invitation
        let result = service.join(bob_id, room.id).await;
        assert!(matches!(result, Err(Error::RoomPrivate)));
    }

    #[tokio::test]
    async fn test_leave_room_success() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let _ = service.join(bob_id, room.id).await.unwrap();

        let result = service.leave(bob_id, room.id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_leave_room_not_member() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();

        let result = service.leave(bob_id, room.id).await;
        assert!(matches!(result, Err(Error::NotRoomMember)));
    }

    #[tokio::test]
    async fn test_leave_room_last_owner() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();

        // Alice is the only owner, can't leave
        let result = service.leave(alice_id, room.id).await;
        assert!(matches!(result, Err(Error::LastOwner)));
    }

    // ========================================================================
    // Room Update Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_room_success() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        let room = service
            .create(owner_id, "general", None, None)
            .await
            .unwrap();

        let updated = service
            .update(
                owner_id,
                room.id,
                Some("new-general"),
                Some("Updated description".to_string()),
                None,
            )
            .await;

        assert!(updated.is_ok());
        let updated = updated.unwrap();
        assert_eq!(updated.name.as_str(), "new-general");
        assert_eq!(
            updated.settings.description,
            Some("Updated description".to_string())
        );
    }

    #[tokio::test]
    async fn test_update_room_permission_denied() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let _ = service.join(bob_id, room.id).await.unwrap();

        // Bob (not owner/moderator) tries to update
        let result = service
            .update(bob_id, room.id, Some("hacked"), None, None)
            .await;
        assert!(matches!(result, Err(Error::PermissionDenied)));
    }

    // ========================================================================
    // Room Delete Tests
    // ========================================================================

    #[tokio::test]
    async fn test_delete_room_success() {
        let (service, storage) = create_test_service().await;
        let owner_id = create_user(&storage, "alice", "alice@example.com").await;

        let room = service
            .create(owner_id, "general", None, None)
            .await
            .unwrap();

        let result = service.delete(owner_id, room.id).await;
        assert!(result.is_ok());

        // Verify room is deleted
        let found = service.get(room.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_room_permission_denied() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let _ = service.join(bob_id, room.id).await.unwrap();

        // Bob tries to delete
        let result = service.delete(bob_id, room.id).await;
        assert!(matches!(result, Err(Error::PermissionDenied)));
    }

    // ========================================================================
    // Invitation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_invite_user_success() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        let invitation = service.invite(alice_id, room.id, bob_id).await;

        assert!(invitation.is_ok());
        let invitation = invitation.unwrap();
        assert_eq!(invitation.inviter, alice_id);
        assert_eq!(invitation.invitee, bob_id);
        assert_eq!(invitation.room_id, room.id);
    }

    #[tokio::test]
    async fn test_invite_already_member() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(alice_id, "general", None, None)
            .await
            .unwrap();
        let _ = service.join(bob_id, room.id).await.unwrap();

        // Can't invite someone already in the room
        let result = service.invite(alice_id, room.id, bob_id).await;
        assert!(matches!(result, Err(Error::AlreadyMember)));
    }

    #[tokio::test]
    async fn test_invite_already_invited() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        let _ = service.invite(alice_id, room.id, bob_id).await.unwrap();

        // Can't invite again while pending
        let result = service.invite(alice_id, room.id, bob_id).await;
        assert!(matches!(result, Err(Error::AlreadyInvited)));
    }

    #[tokio::test]
    async fn test_accept_invitation_success() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        // Use a private room to verify the invitation acceptance works
        // correctly with the join() pending-invitation check.
        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        let invitation = service.invite(alice_id, room.id, bob_id).await.unwrap();
        let membership = service.accept_invitation(bob_id, invitation.id).await;

        assert!(membership.is_ok());
        let membership = membership.unwrap();
        assert_eq!(membership.user_id, bob_id);
        assert_eq!(membership.room_id, room.id);
    }

    #[tokio::test]
    async fn test_decline_invitation_success() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        let invitation = service.invite(alice_id, room.id, bob_id).await.unwrap();
        let result = service.decline_invitation(bob_id, invitation.id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reinvite_after_decline() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        // First invitation
        let invitation1 = service.invite(alice_id, room.id, bob_id).await.unwrap();

        // Decline it
        service
            .decline_invitation(bob_id, invitation1.id)
            .await
            .unwrap();

        // Re-invite should succeed (declined invitation should not block)
        let invitation2 = service.invite(alice_id, room.id, bob_id).await;
        assert!(
            invitation2.is_ok(),
            "Re-invitation after decline should succeed"
        );

        // The new invitation should be different
        let inv2 = invitation2.unwrap();
        assert_ne!(inv2.id, invitation1.id);
    }

    #[tokio::test]
    async fn test_list_invitations() {
        let (service, storage) = create_test_service().await;
        let alice_id = create_user(&storage, "alice", "alice@example.com").await;
        let bob_id = create_user(&storage, "bob", "bob@example.com").await;

        let room = service
            .create(
                alice_id,
                "private-room",
                None,
                Some(RoomSettings {
                    is_private: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        let _ = service.invite(alice_id, room.id, bob_id).await.unwrap();

        let invitations = service.list_invitations(bob_id).await.unwrap();
        assert_eq!(invitations.len(), 1);
        assert_eq!(invitations[0].room_id, room.id);
    }
}
