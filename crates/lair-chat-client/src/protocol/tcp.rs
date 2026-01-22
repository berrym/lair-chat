//! TCP client implementation.
//!
//! Handles connection, message framing, and communication with the server.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::messages::{ClientMessage, ServerMessage};

/// Maximum message size (1 MB).
const MAX_MESSAGE_SIZE: u32 = 1_048_576;

/// Protocol version.
pub const PROTOCOL_VERSION: &str = "1.0";

/// TCP client errors.
#[derive(Debug, thiserror::Error)]
pub enum TcpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Message too large: {size} bytes (max {MAX_MESSAGE_SIZE})")]
    MessageTooLarge { size: u32 },

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Not connected")]
    NotConnected,
}

/// TCP client for communicating with the server.
pub struct TcpClient {
    stream: Option<TcpStream>,
    server_addr: SocketAddr,
    connect_timeout: Duration,
    read_timeout: Duration,
}

impl TcpClient {
    /// Create a new TCP client.
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            stream: None,
            server_addr,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
        }
    }

    /// Set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set read timeout.
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Connect to the server.
    pub async fn connect(&mut self) -> Result<(), TcpError> {
        info!("Connecting to server at {}", self.server_addr);

        let stream = timeout(self.connect_timeout, TcpStream::connect(self.server_addr))
            .await
            .map_err(|_| TcpError::Timeout)?
            .map_err(|e| TcpError::ConnectionFailed(e.to_string()))?;

        stream.set_nodelay(true)?;
        self.stream = Some(stream);

        info!("Connected to server");
        Ok(())
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
        }
        info!("Disconnected from server");
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Send a message to the server.
    pub async fn send(&mut self, message: &ClientMessage) -> Result<(), TcpError> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;

        let json = serde_json::to_string(message)?;
        let payload = json.as_bytes();

        if payload.len() > MAX_MESSAGE_SIZE as usize {
            return Err(TcpError::MessageTooLarge {
                size: payload.len() as u32,
            });
        }

        debug!("Sending: {}", json);

        // Write length prefix (big-endian u32)
        let length = payload.len() as u32;
        stream.write_all(&length.to_be_bytes()).await?;

        // Write payload
        stream.write_all(payload).await?;
        stream.flush().await?;

        Ok(())
    }

    /// Receive a message from the server.
    pub async fn recv(&mut self) -> Result<ServerMessage, TcpError> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;

        // Read length prefix
        let mut length_bytes = [0u8; 4];
        match timeout(self.read_timeout, stream.read_exact(&mut length_bytes)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TcpError::ConnectionClosed);
            }
            Ok(Err(e)) => return Err(TcpError::Io(e)),
            Err(_) => return Err(TcpError::Timeout),
        }

        let length = u32::from_be_bytes(length_bytes);

        if length > MAX_MESSAGE_SIZE {
            return Err(TcpError::MessageTooLarge { size: length });
        }

        // Read payload
        let mut payload = vec![0u8; length as usize];
        stream.read_exact(&mut payload).await?;

        let json = String::from_utf8_lossy(&payload);
        debug!("Received: {}", json);

        let message: ServerMessage = serde_json::from_slice(&payload)?;
        Ok(message)
    }

    /// Perform the initial handshake with the server.
    pub async fn handshake(&mut self) -> Result<(), TcpError> {
        // Wait for ServerHello
        let server_hello = self.recv().await?;

        match server_hello {
            ServerMessage::ServerHello {
                version,
                server_name,
                features,
                encryption_required,
            } => {
                info!(
                    "Server: {} v{}, features: {:?}, encryption_required: {}",
                    server_name, version, features, encryption_required
                );

                if version != PROTOCOL_VERSION {
                    warn!(
                        "Server version {} differs from client version {}",
                        version, PROTOCOL_VERSION
                    );
                }

                if encryption_required {
                    return Err(TcpError::Protocol(
                        "Encryption required but not implemented".to_string(),
                    ));
                }
            }
            ServerMessage::Error { code, message, .. } => {
                return Err(TcpError::Protocol(format!("{}: {}", code, message)));
            }
            other => {
                return Err(TcpError::Protocol(format!(
                    "Expected ServerHello, got {:?}",
                    other
                )));
            }
        }

        // Send ClientHello
        self.send(&ClientMessage::client_hello()).await?;

        Ok(())
    }
}

/// Connection handle for managing the TCP connection in a background task.
pub struct Connection {
    /// Channel for sending messages to the server.
    pub tx: mpsc::Sender<ClientMessage>,
    /// Channel for receiving messages from the server.
    pub rx: mpsc::Receiver<ServerMessage>,
    /// Shutdown signal.
    shutdown_tx: mpsc::Sender<()>,
}

impl Connection {
    /// Create a new connection to the server.
    pub async fn connect(server_addr: SocketAddr) -> Result<Self, TcpError> {
        let mut client = TcpClient::new(server_addr);
        client.connect().await?;
        client.handshake().await?;

        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<ClientMessage>(32);
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMessage>(32);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Split the stream for concurrent read/write
        let stream = client.stream.take().unwrap();
        let (mut reader, mut writer) = stream.into_split();

        // Spawn writer task
        let writer_shutdown = shutdown_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = outgoing_rx.recv() => {
                        match msg {
                            Some(message) => {
                                let json = match serde_json::to_string(&message) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        error!("Failed to serialize message: {}", e);
                                        continue;
                                    }
                                };

                                let payload = json.as_bytes();
                                let length = payload.len() as u32;

                                if let Err(e) = writer.write_all(&length.to_be_bytes()).await {
                                    error!("Failed to write length: {}", e);
                                    break;
                                }
                                if let Err(e) = writer.write_all(payload).await {
                                    error!("Failed to write payload: {}", e);
                                    break;
                                }
                                if let Err(e) = writer.flush().await {
                                    error!("Failed to flush: {}", e);
                                    break;
                                }

                                debug!("Sent: {}", json);
                            }
                            None => {
                                debug!("Outgoing channel closed");
                                break;
                            }
                        }
                    }
                    _ = writer_shutdown.closed() => {
                        debug!("Writer shutdown signal received");
                        break;
                    }
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = read_message(&mut reader) => {
                        match result {
                            Ok(message) => {
                                if incoming_tx.send(message).await.is_err() {
                                    debug!("Incoming channel closed");
                                    break;
                                }
                            }
                            Err(TcpError::ConnectionClosed) => {
                                info!("Server closed connection");
                                break;
                            }
                            Err(e) => {
                                error!("Read error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Reader shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            tx: outgoing_tx,
            rx: incoming_rx,
            shutdown_tx,
        })
    }

    /// Send a message to the server.
    pub async fn send(&self, message: ClientMessage) -> Result<(), TcpError> {
        self.tx
            .send(message)
            .await
            .map_err(|_| TcpError::ConnectionClosed)
    }

    /// Receive the next message from the server.
    pub async fn recv(&mut self) -> Option<ServerMessage> {
        self.rx.recv().await
    }

    /// Shutdown the connection.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// Read a single message from the stream.
async fn read_message(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
) -> Result<ServerMessage, TcpError> {
    // Read length prefix
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            TcpError::ConnectionClosed
        } else {
            TcpError::Io(e)
        }
    })?;

    let length = u32::from_be_bytes(length_bytes);

    if length > MAX_MESSAGE_SIZE {
        return Err(TcpError::MessageTooLarge { size: length });
    }

    // Read payload
    let mut payload = vec![0u8; length as usize];
    reader.read_exact(&mut payload).await?;

    let json = String::from_utf8_lossy(&payload);
    debug!("Received: {}", json);

    let message: ServerMessage = serde_json::from_slice(&payload)?;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::login("alice", "secret");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"login\""));
        assert!(json.contains("\"identifier\":\"alice\""));
    }

    #[test]
    fn test_server_message_deserialization() {
        let json = r#"{"type":"server_hello","version":"1.0","server_name":"Test","features":[],"encryption_required":false}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ServerMessage::ServerHello { .. }));
    }
}
