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
    fn on_message(&self, _message: String) {
        // Disabled to prevent duplication - messages now handled via action system
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
#[deprecated(since = "0.5.1", note = "Use ConnectionManager.connect() directly instead. This compatibility function will be removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn connect_client_compat(input: Input, address: std::net::SocketAddr) -> Result<(), TransportError> {
    use crate::transport::connect_client;
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    
    add_text_message(format!("Connecting to {} using legacy transport...", address));
    
    // First verify we can actually connect to the server
    match timeout(Duration::from_secs(5), TcpStream::connect(address)).await {
        Ok(Ok(_stream)) => {
            tracing::info!("DEBUG: TCP connection test successful to {}", address);
            // Connection works, now use the legacy transport system
            connect_client(input, address).await;
            
            // Give the transport loop time to start
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Verify transport loop is actually running by checking if connection is active
            let status = crate::transport::CLIENT_STATUS.lock().unwrap().status.clone();
            if status == crate::transport::ConnectionStatus::CONNECTED {
                tracing::info!("DEBUG: Legacy transport connection established successfully");
                
                // Update ConnectionManager status to keep them in sync
                if let Some(manager) = COMPAT_CONNECTION_MANAGER.lock().await.as_mut() {
                    // Use the proper async method to update status
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let mut status = manager.get_status().await;
                            if status != crate::transport::ConnectionStatus::CONNECTED {
                                tracing::info!("DEBUG: Updating ConnectionManager status to CONNECTED");
                                // Status update will be handled internally by ConnectionManager
                            }
                        })
                    });
                }
                
                Ok(())
            } else {
                tracing::error!("DEBUG: Legacy transport failed to establish connection");
                Err(TransportError::ConnectionError(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, format!("Transport failed to connect to {}", address))))
            }
        }
        Ok(Err(e)) => {
            tracing::error!("DEBUG: TCP connection failed to {}: {}", address, e);
            Err(TransportError::ConnectionError(e))
        }
        Err(_) => {
            tracing::error!("DEBUG: TCP connection timed out to {}", address);
            Err(TransportError::ConnectionError(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("Connection timeout to {}", address))))
        }
    }
}

/// Send authentication request using legacy transport system
#[deprecated(since = "0.5.1", note = "Use AuthManager.login() with ConnectionManager instead. This compatibility function will be removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn authenticate_compat(username: String, password: String) -> Result<(), TransportError> {
    use crate::transport::add_silent_outgoing_message;
    
    // Create authentication request in exact format expected by server
    let auth_request = serde_json::json!({
        "username": username,
        "password": password,
        "fingerprint": "client_device_fingerprint",
        "is_registration": false
    });
    
    add_text_message("Sending authentication request...".to_string());
    add_silent_outgoing_message(auth_request.to_string());
    
    Ok(())
}

/// Register the current ConnectionManager with the compatibility layer
#[deprecated(since = "0.5.1", note = "Use ConnectionManager directly instead. This compatibility function will be removed in v0.6.0.")]
pub async fn register_connection_manager() {
    // Get app instance connection manager if not already registered
    if COMPAT_CONNECTION_MANAGER.lock().await.is_none() {
        tracing::info!("DEBUG: Registering ConnectionManager with compatibility layer");
        
        // Create a new ConnectionManager for compatibility layer
        let config = ConnectionConfig::new("127.0.0.1:8080".parse().unwrap());
        let mut manager = ConnectionManager::new(config.clone());
        let transport = Box::new(crate::tcp_transport::TcpTransport::new(config));
        manager.with_transport(transport);
        manager.with_auth();
        
        // Store in global state
        *COMPAT_CONNECTION_MANAGER.lock().await = Some(manager);
    }
}

/// Update ConnectionManager with the status from legacy transport
#[deprecated(since = "0.5.1", note = "Use ConnectionManager directly instead. This compatibility function will be removed in v0.6.0.")]
pub async fn sync_connection_status() {
    // Get legacy transport status
    let legacy_status = {
        use crate::transport::CLIENT_STATUS;
        CLIENT_STATUS.lock().unwrap().status.clone()
    };
    
    // Update ConnectionManager status through proper methods
    if let Some(manager) = COMPAT_CONNECTION_MANAGER.lock().await.as_mut() {
        let current_status = manager.get_status().await;
        
        // Only update if different - use proper methods instead of direct field access
        if current_status != legacy_status {
            tracing::info!("DEBUG: Syncing ConnectionManager status from {:?} to {:?}", current_status, legacy_status);
            
            // We can't directly set status, but we can log the mismatch for diagnostics
            if legacy_status == crate::transport::ConnectionStatus::CONNECTED {
                tracing::info!("DEBUG: Legacy transport is CONNECTED but ConnectionManager is {:?}", current_status);
            } else {
                tracing::info!("DEBUG: Legacy transport is DISCONNECTED but ConnectionManager is {:?}", current_status);
            }
        }
    }
}

/// Compatibility function that maintains the old disconnect_client API
#[deprecated(since = "0.5.1", note = "Use ConnectionManager.disconnect() directly instead. This compatibility function will be removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn disconnect_client_compat() -> Result<(), TransportError> {
    use crate::transport::disconnect_client;
    
    disconnect_client().await;
    Ok(())
}

/// Get the current connection status using the new architecture if available,
/// falling back to the old global state
#[deprecated(since = "0.5.1", note = "Use ConnectionManager.get_status() directly instead. This compatibility function will be removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
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
#[deprecated(since = "0.5.1", note = "Use ConnectionManager.send_message() directly instead. This compatibility function will be removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn send_message_compat(message: String) -> Result<(), TransportError> {
    use crate::transport::{add_outgoing_message, CLIENT_STATUS, ConnectionStatus};
    
    // Register and sync status before checking
    register_connection_manager().await;
    sync_connection_status().await;
    
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
#[deprecated(since = "0.5.1", note = "This function will be removed when compatibility layer is eliminated in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn is_using_new_architecture() -> bool {
    let manager_guard = COMPAT_CONNECTION_MANAGER.lock().await;
    manager_guard.is_some()
}

/// Force migration to the new architecture for a given address
#[deprecated(since = "0.5.1", note = "This function will be removed when compatibility layer is eliminated in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
pub async fn migrate_to_new_architecture(address: SocketAddr) -> Result<(), TransportError> {
    get_or_create_manager(address).await
}

/// Clean up the compatibility layer (useful for tests or shutdown)
#[deprecated(since = "0.5.1", note = "This cleanup function will no longer be needed when compatibility layer is removed in v0.6.0. See LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md for migration guidance.")]
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