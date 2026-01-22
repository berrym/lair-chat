//! # Storage Layer
//!
//! Database abstraction via repository traits.
//!
//! The storage layer defines traits for all data operations. The core
//! engine depends on these traits, not concrete implementations.
//!
//! ## Traits
//!
//! - `UserRepository`: User CRUD operations
//! - `RoomRepository`: Room and membership operations
//! - `MessageRepository`: Message storage and retrieval
//! - `SessionRepository`: Session management
//! - `InvitationRepository`: Invitation management
//!
//! ## Implementations
//!
//! - `sqlite`: SQLite implementation (current)
//! - `postgres`: PostgreSQL implementation (future)
//!
//! ## Design
//!
//! ```text
//! Core Engine
//!     ↓ (depends on traits)
//! Storage Traits
//!     ↓ (implemented by)
//! SQLite / PostgreSQL / etc.
//! ```
//!
//! This allows:
//! - Swapping databases without changing core code
//! - Testing with mock implementations
//! - Multiple database support

pub mod sqlite;
pub mod traits;

// Re-export traits
pub use traits::{
    InvitationRepository, MembershipRepository, MessageRepository, RoomRepository,
    SessionRepository, Storage, Transaction, TransactionalStorage, UserRepository,
};
