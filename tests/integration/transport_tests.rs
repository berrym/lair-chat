use std::net::SocketAddr;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lair_chat::client::aes_gcm_encryption::AesGcmEncryption;
use lair_chat::client::config::ConnectionConfig;
use lair_chat::client::connection_manager::{ConnectionManager, ConnectionStatus};
use lair_chat::client::tcp_transport::TcpTransport;

// Helper function to create a test server
async fn start_test_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        
        let mut buf = [0u8; 1024];
        loop {
            match socket.read(&mut buf).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    // Echo the data back
                    if let Err(_) = socket.write_all(&buf[..n]).await {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

#[tokio::test]
async fn test_basic_connection_lifecycle() {
    let addr = "127.0.0.1:50001".parse::<SocketAddr>().unwrap();
    let server_handle = start_test_server(addr).await;
    
    // Create connection manager with components
    let config = ConnectionConfig::new(addr);
    let mut manager = ConnectionManager::new(config.clone());
    let transport = TcpTransport::new(config);
    let encryption = AesGcmEncryption::new("test_password");
    
    manager.set_transport(Box::new(transport));
    manager.set_encryption(Box::new(encryption));
    
    // Test initial state
    assert_eq!(manager.get_status().await, ConnectionStatus::Disconnected);
    
    // Test connection
    let connect_result = manager.connect().await;
    assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result);
    assert_eq!(manager.get_status().await, ConnectionStatus::Connected);
    
    // Test disconnection
    let disconnect_result = manager.disconnect().await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect: {:?}", disconnect_result);
    assert_eq!(manager.get_status().await, ConnectionStatus::Disconnected);
    
    server_handle.abort();
}

#[tokio::test]
async fn test_message_exchange() {
    let addr = "127.0.0.1:50002".parse::<SocketAddr>().unwrap();
    let server_handle = start_test_server(addr).await;
    
    // Create connection manager
    let config = ConnectionConfig::new(addr);
    let mut manager = ConnectionManager::new(config.clone());
    let transport = TcpTransport::new(config);
    let encryption = AesGcmEncryption::new("test_password");
    
    manager.set_transport(Box::new(transport));
    manager.set_encryption(Box::new(encryption));
    
    // Create message channel
    let (tx, mut rx) = mpsc::channel(32);
    manager.register_message_channel(tx).await;
    
    // Connect
    manager.connect().await.unwrap();
    
    // Send test message
    let test_message = "Hello, Transport!".to_string();
    manager.send_message(test_message.clone()).await.unwrap();
    
    // Wait for echo response
    match timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(received)) => {
            assert_eq!(received, test_message, "Received message doesn't match sent message");
        },
        Ok(None) => panic!("Channel closed unexpectedly"),
        Err(_) => panic!("Timeout waiting for message response"),
    }
    
    manager.disconnect().await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_connection_failure_handling() {
    let addr = "127.0.0.1:50003".parse::<SocketAddr>().unwrap();
    // Don't start the server - connection should fail
    
    let config = ConnectionConfig::new(addr);
    let mut manager = ConnectionManager::new(config.clone());
    let transport = TcpTransport::new(config);
    let encryption = AesGcmEncryption::new("test_password");
    
    manager.set_transport(Box::new(transport));
    manager.set_encryption(Box::new(encryption));
    
    // Attempt connection
    let connect_result = manager.connect().await;
    assert!(connect_result.is_err(), "Expected connection to fail");
    assert_eq!(manager.get_status().await, ConnectionStatus::Disconnected);
}

#[tokio::test]
async fn test_concurrent_connections() {
    let addr1 = "127.0.0.1:50004".parse::<SocketAddr>().unwrap();
    let addr2 = "127.0.0.1:50005".parse::<SocketAddr>().unwrap();
    
    let server1_handle = start_test_server(addr1).await;
    let server2_handle = start_test_server(addr2).await;
    
    // Create two connection managers
    let config1 = ConnectionConfig::new(addr1);
    let config2 = ConnectionConfig::new(addr2);
    
    let mut manager1 = ConnectionManager::new(config1.clone());
    let mut manager2 = ConnectionManager::new(config2.clone());
    
    manager1.set_transport(Box::new(TcpTransport::new(config1)));
    manager1.set_encryption(Box::new(AesGcmEncryption::new("test_password_1")));
    
    manager2.set_transport(Box::new(TcpTransport::new(config2)));
    manager2.set_encryption(Box::new(AesGcmEncryption::new("test_password_2")));
    
    // Connect both simultaneously
    let (result1, result2) = tokio::join!(
        manager1.connect(),
        manager2.connect()
    );
    
    assert!(result1.is_ok(), "First connection failed");
    assert!(result2.is_ok(), "Second connection failed");
    
    // Verify both are connected
    assert_eq!(manager1.get_status().await, ConnectionStatus::Connected);
    assert_eq!(manager2.get_status().await, ConnectionStatus::Connected);
    
    // Clean up
    let (disconnect1, disconnect2) = tokio::join!(
        manager1.disconnect(),
        manager2.disconnect()
    );
    
    assert!(disconnect1.is_ok(), "First disconnect failed");
    assert!(disconnect2.is_ok(), "Second disconnect failed");
    
    server1_handle.abort();
    server2_handle.abort();
}

#[tokio::test]
async fn test_message_ordering() {
    let addr = "127.0.0.1:50006".parse::<SocketAddr>().unwrap();
    let server_handle = start_test_server(addr).await;
    
    let config = ConnectionConfig::new(addr);
    let mut manager = ConnectionManager::new(config.clone());
    let transport = TcpTransport::new(config);
    let encryption = AesGcmEncryption::new("test_password");
    
    manager.set_transport(Box::new(transport));
    manager.set_encryption(Box::new(encryption));
    
    // Create message channel
    let (tx, mut rx) = mpsc::channel(32);
    manager.register_message_channel(tx).await;
    
    // Connect
    manager.connect().await.unwrap();
    
    // Send multiple messages
    let messages = vec![
        "Message 1".to_string(),
        "Message 2".to_string(),
        "Message 3".to_string(),
    ];
    
    for msg in messages.clone() {
        manager.send_message(msg).await.unwrap();
    }
    
    // Verify message order
    for expected_msg in messages {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Some(received)) => {
                assert_eq!(received, expected_msg, "Messages received out of order");
            },
            Ok(None) => panic!("Channel closed unexpectedly"),
            Err(_) => panic!("Timeout waiting for message"),
        }
    }
    
    manager.disconnect().await.unwrap();
    server_handle.abort();
}