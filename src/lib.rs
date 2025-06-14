//! Lair-Chat Library
//!
//! This is the main library crate that exposes functionality for both client and server
//! components, along with shared common utilities.

// Common modules shared between client and server
pub mod common;

// Include all client modules directly
#[path = "client/action.rs"]
pub mod action;

#[path = "client/app.rs"]
pub mod app;

#[path = "client/auth/mod.rs"]
pub mod auth;

#[path = "client/cli.rs"]
pub mod cli;

#[path = "client/chat/mod.rs"]
pub mod chat;

#[path = "client/components.rs"]
pub mod components;

#[path = "client/config.rs"]
pub mod config;

#[path = "client/connection_manager.rs"]
pub mod connection_manager;

#[path = "client/errors.rs"]
pub mod errors;

#[path = "client/history/mod.rs"]
pub mod history;

#[path = "client/logging.rs"]
pub mod logging;

#[path = "client/tui.rs"]
pub mod tui;

// Re-export common modules for backward compatibility
pub use common::crypto as aes_gcm_encryption;
pub use common::crypto as encryption;
pub use common::protocol;
pub use common::transport as encrypted_transport;
pub use common::transport as tcp_transport;
pub use common::transport;

// Group client modules under a client namespace for cleaner imports
pub mod client {
    pub use super::{
        action::*, app::*, auth::*, chat::*, cli::*, components::*, config::*,
        connection_manager::*, errors::*, history::*, logging::*, tui::*,
    };

    // Re-export common functionality through client namespace
    pub use super::common::{
        crypto, errors as common_errors, protocol as common_protocol, transport as common_transport,
    };
}
