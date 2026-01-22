//! Protocol layer for client-server communication.
//!
//! This module implements the TCP wire protocol as specified in docs/protocols/TCP.md.

pub mod messages;
pub mod tcp;

pub use messages::{
    ClientMessage, ErrorInfo, Invitation, Message, MessageTarget, Room, RoomFilter, RoomListItem,
    RoomMembership, RoomSettings, ServerMessage, Session, User, UserFilter,
};
pub use tcp::{Connection, TcpClient, TcpError, PROTOCOL_VERSION};
