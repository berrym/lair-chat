//! Network layer for lair-chat server
//!
//! This module contains networking functionality for the server, including
//! connection handling, session management, and network protocol implementations.

pub mod connection_handler;
pub mod session_manager;

// Re-export commonly used types
pub use connection_handler::ConnectionHandler;
pub use session_manager::{Session, SessionManager, SessionStats};
