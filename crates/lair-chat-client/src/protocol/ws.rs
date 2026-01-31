//! WebSocket client implementation.
//!
//! Provides a WebSocket transport option that shares the same message types
//! as the TCP protocol. WebSocket is useful for:
//! - Browser compatibility (for future web clients)
//! - Passing through HTTP-only firewalls and proxies
//! - Using standard HTTP ports (typically 8082)
//!
//! The WebSocket protocol uses the same JSON message format as TCP but without
//! length-prefix framing (WebSocket handles message boundaries natively).

use std::time::Duration;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};
use url::Url;

use super::messages::{ClientMessage, ServerMessage};
use super::tcp::PROTOCOL_VERSION;

/// WebSocket client errors.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum WsError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Type alias for the WebSocket stream.
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket connection handle for managing the connection in background tasks.
///
/// This provides the same interface as the TCP `Connection` struct, allowing
/// the application to use either transport interchangeably.
pub struct WsConnection {
    /// Channel for sending messages to the server.
    pub tx: mpsc::Sender<ClientMessage>,
    /// Channel for receiving messages from the server.
    pub rx: mpsc::Receiver<ServerMessage>,
    /// Shutdown signal.
    shutdown_tx: mpsc::Sender<()>,
}

impl WsConnection {
    /// Create a new WebSocket connection to the server.
    ///
    /// # Arguments
    /// * `ws_url` - WebSocket URL (e.g., "ws://localhost:8082/ws" or "wss://...")
    /// * `token` - Optional JWT token for pre-authentication via query parameter
    ///
    /// # Example
    /// ```ignore
    /// // Without pre-auth (will need to authenticate via messages)
    /// let conn = WsConnection::connect("ws://localhost:8082/ws", None).await?;
    ///
    /// // With pre-auth (authenticated immediately)
    /// let conn = WsConnection::connect("ws://localhost:8082/ws", Some(token)).await?;
    /// ```
    pub async fn connect(ws_url: &str, token: Option<&str>) -> Result<Self, WsError> {
        // Build URL with optional token
        let mut url = Url::parse(ws_url).map_err(|e| WsError::InvalidUrl(e.to_string()))?;

        if let Some(t) = token {
            url.query_pairs_mut().append_pair("token", t);
        }

        info!("Connecting to WebSocket at {}", url.as_str());

        // Connect with timeout
        let connect_timeout = Duration::from_secs(10);
        let (ws_stream, response) = timeout(connect_timeout, connect_async(url.as_str()))
            .await
            .map_err(|_| WsError::Timeout)?
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        info!("WebSocket connected, status: {}", response.status());

        // Split the stream
        let (write, read) = ws_stream.split();

        // Create channels
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<ClientMessage>(32);
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMessage>(32);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn writer task
        let writer_shutdown = shutdown_tx.clone();
        tokio::spawn(ws_writer_task(write, outgoing_rx, writer_shutdown));

        // Spawn reader task
        tokio::spawn(ws_reader_task(read, incoming_tx, shutdown_rx));

        Ok(Self {
            tx: outgoing_tx,
            rx: incoming_rx,
            shutdown_tx,
        })
    }

    /// Create a new WebSocket connection and perform the handshake.
    ///
    /// This method connects, waits for ServerHello, sends ClientHello,
    /// and returns a ready-to-use connection. Note that WebSocket connections
    /// do NOT support encryption (TLS is handled at the transport layer).
    pub async fn connect_and_handshake(ws_url: &str) -> Result<Self, WsError> {
        let mut conn = Self::connect(ws_url, None).await?;

        // Wait for ServerHello
        let server_hello = timeout(Duration::from_secs(30), conn.rx.recv())
            .await
            .map_err(|_| WsError::Timeout)?
            .ok_or(WsError::ConnectionClosed)?;

        match server_hello {
            ServerMessage::ServerHello {
                version,
                server_name,
                features,
                ..
            } => {
                info!(
                    "Server: {} v{}, features: {:?}",
                    server_name, version, features
                );

                if version != PROTOCOL_VERSION {
                    warn!(
                        "Server version {} differs from client version {}",
                        version, PROTOCOL_VERSION
                    );
                }
            }
            ServerMessage::Error { code, message, .. } => {
                return Err(WsError::Protocol(format!("{}: {}", code, message)));
            }
            other => {
                return Err(WsError::Protocol(format!(
                    "Expected ServerHello, got {:?}",
                    other
                )));
            }
        }

        // Send ClientHello (no encryption for WebSocket - use TLS at transport layer)
        conn.send(ClientMessage::client_hello()).await?;

        Ok(conn)
    }

    /// Create a new WebSocket connection with pre-authentication.
    ///
    /// The JWT token is passed as a query parameter, and the server
    /// will authenticate the connection before sending ServerHello.
    /// This skips the separate authenticate message flow.
    #[allow(dead_code)]
    pub async fn connect_with_token(ws_url: &str, token: &str) -> Result<Self, WsError> {
        let mut conn = Self::connect(ws_url, Some(token)).await?;

        // Wait for ServerHello (pre-auth means we're already authenticated)
        let server_hello = timeout(Duration::from_secs(30), conn.rx.recv())
            .await
            .map_err(|_| WsError::Timeout)?
            .ok_or(WsError::ConnectionClosed)?;

        match server_hello {
            ServerMessage::ServerHello {
                version,
                server_name,
                features,
                ..
            } => {
                info!(
                    "Server: {} v{}, features: {:?} (pre-authenticated)",
                    server_name, version, features
                );

                if version != PROTOCOL_VERSION {
                    warn!(
                        "Server version {} differs from client version {}",
                        version, PROTOCOL_VERSION
                    );
                }
            }
            ServerMessage::Error { code, message, .. } => {
                return Err(WsError::Protocol(format!("{}: {}", code, message)));
            }
            other => {
                return Err(WsError::Protocol(format!(
                    "Expected ServerHello, got {:?}",
                    other
                )));
            }
        }

        // Send ClientHello
        conn.send(ClientMessage::client_hello()).await?;

        Ok(conn)
    }

    /// Send a message to the server.
    pub async fn send(&self, message: ClientMessage) -> Result<(), WsError> {
        self.tx
            .send(message)
            .await
            .map_err(|_| WsError::ConnectionClosed)
    }

    /// Receive the next message from the server.
    #[allow(dead_code)]
    pub async fn recv(&mut self) -> Option<ServerMessage> {
        self.rx.recv().await
    }

    /// Shutdown the connection.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// WebSocket writer task - sends ClientMessages to the server.
async fn ws_writer_task(
    mut write: SplitSink<WsStream, Message>,
    mut outgoing_rx: mpsc::Receiver<ClientMessage>,
    shutdown_tx: mpsc::Sender<()>,
) {
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

                        debug!("WS Sent: {}", json);

                        if let Err(e) = write.send(Message::Text(json)).await {
                            error!("WebSocket write error: {}", e);
                            break;
                        }
                    }
                    None => {
                        debug!("Outgoing channel closed");
                        break;
                    }
                }
            }
            _ = shutdown_tx.closed() => {
                debug!("Writer shutdown signal received");
                // Send close frame
                let _ = write.send(Message::Close(None)).await;
                break;
            }
        }
    }
}

/// WebSocket reader task - receives ServerMessages from the server.
async fn ws_reader_task(
    mut read: SplitStream<WsStream>,
    incoming_tx: mpsc::Sender<ServerMessage>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("WS Received: {}", text);

                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(message) => {
                                if incoming_tx.send(message).await.is_err() {
                                    debug!("Incoming channel closed");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse server message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Pong is handled automatically by tungstenite
                        debug!("Received ping: {:?}", data);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!("Received pong");
                    }
                    Some(Ok(Message::Close(frame))) => {
                        info!("Server closed connection: {:?}", frame);
                        break;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        warn!("Received unexpected binary message");
                    }
                    Some(Ok(Message::Frame(_))) => {
                        // Raw frame, shouldn't happen in normal operation
                    }
                    Some(Err(e)) => {
                        error!("WebSocket read error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended");
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // WsError Tests
    // ========================================================================

    #[test]
    fn test_ws_error_display() {
        let err = WsError::ConnectionClosed;
        assert_eq!(err.to_string(), "Connection closed");

        let err = WsError::Timeout;
        assert_eq!(err.to_string(), "Timeout");

        let err = WsError::NotConnected;
        assert_eq!(err.to_string(), "Not connected");

        let err = WsError::Protocol("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = WsError::InvalidUrl("bad url".to_string());
        assert!(err.to_string().contains("bad url"));
    }

    #[test]
    fn test_url_construction() {
        let base_url = "ws://localhost:8082/ws";
        let url = Url::parse(base_url).unwrap();
        assert_eq!(url.scheme(), "ws");
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.port(), Some(8082));
        assert_eq!(url.path(), "/ws");
    }

    #[test]
    fn test_url_with_token() {
        let mut url = Url::parse("ws://localhost:8082/ws").unwrap();
        url.query_pairs_mut().append_pair("token", "my-jwt-token");
        assert_eq!(url.query(), Some("token=my-jwt-token"));
    }

    #[test]
    fn test_wss_url() {
        let url = Url::parse("wss://secure.example.com:443/ws").unwrap();
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.host_str(), Some("secure.example.com"));
    }
}
