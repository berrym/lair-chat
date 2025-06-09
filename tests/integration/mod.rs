mod transport_tests;

use tokio;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio::time::timeout;

// Re-export commonly used test utilities
pub(crate) use crate::client::{
    aes_gcm_encryption::AesGcmEncryption,
    config::ConnectionConfig,
    connection_manager::{ConnectionManager, ConnectionStatus},
    tcp_transport::TcpTransport,
    encryption::Encryption,
    transport::Transport,
};

// Common test timeout duration
pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);

// Helper function to create a test config with random available port
pub async fn create_test_config() -> ConnectionConfig {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    ConnectionConfig::new(addr)
}

// Helper function to create a fully configured connection manager for testing
pub async fn create_test_manager() -> (ConnectionManager, SocketAddr) {
    let config = create_test_config().await;
    let addr = config.address().clone();
    let mut manager = ConnectionManager::new(config.clone());
    
    let transport = TcpTransport::new(config);
    let encryption = AesGcmEncryption::new("test_password");
    
    manager.set_transport(Box::new(transport));
    manager.set_encryption(Box::new(encryption));
    
    (manager, addr)
}

// Helper struct for tracking test messages
#[derive(Debug, Clone)]
pub struct TestMessage {
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

impl TestMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

// Helper function to collect messages from a channel with timeout
pub async fn collect_messages(
    rx: &mut mpsc::Receiver<String>,
    count: usize,
    timeout_duration: Duration,
) -> Vec<TestMessage> {
    let mut messages = Vec::with_capacity(count);
    
    for _ in 0..count {
        match timeout(timeout_duration, rx.recv()).await {
            Ok(Some(msg)) => messages.push(TestMessage::new(msg)),
            Ok(None) => break,
            Err(_) => break,
        }
    }
    
    messages
}