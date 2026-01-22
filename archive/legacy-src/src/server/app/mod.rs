//! Server application logic for lair-chat
//!
//! This module contains the core server application state, configuration,
//! and main server loop management.

pub mod server;
pub mod state;

// Re-export commonly used types
pub use server::*;
pub use state::*;
