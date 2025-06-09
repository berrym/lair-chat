//! Authentication module for Lair-Chat
//! Provides user authentication, session management, and rate limiting functionality.

mod types;
mod storage;
mod service;

pub use types::{
    AuthError,
    AuthResult,
    AuthRequest,
    AuthResponse,
    Role,
    Session,
    User,
    UserStatus,
};

pub use storage::{
    UserStorage,
    SessionStorage,
    MemoryUserStorage,
    MemorySessionStorage,
};

pub use service::{
    AuthService,
    RateLimitConfig,
};

/// Default rate limiting configuration
pub const DEFAULT_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 5,
    window_seconds: 300,  // 5 minutes
    lockout_duration: 900, // 15 minutes
};
