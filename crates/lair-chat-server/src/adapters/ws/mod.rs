//! WebSocket adapter for real-time web clients.
//!
//! The WebSocket implementation is integrated with the HTTP adapter
//! as an upgrade handler on the `/ws` endpoint. This module re-exports
//! the WebSocket types for API consistency.
//!
//! ## Usage
//!
//! Connect to `ws://server:8082/ws` or `wss://server:8082/ws` (with TLS).
//!
//! Optional pre-authentication via query parameter:
//! ```text
//! wss://server:8082/ws?token=JWT_TOKEN
//! ```
//!
//! The WebSocket protocol uses the same JSON messages as TCP (see `tcp::protocol`),
//! but without length-prefix framing (WebSocket handles message boundaries).

pub use crate::adapters::http::handlers::websocket::*;
