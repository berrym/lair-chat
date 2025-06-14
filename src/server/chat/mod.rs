//! Chat functionality for lair-chat server
//!
//! This module contains chat message handling, room management, and user messaging
//! functionality for the server-side of the lair-chat application.

pub mod messages;
pub mod rooms;
pub mod users;

// Re-export commonly used types
pub use messages::*;
pub use rooms::{Room, RoomManager};
pub use users::{ConnectedUser, PresenceStatus, UserManager};
