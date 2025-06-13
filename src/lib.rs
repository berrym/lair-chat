//! Lair-Chat Library
//!
//! This is the main library crate that exposes all client functionality
//! for use by binaries, tests, and other consumers.

// Include all client modules directly
#[path = "client/action.rs"]
pub mod action;

#[path = "client/aes_gcm_encryption.rs"]
pub mod aes_gcm_encryption;

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

#[path = "client/encryption.rs"]
pub mod encryption;

#[path = "client/encrypted_transport.rs"]
pub mod encrypted_transport;

#[path = "client/errors.rs"]
pub mod errors;

#[path = "client/history/mod.rs"]
pub mod history;

#[path = "client/logging.rs"]
pub mod logging;

#[path = "client/server_compatible_encryption.rs"]
pub mod server_compatible_encryption;

#[path = "client/tcp_transport.rs"]
pub mod tcp_transport;

#[path = "client/transport.rs"]
pub mod transport;

#[path = "client/protocol.rs"]
pub mod protocol;

#[path = "client/tui.rs"]
pub mod tui;

// Group client modules under a client namespace for cleaner imports
pub mod client {
    pub use super::{
        action::*, aes_gcm_encryption::*, app::*, auth::*, chat::*, cli::*, components::*,
        config::*, connection_manager::*, encrypted_transport::*, encryption::*, errors::*,
        history::*, logging::*, protocol::*, server_compatible_encryption::*, tcp_transport::*,
        transport::*, tui::*,
    };
}
