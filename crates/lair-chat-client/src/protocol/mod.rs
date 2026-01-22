//! Protocol layer for client-server communication.
//!
//! This module implements the TCP wire protocol as specified in docs/protocols/TCP.md.

pub mod messages;
pub mod tcp;

pub use messages::{
    ClientMessage, MessageTarget, Room, RoomListItem, ServerMessage, Session, User,
};
pub use tcp::{Connection, TcpError};
