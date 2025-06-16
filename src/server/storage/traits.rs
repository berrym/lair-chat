//! Storage trait definitions for lair-chat server
//!
//! This module defines the trait interfaces for all storage operations,
//! providing a database-agnostic API for user management, message storage,
//! room management, and session handling.

use super::{models::*, OrderBy, Pagination, StorageResult};
use async_trait::async_trait;

/// User storage operations
#[async_trait]
pub trait UserStorage: Send + Sync {
    /// Create a new user
    async fn create_user(&self, user: User) -> StorageResult<User>;

    /// Get user by ID
    async fn get_user_by_id(&self, id: &str) -> StorageResult<Option<User>>;

    /// Get user by username
    async fn get_user_by_username(&self, username: &str) -> StorageResult<Option<User>>;

    /// Get user by email
    async fn get_user_by_email(&self, email: &str) -> StorageResult<Option<User>>;

    /// Update user information
    async fn update_user(&self, user: User) -> StorageResult<User>;

    /// Update user's last seen timestamp
    async fn update_last_seen(&self, user_id: &str, timestamp: u64) -> StorageResult<()>;

    /// Update user password
    async fn update_password(
        &self,
        user_id: &str,
        password_hash: &str,
        salt: &str,
    ) -> StorageResult<()>;

    /// Update user profile
    async fn update_profile(&self, user_id: &str, profile: UserProfile) -> StorageResult<()>;

    /// Update user settings
    async fn update_settings(&self, user_id: &str, settings: UserSettings) -> StorageResult<()>;

    /// Update user role
    async fn update_role(&self, user_id: &str, role: UserRole) -> StorageResult<()>;

    /// Deactivate user account
    async fn deactivate_user(&self, user_id: &str) -> StorageResult<()>;

    /// Reactivate user account
    async fn reactivate_user(&self, user_id: &str) -> StorageResult<()>;

    /// Delete user (hard delete)
    async fn delete_user(&self, user_id: &str) -> StorageResult<()>;

    /// List users with pagination
    async fn list_users(
        &self,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<User>>;

    /// Search users by username or display name
    async fn search_users(&self, query: &str, pagination: Pagination) -> StorageResult<Vec<User>>;

    /// Get users by role
    async fn get_users_by_role(
        &self,
        role: UserRole,
        pagination: Pagination,
    ) -> StorageResult<Vec<User>>;

    /// Get active users (recently seen)
    async fn get_active_users(&self, since: u64) -> StorageResult<Vec<User>>;

    /// Count total users
    async fn count_users(&self) -> StorageResult<u64>;

    /// Count active users
    async fn count_active_users(&self, since: u64) -> StorageResult<u64>;

    /// Check if username exists
    async fn username_exists(&self, username: &str) -> StorageResult<bool>;

    /// Check if email exists
    async fn email_exists(&self, email: &str) -> StorageResult<bool>;

    /// Get user statistics
    async fn get_user_stats(&self) -> StorageResult<UserStats>;
}

/// Message storage operations
#[async_trait]
pub trait MessageStorage: Send + Sync {
    /// Store a new message
    async fn store_message(&self, message: Message) -> StorageResult<Message>;

    /// Get message by ID
    async fn get_message_by_id(&self, id: &str) -> StorageResult<Option<Message>>;

    /// Update message content (for edits)
    async fn update_message(&self, message: Message) -> StorageResult<Message>;

    /// Delete message (soft delete)
    async fn delete_message(&self, message_id: &str, deleted_at: u64) -> StorageResult<()>;

    /// Hard delete message
    async fn hard_delete_message(&self, message_id: &str) -> StorageResult<()>;

    /// Get messages in a room
    async fn get_room_messages(
        &self,
        room_id: &str,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Message>>;

    /// Get messages by user
    async fn get_user_messages(
        &self,
        user_id: &str,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Message>>;

    /// Get messages in date range
    async fn get_messages_in_range(
        &self,
        room_id: &str,
        start_time: u64,
        end_time: u64,
        pagination: Pagination,
    ) -> StorageResult<Vec<Message>>;

    /// Get messages after a specific message (for real-time sync)
    async fn get_messages_after(
        &self,
        room_id: &str,
        after_message_id: &str,
        limit: u64,
    ) -> StorageResult<Vec<Message>>;

    /// Get messages before a specific message (for history loading)
    async fn get_messages_before(
        &self,
        room_id: &str,
        before_message_id: &str,
        limit: u64,
    ) -> StorageResult<Vec<Message>>;

    /// Search messages by content
    async fn search_messages(&self, query: SearchQuery) -> StorageResult<SearchResult>;

    /// Get message thread (replies to a message)
    async fn get_message_thread(
        &self,
        parent_message_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Message>>;

    /// Add reaction to message
    async fn add_reaction(&self, message_id: &str, reaction: MessageReaction) -> StorageResult<()>;

    /// Remove reaction from message
    async fn remove_reaction(
        &self,
        message_id: &str,
        user_id: &str,
        reaction: &str,
    ) -> StorageResult<()>;

    /// Add read receipt
    async fn add_read_receipt(
        &self,
        message_id: &str,
        receipt: MessageReadReceipt,
    ) -> StorageResult<()>;

    /// Get unread messages for user in room
    async fn get_unread_messages(
        &self,
        user_id: &str,
        room_id: &str,
        since: u64,
    ) -> StorageResult<Vec<Message>>;

    /// Mark messages as read
    async fn mark_messages_read(
        &self,
        user_id: &str,
        room_id: &str,
        up_to_message_id: &str,
        timestamp: u64,
    ) -> StorageResult<()>;

    /// Count messages in room
    async fn count_room_messages(&self, room_id: &str) -> StorageResult<u64>;

    /// Count messages by user
    async fn count_user_messages(&self, user_id: &str) -> StorageResult<u64>;

    /// Count total messages
    async fn count_messages(&self) -> StorageResult<u64>;

    /// Delete old messages (for retention policy)
    async fn delete_old_messages(&self, before_timestamp: u64) -> StorageResult<u64>;

    /// Get message statistics
    async fn get_message_stats(&self) -> StorageResult<MessageStats>;
}

/// Room storage operations
#[async_trait]
pub trait RoomStorage: Send + Sync {
    /// Create a new room
    async fn create_room(&self, room: Room) -> StorageResult<Room>;

    /// Get room by ID
    async fn get_room_by_id(&self, id: &str) -> StorageResult<Option<Room>>;

    /// Get room by name
    async fn get_room_by_name(&self, name: &str) -> StorageResult<Option<Room>>;

    /// Update room information
    async fn update_room(&self, room: Room) -> StorageResult<Room>;

    /// Update room settings
    async fn update_room_settings(
        &self,
        room_id: &str,
        settings: RoomSettings,
    ) -> StorageResult<()>;

    /// Deactivate room
    async fn deactivate_room(&self, room_id: &str) -> StorageResult<()>;

    /// Reactivate room
    async fn reactivate_room(&self, room_id: &str) -> StorageResult<()>;

    /// Delete room (hard delete)
    async fn delete_room(&self, room_id: &str) -> StorageResult<()>;

    /// List rooms with pagination
    async fn list_rooms(
        &self,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Room>>;

    /// List public rooms
    async fn list_public_rooms(&self, pagination: Pagination) -> StorageResult<Vec<Room>>;

    /// List rooms by type
    async fn list_rooms_by_type(
        &self,
        room_type: RoomType,
        pagination: Pagination,
    ) -> StorageResult<Vec<Room>>;

    /// Search rooms by name or description
    async fn search_rooms(&self, query: &str, pagination: Pagination) -> StorageResult<Vec<Room>>;

    /// Get rooms created by user
    async fn get_user_created_rooms(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Room>>;

    /// Add user to room
    async fn add_room_member(&self, membership: RoomMembership) -> StorageResult<RoomMembership>;

    /// Remove user from room
    async fn remove_room_member(&self, room_id: &str, user_id: &str) -> StorageResult<()>;

    /// Update room member role
    async fn update_member_role(
        &self,
        room_id: &str,
        user_id: &str,
        role: RoomRole,
    ) -> StorageResult<()>;

    /// Update room member settings
    async fn update_member_settings(
        &self,
        room_id: &str,
        user_id: &str,
        settings: RoomMemberSettings,
    ) -> StorageResult<()>;

    /// Get room membership
    async fn get_room_membership(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> StorageResult<Option<RoomMembership>>;

    /// List room members
    async fn list_room_members(
        &self,
        room_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<RoomMembership>>;

    /// List user's room memberships
    async fn list_user_memberships(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<RoomMembership>>;

    /// Get active room members
    async fn get_active_room_members(
        &self,
        room_id: &str,
        since: u64,
    ) -> StorageResult<Vec<RoomMembership>>;

    /// Count room members
    async fn count_room_members(&self, room_id: &str) -> StorageResult<u64>;

    /// Count total rooms
    async fn count_rooms(&self) -> StorageResult<u64>;

    /// Check if room name exists
    async fn room_name_exists(&self, name: &str) -> StorageResult<bool>;

    /// Check if user is room member
    async fn is_room_member(&self, room_id: &str, user_id: &str) -> StorageResult<bool>;

    /// Get room statistics
    async fn get_room_stats(&self) -> StorageResult<RoomStats>;
}

/// Session storage operations
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Create a new session
    async fn create_session(&self, session: Session) -> StorageResult<Session>;

    /// Get session by ID
    async fn get_session_by_id(&self, id: &str) -> StorageResult<Option<Session>>;

    /// Get session by token
    async fn get_session_by_token(&self, token: &str) -> StorageResult<Option<Session>>;

    /// Update session activity
    async fn update_session_activity(&self, session_id: &str, timestamp: u64) -> StorageResult<()>;

    /// Update session metadata
    async fn update_session_metadata(
        &self,
        session_id: &str,
        metadata: SessionMetadata,
    ) -> StorageResult<()>;

    /// Deactivate session
    async fn deactivate_session(&self, session_id: &str) -> StorageResult<()>;

    /// Delete session
    async fn delete_session(&self, session_id: &str) -> StorageResult<()>;

    /// Get user sessions
    async fn get_user_sessions(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Session>>;

    /// Get active user sessions
    async fn get_active_user_sessions(&self, user_id: &str) -> StorageResult<Vec<Session>>;

    /// Deactivate all user sessions
    async fn deactivate_user_sessions(&self, user_id: &str) -> StorageResult<u64>;

    /// Deactivate all user sessions except specified one
    async fn deactivate_user_sessions_except(
        &self,
        user_id: &str,
        except_session_id: &str,
    ) -> StorageResult<u64>;

    /// Update entire session
    async fn update_session(&self, session: &Session) -> StorageResult<()>;

    /// Get session by UUID
    async fn get_session(&self, session_id: &str) -> StorageResult<Option<Session>>;

    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) -> StorageResult<u64>;

    /// Count active sessions
    async fn count_active_sessions(&self) -> StorageResult<u64>;

    /// Count user sessions
    async fn count_user_sessions(&self, user_id: &str) -> StorageResult<u64>;

    /// Get session statistics
    async fn get_session_stats(&self) -> StorageResult<SessionStats>;
}

/// User statistics
#[derive(Debug, Clone)]
pub struct UserStats {
    pub total_users: u64,
    pub active_users: u64,
    pub new_users_today: u64,
    pub new_users_this_week: u64,
    pub new_users_this_month: u64,
    pub users_by_role: std::collections::HashMap<String, u64>,
}

/// Message statistics
#[derive(Debug, Clone)]
pub struct MessageStats {
    pub total_messages: u64,
    pub messages_today: u64,
    pub messages_this_week: u64,
    pub messages_this_month: u64,
    pub messages_by_type: std::collections::HashMap<String, u64>,
    pub most_active_rooms: Vec<(String, u64)>,
    pub most_active_users: Vec<(String, u64)>,
}

/// Room statistics
#[derive(Debug, Clone)]
pub struct RoomStats {
    pub total_rooms: u64,
    pub active_rooms: u64,
    pub public_rooms: u64,
    pub private_rooms: u64,
    pub rooms_by_type: std::collections::HashMap<String, u64>,
    pub average_members_per_room: f64,
    pub largest_rooms: Vec<(String, u64)>,
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub sessions_today: u64,
    pub sessions_this_week: u64,
    pub sessions_by_client: std::collections::HashMap<String, u64>,
    pub average_session_duration: f64,
}
