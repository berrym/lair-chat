//! Cryptographic utilities for lair-chat
//!
//! This module provides encryption and decryption functionality used by both
//! client and server components, including AES-GCM encryption and key management.

pub mod aes_gcm;
pub mod encryption;

// Re-export commonly used types and functions
pub use aes_gcm::*;
pub use encryption::*;
