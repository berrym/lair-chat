use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::common::transport::{ConnectionConfig, Transport, TransportError};

/// TCP-based implementation of the Transport trait
pub struct TcpTransport {
    reader: Option<Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>>>,
    writer: Option<Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>>,
    config: ConnectionConfig,
}

impl TcpTransport {
    /// Create a new TCP transport with the given configuration
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            reader: None,
            writer: None,
            config,
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    /// Establish a connection to the remote endpoint
    async fn connect(&mut self) -> Result<(), TransportError> {
        // Connect to the remote address
        let stream = TcpStream::connect(self.config.address)
            .await
            .map_err(TransportError::ConnectionError)?;

        // Split the stream into read and write halves
        let (read_half, write_half) = stream.into_split();

        // Store the split streams
        self.reader = Some(Arc::new(Mutex::new(BufReader::new(read_half))));
        self.writer = Some(Arc::new(Mutex::new(write_half)));

        Ok(())
    }

    /// Send data over the transport
    async fn send(&mut self, data: &str) -> Result<(), TransportError> {
        if let Some(writer) = &self.writer {
            let mut writer_guard = writer.lock().await;

            // Send the data with a newline terminator
            writer_guard
                .write_all(format!("{}\n", data).as_bytes())
                .await
                .map_err(TransportError::ConnectionError)?;

            // Ensure data is sent immediately
            writer_guard
                .flush()
                .await
                .map_err(TransportError::ConnectionError)?;

            Ok(())
        } else {
            Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )))
        }
    }

    /// Receive data from the transport
    async fn receive(&mut self) -> Result<Option<String>, TransportError> {
        if let Some(reader) = &self.reader {
            let mut reader_guard = reader.lock().await;

            // Read a line from the stream
            let mut line = String::new();
            let bytes_read = reader_guard
                .read_line(&mut line)
                .await
                .map_err(TransportError::ConnectionError)?;

            // If we read 0 bytes, the connection was closed
            if bytes_read == 0 {
                return Ok(None);
            }

            // Trim the newline character
            let line = line.trim_end().to_string();

            // Return the received line
            Ok(Some(line))
        } else {
            Err(TransportError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )))
        }
    }

    /// Close the transport connection
    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(writer) = &self.writer {
            let mut writer_guard = writer.lock().await;

            // Shutdown the write half
            writer_guard
                .shutdown()
                .await
                .map_err(TransportError::ConnectionError)?;
        }

        // Clear the streams
        self.reader = None;
        self.writer = None;

        Ok(())
    }
}

/// Create a new TCP transport with the given socket address
pub fn create_tcp_transport(addr: SocketAddr) -> TcpTransport {
    let config = ConnectionConfig::new(addr);
    TcpTransport::new(config)
}

/// Create a boxed TCP transport for use with ConnectionManager
pub fn create_boxed_tcp_transport(addr: SocketAddr) -> Box<dyn Transport> {
    Box::new(create_tcp_transport(addr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // Helper function to create an echo server for testing
    async fn start_echo_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
        let listener = TcpListener::bind(addr).await.unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let (mut reader, mut writer) = socket.split();
                // Echo back any data received
                tokio::io::copy(&mut reader, &mut writer).await.unwrap();
            }
        })
    }

    // Helper function for a server that responds with a specific message
    async fn start_response_server(
        addr: SocketAddr,
        response: String,
    ) -> tokio::task::JoinHandle<()> {
        let listener = TcpListener::bind(addr).await.unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                // Read incoming data but ignore it
                let mut buffer = [0; 1024];
                let _ = AsyncReadExt::read(&mut socket, &mut buffer).await;

                // Send our canned response with a newline
                let _ = socket.write_all(format!("{}\n", response).as_bytes()).await;
                let _ = socket.flush().await;
            }
        })
    }

    #[tokio::test]
    async fn test_tcp_transport_connection() {
        // Use a different port for each test to avoid conflicts
        let addr = "127.0.0.1:50001".parse::<SocketAddr>().unwrap();

        // Start a server
        let _server = start_echo_server(addr).await;

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create and connect a transport
        let mut transport = create_tcp_transport(addr);
        let result = transport.connect().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tcp_transport_send_receive() {
        // Use a different port for each test
        let addr = "127.0.0.1:50002".parse::<SocketAddr>().unwrap();

        // Start a server
        let _server = start_echo_server(addr).await;

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create and connect a transport
        let mut transport = create_tcp_transport(addr);
        transport.connect().await.unwrap();

        // Send data
        let message = "Hello, world!";
        transport.send(message).await.unwrap();

        // Receive the echoed data
        let response = transport.receive().await.unwrap();

        assert_eq!(response, Some(message.to_string()));
    }

    #[tokio::test]
    async fn test_tcp_transport_receive_only() {
        // Use a different port for each test
        let addr = "127.0.0.1:50003".parse::<SocketAddr>().unwrap();

        // Prepare a response
        let response = "Server message";

        // Start a server
        let _server = start_response_server(addr, response.to_string()).await;

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create and connect a transport
        let mut transport = create_tcp_transport(addr);
        transport.connect().await.unwrap();

        // Send an empty message to trigger the server response
        transport.send("").await.unwrap();

        // Receive the server's response
        let received = transport.receive().await.unwrap();

        assert_eq!(received, Some(response.to_string()));
    }

    #[tokio::test]
    async fn test_tcp_transport_close() {
        // Use a different port for each test
        let addr = "127.0.0.1:50004".parse::<SocketAddr>().unwrap();

        // Start a server
        let _server = start_echo_server(addr).await;

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create and connect a transport
        let mut transport = create_tcp_transport(addr);
        transport.connect().await.unwrap();

        // Close the connection
        let result = transport.close().await;

        assert!(result.is_ok());

        // Verify that operations fail after closing
        let send_result = transport.send("test").await;
        assert!(send_result.is_err());
    }
}
