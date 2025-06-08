//! Migration facade providing feature flag support for choosing between old and new implementations
//! 
//! This module acts as the primary interface for connection management during the migration period.
//! It provides feature flags and runtime configuration to control whether to use the legacy
//! global state implementation or the new ConnectionManager architecture.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::env;

use tui_input::Input;
use crate::transport::{
    ConnectionStatus, TransportError, connect_client as legacy_connect_client,
    disconnect_client as legacy_disconnect_client, CLIENT_STATUS,
};
use crate::compatibility_layer::{
    connect_client_compat, disconnect_client_compat, get_connection_status_compat,
    send_message_compat, is_using_new_architecture, migrate_to_new_architecture,
    cleanup_compatibility_layer,
};

/// Global flag to control which implementation to use
static USE_NEW_ARCHITECTURE: AtomicBool = AtomicBool::new(false);

/// Configuration for migration behavior
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Whether to use the new ConnectionManager architecture
    pub use_new_architecture: bool,
    /// Whether to automatically migrate based on environment variables
    pub auto_detect: bool,
    /// Environment variable name to check for migration flag
    pub env_var_name: String,
    /// Whether to enable enhanced logging during migration
    pub verbose_logging: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            use_new_architecture: false,
            auto_detect: true,
            env_var_name: "LAIR_CHAT_USE_NEW_TRANSPORT".to_string(),
            verbose_logging: false,
        }
    }
}

/// Initialize the migration system with the given configuration
pub fn init_migration(config: MigrationConfig) {
    if config.auto_detect {
        // Check environment variable
        if let Ok(env_value) = env::var(&config.env_var_name) {
            let use_new = env_value.to_lowercase() == "true" || env_value == "1";
            USE_NEW_ARCHITECTURE.store(use_new, Ordering::SeqCst);
            
            if config.verbose_logging {
                eprintln!("Migration: Using {} architecture (env: {}={})", 
                    if use_new { "new" } else { "legacy" }, 
                    config.env_var_name, 
                    env_value);
            }
        } else {
            USE_NEW_ARCHITECTURE.store(config.use_new_architecture, Ordering::SeqCst);
            
            if config.verbose_logging {
                eprintln!("Migration: Using {} architecture (default)", 
                    if config.use_new_architecture { "new" } else { "legacy" });
            }
        }
    } else {
        USE_NEW_ARCHITECTURE.store(config.use_new_architecture, Ordering::SeqCst);
        
        if config.verbose_logging {
            eprintln!("Migration: Using {} architecture (configured)", 
                if config.use_new_architecture { "new" } else { "legacy" });
        }
    }
}

/// Check if we're currently using the new architecture
pub fn is_using_new_impl() -> bool {
    USE_NEW_ARCHITECTURE.load(Ordering::SeqCst)
}

/// Force enable the new architecture
pub fn enable_new_architecture() {
    USE_NEW_ARCHITECTURE.store(true, Ordering::SeqCst);
}

/// Force enable the legacy architecture
pub fn enable_legacy_architecture() {
    USE_NEW_ARCHITECTURE.store(false, Ordering::SeqCst);
}

/// Main facade for client connection - automatically delegates to correct implementation
pub async fn connect_client(input: Input, address: SocketAddr) -> Result<(), TransportError> {
    if is_using_new_impl() {
        // Use new ConnectionManager-based implementation
        connect_client_compat(input, address).await
    } else {
        // Use legacy global state implementation
        legacy_connect_client(input, address).await;
        Ok(())
    }
}

/// Main facade for client disconnection - automatically delegates to correct implementation
pub async fn disconnect_client() -> Result<(), TransportError> {
    if is_using_new_impl() {
        // Use new ConnectionManager-based implementation
        disconnect_client_compat().await
    } else {
        // Use legacy global state implementation
        legacy_disconnect_client().await;
        Ok(())
    }
}

/// Get connection status using the appropriate implementation
pub async fn get_connection_status() -> ConnectionStatus {
    if is_using_new_impl() {
        get_connection_status_compat().await
    } else {
        // Use legacy global state
        if let Ok(status_guard) = CLIENT_STATUS.try_lock() {
            status_guard.status.clone()
        } else {
            ConnectionStatus::DISCONNECTED
        }
    }
}

/// Send a message using the appropriate implementation
///
/// This function will use the ConnectionManager to send messages when using the new
/// architecture, or fall back to the legacy global queue when using the old system.
///
/// # Arguments
///
/// * `message` - The message content to send
///
/// # Returns
///
/// * `Ok(())` - If the message was successfully sent or queued
/// * `Err(TransportError)` - If there was an error sending the message
pub async fn send_message(message: String) -> Result<(), TransportError> {
    if is_using_new_impl() {
        // Use the ConnectionManager from the compatibility layer
        let result = send_message_compat(message.clone()).await;
        
        // Log the result for debugging
        match &result {
            Ok(()) => {
                tracing::debug!("Message sent successfully through new architecture");
            }
            Err(e) => {
                tracing::warn!("Failed to send message through new architecture: {:?}", e);
            }
        }
        
        result
    } else {
        // For legacy implementation, we'll add to the outgoing queue
        // This maintains compatibility with the old system
        use crate::transport::{add_outgoing_message};
        tracing::debug!("Sending message through legacy architecture");
        add_outgoing_message(message);
        Ok(())
    }
}

/// Migrate an existing connection to the new architecture
/// 
/// This function attempts to gracefully migrate from the legacy implementation
/// to the new ConnectionManager while preserving connection state.
pub async fn migrate_connection(_address: SocketAddr) -> Result<(), TransportError> {
    if is_using_new_impl() {
        // Already using new architecture
        return Ok(());
    }
    
    // Check if we're currently connected using legacy system
    let was_connected = {
        if let Ok(status_guard) = CLIENT_STATUS.try_lock() {
            status_guard.status == ConnectionStatus::CONNECTED
        } else {
            false
        }
    };
    
    if was_connected {
        // Gracefully disconnect from legacy system
        legacy_disconnect_client().await;
    }
    
    // Switch to new architecture
    enable_new_architecture();
    
    // Initialize new architecture
    migrate_to_new_architecture(_address).await?;
    
    if was_connected {
        // Reconnect using new system
        // Note: We can't easily pass the Input here, so we'll just initialize
        // the new architecture and let the UI handle reconnection
        eprintln!("Migration: Switched to new architecture. Please reconnect.");
    }
    
    Ok(())
}

/// Rollback to legacy architecture (useful for testing or if issues arise)
pub async fn rollback_to_legacy() -> Result<(), TransportError> {
    if !is_using_new_impl() {
        // Already using legacy architecture
        return Ok(());
    }
    
    // Check if we're currently connected using new system
    let was_connected = get_connection_status_compat().await == ConnectionStatus::CONNECTED;
    
    if was_connected {
        // Disconnect from new system
        disconnect_client_compat().await?;
    }
    
    // Clean up new architecture
    cleanup_compatibility_layer().await;
    
    // Switch to legacy architecture
    enable_legacy_architecture();
    
    if was_connected {
        eprintln!("Rollback: Switched to legacy architecture. Please reconnect.");
    }
    
    Ok(())
}

/// Get migration statistics and status
pub async fn get_migration_status() -> MigrationStatus {
    let using_new = is_using_new_impl();
    let has_new_manager = if using_new {
        is_using_new_architecture().await
    } else {
        false
    };
    
    let current_status = get_connection_status().await;
    
    MigrationStatus {
        using_new_architecture: using_new,
        has_connection_manager: has_new_manager,
        current_connection_status: current_status,
        migration_complete: using_new && has_new_manager,
    }
}

/// Status information about the migration process
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub using_new_architecture: bool,
    pub has_connection_manager: bool,
    pub current_connection_status: ConnectionStatus,
    pub migration_complete: bool,
}

impl MigrationStatus {
    pub fn describe(&self) -> String {
        match (self.using_new_architecture, self.has_connection_manager, self.migration_complete) {
            (false, _, false) => "Using legacy global state architecture".to_string(),
            (true, false, false) => "Configured for new architecture but not initialized".to_string(),
            (true, true, true) => "Successfully migrated to new ConnectionManager architecture".to_string(),
            (true, true, false) => "Partially migrated - using new architecture".to_string(),
            _ => "Unknown migration state".to_string(),
        }
    }
}

/// Quick initialization with sensible defaults
pub fn init_with_defaults() {
    init_migration(MigrationConfig::default());
}

/// Initialize with verbose logging enabled
pub fn init_with_verbose_logging() {
    let config = MigrationConfig {
        verbose_logging: true,
        ..Default::default()
    };
    init_migration(config);
}

/// Force initialization with new architecture enabled
pub fn init_with_new_architecture() {
    let config = MigrationConfig {
        use_new_architecture: true,
        auto_detect: false,
        verbose_logging: true,
        ..Default::default()
    };
    init_migration(config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_config_default() {
        let config = MigrationConfig::default();
        assert!(!config.use_new_architecture);
        assert!(config.auto_detect);
        assert_eq!(config.env_var_name, "LAIR_CHAT_USE_NEW_TRANSPORT");
        assert!(!config.verbose_logging);
    }

    #[test]
    fn test_architecture_flag_control() {
        // Reset global state to ensure test isolation
        enable_legacy_architecture();
        
        // Test initial state
        assert!(!is_using_new_impl());
        
        // Test enabling new architecture
        enable_new_architecture();
        assert!(is_using_new_impl());
        
        // Test disabling
        enable_legacy_architecture();
        assert!(!is_using_new_impl());
        
        // Ensure clean state for next test
        enable_legacy_architecture();
    }

    #[test]
    fn test_init_migration_manual() {
        // Reset global state to ensure test isolation
        enable_legacy_architecture();
        assert!(!is_using_new_impl()); // Verify reset worked
        
        let config = MigrationConfig {
            use_new_architecture: true,
            auto_detect: false,
            env_var_name: "TEST_VAR".to_string(),
            verbose_logging: false,
        };
        
        init_migration(config);
        assert!(is_using_new_impl());
        
        // Clean up for next test
        enable_legacy_architecture();
    }

    #[test]
    fn test_env_var_detection() {
        // Reset global state to ensure test isolation
        enable_legacy_architecture();
        assert!(!is_using_new_impl()); // Verify reset worked
        
        // Set environment variable
        env::set_var("TEST_MIGRATION_VAR", "true");
        
        let config = MigrationConfig {
            use_new_architecture: false, // This should be overridden by env var
            auto_detect: true,
            env_var_name: "TEST_MIGRATION_VAR".to_string(),
            verbose_logging: false,
        };
        
        init_migration(config);
        assert!(is_using_new_impl());
        
        // Clean up
        env::remove_var("TEST_MIGRATION_VAR");
        enable_legacy_architecture();
    }

    #[test]
    fn test_env_var_false_value() {
        // Reset global state to ensure test isolation
        enable_legacy_architecture();
        assert!(!is_using_new_impl()); // Verify reset worked
        
        // Set environment variable to false
        env::set_var("TEST_MIGRATION_VAR_FALSE", "false");
        
        let config = MigrationConfig {
            use_new_architecture: true, // This should be overridden by env var
            auto_detect: true,
            env_var_name: "TEST_MIGRATION_VAR_FALSE".to_string(),
            verbose_logging: false,
        };
        
        init_migration(config);
        assert!(!is_using_new_impl());
        
        // Clean up
        env::remove_var("TEST_MIGRATION_VAR_FALSE");
        enable_legacy_architecture();
    }

    #[tokio::test]
    async fn test_migration_status() {
        enable_legacy_architecture();
        
        let status = get_migration_status().await;
        assert!(!status.using_new_architecture);
        assert!(!status.migration_complete);
        assert!(status.describe().contains("legacy"));
    }

    #[tokio::test]
    async fn test_get_connection_status_legacy() {
        enable_legacy_architecture();
        
        // Should fall back to legacy global state
        let status = get_connection_status().await;
        assert!(matches!(status, ConnectionStatus::CONNECTED | ConnectionStatus::DISCONNECTED));
    }

    #[test]
    fn test_init_functions() {
        // Reset global state to ensure test isolation
        enable_legacy_architecture();
        assert!(!is_using_new_impl()); // Verify reset worked
        
        // Test default initialization
        init_with_defaults();
        // Should not panic and should use auto-detection
        
        // Test verbose initialization
        init_with_verbose_logging();
        // Should not panic
        
        // Test new architecture initialization
        init_with_new_architecture();
        assert!(is_using_new_impl());
        
        // Clean up for next test
        enable_legacy_architecture();
    }
}