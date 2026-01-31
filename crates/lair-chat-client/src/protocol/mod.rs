//! Protocol layer for client-server communication.
//!
//! This module implements:
//! - HTTP REST API for authentication (see docs/protocols/HTTP.md)
//! - TCP wire protocol for real-time messaging (see docs/protocols/TCP.md)
//! - WebSocket protocol for real-time messaging (see docs/protocols/WEBSOCKET.md)
//!
//! Per ADR-013, authentication is done via HTTP, returning a JWT token
//! that is then used to authenticate the TCP or WebSocket connection.
//!
//! ## Transport Options
//!
//! - **TCP**: Lower latency, supports end-to-end encryption, requires direct connection
//! - **WebSocket**: HTTP-compatible, passes through firewalls/proxies, uses TLS for encryption

pub mod http;
pub mod messages;
pub mod tcp;
pub mod ws;

pub use http::{HttpClient, HttpClientConfig};
pub use messages::{
    ClientMessage, Invitation, MessageTarget, Room, RoomListItem, RoomMember, ServerMessage,
    Session, User,
};
pub use tcp::{Connection, TcpError};
pub use ws::{WsConnection, WsError};
