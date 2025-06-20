//! User-friendly error message display system for Lair-Chat
//! Provides centralized error handling and user messaging.

use crate::action::Action;
use std::sync::{OnceLock, RwLock};
use tokio::sync::mpsc::UnboundedSender;

/// Error display configuration
#[derive(Debug, Clone)]
pub struct ErrorDisplayConfig {
    /// Whether to show technical details
    pub show_details: bool,
    /// Whether to use color formatting
    pub use_colors: bool,
    /// Whether to show error categories
    pub show_categories: bool,
}

impl Default for ErrorDisplayConfig {
    fn default() -> Self {
        Self {
            show_details: false,
            use_colors: true,
            show_categories: true,
        }
    }
}

/// Central error display system
pub struct ErrorDisplay {
    config: ErrorDisplayConfig,
    action_sender: Option<UnboundedSender<Action>>,
}

impl ErrorDisplay {
    /// Create a new error display system
    pub fn new(config: ErrorDisplayConfig) -> Self {
        Self {
            config,
            action_sender: None,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ErrorDisplayConfig::default())
    }

    /// Set the action sender for modern message display
    pub fn with_action_sender(&mut self, sender: UnboundedSender<Action>) {
        self.action_sender = Some(sender);
    }

    /// Send a message via action system
    fn send_message(&self, message: String) {
        if let Some(sender) = &self.action_sender {
            let _ = sender.send(Action::DisplayMessage {
                content: message,
                is_system: true,
            });
        } else {
            // Log error if no action sender is available - this should not happen in modern usage
            tracing::error!(
                "ErrorDisplay: No action sender available for message: {}",
                message
            );
        }
    }

    /// Display a connection error with specific suggestions
    pub fn show_connection_error(&self, reason: &str) {
        self.send_message(" ".to_string());
        self.send_message(format!(
            "❌ Connection: Unable to connect to the chat server"
        ));
        self.send_message(format!(
            "💡 Check your internet connection and try restarting the application"
        ));
        if self.config.show_details {
            self.send_message(format!("   Details: {}", reason));
        }
        self.send_message(" ".to_string());
    }

    /// Display a validation error with specific guidance
    pub fn show_validation_error(&self, field: &str, reason: &str) {
        self.send_message(" ".to_string());
        self.send_message(format!("❌ Invalid {}: {}", field, reason));
        self.send_message("💡 Please check your input and try again".to_string());
        self.send_message(" ".to_string());
    }

    /// Display a generic user-friendly message
    pub fn show_info(&self, message: &str) {
        self.send_message(" ".to_string());
        self.send_message(format!("ℹ️  {}", message));
        self.send_message(" ".to_string());
    }

    /// Display a success message
    pub fn show_success(&self, message: &str) {
        self.send_message(" ".to_string());
        self.send_message(format!("✅ {}", message));
        self.send_message(" ".to_string());
    }

    /// Display a warning message
    pub fn show_warning(&self, message: &str) {
        self.send_message(" ".to_string());
        self.send_message(format!("⚠️  {}", message));
        self.send_message(" ".to_string());
    }

    /// Display a disconnection message with reconnection guidance
    pub fn show_disconnection_message(&self) {
        self.send_message(" ".to_string());
        self.send_message(
            "🔌 Connection lost! You are now disconnected from the chat server.".to_string(),
        );
        self.send_message(
            "💡 Please restart the application to reconnect and re-authenticate.".to_string(),
        );
        self.send_message(" ".to_string());
    }

    /// Display help for common connection issues
    pub fn show_connection_help(&self) {
        self.send_message(" ".to_string());
        self.send_message("🛠️  Connection Troubleshooting:".to_string());
        self.send_message("   • Check your internet connection".to_string());
        self.send_message("   • Verify the server is running".to_string());
        self.send_message("   • Try restarting the application".to_string());
        self.send_message("   • Contact your system administrator if problems persist".to_string());
        self.send_message(" ".to_string());
    }

    /// Display usage guidance for new users
    pub fn show_usage_help(&self) {
        self.send_message(" ".to_string());
        self.send_message("💬 Quick Start Guide:".to_string());
        self.send_message("   • Press '/' to start typing a message".to_string());
        self.send_message("   • Press Enter to send your message".to_string());
        self.send_message("   • Use ↑/↓ arrows to navigate command history".to_string());
        self.send_message("   • Press '?' for more help".to_string());
        self.send_message("   • Press 'q' or Ctrl+C to quit".to_string());
        self.send_message(" ".to_string());
    }
}

/// Global error display instance with modern action support
static GLOBAL_ERROR_DISPLAY: OnceLock<RwLock<ErrorDisplay>> = OnceLock::new();

/// Initialize the global error display system
pub fn init_error_display(config: ErrorDisplayConfig) {
    let display = ErrorDisplay::new(config);
    let _ = GLOBAL_ERROR_DISPLAY.set(RwLock::new(display));
}

/// Initialize the global error display system with action sender
pub fn init_error_display_with_action_sender(
    config: ErrorDisplayConfig,
    sender: UnboundedSender<Action>,
) {
    let mut display = ErrorDisplay::new(config);
    display.with_action_sender(sender);
    let _ = GLOBAL_ERROR_DISPLAY.set(RwLock::new(display));
}

/// Get the global error display instance
fn with_error_display<F, R>(f: F) -> R
where
    F: FnOnce(&ErrorDisplay) -> R,
{
    let display_lock = GLOBAL_ERROR_DISPLAY.get_or_init(|| RwLock::new(ErrorDisplay::default()));
    let display = display_lock.read().unwrap();
    f(&display)
}

/// Set action sender for existing global error display
pub fn set_global_error_display_action_sender(sender: UnboundedSender<Action>) {
    let display_lock = GLOBAL_ERROR_DISPLAY.get_or_init(|| RwLock::new(ErrorDisplay::default()));
    let mut display = display_lock.write().unwrap();
    display.with_action_sender(sender);
}

/// Convenience functions for common error scenarios

/// Show a connection error
pub fn show_connection_error(reason: &str) {
    with_error_display(|display| display.show_connection_error(reason));
}

/// Show a validation error
pub fn show_validation_error(field: &str, reason: &str) {
    with_error_display(|display| display.show_validation_error(field, reason));
}

/// Show an info message
pub fn show_info(message: &str) {
    with_error_display(|display| display.show_info(message));
}

/// Show a success message
pub fn show_success(message: &str) {
    with_error_display(|display| display.show_success(message));
}

/// Show a warning message
pub fn show_warning(message: &str) {
    with_error_display(|display| display.show_warning(message));
}

/// Show disconnection message
pub fn show_disconnection() {
    with_error_display(|display| display.show_disconnection_message());
}

/// Show connection help
pub fn show_connection_help() {
    with_error_display(|display| display.show_connection_help());
}

/// Show usage help
pub fn show_usage_help() {
    with_error_display(|display| display.show_usage_help());
}
