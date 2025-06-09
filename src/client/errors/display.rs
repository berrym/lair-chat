//! User-friendly error message display system for Lair-Chat
//! Provides centralized error handling and user messaging.

use crate::transport::add_text_message;

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
}

impl ErrorDisplay {
    /// Create a new error display system
    pub fn new(config: ErrorDisplayConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ErrorDisplayConfig::default())
    }

    /// Display a connection error with specific suggestions
    pub fn show_connection_error(&self, reason: &str) {
        add_text_message(" ".to_string());
        add_text_message(format!("❌ Connection: Unable to connect to the chat server"));
        add_text_message(format!("💡 Check your internet connection and try restarting the application"));
        if self.config.show_details {
            add_text_message(format!("   Details: {}", reason));
        }
        add_text_message(" ".to_string());
    }

    /// Display a validation error with specific guidance
    pub fn show_validation_error(&self, field: &str, reason: &str) {
        add_text_message(" ".to_string());
        add_text_message(format!("❌ Invalid {}: {}", field, reason));
        add_text_message("💡 Please check your input and try again".to_string());
        add_text_message(" ".to_string());
    }

    /// Display a generic user-friendly message
    pub fn show_info(&self, message: &str) {
        add_text_message(" ".to_string());
        add_text_message(format!("ℹ️  {}", message));
        add_text_message(" ".to_string());
    }

    /// Display a success message
    pub fn show_success(&self, message: &str) {
        add_text_message(" ".to_string());
        add_text_message(format!("✅ {}", message));
        add_text_message(" ".to_string());
    }

    /// Display a warning message
    pub fn show_warning(&self, message: &str) {
        add_text_message(" ".to_string());
        add_text_message(format!("⚠️  {}", message));
        add_text_message(" ".to_string());
    }

    /// Display a disconnection message with reconnection guidance
    pub fn show_disconnection_message(&self) {
        add_text_message(" ".to_string());
        add_text_message("🔌 Connection lost! You are now disconnected from the chat server.".to_string());
        add_text_message("💡 Please restart the application to reconnect and re-authenticate.".to_string());
        add_text_message(" ".to_string());
    }

    /// Display help for common connection issues
    pub fn show_connection_help(&self) {
        add_text_message(" ".to_string());
        add_text_message("🛠️  Connection Troubleshooting:".to_string());
        add_text_message("   • Check your internet connection".to_string());
        add_text_message("   • Verify the server is running".to_string());
        add_text_message("   • Try restarting the application".to_string());
        add_text_message("   • Contact your system administrator if problems persist".to_string());
        add_text_message(" ".to_string());
    }

    /// Display usage guidance for new users
    pub fn show_usage_help(&self) {
        add_text_message(" ".to_string());
        add_text_message("💬 Quick Start Guide:".to_string());
        add_text_message("   • Press '/' to start typing a message".to_string());
        add_text_message("   • Press Enter to send your message".to_string());
        add_text_message("   • Use ↑/↓ arrows to navigate command history".to_string());
        add_text_message("   • Press '?' for more help".to_string());
        add_text_message("   • Press 'q' or Ctrl+C to quit".to_string());
        add_text_message(" ".to_string());
    }
}

/// Global error display instance
static mut GLOBAL_ERROR_DISPLAY: Option<ErrorDisplay> = None;

/// Initialize the global error display system
pub fn init_error_display(config: ErrorDisplayConfig) {
    unsafe {
        GLOBAL_ERROR_DISPLAY = Some(ErrorDisplay::new(config));
    }
}

/// Get the global error display instance
pub fn get_error_display() -> &'static ErrorDisplay {
    unsafe {
        GLOBAL_ERROR_DISPLAY.as_ref().unwrap_or_else(|| {
            // Initialize with default config if not already initialized
            GLOBAL_ERROR_DISPLAY = Some(ErrorDisplay::default());
            GLOBAL_ERROR_DISPLAY.as_ref().unwrap()
        })
    }
}

/// Convenience functions for common error scenarios

/// Show a connection error
pub fn show_connection_error(reason: &str) {
    get_error_display().show_connection_error(reason);
}

/// Show a validation error
pub fn show_validation_error(field: &str, reason: &str) {
    get_error_display().show_validation_error(field, reason);
}

/// Show an info message
pub fn show_info(message: &str) {
    get_error_display().show_info(message);
}

/// Show a success message
pub fn show_success(message: &str) {
    get_error_display().show_success(message);
}

/// Show a warning message
pub fn show_warning(message: &str) {
    get_error_display().show_warning(message);
}

/// Show disconnection message
pub fn show_disconnection() {
    get_error_display().show_disconnection_message();
}

/// Show connection help
pub fn show_connection_help() {
    get_error_display().show_connection_help();
}

/// Show usage help
pub fn show_usage_help() {
    get_error_display().show_usage_help();
}