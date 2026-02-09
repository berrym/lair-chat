//! TCP connection handler.
//!
//! Handles the lifecycle of a single TCP client connection:
//! - Handshake
//! - Authentication
//! - Message processing
//! - Event delivery
//! - Cleanup

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use tokio::io::{BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::core::engine::ChatEngine;
use crate::core::events::{should_receive_event, EventDispatcher};
use crate::crypto::{parse_public_key, Cipher, KeyPair};
use crate::domain::events::{Event, EventPayload};
use crate::domain::{Pagination, Protocol, RoomId, Session, SessionId, User};
use crate::storage::{RoomRepository, Storage, UserRepository};

use super::commands::CommandHandler;
use super::protocol::{
    read_encrypted_message, read_message, write_encrypted_message, write_message, ClientMessage,
    ProtocolError, ServerMessage, PROTOCOL_VERSION,
};

/// Timeout for handshake completion.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for authentication after handshake.
const AUTH_TIMEOUT: Duration = Duration::from_secs(60);

/// Timeout for receiving any message (keepalive).
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Connection state.
#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    /// Waiting for client hello.
    AwaitingHandshake,
    /// Waiting for key exchange (client requested encryption).
    AwaitingKeyExchange,
    /// Handshake complete, waiting for authentication.
    AwaitingAuth,
    /// Fully authenticated and operational.
    Authenticated,
    /// Connection is closing.
    Closing,
}

/// Encryption state shared between connection handler and writer task.
///
/// The `pending` flag handles the race condition during key exchange:
/// - When cipher is first set, `pending` is true (not yet active for writes)
/// - After the writer sends a message with pending cipher, it sets pending=false
/// - This ensures KeyExchangeResponse is sent unencrypted
#[derive(Default)]
struct EncryptionState {
    cipher: Option<Arc<Cipher>>,
    /// True when cipher is set but not yet active for writing.
    /// The writer task will activate it after sending the next message.
    pending: bool,
}

/// A single TCP client connection.
pub struct Connection<S: Storage> {
    /// Client address.
    addr: SocketAddr,
    /// Connection state.
    state: ConnectionState,
    /// Authenticated user (if any).
    user: Option<User>,
    /// Active session (if any).
    session: Option<Session>,
    /// Command handler for processing requests.
    commands: CommandHandler<S>,
    /// Channel for sending outgoing messages.
    outgoing_tx: mpsc::Sender<ServerMessage>,
    /// Whether encryption is enabled for this connection.
    encryption_enabled: bool,
    /// Server's keypair for key exchange (consumed during exchange).
    keypair: Option<KeyPair>,
    /// Encryption state (shared with writer task).
    encryption_state: Arc<RwLock<EncryptionState>>,
    /// Event listener task handle (spawned after authentication).
    event_task: Option<tokio::task::JoinHandle<()>>,
    /// Storage backend for fetching room memberships.
    storage: Arc<S>,
    /// Event dispatcher for subscribing to events.
    events: EventDispatcher,
}

impl<S: Storage + 'static> Connection<S> {
    /// Handle a new TCP connection.
    pub async fn handle(stream: TcpStream, addr: SocketAddr, engine: Arc<ChatEngine<S>>) {
        info!("New connection from {}", addr);

        // Split the stream for concurrent read/write
        let (read_half, write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut writer = BufWriter::new(write_half);

        // Create outgoing message channel
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<ServerMessage>(100);

        // Create encryption state holder (shared with writer task)
        let encryption_state = Arc::new(RwLock::new(EncryptionState::default()));

        // Create connection handler
        let mut conn = Connection {
            addr,
            state: ConnectionState::AwaitingHandshake,
            user: None,
            session: None,
            commands: CommandHandler::new(engine.clone()),
            outgoing_tx: outgoing_tx.clone(),
            encryption_enabled: false,
            keypair: None,
            encryption_state: encryption_state.clone(),
            event_task: None,
            storage: engine.storage_clone(),
            events: engine.events_clone(),
        };

        // Send server hello
        if let Err(e) = conn.send_server_hello(&mut writer).await {
            error!("Failed to send server hello to {}: {}", addr, e);
            return;
        }

        // Spawn writer task with encryption state for encrypted writes
        let writer_encryption = encryption_state;
        let writer_handle =
            tokio::spawn(
                async move { Self::writer_task(writer, outgoing_rx, writer_encryption).await },
            );

        // Process messages
        let result = conn
            .process_messages(&mut reader, outgoing_tx.clone())
            .await;

        // Cleanup
        conn.cleanup().await;

        // Close writer
        drop(outgoing_tx);
        let _ = writer_handle.await;

        match result {
            Ok(()) => info!("Connection from {} closed normally", addr),
            Err(e) => {
                if !matches!(e, ProtocolError::ConnectionClosed) {
                    warn!("Connection from {} closed with error: {}", addr, e);
                }
            }
        }
    }

    /// Writer task - sends messages from the channel to the socket.
    async fn writer_task(
        mut writer: BufWriter<OwnedWriteHalf>,
        mut rx: mpsc::Receiver<ServerMessage>,
        encryption_state: Arc<RwLock<EncryptionState>>,
    ) {
        while let Some(msg) = rx.recv().await {
            match msg.to_json() {
                Ok(json) => {
                    // Get cipher and pending state
                    let (cipher_opt, was_pending) = {
                        let state = encryption_state.read().unwrap();
                        // Only use cipher for encryption if it's not pending
                        // (pending means KeyExchangeResponse hasn't been sent yet)
                        let cipher = if state.pending {
                            None // Don't encrypt while pending
                        } else {
                            state.cipher.as_ref().cloned()
                        };
                        (cipher, state.pending)
                    };

                    let result = match cipher_opt {
                        Some(c) => write_encrypted_message(&mut writer, &json, &c).await,
                        None => write_message(&mut writer, &json).await,
                    };

                    // If cipher was pending, activate it now (after writing unencrypted)
                    if was_pending {
                        let mut state = encryption_state.write().unwrap();
                        if state.pending {
                            state.pending = false;
                            debug!("Encryption activated for subsequent messages");
                        }
                    }

                    if let Err(e) = result {
                        error!("Failed to write message: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    }

    /// Send server hello message.
    async fn send_server_hello(
        &self,
        writer: &mut BufWriter<OwnedWriteHalf>,
    ) -> Result<(), ProtocolError> {
        let hello = ServerMessage::server_hello();
        let json = hello.to_json()?;
        write_message(writer, &json).await
    }

    /// Send a message through the outgoing channel.
    async fn send(&self, msg: ServerMessage) -> Result<(), ProtocolError> {
        self.outgoing_tx
            .send(msg)
            .await
            .map_err(|_| ProtocolError::ConnectionClosed)
    }

    /// Process incoming messages.
    async fn process_messages(
        &mut self,
        reader: &mut BufReader<OwnedReadHalf>,
        _outgoing_tx: mpsc::Sender<ServerMessage>,
    ) -> Result<(), ProtocolError> {
        loop {
            // Determine timeout based on state
            let read_timeout = match self.state {
                ConnectionState::AwaitingHandshake => HANDSHAKE_TIMEOUT,
                ConnectionState::AwaitingKeyExchange => HANDSHAKE_TIMEOUT,
                ConnectionState::AwaitingAuth => AUTH_TIMEOUT,
                ConnectionState::Authenticated => IDLE_TIMEOUT,
                ConnectionState::Closing => return Ok(()),
            };

            // Read with timeout - use encrypted read if encryption is enabled
            // Clone the cipher Arc before releasing lock (if encryption enabled and not pending)
            let cipher_opt = if self.encryption_enabled {
                let state = self.encryption_state.read().unwrap();
                // Only use cipher for reading if it's active (not pending)
                // Note: for reading, we should always decrypt after key exchange,
                // because the client will start encrypting after receiving KeyExchangeResponse
                state.cipher.as_ref().cloned()
            } else {
                None
            };

            let json = match cipher_opt {
                Some(cipher) => {
                    match timeout(read_timeout, read_encrypted_message(reader, &cipher)).await {
                        Ok(Ok(json)) => json,
                        Ok(Err(e)) => return Err(e),
                        Err(_) => {
                            warn!(
                                "Connection {} timed out in state {:?}",
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
                    }
                }
                None => match timeout(read_timeout, read_message(reader)).await {
                    Ok(Ok(json)) => json,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        warn!(
                            "Connection {} timed out in state {:?}",
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
                },
            };

            // Parse message
            let msg = match ClientMessage::parse(&json) {
                Ok(msg) => msg,
                Err(e) => {
                    warn!("Invalid message from {}: {}", self.addr, e);
                    self.send(ServerMessage::error(
                        None,
                        "invalid_message",
                        &e.to_string(),
                    ))
                    .await?;
                    continue;
                }
            };

            // Handle message
            if let Err(e) = self.handle_message(msg).await {
                error!("Error handling message from {}: {}", self.addr, e);
                // Don't break on handler errors, just continue
            }

            // Check if we should close
            if self.state == ConnectionState::Closing {
                return Ok(());
            }
        }
    }

    /// Handle a single client message.
    async fn handle_message(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
        match &self.state {
            ConnectionState::AwaitingHandshake => self.handle_handshake(msg).await,
            ConnectionState::AwaitingKeyExchange => self.handle_key_exchange(msg).await,
            ConnectionState::AwaitingAuth => self.handle_auth(msg).await,
            ConnectionState::Authenticated => self.handle_authenticated(msg).await,
            ConnectionState::Closing => Ok(()),
        }
    }

    /// Handle messages during handshake phase.
    async fn handle_handshake(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
        match msg {
            ClientMessage::ClientHello {
                version, features, ..
            } => {
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
                    self.state = ConnectionState::Closing;
                    return Ok(());
                }

                // Check if client wants encryption
                let wants_encryption = features.iter().any(|f| f == "encryption");

                if wants_encryption {
                    debug!(
                        "Client {} requested encryption, generating keypair",
                        self.addr
                    );
                    // Generate server keypair for key exchange
                    self.keypair = Some(KeyPair::generate());
                    self.state = ConnectionState::AwaitingKeyExchange;

                    debug!(
                        "Handshake complete with {} (version {}), awaiting key exchange",
                        self.addr, version
                    );
                } else {
                    debug!(
                        "Handshake complete with {} (version {}), no encryption",
                        self.addr, version
                    );
                    self.state = ConnectionState::AwaitingAuth;
                }

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

    /// Handle key exchange message.
    async fn handle_key_exchange(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
        match msg {
            ClientMessage::KeyExchange { public_key } => {
                // Parse client's public key
                let client_public = parse_public_key(&public_key).map_err(|e| {
                    ProtocolError::KeyExchangeFailed(format!("Invalid client public key: {}", e))
                })?;

                // Get server's keypair
                let keypair = self.keypair.take().ok_or_else(|| {
                    ProtocolError::KeyExchangeFailed("Server keypair not available".to_string())
                })?;

                // Send our public key to client
                let server_public = keypair.public_key_base64();
                self.send(ServerMessage::KeyExchangeResponse {
                    public_key: server_public,
                })
                .await?;

                // Derive shared secret
                let shared_secret = keypair.diffie_hellman(client_public);

                // Create cipher from shared secret and store in shared holder.
                // Set pending=true so the KeyExchangeResponse is sent unencrypted.
                // The writer task will clear pending after sending that message.
                {
                    let mut state = self.encryption_state.write().unwrap();
                    state.cipher = Some(Arc::new(Cipher::new(&shared_secret)));
                    state.pending = true;
                }
                self.encryption_enabled = true;

                info!("Encryption enabled for connection from {}", self.addr);
                self.state = ConnectionState::AwaitingAuth;
                Ok(())
            }
            ClientMessage::Ping => self.send(ServerMessage::pong()).await,
            _ => {
                self.send(ServerMessage::error(
                    None,
                    "unauthorized",
                    "Must complete key exchange first",
                ))
                .await
            }
        }
    }

    /// Handle messages during authentication phase.
    async fn handle_auth(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
        match msg {
            // Recommended: Token-based authentication using JWT from HTTP API
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
                        let mut session = Session::new(user.id, Protocol::Tcp);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = ConnectionState::Authenticated;
                        info!(
                            "User {} authenticated via token from {}",
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
            // DEPRECATED: Direct login - use HTTP + Authenticate instead
            ClientMessage::Login {
                request_id: _,
                identifier,
                password,
            } => {
                warn!(
                    "Client {} using deprecated Login command. Use HTTP POST /auth/login + Authenticate instead.",
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
                    // Store session info for later use
                    if let Ok(session_id) = SessionId::parse(&session_info.id) {
                        self.user = Some(user.clone());
                        // Create a session object with the actual ID from the response
                        let mut session = Session::new(user.id, Protocol::Tcp);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = ConnectionState::Authenticated;
                        info!("User {} authenticated from {}", user.username, self.addr);

                        // Notify engine of user connection
                        self.commands
                            .user_connected(user.id, user.username.to_string())
                            .await;

                        // Spawn event listener to receive real-time updates
                        self.spawn_event_listener(user.id);
                    }
                }

                self.send(response).await
            }
            // DEPRECATED: Direct register - use HTTP + Authenticate instead
            ClientMessage::Register {
                request_id: _,
                username,
                email,
                password,
            } => {
                warn!(
                    "Client {} using deprecated Register command. Use HTTP POST /auth/register + Authenticate instead.",
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
                        // Create a session object with the actual ID from the response
                        let mut session = Session::new(user.id, Protocol::Tcp);
                        session.id = session_id;
                        self.session = Some(session);
                        self.state = ConnectionState::Authenticated;
                        info!("New user {} registered from {}", user.username, self.addr);

                        self.commands
                            .user_connected(user.id, user.username.to_string())
                            .await;

                        // Spawn event listener to receive real-time updates
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
    async fn handle_authenticated(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
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
                self.state = ConnectionState::Closing;
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
            // DEPRECATED: Use HTTP GET /messages instead
            ClientMessage::GetMessages {
                request_id: _,
                target,
                limit,
                before,
            } => {
                warn!("Client using deprecated GetMessages. Use HTTP GET /messages instead.");
                self.commands
                    .handle_get_messages(session_id, &target, limit, before.as_deref())
                    .await
            }
            // DEPRECATED: Use HTTP POST /rooms instead
            ClientMessage::CreateRoom {
                request_id: _,
                name,
                description,
                settings,
            } => {
                warn!("Client using deprecated CreateRoom. Use HTTP POST /rooms instead.");
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
            // DEPRECATED: Use HTTP GET /rooms instead
            ClientMessage::ListRooms {
                request_id: _,
                filter,
                limit,
                offset,
            } => {
                warn!("Client using deprecated ListRooms. Use HTTP GET /rooms instead.");
                self.commands
                    .handle_list_rooms(session_id, filter, limit, offset)
                    .await
            }
            // DEPRECATED: Use HTTP GET /rooms/{room_id} instead
            ClientMessage::GetRoom {
                request_id: _,
                room_id,
            } => {
                warn!("Client using deprecated GetRoom. Use HTTP GET /rooms/{{room_id}} instead.");
                self.commands.handle_get_room(session_id, &room_id).await
            }
            // DEPRECATED: Use HTTP GET /invitations instead
            ClientMessage::ListInvitations { request_id: _ } => {
                warn!(
                    "Client using deprecated ListInvitations. Use HTTP GET /invitations instead."
                );
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
            // DEPRECATED: Use HTTP GET /users/me instead
            ClientMessage::GetCurrentUser { request_id } => {
                warn!("Client using deprecated GetCurrentUser. Use HTTP GET /users/me instead.");
                ServerMessage::GetCurrentUserResponse {
                    request_id,
                    success: true,
                    user: Some(user.clone()),
                    error: None,
                }
            }
            // DEPRECATED: Use HTTP GET /users/{user_id} instead
            ClientMessage::GetUser {
                request_id: _,
                user_id,
            } => {
                warn!("Client using deprecated GetUser. Use HTTP GET /users/{{user_id}} instead.");
                self.commands.handle_get_user(&user_id).await
            }
            // DEPRECATED: Use HTTP GET /users instead
            ClientMessage::ListUsers {
                request_id: _,
                filter,
                limit,
                offset,
            } => {
                warn!("Client using deprecated ListUsers. Use HTTP GET /users instead.");
                self.commands.handle_list_users(filter, limit, offset).await
            }
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
            // Unimplemented
            _ => ServerMessage::error(None, "not_implemented", "Operation not yet implemented"),
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
            info!("Cleaning up connection for user {}", user.username);
            self.commands
                .user_disconnected(user.id, user.username.to_string())
                .await;
        }
    }

    /// Convert a domain event to a protocol ServerMessage.
    /// For MessageReceived, use `convert_message_received_event` instead to include author username.
    fn event_to_server_message(event: &Event) -> Option<ServerMessage> {
        match &event.payload {
            // MessageReceived is handled separately in event_listener_task to include author_username
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
        user_id: crate::domain::UserId,
        storage: Arc<S>,
    ) {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    // Log event receipt for debugging
                    debug!(
                        "Event listener for user {} received event: {:?}",
                        user_id,
                        event.payload.event_type()
                    );

                    // Fetch user's current room memberships for filtering
                    let user_rooms: Vec<RoomId> =
                        match RoomRepository::list_for_user(&*storage, user_id, Pagination { offset: 0, limit: u32::MAX }).await {
                            Ok(rooms) => rooms.into_iter().map(|r| r.id).collect(),
                            Err(e) => {
                                debug!("Failed to fetch user rooms for event filtering: {}", e);
                                Vec::new()
                            }
                        };

                    // Check if this user should receive this event
                    if !should_receive_event(&event, user_id, &user_rooms) {
                        debug!(
                            "Event filtered out for user {}: {:?}",
                            user_id,
                            event.payload.event_type()
                        );
                        continue;
                    }

                    debug!(
                        "Event passed filter for user {}: {:?}",
                        user_id,
                        event.payload.event_type()
                    );

                    // Handle MessageReceived specially to include author username
                    let msg = if let EventPayload::MessageReceived(e) = &event.payload {
                        // Look up author username
                        let author_username =
                            match UserRepository::find_by_id(&*storage, e.message.author).await {
                                Ok(Some(user)) => user.username.to_string(),
                                _ => e.message.author.to_string(), // Fallback to ID
                            };
                        Some(ServerMessage::MessageReceived {
                            message: e.message.clone(),
                            author_username,
                        })
                    } else {
                        Self::event_to_server_message(&event)
                    };

                    // Convert to server message and send
                    if let Some(msg) = msg {
                        if outgoing_tx.send(msg).await.is_err() {
                            // Channel closed, connection is shutting down
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Dispatcher shut down
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Event listener lagged, missed {} events", n);
                    // Continue receiving
                }
            }
        }
    }

    /// Spawn the event listener task after successful authentication.
    fn spawn_event_listener(&mut self, user_id: crate::domain::UserId) {
        info!("Spawning event listener for user {}", user_id);
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
        debug!("Event listener spawned for user {}", user_id);
    }
}
