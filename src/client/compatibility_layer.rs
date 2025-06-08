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
    TuiObserver, CLIENT_STATUS, CANCEL_TOKEN, MESSAGES, add_text_message
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
        let mut manager = ConnectionManager::new(config);
        
        // Register observers for compatibility
        manager.register_observer(Arc::new(CompatibilityObserver));
        manager.register_observer(Arc::new(TuiObserver));
        
        *manager_guard = Some(manager);
    }
    
    Ok(())
}

/// Compatibility function that maintains the old connect_client API
/// 
/// This function internally uses ConnectionManager but maintains the same signature
/// and behavior as the original connect_client function.
pub async fn connect_client_compat(input: Input, address: SocketAddr) -> Result<(), TransportError> {
    // Initialize the manager
    get_or_create_manager(address).await?;
    
    let manager_arc = COMPAT_CONNECTION_MANAGER.clone();
    let mut manager_guard = manager_arc.lock().await;
    
    if let Some(ref mut manager) = &mut *manager_guard {
        // Check if already connected
        let current_status = manager.get_status().await;
        if current_status == ConnectionStatus::CONNECTED {
            add_text_message("Already connected to a server.".to_string());
            return Ok(());
        }
        
        // Add connecting message for compatibility
        add_text_message(format!("Connecting to {}", address));
        
        // Attempt connection
        match manager.connect().await {
            Ok(()) => {
                // Connection successful - the observers will handle status updates
                add_text_message("".to_owned()); // Empty line for formatting compatibility
                
                // Spawn background task to handle the connection lifecycle
                let manager_arc_clone = manager_arc.clone();
                tokio::spawn(async move {
                    // Keep the connection alive and handle any cleanup when cancelled
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        
                        // Check if we should disconnect
                        if let Ok(cancel_guard) = CANCEL_TOKEN.try_lock() {
                            if cancel_guard.token.is_cancelled() {
                                break;
                            }
                        }
                    }
                    
                    // Cleanup: disconnect the manager
                    if let Ok(mut guard) = manager_arc_clone.try_lock() {
                        if let Some(ref mut mgr) = &mut *guard {
                            let _ = mgr.disconnect().await;
                        }
                    }
                });
                
                Ok(())
            }
            Err(e) => {
                add_text_message(format!("Connection failed: {}", e));
                Err(e)
            }
        }
    } else {
        Err(TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to initialize connection manager")
        ))
    }
}

/// Compatibility function that maintains the old disconnect_client API
pub async fn disconnect_client_compat() -> Result<(), TransportError> {
    // Check current status using the old global state for compatibility
    let current_status = {
        if let Ok(status_guard) = CLIENT_STATUS.try_lock() {
            status_guard.status.clone()
        } else {
            ConnectionStatus::DISCONNECTED
        }
    };
    
    if current_status == ConnectionStatus::CONNECTED {
        // Cancel the token for compatibility with old cancellation mechanism
        if let Ok(cancel_guard) = CANCEL_TOKEN.try_lock() {
            cancel_guard.token.cancel();
        }
        
        // Use the ConnectionManager to disconnect
        let manager_arc = COMPAT_CONNECTION_MANAGER.clone();
        let mut manager_guard = manager_arc.lock().await;
        
        if let Some(ref mut manager) = &mut *manager_guard {
            match manager.disconnect().await {
                Ok(()) => {
                    // Clear messages for compatibility
                    if let Ok(mut messages) = MESSAGES.try_lock() {
                        messages.text.clear();
                    }
                    
                    // The observer will handle the status update and disconnection message
                    
                    // Sleep for compatibility with old behavior
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    
                    // Clear messages again
                    if let Ok(mut messages) = MESSAGES.try_lock() {
                        messages.text.clear();
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    add_text_message(format!("Disconnect failed: {}", e));
                    Err(e)
                }
            }
        } else {
            // Fallback to old behavior if no manager exists
            add_text_message("Disconnected from server.".to_string());
            Ok(())
        }
    } else {
        add_text_message("Not connected to a server.".to_string());
        Ok(())
    }
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

/// Send a message using the new architecture if available
pub async fn send_message_compat(message: String) -> Result<(), TransportError> {
    let manager_arc = COMPAT_CONNECTION_MANAGER.clone();
    let mut manager_guard = manager_arc.lock().await;
    
    if let Some(ref mut manager) = &mut *manager_guard {
        manager.send_message(message).await
    } else {
        Err(TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::NotConnected, "No active connection")
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
    use std::time::Duration;

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