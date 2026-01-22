//! Transport layer abstractions for lair-chat
//!
//! This module provides networking and transport functionality shared between
//! client and server components, including TCP transport, encrypted transport,
//! and common transport traits.

pub mod encrypted;
pub mod tcp;
pub mod traits;

// Re-export commonly used types and traits
pub use encrypted::*;
pub use tcp::*;
pub use traits::*;
