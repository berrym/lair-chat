//! Common protocol definitions for lair-chat
//!
//! This module contains message types and protocol structures that are shared
//! between the client and server components.

pub mod messages;

// Re-export the main protocol types
pub use messages::*;
