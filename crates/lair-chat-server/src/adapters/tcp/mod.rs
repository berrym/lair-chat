//! # TCP Adapter
//!
//! TCP socket protocol for persistent connections.
//! See docs/protocols/TCP.md for the wire protocol specification.
//!
//! ## Components
//!
//! - `server`: TCP listener that accepts connections
//! - `connection`: Individual connection handler
//! - `protocol`: Wire format parsing/serialization
//! - `commands`: Maps protocol messages to core commands
//!
//! ## Current Status
//!
//! Placeholder for Phase 3 implementation.

pub mod server;
pub mod connection;
pub mod protocol;
pub mod commands;

/// TCP server configuration
pub struct TcpConfig {
    /// Port to listen on
    pub port: u16,
}

// TCP server implementation will be added in Phase 3
