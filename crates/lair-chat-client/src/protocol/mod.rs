//! Protocol layer for client-server communication.
//!
//! This module implements both:
//! - HTTP REST API for authentication (see docs/protocols/HTTP.md)
//! - TCP wire protocol for real-time messaging (see docs/protocols/TCP.md)
//!
//! Per ADR-013, authentication is done via HTTP, returning a JWT token
//! that is then used to authenticate the TCP connection.

pub mod http;
pub mod messages;
pub mod tcp;

pub use http::{HttpClient, HttpClientConfig};
pub use messages::{
    ClientMessage, Invitation, MessageTarget, Room, RoomListItem, RoomMember, ServerMessage,
    Session, User,
};
pub use tcp::{Connection, TcpError};
