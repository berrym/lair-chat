//! Authentication module for Lair-Chat
//! Provides user authentication, session management, and rate limiting functionality.

mod protocol;
mod service;
mod storage;
mod types;

pub use types::{AuthError, AuthRequest, Session, User};

pub use storage::{MemorySessionStorage, MemoryUserStorage, UserStorage};

pub use service::{AuthService, RateLimitConfig};

pub use protocol::AuthenticationMessage;

/// Default rate limiting configuration
pub const DEFAULT_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 5,
    window_seconds: 300,   // 5 minutes
    lockout_duration: 900, // 15 minutes
};
