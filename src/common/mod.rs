//! Common modules shared between client and server
//!
//! This module contains shared functionality, types, and utilities that are used
//! by both the client and server components of the lair-chat application.

pub mod crypto;
pub mod errors;
pub mod protocol;
pub mod transport;

// Re-export commonly used types for convenience
pub use errors::*;
pub use protocol::*;
