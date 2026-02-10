//! WebSocket handler for real-time web clients.
//!
//! Provides TCP protocol parity over WebSocket, allowing browser clients
//! to use the same message format as native TCP clients.
//!
//! ## Differences from TCP
//!
//! - No length-prefix framing (WebSocket handles message boundaries)
//! - Runs on HTTP port (via upgrade)
//! - Supports pre-authentication via query parameter: `/ws?token=JWT`
//! - No encryption negotiation (TLS handled at HTTP layer)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::adapters::http::routes::AppState;
use crate::adapters::tcp::commands::CommandHandler;
use crate::adapters::tcp::protocol::{ClientMessage, ServerMessage, PROTOCOL_VERSION};
use crate::core::events::{should_receive_event, EventDispatcher};
use crate::domain::events::{Event, EventPayload};
use crate::domain::{Pagination, Protocol, RoomId, Session, SessionId, User, UserId};
use crate::storage::{RoomRepository, Storage, UserRepository};

/// Timeout for handshake completion.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for authentication after handshake.
const AUTH_TIMEOUT: Duration = Duration::from_secs(60);

/// Timeout for receiving any message (keepalive).
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Query parameters for WebSocket upgrade.
#[derive(Debug, Deserialize)]
pub struct WsUpgradeParams {
    /// Optional JWT token for pre-authentication.
    /// If provided, the connection skips the handshake and auth phases.
    pub token: Option<String>,
}

/// WebSocket connection state.
#[derive(Debug, Clone, PartialEq)]
enum WsConnectionState {
    /// Waiting for client hello.
    AwaitingHandshake,
    /// Handshake complete, waiting for authentication.
    AwaitingAuth,
    /// Fully authenticated and operational.
    Authenticated,
    /// Connection is closing.
    Closing,
}

/// Handler for WebSocket upgrade at GET /ws.
pub async fn ws_upgrade<S: Storage + Clone + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<AppState<S>>,
    Query(params): Query<WsUpgradeParams>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state, params, addr))
}

/// Handle a WebSocket connection.
async fn handle_ws_connection<S: Storage + Clone + 'static>(
    socket: WebSocket,
    state: AppState<S>,
    params: WsUpgradeParams,
    addr: SocketAddr,
) {
    info!("New WebSocket connection from {}", addr);

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for outgoing messages
    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<ServerMessage>(100);

    // Spawn writer task
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = outgoing_rx.recv().await {
            match msg.to_json() {
                Ok(json) => {
                    if ws_sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Create connection handler
    let mut conn = WsConnection {
        addr,
        state: WsConnectionState::AwaitingHandshake,
        user: None,
        session: None,
        commands: CommandHandler::new(state.engine.clone()),
        outgoing_tx: outgoing_tx.clone(),
        event_task: None,
        storage: state.engine.storage_clone(),
        events: state.engine.events_clone(),
    };

    // Send server hello
    if let Err(e) = conn.send(ServerMessage::server_hello()).await {
        error!("Failed to send server hello to {}: {}", addr, e);
        return;
    }

    // Handle pre-authentication if token provided
    if let Some(token) = params.token {
        if conn.handle_pre_auth(&token).await.is_ok() {
            info!("WebSocket pre-authenticated from {}", addr);
        }
    }

    // Process messages
    let result = conn.process_messages(&mut ws_receiver).await;

    // Cleanup
    conn.cleanup().await;

    // Close writer
    drop(outgoing_tx);
    let _ = writer_handle.await;

    match result {
        Ok(()) => info!("WebSocket connection from {} closed normally", addr),
        Err(e) => {
            if e != "Connection closed" {
                warn!(
                    "WebSocket connection from {} closed with error: {}",
                    addr, e
                );
            }
        }
    }
}

/// WebSocket connection handler.
struct WsConnection<S: Storage> {
    /// Client address.
    addr: SocketAddr,
    /// Connection state.
    state: WsConnectionState,
    /// Authenticated user (if any).
    user: Option<User>,
    /// Active session (if any).
    session: Option<Session>,
    /// Command handler for processing requests.
    commands: CommandHandler<S>,
    /// Channel for sending outgoing messages.
    outgoing_tx: mpsc::Sender<ServerMessage>,
    /// Event listener task handle (spawned after authentication).
    event_task: Option<tokio::task::JoinHandle<()>>,
    /// Storage backend for fetching room memberships.
    storage: Arc<S>,
    /// Event dispatcher for subscribing to events.
    events: EventDispatcher,
}

impl<S: Storage + 'static> WsConnection<S> {
    /// Send a message through the outgoing channel.
    async fn send(&self, msg: ServerMessage) -> Result<(), String> {
        self.outgoing_tx
            .send(msg)
            .await
            .map_err(|_| "Connection closed".to_string())
    }

    /// Handle pre-authentication with a JWT token.
    async fn handle_pre_auth(&mut self, token: &str) -> Result<(), String> {
        let response = self.commands.handle_authenticate(token, None).await;

        if let ServerMessage::AuthenticateResponse {
            success: true,
            user: Some(ref user),
            session: Some(ref session_info),
            ..
        } = response
        {
            if let Ok(session_id) = SessionId::parse(&session_info.id) {
                self.user = Some(user.clone());
                let mut session = Session::new(user.id, Protocol::WebSocket);
                session.id = session_id;
                self.session = Some(session);
                self.state = WsConnectionState::Authenticated;

                self.commands
                    .user_connected(user.id, user.username.to_string())
                    .await;
                self.spawn_event_listener(user.id);

                // Send auth response to confirm authentication
                self.send(response).await?;
                return Ok(());
            }
        }

        // Send the failed response
        self.send(response).await?;
        Err("Pre-authentication failed".to_string())
    }

    /// Process incoming WebSocket messages.
    async fn process_messages(
        &mut self,
        receiver: &mut futures::stream::SplitStream<WebSocket>,
    ) -> Result<(), String> {
        loop {
            // Determine timeout based on state
            let read_timeout = match self.state {
                WsConnectionState::AwaitingHandshake => HANDSHAKE_TIMEOUT,
                WsConnectionState::AwaitingAuth => AUTH_TIMEOUT,
                WsConnectionState::Authenticated => IDLE_TIMEOUT,
                WsConnectionState::Closing => return Ok(()),
            };

            // Read with timeout
            let msg = match timeout(read_timeout, receiver.next()).await {
                Ok(Some(Ok(msg))) => msg,
                Ok(Some(Err(e))) => {
                    return Err(format!("WebSocket error: {}", e));
                }
                Ok(None) => {
                    return Err("Connection closed".to_string());
                }
                Err(_) => {
                    warn!(
                        "WebSocket connection {} timed out in state {:?}",
                        self.addr, self.state
                    );
                    let _ = self
                        .send(ServerMessage::error(
                            None,
                            "timeout",
                            "Connection timed out",
                        ))
                        .await;
                    return Ok(());
                }
            };

            // Handle message
            match msg {
                Message::Text(text) => {
                    if let Err(e) = self.handle_text_message(&text).await {
                        error!("Error handling message from {}: {}", self.addr, e);
                    }
                }
                Message::Binary(_) => {
                    // Binary messages not supported
                    let _ = self
                        .send(ServerMessage::error(
                            None,
                            "unsupported",
                            "Binary messages not supported",
                        ))
                        .await;
                }
                Message::Ping(data) => {
                    // Axum handles pong automatically, but we can respond too
                    debug!("Received ping from {}", self.addr);
                    let _ = data; // Acknowledge receipt
                }
                Message::Pong(_) => {
                    debug!("Received pong from {}", self.addr);
                }
                Message::Close(_) => {
                    return Ok(());
                }
            }

            // Check if we should close
            if self.state == WsConnectionState::Closing {
                return Ok(());
            }
        }
    }

    /// Handle a text message (JSON).
    async fn handle_text_message(&mut self, text: &str) -> Result<(), String> {
        // Parse the message
        let msg = match ClientMessage::parse(text) {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Invalid message from {}: {}", self.addr, e);
                self.send(ServerMessage::error(
                    None,
                    "invalid_message",
                    &e.to_string(),
                ))
                .await?;
                return Ok(());
            }
        };

        // Handle based on state
        match &self.state {
            WsConnectionState::AwaitingHandshake => self.handle_handshake(msg).await,
            WsConnectionState::AwaitingAuth => self.handle_auth(msg).await,
            WsConnectionState::Authenticated => self.handle_authenticated(msg).await,
            WsConnectionState::Closing => Ok(()),
        }
    }

    /// Handle messages during handshake phase.
    async fn handle_handshake(&mut self, msg: ClientMessage) -> Result<(), String> {
        match msg {
            ClientMessage::ClientHello { version, .. } => {
                // Check version compatibility
                if !version.starts_with("1.") {
                    self.send(ServerMessage::error(
                        None,
                        "version_mismatch",
                        &format!(
                            "Unsupported protocol version: {}. Server supports: {}",
                            version, PROTOCOL_VERSION
                        ),
                    ))
                    .await?;
                    self.state = WsConnectionState::Closing;
                    return Ok(());
                }

                // WebSocket doesn't need encryption negotiation (TLS at HTTP layer)
                debug!(
                    "WebSocket handshake complete with {} (version {})",
                    self.addr, version
                );
                self.state = WsConnectionState::AwaitingAuth;
                Ok(())
            }
            ClientMessage::Ping => self.send(ServerMessage::pong()).await,
            _ => {
                self.send(ServerMessage::error(
                    None,
                    "unauthorized",
                    "Must complete handshake first",
                ))
                .await
            }
        }
    }

    /// Handle messages during authentication phase.
    async fn handle_auth(&mut self, msg: ClientMessage) -> Result<(), String> {
        match msg {
            ClientMessage::Authenticate { request_id, token } => {
                let response = self
                    .commands
                    .handle_authenticate(&token, request_id.clone())
                    .await;

                if let ServerMessage::AuthenticateResponse {
                    success: true,
                    user: Some(ref user),
                    session: Some(ref session_info),
                    ..
                } = response
                {
                    if let Ok(session_id) = SessionId::parse(&session_info.id) {
                        self.user = Some(user.clone());
                        let mut session = Session::new(user.id, Protocol::WebSocket);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = WsConnectionState::Authenticated;
                        info!(
                            "WebSocket user {} authenticated from {}",
                            user.username, self.addr
                        );

                        self.commands
                            .user_connected(user.id, user.username.to_string())
                            .await;
                        self.spawn_event_listener(user.id);
                    }
                }

                self.send(response).await
            }
            // Legacy login/register - still supported but deprecated
            ClientMessage::Login {
                request_id: _,
                identifier,
                password,
            } => {
                warn!(
                    "WebSocket client {} using deprecated Login. Use HTTP + Authenticate.",
                    self.addr
                );
                let response = self.commands.handle_login(&identifier, &password).await;

                if let ServerMessage::LoginResponse {
                    success: true,
                    user: Some(ref user),
                    session: Some(ref session_info),
                    ..
                } = response
                {
                    if let Ok(session_id) = SessionId::parse(&session_info.id) {
                        self.user = Some(user.clone());
                        let mut session = Session::new(user.id, Protocol::WebSocket);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = WsConnectionState::Authenticated;
                        info!(
                            "WebSocket user {} logged in from {}",
                            user.username, self.addr
                        );

                        self.commands
                            .user_connected(user.id, user.username.to_string())
                            .await;
                        self.spawn_event_listener(user.id);
                    }
                }

                self.send(response).await
            }
            ClientMessage::Register {
                request_id: _,
                username,
                email,
                password,
            } => {
                warn!(
                    "WebSocket client {} using deprecated Register. Use HTTP + Authenticate.",
                    self.addr
                );
                let response = self
                    .commands
                    .handle_register(&username, &email, &password)
                    .await;

                if let ServerMessage::RegisterResponse {
                    success: true,
                    user: Some(ref user),
                    session: Some(ref session_info),
                    ..
                } = response
                {
                    if let Ok(session_id) = SessionId::parse(&session_info.id) {
                        self.user = Some(user.clone());
                        let mut session = Session::new(user.id, Protocol::WebSocket);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = WsConnectionState::Authenticated;
                        info!(
                            "New WebSocket user {} registered from {}",
                            user.username, self.addr
                        );

                        self.commands
                            .user_connected(user.id, user.username.to_string())
                            .await;
                        self.spawn_event_listener(user.id);
                    }
                }

                self.send(response).await
            }
            ClientMessage::Ping => self.send(ServerMessage::pong()).await,
            _ => {
                self.send(ServerMessage::error(
                    None,
                    "unauthorized",
                    "Must authenticate first",
                ))
                .await
            }
        }
    }

    /// Handle messages when authenticated.
    async fn handle_authenticated(&mut self, msg: ClientMessage) -> Result<(), String> {
        let user = self.user.as_ref().expect("Must be authenticated");
        let session_id = self.session.as_ref().map(|s| s.id);

        let response = match msg {
            ClientMessage::Ping => {
                return self.send(ServerMessage::pong()).await;
            }
            ClientMessage::Logout { request_id } => {
                if let Some(sid) = session_id {
                    self.commands.handle_logout(sid).await;
                }
                self.state = WsConnectionState::Closing;
                ServerMessage::LogoutResponse {
                    request_id,
                    success: true,
                }
            }
            ClientMessage::SendMessage {
                request_id: _,
                target,
                content,
            } => {
                self.commands
                    .handle_send_message(session_id, &target, &content)
                    .await
            }
            ClientMessage::EditMessage {
                request_id: _,
                message_id,
                content,
            } => {
                self.commands
                    .handle_edit_message(session_id, &message_id, &content)
                    .await
            }
            ClientMessage::DeleteMessage {
                request_id: _,
                message_id,
            } => {
                self.commands
                    .handle_delete_message(session_id, &message_id)
                    .await
            }
            ClientMessage::GetMessages {
                request_id: _,
                target,
                limit,
                before,
            } => {
                self.commands
                    .handle_get_messages(session_id, &target, limit, before.as_deref())
                    .await
            }
            ClientMessage::CreateRoom {
                request_id: _,
                name,
                description,
                settings,
            } => {
                self.commands
                    .handle_create_room(session_id, &name, description, settings)
                    .await
            }
            ClientMessage::JoinRoom {
                request_id: _,
                room_id,
            } => self.commands.handle_join_room(session_id, &room_id).await,
            ClientMessage::LeaveRoom {
                request_id: _,
                room_id,
            } => self.commands.handle_leave_room(session_id, &room_id).await,
            ClientMessage::ListRooms {
                request_id: _,
                filter,
                limit,
                offset,
            } => {
                self.commands
                    .handle_list_rooms(session_id, filter, limit, offset)
                    .await
            }
            ClientMessage::GetRoom {
                request_id: _,
                room_id,
            } => self.commands.handle_get_room(session_id, &room_id).await,
            ClientMessage::ListInvitations { request_id: _ } => {
                self.commands.handle_list_invitations(session_id).await
            }
            ClientMessage::AcceptInvitation {
                request_id: _,
                invitation_id,
            } => {
                self.commands
                    .handle_accept_invitation(session_id, &invitation_id)
                    .await
            }
            ClientMessage::DeclineInvitation {
                request_id: _,
                invitation_id,
            } => {
                self.commands
                    .handle_decline_invitation(session_id, &invitation_id)
                    .await
            }
            ClientMessage::GetCurrentUser { request_id } => ServerMessage::GetCurrentUserResponse {
                request_id,
                success: true,
                user: Some(user.clone()),
                error: None,
            },
            ClientMessage::GetUser {
                request_id: _,
                user_id,
            } => self.commands.handle_get_user(&user_id).await,
            ClientMessage::ListUsers {
                request_id: _,
                filter,
                limit,
                offset,
            } => self.commands.handle_list_users(filter, limit, offset).await,
            ClientMessage::Typing { target } => {
                if let Some(sid) = session_id {
                    let _ = self.commands.handle_typing(sid, &target).await;
                }
                return Ok(()); // No response for typing
            }
            // Messages that shouldn't be sent when authenticated
            ClientMessage::ClientHello { .. }
            | ClientMessage::Authenticate { .. }
            | ClientMessage::Login { .. }
            | ClientMessage::Register { .. } => {
                ServerMessage::error(None, "invalid_state", "Already authenticated")
            }
            // Key exchange not needed for WebSocket (TLS at HTTP layer)
            ClientMessage::KeyExchange { .. } => ServerMessage::error(
                None,
                "not_supported",
                "Key exchange not needed for WebSocket",
            ),
            ClientMessage::InviteToRoom {
                request_id: _,
                room_id,
                user_id,
                message: _,
            } => {
                self.commands
                    .handle_invite_to_room(session_id, &room_id, &user_id)
                    .await
            }
        };

        self.send(response).await
    }

    /// Clean up when connection closes.
    async fn cleanup(&mut self) {
        // Abort the event listener task if running
        if let Some(handle) = self.event_task.take() {
            handle.abort();
        }

        if let Some(user) = &self.user {
            info!(
                "Cleaning up WebSocket connection for user {}",
                user.username
            );
            self.commands
                .user_disconnected(user.id, user.username.to_string())
                .await;
        }
    }

    /// Convert a domain event to a protocol ServerMessage.
    fn event_to_server_message(event: &Event) -> Option<ServerMessage> {
        match &event.payload {
            // MessageReceived is handled separately to include author_username
            EventPayload::MessageReceived(_) => None,
            EventPayload::MessageEdited(e) => Some(ServerMessage::MessageEdited {
                message: e.message.clone(),
                previous_content: e.previous_content.clone(),
            }),
            EventPayload::MessageDeleted(e) => Some(ServerMessage::MessageDeleted {
                message_id: e.message_id.to_string(),
                target: e.target.clone(),
                deleted_by: e.deleted_by.to_string(),
            }),
            EventPayload::UserJoinedRoom(e) => Some(ServerMessage::UserJoinedRoom {
                room_id: e.room_id.to_string(),
                user: e.user.clone(),
                membership: e.membership.clone(),
            }),
            EventPayload::UserLeftRoom(e) => Some(ServerMessage::UserLeftRoom {
                room_id: e.room_id.to_string(),
                user_id: e.user_id.to_string(),
                reason: e.reason.to_string(),
            }),
            EventPayload::MemberRoleChanged(e) => Some(ServerMessage::MemberRoleChanged {
                room_id: e.room_id.to_string(),
                user_id: e.user_id.to_string(),
                username: e.username.clone(),
                old_role: e.old_role.to_string(),
                new_role: e.new_role.to_string(),
                changed_by: e.changed_by.to_string(),
            }),
            EventPayload::RoomUpdated(e) => Some(ServerMessage::RoomUpdated {
                room: e.room.clone(),
                changed_by: e.changed_by.to_string(),
            }),
            EventPayload::RoomDeleted(e) => Some(ServerMessage::RoomDeleted {
                room_id: e.room_id.to_string(),
                room_name: e.room_name.clone(),
                deleted_by: e.deleted_by.to_string(),
            }),
            EventPayload::UserOnline(e) => Some(ServerMessage::UserOnline {
                user_id: e.user_id.to_string(),
                username: e.username.clone(),
            }),
            EventPayload::UserOffline(e) => Some(ServerMessage::UserOffline {
                user_id: e.user_id.to_string(),
                username: e.username.clone(),
            }),
            EventPayload::UserTyping(e) => Some(ServerMessage::UserTyping {
                user_id: e.user_id.to_string(),
                target: e.target.clone(),
            }),
            EventPayload::InvitationReceived(e) => Some(ServerMessage::InvitationReceived {
                invitation: e.invitation.clone(),
            }),
            EventPayload::ServerNotice(e) => Some(ServerMessage::ServerNotice {
                message: e.message.clone(),
                severity: e.severity.to_string(),
            }),
            // Session-specific events are not broadcast to connections
            EventPayload::InvitationCancelled(_) | EventPayload::SessionExpiring(_) => None,
        }
    }

    /// Event listener task - receives events and forwards to client.
    async fn event_listener_task(
        mut event_rx: tokio::sync::broadcast::Receiver<Event>,
        outgoing_tx: mpsc::Sender<ServerMessage>,
        user_id: UserId,
        storage: Arc<S>,
    ) {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    debug!(
                        "WebSocket event listener for user {} received event: {:?}",
                        user_id,
                        event.payload.event_type()
                    );

                    // Fetch user's current room memberships for filtering
                    let user_rooms: Vec<RoomId> = match RoomRepository::list_for_user(
                        &*storage,
                        user_id,
                        Pagination {
                            offset: 0,
                            limit: u32::MAX,
                        },
                    )
                    .await
                    {
                        Ok(rooms) => rooms.into_iter().map(|r| r.id).collect(),
                        Err(e) => {
                            debug!("Failed to fetch user rooms for event filtering: {}", e);
                            Vec::new()
                        }
                    };

                    // Check if this user should receive this event
                    if !should_receive_event(&event, user_id, &user_rooms) {
                        debug!(
                            "Event filtered out for WebSocket user {}: {:?}",
                            user_id,
                            event.payload.event_type()
                        );
                        continue;
                    }

                    // Handle MessageReceived specially to include author username
                    let msg = if let EventPayload::MessageReceived(e) = &event.payload {
                        let author_username =
                            match UserRepository::find_by_id(&*storage, e.message.author).await {
                                Ok(Some(user)) => user.username.to_string(),
                                _ => e.message.author.to_string(),
                            };
                        Some(ServerMessage::MessageReceived {
                            message: e.message.clone(),
                            author_username,
                        })
                    } else {
                        Self::event_to_server_message(&event)
                    };

                    // Send to client
                    if let Some(msg) = msg {
                        if outgoing_tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("WebSocket event listener lagged, missed {} events", n);
                }
            }
        }
    }

    /// Spawn the event listener task after successful authentication.
    fn spawn_event_listener(&mut self, user_id: UserId) {
        info!("Spawning WebSocket event listener for user {}", user_id);
        let event_rx = self.events.subscribe();
        let outgoing_tx = self.outgoing_tx.clone();
        let storage = self.storage.clone();

        let handle = tokio::spawn(Self::event_listener_task(
            event_rx,
            outgoing_tx,
            user_id,
            storage,
        ));

        self.event_task = Some(handle);
    }
}
