//! Storage traits - repository interfaces.
//!
//! These traits define the contract for all data persistence operations.
//! The core engine depends on these traits, not concrete implementations,
//! enabling database swapping and easy testing with mocks.

use async_trait::async_trait;

use crate::domain::{
    Invitation, InvitationId, InvitationStatus, Message, MessageId, MessageTarget, Pagination,
    Room, RoomId, RoomMembership, RoomRole, Session, SessionId, User, UserId,
};
use crate::Result;

// ============================================================================
// User Repository
// ============================================================================

/// Repository for user account operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user account.
    ///
    /// The password hash should be pre-computed by the caller.
    async fn create(&self, user: &User, password_hash: &str) -> Result<()>;

    /// Find a user by their ID.
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>>;

    /// Find a user by their username (case-insensitive).
    async fn find_by_username(&self, username: &str) -> Result<Option<User>>;

    /// Find a user by their email (case-insensitive).
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Get the password hash for a user.
    ///
    /// Returns None if user doesn't exist.
    async fn get_password_hash(&self, user_id: UserId) -> Result<Option<String>>;

    /// Update a user's password hash.
    async fn update_password_hash(&self, user_id: UserId, password_hash: &str) -> Result<()>;

    /// Update a user's profile.
    async fn update(&self, user: &User) -> Result<()>;

    /// Delete a user account.
    async fn delete(&self, id: UserId) -> Result<()>;

    /// List all users with pagination.
    async fn list(&self, pagination: Pagination) -> Result<Vec<User>>;

    /// Count total users.
    async fn count(&self) -> Result<u64>;

    /// Check if a username is already taken.
    async fn username_exists(&self, username: &str) -> Result<bool>;

    /// Check if an email is already taken.
    async fn email_exists(&self, email: &str) -> Result<bool>;
}

// ============================================================================
// Room Repository
// ============================================================================

/// Repository for room operations.
#[async_trait]
pub trait RoomRepository: Send + Sync {
    /// Create a new room.
    async fn create(&self, room: &Room) -> Result<()>;

    /// Find a room by its ID.
    async fn find_by_id(&self, id: RoomId) -> Result<Option<Room>>;

    /// Find a room by its name.
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;

    /// Update a room.
    async fn update(&self, room: &Room) -> Result<()>;

    /// Delete a room.
    async fn delete(&self, id: RoomId) -> Result<()>;

    /// List all public rooms with pagination.
    async fn list_public(&self, pagination: Pagination) -> Result<Vec<Room>>;

    /// List all rooms a user is a member of.
    async fn list_for_user(&self, user_id: UserId, pagination: Pagination) -> Result<Vec<Room>>;

    /// Count total rooms.
    async fn count(&self) -> Result<u64>;

    /// Check if a room name is already taken.
    async fn name_exists(&self, name: &str) -> Result<bool>;
}

// ============================================================================
// Room Membership Repository
// ============================================================================

/// Repository for room membership operations.
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// Add a user to a room.
    async fn add_member(&self, membership: &RoomMembership) -> Result<()>;

    /// Remove a user from a room.
    async fn remove_member(&self, room_id: RoomId, user_id: UserId) -> Result<()>;

    /// Get a user's membership in a room.
    async fn get_membership(
        &self,
        room_id: RoomId,
        user_id: UserId,
    ) -> Result<Option<RoomMembership>>;

    /// Update a user's role in a room.
    async fn update_role(&self, room_id: RoomId, user_id: UserId, role: RoomRole) -> Result<()>;

    /// List all members of a room.
    async fn list_members(&self, room_id: RoomId) -> Result<Vec<RoomMembership>>;

    /// List all members of a room with user details.
    async fn list_members_with_users(&self, room_id: RoomId)
        -> Result<Vec<(User, RoomMembership)>>;

    /// Count members in a room.
    async fn count_members(&self, room_id: RoomId) -> Result<u32>;

    /// Check if a user is a member of a room.
    async fn is_member(&self, room_id: RoomId, user_id: UserId) -> Result<bool>;
}

// ============================================================================
// Message Repository
// ============================================================================

/// Repository for message operations.
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// Create a new message.
    async fn create(&self, message: &Message) -> Result<()>;

    /// Find a message by its ID.
    async fn find_by_id(&self, id: MessageId) -> Result<Option<Message>>;

    /// Update a message (for edits).
    async fn update(&self, message: &Message) -> Result<()>;

    /// Delete a message.
    async fn delete(&self, id: MessageId) -> Result<()>;

    /// Get messages for a room with pagination.
    ///
    /// Messages are returned in reverse chronological order (newest first).
    async fn find_by_room(&self, room_id: RoomId, pagination: Pagination) -> Result<Vec<Message>>;

    /// Get direct messages between two users with pagination.
    ///
    /// Messages are returned in reverse chronological order (newest first).
    async fn find_direct_messages(
        &self,
        user1: UserId,
        user2: UserId,
        pagination: Pagination,
    ) -> Result<Vec<Message>>;

    /// Get all messages for a target (room or DM).
    async fn find_by_target(
        &self,
        target: &MessageTarget,
        pagination: Pagination,
    ) -> Result<Vec<Message>>;

    /// Count messages in a room.
    async fn count_by_room(&self, room_id: RoomId) -> Result<u64>;

    /// Count direct messages between two users.
    async fn count_direct_messages(&self, user1: UserId, user2: UserId) -> Result<u64>;

    /// Get the most recent message in a room.
    async fn get_latest_in_room(&self, room_id: RoomId) -> Result<Option<Message>>;

    /// Delete all messages in a room.
    async fn delete_by_room(&self, room_id: RoomId) -> Result<u64>;

    /// Delete all messages by a user.
    async fn delete_by_author(&self, author_id: UserId) -> Result<u64>;
}

// ============================================================================
// Session Repository
// ============================================================================

/// Repository for session operations.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Create a new session.
    async fn create(&self, session: &Session) -> Result<()>;

    /// Find a session by its ID.
    async fn find_by_id(&self, id: SessionId) -> Result<Option<Session>>;

    /// Update a session (for touch/refresh).
    async fn update(&self, session: &Session) -> Result<()>;

    /// Delete a session (logout).
    async fn delete(&self, id: SessionId) -> Result<()>;

    /// Delete all sessions for a user.
    async fn delete_by_user(&self, user_id: UserId) -> Result<u64>;

    /// List all active sessions for a user.
    async fn list_by_user(&self, user_id: UserId) -> Result<Vec<Session>>;

    /// Count active sessions for a user.
    async fn count_by_user(&self, user_id: UserId) -> Result<u32>;

    /// Delete expired sessions (maintenance).
    async fn delete_expired(&self) -> Result<u64>;

    /// Check if a session exists and is valid.
    async fn is_valid(&self, id: SessionId) -> Result<bool>;
}

// ============================================================================
// Invitation Repository
// ============================================================================

/// Repository for invitation operations.
#[async_trait]
pub trait InvitationRepository: Send + Sync {
    /// Create a new invitation.
    async fn create(&self, invitation: &Invitation) -> Result<()>;

    /// Find an invitation by its ID.
    async fn find_by_id(&self, id: InvitationId) -> Result<Option<Invitation>>;

    /// Update an invitation (status change).
    async fn update(&self, invitation: &Invitation) -> Result<()>;

    /// Delete an invitation.
    async fn delete(&self, id: InvitationId) -> Result<()>;

    /// List pending invitations for a user (invitee).
    async fn list_pending_for_user(&self, user_id: UserId) -> Result<Vec<Invitation>>;

    /// List invitations sent by a user (inviter).
    async fn list_sent_by_user(&self, user_id: UserId) -> Result<Vec<Invitation>>;

    /// List invitations for a room.
    async fn list_for_room(&self, room_id: RoomId) -> Result<Vec<Invitation>>;

    /// Find pending invitation for a specific user to a specific room.
    async fn find_pending(&self, room_id: RoomId, invitee: UserId) -> Result<Option<Invitation>>;

    /// Update invitation status.
    async fn update_status(&self, id: InvitationId, status: InvitationStatus) -> Result<()>;

    /// Delete all invitations for a room (when room is deleted).
    async fn delete_by_room(&self, room_id: RoomId) -> Result<u64>;

    /// Expire old pending invitations.
    async fn expire_old(&self) -> Result<u64>;
}

// ============================================================================
// Combined Storage Trait
// ============================================================================

/// Combined storage trait for dependency injection.
///
/// This trait combines all repository traits into a single interface,
/// making it easier to pass storage to the core engine.
pub trait Storage:
    UserRepository
    + RoomRepository
    + MembershipRepository
    + MessageRepository
    + SessionRepository
    + InvitationRepository
{
}

/// Blanket implementation for any type that implements all repositories.
impl<T> Storage for T where
    T: UserRepository
        + RoomRepository
        + MembershipRepository
        + MessageRepository
        + SessionRepository
        + InvitationRepository
{
}

// ============================================================================
// Transaction Support (Future)
// ============================================================================

/// Transaction handle for atomic operations.
///
/// This will be implemented in Phase 2 when we add SQLite support.
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Commit the transaction.
    async fn commit(self) -> Result<()>;

    /// Rollback the transaction.
    async fn rollback(self) -> Result<()>;
}

/// Storage that supports transactions.
#[async_trait]
pub trait TransactionalStorage: Storage {
    /// The transaction type.
    type Tx: Transaction + Storage;

    /// Begin a new transaction.
    async fn begin(&self) -> Result<Self::Tx>;
}
