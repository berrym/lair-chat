//! Compatibility layer for gradual migration from global state to ConnectionManager
//! 
//! This module provides a bridge between the old global state-based API and the new
//! ConnectionManager architecture, allowing for gradual migration of existing code.

use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use std::net::SocketAddr;


use tui_input::Input;
use crate::connection_manager::ConnectionManager;
use crate::transport::{
    ConnectionConfig, ConnectionObserver, ConnectionStatus, TransportError,
    TuiObserver, CLIENT_STATUS, add_text_message
};

/// Global ConnectionManager instance for compatibility
static COMPAT_CONNECTION_MANAGER: Lazy<Arc<Mutex<Option<ConnectionManager>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Bridge observer that synchronizes new ConnectionManager state with old global state
pub struct CompatibilityObserver;

impl ConnectionObserver for CompatibilityObserver {
    fn on_message(&self, message: String) {
        // Delegate to the global message system
        add_text_message(message);
    }
    
    fn on_error(&self, error: String) {
        // Delegate to the global message system with enhanced formatting
        add_text_message(format!("ERROR: {}", error));
    }
    
    fn on_status_change(&self, connected: bool) {
        // Update the global CLIENT_STATUS to maintain compatibility
        if let Ok(mut status) = CLIENT_STATUS.try_lock() {
            status.status = if connected {
                ConnectionStatus::CONNECTED
            } else {
                ConnectionStatus::DISCONNECTED
            };
        }
        
        // Also add a message for visual feedback
        if connected {
            add_text_message("STATUS: Connected to server.".to_string());
        } else {
            add_text_message("STATUS: Disconnected from server.".to_string());
        }
    }
}

/// Initialize or get the global ConnectionManager
async fn get_or_create_manager(address: SocketAddr) -> Result<(), TransportError> {
    let mut manager_guard = COMPAT_CONNECTION_MANAGER.lock().await;
    
    // If no manager exists or we're connecting to a different address, create a new one
    if manager_guard.is_none() {
        let config = ConnectionConfig::new(address);
        let mut manager = ConnectionManager::new(config.clone());
        
        // Create and configure a TCP transport
        use crate::tcp_transport::TcpTransport;
        let transport = Box::new(TcpTransport::new(config));
        manager.with_transport(transport);
        
        // Verify transport was set correctly
        if !manager.has_transport() {
            eprintln!("ERROR: Failed to configure transport for ConnectionManager");
            return Err(TransportError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to configure transport")
            ));
        }
        
        // Register observers for compatibility
        manager.register_observer(Arc::new(CompatibilityObserver));
        manager.register_observer(Arc::new(TuiObserver));
        
        *manager_guard = Some(manager);
    }
    
    Ok(())
}

/// Compatibility function that maintains the old connect_client API
/// 
/// This function uses the old transport system that properly handles encryption
/// and authentication as expected by the server.
pub async fn connect_client_compat(input: Input, address: SocketAddr) -> Result<(), TransportError> {
    use crate::transport::connect_client;
    
    add_text_message(format!("Connecting to {} using legacy transport...", address));
    
    // Use the old transport system that handles encryption properly
    connect_client(input, address).await;
    
    Ok(())
}

/// Send authentication request using legacy transport system
pub async fn authenticate_compat(username: String, password: String) -> Result<(), TransportError> {
    use crate::transport::add_silent_outgoing_message;
    
    // Create authentication request in the format expected by server
    let auth_request = serde_json::json!({
        "username": username,
        "password": password,
        "fingerprint": "client_fingerprint_placeholder",
        "is_registration": false
    });
    
    add_text_message("Sending authentication request...".to_string());
    add_silent_outgoing_message(auth_request.to_string());
    
    Ok(())
}

/// Compatibility function that maintains the old disconnect_client API
pub async fn disconnect_client_compat() -> Result<(), TransportError> {
    use crate::transport::disconnect_client;
    
    disconnect_client().await;
    Ok(())
}

/// Get the current connection status using the new architecture if available,
/// falling back to the old global state
pub async fn get_connection_status_compat() -> ConnectionStatus {
    let manager_guard = COMPAT_CONNECTION_MANAGER.lock().await;
    
    if let Some(ref manager) = &*manager_guard {
        manager.get_status().await
    } else {
        // Fallback to old global state
        if let Ok(status_guard) = CLIENT_STATUS.try_lock() {
            status_guard.status.clone()
        } else {
            ConnectionStatus::DISCONNECTED
        }
    }
}

/// Send a message using the legacy transport system
pub async fn send_message_compat(message: String) -> Result<(), TransportError> {
    use crate::transport::{add_outgoing_message, CLIENT_STATUS, ConnectionStatus};
    
    let client_status = CLIENT_STATUS.lock().unwrap();
    if client_status.status == ConnectionStatus::CONNECTED {
        add_outgoing_message(message);
        Ok(())
    } else {
        Err(TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server")
        ))
    }
}

/// Check if we're using the new ConnectionManager architecture
pub async fn is_using_new_architecture() -> bool {
    let manager_guard = COMPAT_CONNECTION_MANAGER.lock().await;
    manager_guard.is_some()
}

/// Force migration to the new architecture for a given address
pub async fn migrate_to_new_architecture(address: SocketAddr) -> Result<(), TransportError> {
    get_or_create_manager(address).await
}

/// Clean up the compatibility layer (useful for tests or shutdown)
pub async fn cleanup_compatibility_layer() {
    let mut manager_guard = COMPAT_CONNECTION_MANAGER.lock().await;
    
    if let Some(ref mut manager) = &mut *manager_guard {
        let _ = manager.disconnect().await;
    }
    
    *manager_guard = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compatibility_observer() {
        let observer = CompatibilityObserver;
        
        // Clear messages for clean test
        if let Ok(mut messages) = MESSAGES.try_lock() {
            messages.text.clear();
        }
        
        // Test message handling
        observer.on_message("Test message".to_string());
        
        // Test error handling
        observer.on_error("Test error".to_string());
        
        // Test status change
        observer.on_status_change(true);
        
        // Verify messages were added
        if let Ok(messages) = MESSAGES.try_lock() {
            assert!(!messages.text.is_empty());
        }
        
        // Verify status was updated
        if let Ok(status) = CLIENT_STATUS.try_lock() {
            assert_eq!(status.status, ConnectionStatus::CONNECTED);
        }
    }
    
    #[tokio::test]
    async fn test_compatibility_status_check() {
        // Test fallback to global state when no manager exists
        cleanup_compatibility_layer().await;
        
        let status = get_connection_status_compat().await;
        // Should fallback to global state
        assert!(matches!(status, ConnectionStatus::CONNECTED | ConnectionStatus::DISCONNECTED));
    }
    
    #[tokio::test]
    async fn test_architecture_detection() {
        // Should start with no new architecture
        cleanup_compatibility_layer().await;
        assert!(!is_using_new_architecture().await);
        
        // After migration, should detect new architecture
        let addr = "127.0.0.1:8080".parse().unwrap();
        migrate_to_new_architecture(addr).await.unwrap();
        assert!(is_using_new_architecture().await);
        
        // Cleanup
        cleanup_compatibility_layer().await;
    }
    
    #[tokio::test]
    async fn test_disconnect_compat_not_connected() {
        // Ensure we're not connected
        if let Ok(mut status) = CLIENT_STATUS.try_lock() {
            status.status = ConnectionStatus::DISCONNECTED;
        }
        
        // Clear messages
        if let Ok(mut messages) = MESSAGES.try_lock() {
            messages.text.clear();
        }
        
        // Should handle gracefully
        let result = disconnect_client_compat().await;
        assert!(result.is_ok());
        
        // Should have added "not connected" message
        if let Ok(messages) = MESSAGES.try_lock() {
            assert!(messages.text.iter().any(|msg| msg.contains("Not connected")));
        }
    }
}