//! Chat engine - main coordinator for all business operations.
//!
//! The `ChatEngine` is the central coordinator that:
//! - Receives commands from protocol adapters
//! - Validates inputs and enforces business rules
//! - Coordinates with the storage layer
//! - Emits events for state changes
//!
//! See [COMMANDS.md](../../../../docs/architecture/COMMANDS.md) for all operations.

use std::sync::Arc;

use crate::domain::{
    EnrichedInvitation, Invitation, InvitationId, Message, MessageId, MessageTarget, Pagination,
    Role, Room, RoomId, RoomMembership, RoomSettings, Session, SessionId, User, UserId,
};
use crate::storage::{RoomRepository, Storage, UserRepository};
use crate::Result;

use super::auth::AuthService;
use super::events::EventDispatcher;
use super::jwt::JwtService;
use super::messaging::MessagingService;
use super::rooms::RoomService;
use super::sessions::SessionManager;

// ============================================================================
// ChatEngine
// ============================================================================

/// The central coordinator for all chat operations.
///
/// All protocol adapters (TCP, HTTP, WebSocket) interact with the system
/// through this engine. The engine ensures consistent business logic
/// regardless of the protocol used.
///
/// # Example
///
/// ```ignore
/// let storage = SqliteStorage::new("chat.db").await?;
/// let engine = ChatEngine::new(Arc::new(storage));
///
/// // From TCP adapter
/// let user = engine.login("alice", "password").await?;
///
/// // From HTTP adapter - same logic
/// let user = engine.login("alice", "password").await?;
/// ```
pub struct ChatEngine<S: Storage> {
    /// Storage backend for persistence.
    storage: Arc<S>,

    /// Authentication service.
    auth: AuthService<S>,

    /// Session management.
    sessions: SessionManager<S>,

    /// Room operations.
    rooms: RoomService<S>,

    /// Message operations.
    messaging: MessagingService<S>,

    /// Event dispatching to connected clients.
    events: EventDispatcher,
}

impl<S: Storage + 'static> ChatEngine<S> {
    /// Create a new chat engine with the given storage backend and JWT secret.
    ///
    /// The JWT secret should be at least 32 bytes for security.
    pub fn new(storage: Arc<S>, jwt_secret: &str) -> Self {
        let events = EventDispatcher::new();

        Self {
            storage: storage.clone(),
            auth: AuthService::new(storage.clone(), jwt_secret),
            sessions: SessionManager::new(storage.clone()),
            rooms: RoomService::new(storage.clone(), events.clone()),
            messaging: MessagingService::new(storage.clone(), events.clone()),
            events,
        }
    }

    /// Get a reference to the event dispatcher.
    ///
    /// Protocol adapters use this to subscribe to events for their connections.
    pub fn events(&self) -> &EventDispatcher {
        &self.events
    }

    /// Get a clone of the event dispatcher.
    pub fn events_clone(&self) -> EventDispatcher {
        self.events.clone()
    }

    /// Get a clone of the storage backend.
    ///
    /// Protocol adapters use this to fetch user room memberships for event filtering.
    pub fn storage_clone(&self) -> Arc<S> {
        self.storage.clone()
    }

    // ========================================================================
    // Authentication Operations
    // ========================================================================

    /// Register a new user account.
    ///
    /// Creates the user and automatically logs them in.
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(User, Session, String)> {
        self.auth.register(username, email, password).await
    }

    /// Authenticate a user and create a session.
    ///
    /// The identifier can be either a username or email.
    pub async fn login(&self, identifier: &str, password: &str) -> Result<(User, Session, String)> {
        self.auth.login(identifier, password).await
    }

    /// End a session.
    pub async fn logout(&self, session_id: SessionId) -> Result<()> {
        self.sessions.logout(session_id).await
    }

    /// Validate a session and return the associated user.
    pub async fn validate_session(&self, session_id: SessionId) -> Result<(Session, User)> {
        self.sessions.validate(session_id).await
    }

    /// Refresh a session token.
    pub async fn refresh_token(&self, session_id: SessionId) -> Result<String> {
        self.auth.refresh_token(session_id).await
    }

    /// Validate a JWT token and return user/session info.
    ///
    /// This is used by TCP connections to authenticate with a token
    /// obtained from the HTTP API.
    pub async fn validate_token(&self, token: &str) -> Result<(User, Session)> {
        self.auth.validate_token_full(token).await
    }

    /// Quick token validation (JWT only, no database check).
    ///
    /// Returns user ID, session ID, and role from the token claims.
    /// Use `validate_token` for full validation including session status.
    pub fn validate_token_quick(&self, token: &str) -> Result<(UserId, SessionId, Role)> {
        self.auth.validate_token(token)
    }

    /// Get a reference to the JWT service.
    pub fn jwt_service(&self) -> &JwtService {
        self.auth.jwt_service()
    }

    // ========================================================================
    // User Operations
    // ========================================================================

    /// Get a user by their ID.
    pub async fn get_user(&self, user_id: UserId) -> Result<Option<User>> {
        self.auth.get_user(user_id).await
    }

    /// Get the current authenticated user's details.
    pub async fn get_current_user(&self, session_id: SessionId) -> Result<User> {
        let (_, user) = self.sessions.validate(session_id).await?;
        Ok(user)
    }

    /// List users with optional filtering and pagination.
    pub async fn list_users(&self, pagination: Pagination) -> Result<Vec<User>> {
        self.auth.list_users(pagination).await
    }

    /// Change a user's password.
    pub async fn change_password(
        &self,
        session_id: SessionId,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let (session, _) = self.sessions.validate(session_id).await?;
        self.auth
            .change_password(session.user_id, current_password, new_password)
            .await
    }

    // ========================================================================
    // Room Operations
    // ========================================================================

    /// Create a new room.
    pub async fn create_room(
        &self,
        session_id: SessionId,
        name: &str,
        description: Option<String>,
        settings: Option<RoomSettings>,
    ) -> Result<Room> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms
            .create(user.id, name, description, settings)
            .await
    }

    /// Get a room by its ID.
    pub async fn get_room(&self, room_id: RoomId) -> Result<Option<Room>> {
        self.rooms.get(room_id).await
    }

    /// List public rooms.
    pub async fn list_public_rooms(&self, pagination: Pagination) -> Result<Vec<Room>> {
        self.rooms.list_public(pagination).await
    }

    /// List rooms the user is a member of.
    pub async fn list_user_rooms(
        &self,
        session_id: SessionId,
        pagination: Pagination,
    ) -> Result<Vec<Room>> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.list_for_user(user.id, pagination).await
    }

    /// Join a room.
    pub async fn join_room(
        &self,
        session_id: SessionId,
        room_id: RoomId,
    ) -> Result<RoomMembership> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.join(user.id, room_id).await
    }

    /// Leave a room.
    pub async fn leave_room(&self, session_id: SessionId, room_id: RoomId) -> Result<()> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.leave(user.id, room_id).await
    }

    /// Update room settings.
    pub async fn update_room(
        &self,
        session_id: SessionId,
        room_id: RoomId,
        name: Option<&str>,
        description: Option<String>,
        settings: Option<RoomSettings>,
    ) -> Result<Room> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms
            .update(user.id, room_id, name, description, settings)
            .await
    }

    /// Delete a room.
    pub async fn delete_room(&self, session_id: SessionId, room_id: RoomId) -> Result<()> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.delete(user.id, room_id).await
    }

    /// Get room members.
    pub async fn get_room_members(&self, room_id: RoomId) -> Result<Vec<(User, RoomMembership)>> {
        self.rooms.get_members(room_id).await
    }

    // ========================================================================
    // Messaging Operations
    // ========================================================================

    /// Send a message.
    pub async fn send_message(
        &self,
        session_id: SessionId,
        target: MessageTarget,
        content: &str,
    ) -> Result<Message> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.messaging.send(user.id, target, content).await
    }

    /// Edit a message.
    pub async fn edit_message(
        &self,
        session_id: SessionId,
        message_id: MessageId,
        content: &str,
    ) -> Result<Message> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.messaging.edit(user.id, message_id, content).await
    }

    /// Delete a message.
    pub async fn delete_message(&self, session_id: SessionId, message_id: MessageId) -> Result<()> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.messaging.delete(user.id, message_id).await
    }

    /// Get messages for a target (room or DM).
    pub async fn get_messages(
        &self,
        session_id: SessionId,
        target: MessageTarget,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.messaging
            .get_messages(user.id, target, pagination)
            .await
    }

    // ========================================================================
    // Invitation Operations
    // ========================================================================

    /// Invite a user to a room.
    pub async fn invite_to_room(
        &self,
        session_id: SessionId,
        room_id: RoomId,
        invitee_id: UserId,
    ) -> Result<Invitation> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.invite(user.id, room_id, invitee_id).await
    }

    /// Accept a room invitation.
    pub async fn accept_invitation(
        &self,
        session_id: SessionId,
        invitation_id: InvitationId,
    ) -> Result<RoomMembership> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.accept_invitation(user.id, invitation_id).await
    }

    /// Decline a room invitation.
    pub async fn decline_invitation(
        &self,
        session_id: SessionId,
        invitation_id: InvitationId,
    ) -> Result<()> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.decline_invitation(user.id, invitation_id).await
    }

    /// List pending invitations for the current user.
    pub async fn list_invitations(&self, session_id: SessionId) -> Result<Vec<Invitation>> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.rooms.list_invitations(user.id).await
    }

    /// List pending invitations with enriched data (names resolved).
    pub async fn list_invitations_enriched(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<EnrichedInvitation>> {
        let invitations = self.list_invitations(session_id).await?;

        let mut enriched = Vec::with_capacity(invitations.len());
        for inv in &invitations {
            // Get room name
            let room = RoomRepository::find_by_id(self.storage.as_ref(), inv.room_id)
                .await?
                .ok_or(crate::Error::RoomNotFound)?;

            // Get inviter name
            let inviter = UserRepository::find_by_id(self.storage.as_ref(), inv.inviter)
                .await?
                .ok_or(crate::Error::UserNotFound)?;

            // Get invitee name
            let invitee = UserRepository::find_by_id(self.storage.as_ref(), inv.invitee)
                .await?
                .ok_or(crate::Error::UserNotFound)?;

            enriched.push(EnrichedInvitation::from_invitation(
                inv,
                room.name.to_string(),
                inviter.username.to_string(),
                invitee.username.to_string(),
            ));
        }

        Ok(enriched)
    }

    // ========================================================================
    // Presence Operations
    // ========================================================================

    /// Mark a user as online (called when connection established).
    pub async fn user_connected(&self, user_id: UserId, username: String) {
        self.events.user_online(user_id, username).await;
    }

    /// Mark a user as offline (called when last connection closed).
    pub async fn user_disconnected(&self, user_id: UserId, username: String) {
        self.events.user_offline(user_id, username).await;
    }

    /// Get list of online user IDs.
    pub async fn online_user_ids(&self) -> Vec<UserId> {
        self.events.online_users().await
    }

    /// Get count of online users.
    pub async fn online_user_count(&self) -> usize {
        self.events.online_user_count().await
    }

    /// Get total number of registered users.
    pub async fn total_user_count(&self) -> Result<u64> {
        UserRepository::count(&*self.storage).await
    }

    /// Get total number of rooms.
    pub async fn total_room_count(&self) -> Result<u64> {
        RoomRepository::count(&*self.storage).await
    }

    /// Check if a user is online.
    pub async fn is_user_online(&self, user_id: UserId) -> bool {
        self.events.is_online(user_id).await
    }

    /// Send a typing indicator.
    pub async fn send_typing(&self, session_id: SessionId, target: MessageTarget) -> Result<()> {
        let (_, user) = self.sessions.validate(session_id).await?;
        self.events.user_typing(user.id, target).await;
        Ok(())
    }

    // ========================================================================
    // Admin Operations
    // ========================================================================

    /// Get system statistics (admin only).
    pub async fn get_stats(&self, session_id: SessionId) -> Result<SystemStats> {
        let (_, user) = self.sessions.validate(session_id).await?;
        if !user.is_admin() {
            return Err(crate::Error::PermissionDenied);
        }
        self.collect_stats().await
    }

    /// Collect system statistics.
    async fn collect_stats(&self) -> Result<SystemStats> {
        let user_count = UserRepository::count(&*self.storage).await?;
        let room_count = RoomRepository::count(&*self.storage).await?;
        let online_count = self.events.online_user_count().await;

        Ok(SystemStats {
            total_users: user_count,
            total_rooms: room_count,
            online_users: online_count as u64,
        })
    }
}

// ============================================================================
// System Stats
// ============================================================================

/// System statistics for admin dashboard.
#[derive(Debug, Clone)]
pub struct SystemStats {
    /// Total registered users.
    pub total_users: u64,
    /// Total rooms.
    pub total_rooms: u64,
    /// Currently online users.
    pub online_users: u64,
}
