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

#[path = "client/compatibility_layer.rs"]
pub mod compatibility_layer;

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

#[path = "client/errors.rs"]
pub mod errors;

#[path = "client/history/mod.rs"]
pub mod history;

#[path = "client/logging.rs"]
pub mod logging;

#[path = "client/migration_facade.rs"]
pub mod migration_facade;

#[path = "client/tcp_transport.rs"]
pub mod tcp_transport;

#[path = "client/transport.rs"]
pub mod transport;

#[path = "client/tui.rs"]
pub mod tui;



// Group client modules under a client namespace for cleaner imports
pub mod client {
    pub use super::{
        action::*,
        aes_gcm_encryption::*,
        app::*,
        auth::*,
        chat::*,
        cli::*,
        compatibility_layer::*,
        components::*,
        config::*,
        connection_manager::*,
        encryption::*,
        errors::*,
        history::*,
        logging::*,
        migration_facade::*,
        tcp_transport::*,
        transport::*,
        tui::*,

    };
}