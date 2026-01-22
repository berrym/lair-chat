//! TCP connection handler.
//!
//! Handles the lifecycle of a single TCP client connection:
//! - Handshake
//! - Authentication
//! - Message processing
//! - Event delivery
//! - Cleanup

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::core::engine::ChatEngine;
use crate::domain::{Protocol, Session, SessionId, User};
use crate::storage::Storage;

use super::commands::CommandHandler;
use super::protocol::{
    read_message, write_message, ClientMessage, ProtocolError, ServerMessage, PROTOCOL_VERSION,
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
    /// Handshake complete, waiting for authentication.
    AwaitingAuth,
    /// Fully authenticated and operational.
    Authenticated,
    /// Connection is closing.
    Closing,
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

        // Create connection handler
        let mut conn = Connection {
            addr,
            state: ConnectionState::AwaitingHandshake,
            user: None,
            session: None,
            commands: CommandHandler::new(engine.clone()),
            outgoing_tx: outgoing_tx.clone(),
        };

        // Send server hello
        if let Err(e) = conn.send_server_hello(&mut writer).await {
            error!("Failed to send server hello to {}: {}", addr, e);
            return;
        }

        // Spawn writer task
        let writer_handle =
            tokio::spawn(async move { Self::writer_task(writer, outgoing_rx).await });

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
    ) {
        while let Some(msg) = rx.recv().await {
            match msg.to_json() {
                Ok(json) => {
                    if let Err(e) = write_message(&mut writer, &json).await {
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
                ConnectionState::AwaitingAuth => AUTH_TIMEOUT,
                ConnectionState::Authenticated => IDLE_TIMEOUT,
                ConnectionState::Closing => return Ok(()),
            };

            // Read with timeout
            let json = match timeout(read_timeout, read_message(reader)).await {
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
            ConnectionState::AwaitingAuth => self.handle_auth(msg).await,
            ConnectionState::Authenticated => self.handle_authenticated(msg).await,
            ConnectionState::Closing => Ok(()),
        }
    }

    /// Handle messages during handshake phase.
    async fn handle_handshake(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
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
                    self.state = ConnectionState::Closing;
                    return Ok(());
                }

                debug!(
                    "Handshake complete with {} (version {})",
                    self.addr, version
                );
                self.state = ConnectionState::AwaitingAuth;
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
    async fn handle_auth(&mut self, msg: ClientMessage) -> Result<(), ProtocolError> {
        match msg {
            ClientMessage::Login {
                request_id: _,
                identifier,
                password,
            } => {
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
                        self.commands.user_connected(user.id).await;
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

                        self.commands.user_connected(user.id).await;
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
        if let Some(user) = &self.user {
            info!("Cleaning up connection for user {}", user.username);
            self.commands.user_disconnected(user.id).await;
        }
    }
}
