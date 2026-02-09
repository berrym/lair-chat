//! TCP command handler - maps protocol messages to core engine operations.

use std::sync::Arc;

use crate::core::engine::ChatEngine;
use crate::domain::{
    InvitationId, MessageId, MessageTarget, Pagination, RoomId, SessionId, UserId,
};
use crate::storage::{MembershipRepository, Storage};

use super::protocol::{
    ErrorInfo, RoomFilter, RoomListItem, RoomSettingsRequest, ServerMessage, SessionInfo,
    UserFilter,
};

/// Handles mapping protocol commands to engine operations.
pub struct CommandHandler<S: Storage> {
    engine: Arc<ChatEngine<S>>,
}

impl<S: Storage + 'static> CommandHandler<S> {
    /// Create a new command handler.
    pub fn new(engine: Arc<ChatEngine<S>>) -> Self {
        Self { engine }
    }

    // ========================================================================
    // Authentication
    // ========================================================================

    /// Handle authenticate request (JWT token validation).
    ///
    /// This is the recommended authentication method for TCP connections.
    /// The token should be obtained from HTTP POST /auth/login.
    pub async fn handle_authenticate(
        &self,
        token: &str,
        request_id: Option<String>,
    ) -> ServerMessage {
        match self.engine.validate_token(token).await {
            Ok((user, session)) => ServerMessage::AuthenticateResponse {
                request_id,
                success: true,
                user: Some(user),
                session: Some(SessionInfo::from(&session)),
                error: None,
            },
            Err(e) => ServerMessage::AuthenticateResponse {
                request_id,
                success: false,
                user: None,
                session: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle login request (DEPRECATED - use handle_authenticate).
    pub async fn handle_login(&self, identifier: &str, password: &str) -> ServerMessage {
        match self.engine.login(identifier, password).await {
            Ok((user, session, token)) => ServerMessage::LoginResponse {
                request_id: None,
                success: true,
                user: Some(user),
                session: Some(SessionInfo::from(&session)),
                token: Some(token),
                error: None,
            },
            Err(e) => ServerMessage::LoginResponse {
                request_id: None,
                success: false,
                user: None,
                session: None,
                token: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle register request (DEPRECATED - use HTTP /auth/register + handle_authenticate).
    pub async fn handle_register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> ServerMessage {
        match self.engine.register(username, email, password).await {
            Ok((user, session, token)) => ServerMessage::RegisterResponse {
                request_id: None,
                success: true,
                user: Some(user),
                session: Some(SessionInfo::from(&session)),
                token: Some(token),
                error: None,
            },
            Err(e) => ServerMessage::RegisterResponse {
                request_id: None,
                success: false,
                user: None,
                session: None,
                token: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle logout request.
    pub async fn handle_logout(&self, session_id: SessionId) {
        let _ = self.engine.logout(session_id).await;
    }

    // ========================================================================
    // Messaging
    // ========================================================================

    /// Handle send message request.
    pub async fn handle_send_message(
        &self,
        session_id: Option<SessionId>,
        target: &MessageTarget,
        content: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::SendMessageResponse {
                request_id: None,
                success: false,
                message: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        match self
            .engine
            .send_message(session_id, target.clone(), content)
            .await
        {
            Ok(message) => ServerMessage::SendMessageResponse {
                request_id: None,
                success: true,
                message: Some(message),
                error: None,
            },
            Err(e) => ServerMessage::SendMessageResponse {
                request_id: None,
                success: false,
                message: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle get messages request.
    pub async fn handle_get_messages(
        &self,
        session_id: Option<SessionId>,
        target: &MessageTarget,
        limit: u32,
        _before: Option<&str>,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::GetMessagesResponse {
                request_id: None,
                success: false,
                messages: None,
                has_more: false,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let pagination = Pagination {
            limit: limit.min(100),
            offset: 0,
        };

        match self
            .engine
            .get_messages(session_id, target.clone(), pagination)
            .await
        {
            Ok(messages) => {
                let has_more = messages.len() == limit as usize;
                ServerMessage::GetMessagesResponse {
                    request_id: None,
                    success: true,
                    messages: Some(messages),
                    has_more,
                    error: None,
                }
            }
            Err(e) => ServerMessage::GetMessagesResponse {
                request_id: None,
                success: false,
                messages: None,
                has_more: false,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle edit message request.
    pub async fn handle_edit_message(
        &self,
        session_id: Option<SessionId>,
        message_id: &str,
        content: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::EditMessageResponse {
                request_id: None,
                success: false,
                message: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let message_id = match MessageId::parse(message_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::EditMessageResponse {
                    request_id: None,
                    success: false,
                    message: None,
                    error: Some(ErrorInfo::new("invalid_id", "Invalid message ID")),
                };
            }
        };

        match self
            .engine
            .edit_message(session_id, message_id, content)
            .await
        {
            Ok(message) => ServerMessage::EditMessageResponse {
                request_id: None,
                success: true,
                message: Some(message),
                error: None,
            },
            Err(e) => ServerMessage::EditMessageResponse {
                request_id: None,
                success: false,
                message: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle delete message request.
    pub async fn handle_delete_message(
        &self,
        session_id: Option<SessionId>,
        message_id: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::DeleteMessageResponse {
                request_id: None,
                success: false,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let message_id = match MessageId::parse(message_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::DeleteMessageResponse {
                    request_id: None,
                    success: false,
                    error: Some(ErrorInfo::new("invalid_id", "Invalid message ID")),
                };
            }
        };

        match self.engine.delete_message(session_id, message_id).await {
            Ok(()) => ServerMessage::DeleteMessageResponse {
                request_id: None,
                success: true,
                error: None,
            },
            Err(e) => ServerMessage::DeleteMessageResponse {
                request_id: None,
                success: false,
                error: Some(error_to_info(&e)),
            },
        }
    }

    // ========================================================================
    // Rooms
    // ========================================================================

    /// Handle create room request.
    pub async fn handle_create_room(
        &self,
        session_id: Option<SessionId>,
        name: &str,
        description: Option<String>,
        settings: Option<RoomSettingsRequest>,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::CreateRoomResponse {
                request_id: None,
                success: false,
                room: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let room_settings = settings.map(|s| s.into());

        match self
            .engine
            .create_room(session_id, name, description, room_settings)
            .await
        {
            Ok(room) => ServerMessage::CreateRoomResponse {
                request_id: None,
                success: true,
                room: Some(room),
                error: None,
            },
            Err(e) => ServerMessage::CreateRoomResponse {
                request_id: None,
                success: false,
                room: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle join room request.
    pub async fn handle_join_room(
        &self,
        session_id: Option<SessionId>,
        room_id: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::JoinRoomResponse {
                request_id: None,
                success: false,
                room: None,
                membership: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let room_id = match RoomId::parse(room_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::JoinRoomResponse {
                    request_id: None,
                    success: false,
                    room: None,
                    membership: None,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid room ID")),
                };
            }
        };

        match self.engine.join_room(session_id, room_id).await {
            Ok(membership) => {
                let room = self.engine.get_room(room_id).await.ok().flatten();
                ServerMessage::JoinRoomResponse {
                    request_id: None,
                    success: true,
                    room,
                    membership: Some(membership),
                    error: None,
                }
            }
            Err(e) => ServerMessage::JoinRoomResponse {
                request_id: None,
                success: false,
                room: None,
                membership: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle leave room request.
    pub async fn handle_leave_room(
        &self,
        session_id: Option<SessionId>,
        room_id: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::LeaveRoomResponse {
                request_id: None,
                success: false,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let room_id = match RoomId::parse(room_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::LeaveRoomResponse {
                    request_id: None,
                    success: false,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid room ID")),
                };
            }
        };

        match self.engine.leave_room(session_id, room_id).await {
            Ok(()) => ServerMessage::LeaveRoomResponse {
                request_id: None,
                success: true,
                error: None,
            },
            Err(e) => ServerMessage::LeaveRoomResponse {
                request_id: None,
                success: false,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle list rooms request.
    pub async fn handle_list_rooms(
        &self,
        session_id: Option<SessionId>,
        filter: Option<RoomFilter>,
        limit: u32,
        offset: u32,
    ) -> ServerMessage {
        let pagination = Pagination {
            limit: limit.min(100),
            offset,
        };

        let filter = filter.unwrap_or_default();

        // If joined_only, need session
        if filter.joined_only {
            let Some(session_id) = session_id else {
                return ServerMessage::ListRoomsResponse {
                    request_id: None,
                    success: false,
                    rooms: None,
                    has_more: false,
                    total_count: 0,
                    error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
                };
            };

            match self
                .engine
                .list_user_rooms(session_id, Pagination::default())
                .await
            {
                Ok(rooms) => {
                    let mut items = Vec::with_capacity(rooms.len());
                    for room in rooms {
                        let member_count = MembershipRepository::count_members(
                            self.engine.storage_clone().as_ref(),
                            room.id,
                        )
                        .await
                        .unwrap_or(0);
                        items.push(RoomListItem {
                            room,
                            member_count,
                            is_member: true,
                        });
                    }
                    let total = items.len() as u32;
                    ServerMessage::ListRoomsResponse {
                        request_id: None,
                        success: true,
                        rooms: Some(items),
                        has_more: false,
                        total_count: total,
                        error: None,
                    }
                }
                Err(e) => ServerMessage::ListRoomsResponse {
                    request_id: None,
                    success: false,
                    rooms: None,
                    has_more: false,
                    total_count: 0,
                    error: Some(error_to_info(&e)),
                },
            }
        } else {
            // List public rooms
            // Get user_id if session is valid (for membership checking)
            let user_id = if let Some(session_id) = session_id {
                self.engine
                    .validate_session(session_id)
                    .await
                    .ok()
                    .map(|(_, user)| user.id)
            } else {
                None
            };

            match self.engine.list_public_rooms(pagination).await {
                Ok(rooms) => {
                    let has_more = rooms.len() == limit as usize;
                    let mut items = Vec::with_capacity(rooms.len());
                    for room in rooms {
                        let member_count = MembershipRepository::count_members(
                            self.engine.storage_clone().as_ref(),
                            room.id,
                        )
                        .await
                        .unwrap_or(0);
                        // Check if current user is a member
                        let is_member = if let Some(uid) = user_id {
                            MembershipRepository::is_member(
                                self.engine.storage_clone().as_ref(),
                                room.id,
                                uid,
                            )
                            .await
                            .unwrap_or(false)
                        } else {
                            false
                        };
                        items.push(RoomListItem {
                            room,
                            member_count,
                            is_member,
                        });
                    }
                    let total = items.len() as u32;
                    ServerMessage::ListRoomsResponse {
                        request_id: None,
                        success: true,
                        rooms: Some(items),
                        has_more,
                        total_count: total,
                        error: None,
                    }
                }
                Err(e) => ServerMessage::ListRoomsResponse {
                    request_id: None,
                    success: false,
                    rooms: None,
                    has_more: false,
                    total_count: 0,
                    error: Some(error_to_info(&e)),
                },
            }
        }
    }

    /// Handle get room request.
    pub async fn handle_get_room(
        &self,
        session_id: Option<SessionId>,
        room_id: &str,
    ) -> ServerMessage {
        let room_id = match RoomId::parse(room_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::GetRoomResponse {
                    request_id: None,
                    success: false,
                    room: None,
                    membership: None,
                    member_count: 0,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid room ID")),
                };
            }
        };

        // Get user ID from session to check membership
        let user_id = if let Some(sid) = session_id {
            self.engine
                .validate_session(sid)
                .await
                .ok()
                .map(|(_, u)| u.id)
        } else {
            None
        };

        match self.engine.get_room(room_id).await {
            Ok(Some(room)) => {
                let members = self.engine.get_room_members(room_id).await.ok();
                let member_count = members.as_ref().map(|m| m.len() as u32).unwrap_or(0);

                // Check if current user is a member
                let membership = if let Some(uid) = user_id {
                    self.engine
                        .storage_clone()
                        .get_membership(room_id, uid)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };

                ServerMessage::GetRoomResponse {
                    request_id: None,
                    success: true,
                    room: Some(room),
                    membership,
                    member_count,
                    error: None,
                }
            }
            Ok(None) => ServerMessage::GetRoomResponse {
                request_id: None,
                success: false,
                room: None,
                membership: None,
                member_count: 0,
                error: Some(ErrorInfo::new("not_found", "Room not found")),
            },
            Err(e) => ServerMessage::GetRoomResponse {
                request_id: None,
                success: false,
                room: None,
                membership: None,
                member_count: 0,
                error: Some(error_to_info(&e)),
            },
        }
    }

    // ========================================================================
    // Invitations
    // ========================================================================

    /// Handle list invitations request.
    pub async fn handle_list_invitations(&self, session_id: Option<SessionId>) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::ListInvitationsResponse {
                request_id: None,
                success: false,
                invitations: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        match self.engine.list_invitations_enriched(session_id).await {
            Ok(invitations) => ServerMessage::ListInvitationsResponse {
                request_id: None,
                success: true,
                invitations: Some(invitations),
                error: None,
            },
            Err(e) => ServerMessage::ListInvitationsResponse {
                request_id: None,
                success: false,
                invitations: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle accept invitation request.
    pub async fn handle_accept_invitation(
        &self,
        session_id: Option<SessionId>,
        invitation_id: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::AcceptInvitationResponse {
                request_id: None,
                success: false,
                membership: None,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let invitation_id = match InvitationId::parse(invitation_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::AcceptInvitationResponse {
                    request_id: None,
                    success: false,
                    membership: None,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid invitation ID")),
                };
            }
        };

        match self
            .engine
            .accept_invitation(session_id, invitation_id)
            .await
        {
            Ok(membership) => ServerMessage::AcceptInvitationResponse {
                request_id: None,
                success: true,
                membership: Some(membership),
                error: None,
            },
            Err(e) => ServerMessage::AcceptInvitationResponse {
                request_id: None,
                success: false,
                membership: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle decline invitation request.
    pub async fn handle_decline_invitation(
        &self,
        session_id: Option<SessionId>,
        invitation_id: &str,
    ) -> ServerMessage {
        let Some(session_id) = session_id else {
            return ServerMessage::DeclineInvitationResponse {
                request_id: None,
                success: false,
                error: Some(ErrorInfo::new("unauthorized", "Not authenticated")),
            };
        };

        let invitation_id = match InvitationId::parse(invitation_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::DeclineInvitationResponse {
                    request_id: None,
                    success: false,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid invitation ID")),
                };
            }
        };

        match self
            .engine
            .decline_invitation(session_id, invitation_id)
            .await
        {
            Ok(()) => ServerMessage::DeclineInvitationResponse {
                request_id: None,
                success: true,
                error: None,
            },
            Err(e) => ServerMessage::DeclineInvitationResponse {
                request_id: None,
                success: false,
                error: Some(error_to_info(&e)),
            },
        }
    }

    // ========================================================================
    // Users
    // ========================================================================

    /// Handle get user request.
    pub async fn handle_get_user(&self, user_id: &str) -> ServerMessage {
        let user_id = match UserId::parse(user_id) {
            Ok(id) => id,
            Err(_) => {
                return ServerMessage::GetUserResponse {
                    request_id: None,
                    success: false,
                    user: None,
                    error: Some(ErrorInfo::new("validation_failed", "Invalid user ID")),
                };
            }
        };

        match self.engine.get_user(user_id).await {
            Ok(Some(user)) => ServerMessage::GetUserResponse {
                request_id: None,
                success: true,
                user: Some(user),
                error: None,
            },
            Ok(None) => ServerMessage::GetUserResponse {
                request_id: None,
                success: false,
                user: None,
                error: Some(ErrorInfo::new("not_found", "User not found")),
            },
            Err(e) => ServerMessage::GetUserResponse {
                request_id: None,
                success: false,
                user: None,
                error: Some(error_to_info(&e)),
            },
        }
    }

    /// Handle list users request.
    pub async fn handle_list_users(
        &self,
        _filter: Option<UserFilter>,
        limit: u32,
        offset: u32,
    ) -> ServerMessage {
        let pagination = Pagination {
            limit: limit.min(100),
            offset,
        };

        match self.engine.list_users(pagination).await {
            Ok(users) => {
                let has_more = users.len() == limit as usize;
                let total = users.len() as u32;
                // Get currently online user IDs
                let online_ids: Vec<String> = self
                    .engine
                    .online_user_ids()
                    .await
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect();
                ServerMessage::ListUsersResponse {
                    request_id: None,
                    success: true,
                    users: Some(users),
                    online_user_ids: Some(online_ids),
                    has_more,
                    total_count: total,
                    error: None,
                }
            }
            Err(e) => ServerMessage::ListUsersResponse {
                request_id: None,
                success: false,
                users: None,
                online_user_ids: None,
                has_more: false,
                total_count: 0,
                error: Some(error_to_info(&e)),
            },
        }
    }

    // ========================================================================
    // Presence
    // ========================================================================

    /// Handle typing indicator.
    pub async fn handle_typing(&self, session_id: SessionId, target: &MessageTarget) {
        let _ = self.engine.send_typing(session_id, target.clone()).await;
    }

    /// Notify that a user connected.
    pub async fn user_connected(&self, user_id: UserId, username: String) {
        self.engine.user_connected(user_id, username).await;
    }

    /// Notify that a user disconnected.
    pub async fn user_disconnected(&self, user_id: UserId, username: String) {
        self.engine.user_disconnected(user_id, username).await;
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Convert an error to an ErrorInfo.
fn error_to_info(error: &crate::Error) -> ErrorInfo {
    ErrorInfo::new(error.code(), error.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::ChatEngine;
    use crate::storage::sqlite::SqliteStorage;
    use std::sync::Arc as StdArc;

    /// Test JWT secret for tests.
    const TEST_JWT_SECRET: &str = "test-jwt-secret-for-unit-tests-only";

    /// Create a test engine with in-memory SQLite database.
    async fn create_test_engine() -> StdArc<ChatEngine<SqliteStorage>> {
        let storage = SqliteStorage::in_memory().await.unwrap();
        let engine = ChatEngine::new(StdArc::new(storage), TEST_JWT_SECRET);
        StdArc::new(engine)
    }

    /// Create a command handler for testing.
    async fn create_test_handler() -> CommandHandler<SqliteStorage> {
        let engine = create_test_engine().await;
        CommandHandler::new(engine)
    }

    /// Register a test user and return their session ID (as SessionId) and user ID.
    async fn register_user(
        handler: &CommandHandler<SqliteStorage>,
        username: &str,
        email: &str,
        password: &str,
    ) -> (SessionId, UserId) {
        let response = handler.handle_register(username, email, password).await;
        match response {
            ServerMessage::RegisterResponse {
                success: true,
                user: Some(user),
                session: Some(session),
                ..
            } => {
                // Parse the session ID string back to SessionId
                let session_id = SessionId::parse(&session.id).unwrap();
                (session_id, user.id)
            }
            other => panic!("Registration failed: {:?}", other),
        }
    }

    // ========================================================================
    // Authentication Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_register_success() {
        let handler = create_test_handler().await;

        let response = handler
            .handle_register("alice", "alice@example.com", "password123")
            .await;

        match response {
            ServerMessage::RegisterResponse {
                success,
                user,
                session,
                token,
                error,
                ..
            } => {
                assert!(success);
                assert!(user.is_some());
                assert!(session.is_some());
                assert!(token.is_some());
                assert!(error.is_none());
                assert_eq!(user.unwrap().username.as_str(), "alice");
            }
            _ => panic!("Expected RegisterResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_register_duplicate_username() {
        let handler = create_test_handler().await;

        // Register first user
        let _ = handler
            .handle_register("alice", "alice1@example.com", "password123")
            .await;

        // Try to register with same username
        let response = handler
            .handle_register("alice", "alice2@example.com", "password123")
            .await;

        match response {
            ServerMessage::RegisterResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "username_taken");
            }
            _ => panic!("Expected RegisterResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_register_invalid_email() {
        let handler = create_test_handler().await;

        let response = handler
            .handle_register("bob", "not-an-email", "password123")
            .await;

        match response {
            ServerMessage::RegisterResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
            }
            _ => panic!("Expected RegisterResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_register_weak_password() {
        let handler = create_test_handler().await;

        let response = handler
            .handle_register("bob", "bob@example.com", "short")
            .await;

        match response {
            ServerMessage::RegisterResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "password_too_weak");
            }
            _ => panic!("Expected RegisterResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_login_success() {
        let handler = create_test_handler().await;

        // Register first
        let _ = handler
            .handle_register("alice", "alice@example.com", "password123")
            .await;

        // Login
        let response = handler.handle_login("alice", "password123").await;

        match response {
            ServerMessage::LoginResponse {
                success,
                user,
                session,
                token,
                error,
                ..
            } => {
                assert!(success);
                assert!(user.is_some());
                assert!(session.is_some());
                assert!(token.is_some());
                assert!(error.is_none());
            }
            _ => panic!("Expected LoginResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_login_invalid_credentials() {
        let handler = create_test_handler().await;

        // Register first
        let _ = handler
            .handle_register("alice", "alice@example.com", "password123")
            .await;

        // Login with wrong password
        let response = handler.handle_login("alice", "wrongpassword").await;

        match response {
            ServerMessage::LoginResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "invalid_credentials");
            }
            _ => panic!("Expected LoginResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_login_user_not_found() {
        let handler = create_test_handler().await;

        let response = handler.handle_login("nonexistent", "password123").await;

        match response {
            ServerMessage::LoginResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
            }
            _ => panic!("Expected LoginResponse"),
        }
    }

    // ========================================================================
    // Room Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_create_room_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler
            .handle_create_room(
                Some(session_id),
                "general",
                Some("General chat".to_string()),
                None,
            )
            .await;

        match response {
            ServerMessage::CreateRoomResponse {
                success,
                room,
                error,
                ..
            } => {
                assert!(success);
                assert!(room.is_some());
                assert!(error.is_none());
                assert_eq!(room.unwrap().name.as_str(), "general");
            }
            _ => panic!("Expected CreateRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_room_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler
            .handle_create_room(
                None, // No session
                "general", None, None,
            )
            .await;

        match response {
            ServerMessage::CreateRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected CreateRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_room_duplicate_name() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create first room
        let _ = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;

        // Try to create room with same name
        let response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;

        match response {
            ServerMessage::CreateRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "room_name_taken");
            }
            _ => panic!("Expected CreateRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_join_room_success() {
        let handler = create_test_handler().await;
        let (alice_session, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;
        let (bob_session, _) =
            register_user(&handler, "bob", "bob@example.com", "password123").await;

        // Alice creates a room
        let create_response = handler
            .handle_create_room(Some(alice_session), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id.to_string(),
            _ => panic!("Room creation failed"),
        };

        // Bob joins the room
        let response = handler.handle_join_room(Some(bob_session), &room_id).await;

        match response {
            ServerMessage::JoinRoomResponse {
                success,
                room,
                membership,
                error,
                ..
            } => {
                assert!(success);
                assert!(room.is_some());
                assert!(membership.is_some());
                assert!(error.is_none());
            }
            _ => panic!("Expected JoinRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_join_room_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler.handle_join_room(None, "some-room-id").await;

        match response {
            ServerMessage::JoinRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected JoinRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_join_room_invalid_id() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler
            .handle_join_room(Some(session_id), "not-a-uuid")
            .await;

        match response {
            ServerMessage::JoinRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "validation_failed");
            }
            _ => panic!("Expected JoinRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_leave_room_success() {
        let handler = create_test_handler().await;
        let (alice_session, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;
        let (bob_session, _) =
            register_user(&handler, "bob", "bob@example.com", "password123").await;

        // Alice creates a room
        let create_response = handler
            .handle_create_room(Some(alice_session), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id.to_string(),
            _ => panic!("Room creation failed"),
        };

        // Bob joins and then leaves
        let _ = handler.handle_join_room(Some(bob_session), &room_id).await;
        let response = handler.handle_leave_room(Some(bob_session), &room_id).await;

        match response {
            ServerMessage::LeaveRoomResponse { success, error, .. } => {
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected LeaveRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_leave_room_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler.handle_leave_room(None, "some-room-id").await;

        match response {
            ServerMessage::LeaveRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected LeaveRoomResponse"),
        }
    }

    // ========================================================================
    // Messaging Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_send_message_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create a room first
        let create_response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id,
            _ => panic!("Room creation failed"),
        };

        let target = MessageTarget::Room { room_id };
        let response = handler
            .handle_send_message(Some(session_id), &target, "Hello, world!")
            .await;

        match response {
            ServerMessage::SendMessageResponse {
                success,
                message,
                error,
                ..
            } => {
                assert!(success);
                assert!(message.is_some());
                assert!(error.is_none());
                assert_eq!(message.unwrap().content.as_str(), "Hello, world!");
            }
            _ => panic!("Expected SendMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_send_message_unauthorized() {
        let handler = create_test_handler().await;

        let target = MessageTarget::Room {
            room_id: RoomId::new(),
        };
        let response = handler.handle_send_message(None, &target, "Hello").await;

        match response {
            ServerMessage::SendMessageResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected SendMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_messages_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create a room and send a message
        let create_response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id,
            _ => panic!("Room creation failed"),
        };

        let target = MessageTarget::Room { room_id };
        let _ = handler
            .handle_send_message(Some(session_id), &target, "Hello!")
            .await;

        // Get messages
        let response = handler
            .handle_get_messages(Some(session_id), &target, 50, None)
            .await;

        match response {
            ServerMessage::GetMessagesResponse {
                success,
                messages,
                error,
                ..
            } => {
                assert!(success);
                assert!(messages.is_some());
                assert!(error.is_none());
                assert!(!messages.unwrap().is_empty());
            }
            _ => panic!("Expected GetMessagesResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_messages_unauthorized() {
        let handler = create_test_handler().await;

        let target = MessageTarget::Room {
            room_id: RoomId::new(),
        };
        let response = handler.handle_get_messages(None, &target, 50, None).await;

        match response {
            ServerMessage::GetMessagesResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected GetMessagesResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_edit_message_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create a room and send a message
        let create_response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id,
            _ => panic!("Room creation failed"),
        };

        let target = MessageTarget::Room { room_id };
        let send_response = handler
            .handle_send_message(Some(session_id), &target, "Hello!")
            .await;
        let message_id = match send_response {
            ServerMessage::SendMessageResponse {
                message: Some(msg), ..
            } => msg.id.to_string(),
            _ => panic!("Send message failed"),
        };

        // Edit the message
        let response = handler
            .handle_edit_message(Some(session_id), &message_id, "Hello, edited!")
            .await;

        match response {
            ServerMessage::EditMessageResponse {
                success,
                message,
                error,
                ..
            } => {
                assert!(success);
                assert!(message.is_some());
                assert!(error.is_none());
                assert_eq!(message.unwrap().content.as_str(), "Hello, edited!");
            }
            _ => panic!("Expected EditMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_edit_message_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler.handle_edit_message(None, "some-id", "Hello").await;

        match response {
            ServerMessage::EditMessageResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected EditMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_edit_message_invalid_id() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler
            .handle_edit_message(Some(session_id), "not-a-uuid", "Hello")
            .await;

        match response {
            ServerMessage::EditMessageResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "invalid_id");
            }
            _ => panic!("Expected EditMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_delete_message_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create a room and send a message
        let create_response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id,
            _ => panic!("Room creation failed"),
        };

        let target = MessageTarget::Room { room_id };
        let send_response = handler
            .handle_send_message(Some(session_id), &target, "Hello!")
            .await;
        let message_id = match send_response {
            ServerMessage::SendMessageResponse {
                message: Some(msg), ..
            } => msg.id.to_string(),
            _ => panic!("Send message failed"),
        };

        // Delete the message
        let response = handler
            .handle_delete_message(Some(session_id), &message_id)
            .await;

        match response {
            ServerMessage::DeleteMessageResponse { success, error, .. } => {
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected DeleteMessageResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_delete_message_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler.handle_delete_message(None, "some-id").await;

        match response {
            ServerMessage::DeleteMessageResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected DeleteMessageResponse"),
        }
    }

    // ========================================================================
    // Room Listing Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_list_rooms_empty() {
        let handler = create_test_handler().await;

        // List rooms without authentication (public rooms only)
        let response = handler.handle_list_rooms(None, None, 50, 0).await;

        match response {
            ServerMessage::ListRoomsResponse {
                success,
                rooms,
                total_count,
                ..
            } => {
                assert!(success);
                assert!(rooms.is_some());
                assert_eq!(rooms.unwrap().len(), 0);
                assert_eq!(total_count, 0);
            }
            _ => panic!("Expected ListRoomsResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_rooms_with_rooms() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create some rooms
        let _ = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let _ = handler
            .handle_create_room(Some(session_id), "random", None, None)
            .await;

        // List all rooms
        let response = handler
            .handle_list_rooms(Some(session_id), None, 50, 0)
            .await;

        match response {
            ServerMessage::ListRoomsResponse { success, rooms, .. } => {
                assert!(success);
                assert!(rooms.is_some());
                assert_eq!(rooms.unwrap().len(), 2);
            }
            _ => panic!("Expected ListRoomsResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_rooms_joined_only() {
        let handler = create_test_handler().await;
        let (alice_session, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;
        let (bob_session, _) =
            register_user(&handler, "bob", "bob@example.com", "password123").await;

        // Alice creates rooms
        let _ = handler
            .handle_create_room(Some(alice_session), "general", None, None)
            .await;
        let _ = handler
            .handle_create_room(Some(alice_session), "random", None, None)
            .await;

        // Bob lists only joined rooms (should be empty)
        let filter = RoomFilter {
            joined_only: true,
            ..Default::default()
        };
        let response = handler
            .handle_list_rooms(Some(bob_session), Some(filter), 50, 0)
            .await;

        match response {
            ServerMessage::ListRoomsResponse { success, rooms, .. } => {
                assert!(success);
                assert!(rooms.is_some());
                // Bob hasn't joined any rooms yet
                assert_eq!(rooms.unwrap().len(), 0);
            }
            _ => panic!("Expected ListRoomsResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_rooms_joined_only_unauthorized() {
        let handler = create_test_handler().await;

        let filter = RoomFilter {
            joined_only: true,
            ..Default::default()
        };
        let response = handler.handle_list_rooms(None, Some(filter), 50, 0).await;

        match response {
            ServerMessage::ListRoomsResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected ListRoomsResponse"),
        }
    }

    // ========================================================================
    // User Listing Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_list_users() {
        let handler = create_test_handler().await;

        // Register some users
        let _ = register_user(&handler, "alice", "alice@example.com", "password123").await;
        let _ = register_user(&handler, "bob", "bob@example.com", "password123").await;

        let response = handler.handle_list_users(None, 50, 0).await;

        match response {
            ServerMessage::ListUsersResponse { success, users, .. } => {
                assert!(success);
                assert!(users.is_some());
                assert_eq!(users.unwrap().len(), 2);
            }
            _ => panic!("Expected ListUsersResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_user_success() {
        let handler = create_test_handler().await;
        let (_, alice_id) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler.handle_get_user(&alice_id.to_string()).await;

        match response {
            ServerMessage::GetUserResponse {
                success,
                user,
                error,
                ..
            } => {
                assert!(success);
                assert!(user.is_some());
                assert!(error.is_none());
                assert_eq!(user.unwrap().username.as_str(), "alice");
            }
            _ => panic!("Expected GetUserResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_user_not_found() {
        let handler = create_test_handler().await;

        let fake_id = UserId::new();
        let response = handler.handle_get_user(&fake_id.to_string()).await;

        match response {
            ServerMessage::GetUserResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "not_found");
            }
            _ => panic!("Expected GetUserResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_user_invalid_id() {
        let handler = create_test_handler().await;

        let response = handler.handle_get_user("not-a-uuid").await;

        match response {
            ServerMessage::GetUserResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "validation_failed");
            }
            _ => panic!("Expected GetUserResponse"),
        }
    }

    // ========================================================================
    // Invitation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_list_invitations_unauthorized() {
        let handler = create_test_handler().await;

        let response = handler.handle_list_invitations(None).await;

        match response {
            ServerMessage::ListInvitationsResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "unauthorized");
            }
            _ => panic!("Expected ListInvitationsResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_invitations_empty() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler.handle_list_invitations(Some(session_id)).await;

        match response {
            ServerMessage::ListInvitationsResponse {
                success,
                invitations,
                ..
            } => {
                assert!(success);
                assert!(invitations.is_some());
                assert!(invitations.unwrap().is_empty());
            }
            _ => panic!("Expected ListInvitationsResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_accept_invitation_invalid_id() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler
            .handle_accept_invitation(Some(session_id), "not-a-uuid")
            .await;

        match response {
            ServerMessage::AcceptInvitationResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "validation_failed");
            }
            _ => panic!("Expected AcceptInvitationResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_decline_invitation_invalid_id() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let response = handler
            .handle_decline_invitation(Some(session_id), "not-a-uuid")
            .await;

        match response {
            ServerMessage::DeclineInvitationResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "validation_failed");
            }
            _ => panic!("Expected DeclineInvitationResponse"),
        }
    }

    // ========================================================================
    // Room Details Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_get_room_success() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        // Create a room
        let create_response = handler
            .handle_create_room(Some(session_id), "general", None, None)
            .await;
        let room_id = match create_response {
            ServerMessage::CreateRoomResponse {
                room: Some(room), ..
            } => room.id.to_string(),
            _ => panic!("Room creation failed"),
        };

        // Get room details
        let response = handler.handle_get_room(Some(session_id), &room_id).await;

        match response {
            ServerMessage::GetRoomResponse {
                success,
                room,
                member_count,
                error,
                ..
            } => {
                assert!(success);
                assert!(room.is_some());
                assert!(error.is_none());
                assert_eq!(member_count, 1); // Only the creator
                assert_eq!(room.unwrap().name.as_str(), "general");
            }
            _ => panic!("Expected GetRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_room_not_found() {
        let handler = create_test_handler().await;
        let (session_id, _) =
            register_user(&handler, "alice", "alice@example.com", "password123").await;

        let fake_id = RoomId::new();
        let response = handler
            .handle_get_room(Some(session_id), &fake_id.to_string())
            .await;

        match response {
            ServerMessage::GetRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "not_found");
            }
            _ => panic!("Expected GetRoomResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_room_invalid_id() {
        let handler = create_test_handler().await;

        let response = handler.handle_get_room(None, "not-a-uuid").await;

        match response {
            ServerMessage::GetRoomResponse { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert_eq!(error.unwrap().code, "validation_failed");
            }
            _ => panic!("Expected GetRoomResponse"),
        }
    }

    // ========================================================================
    // Error Conversion Tests
    // ========================================================================

    #[test]
    fn test_error_to_info() {
        let error = crate::Error::InvalidCredentials;
        let info = error_to_info(&error);

        assert_eq!(info.code, "invalid_credentials");
        assert!(!info.message.is_empty());
    }

    #[test]
    fn test_error_to_info_with_details() {
        let error = crate::Error::ValidationFailed {
            field: "email".to_string(),
            reason: "invalid format".to_string(),
        };
        let info = error_to_info(&error);

        assert_eq!(info.code, "validation_failed");
        assert!(info.message.contains("email"));
    }
}
