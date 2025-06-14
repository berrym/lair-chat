//! Authentication module for Lair-Chat
//! Provides user authentication, session management, and rate limiting functionality.

mod protocol;
mod service;
mod storage;
mod types;

pub use types::{AuthRequest, User};

pub use storage::{MemorySessionStorage, MemoryUserStorage, UserStorage};

pub use service::AuthService;
