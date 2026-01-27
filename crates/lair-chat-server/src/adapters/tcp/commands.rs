//! TCP command handler - maps protocol messages to core engine operations.

use std::sync::Arc;

use crate::core::engine::ChatEngine;
use crate::domain::{InvitationId, MessageTarget, Pagination, RoomId, SessionId, UserId};
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

            match self.engine.list_user_rooms(session_id).await {
                Ok(rooms) => {
                    let mut items = Vec::with_capacity(rooms.len());
                    for room in rooms {
                        let member_count = MembershipRepository::count_members(
                            self.engine.storage_clone().as_ref(),
                            room.id,
                        )
                        .await
                        .unwrap_or(0) as u32;
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
                        .unwrap_or(0) as u32;
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
        _session_id: Option<SessionId>,
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

        match self.engine.get_room(room_id).await {
            Ok(Some(room)) => {
                let members = self.engine.get_room_members(room_id).await.ok();
                let member_count = members.as_ref().map(|m| m.len() as u32).unwrap_or(0);
                ServerMessage::GetRoomResponse {
                    request_id: None,
                    success: true,
                    room: Some(room),
                    membership: None, // TODO: Check if current user is member
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

        match self.engine.list_invitations(session_id).await {
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
