//! # Lair Chat Server
//!
//! A secure, high-performance chat server built with Rust.
//!
//! ## Architecture
//!
//! The server is organized into distinct layers:
//!
//! - **Domain** (`domain/`): Pure types with no I/O dependencies
//! - **Core** (`core/`): Business logic, protocol-agnostic
//! - **Storage** (`storage/`): Database abstraction via traits
//! - **Adapters** (`adapters/`): Protocol handlers (TCP, HTTP, WebSocket)
//! - **Crypto** (`crypto/`): Encryption utilities
//! - **Config** (`config/`): Configuration management
//!
//! ## Design Principles
//!
//! 1. **Protocol-agnostic core**: Business logic doesn't know about wire formats
//! 2. **Trait-based storage**: Swap databases without changing core code
//! 3. **No global state**: All state flows through explicit parameters
//! 4. **Testability**: Core logic testable without I/O
//!
//! ## Example
//!
//! ```rust,ignore
//! use lair_chat_server::{ChatEngine, SqliteStorage, Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = Config::load()?;
//!     let storage = SqliteStorage::new(&config.database_url).await?;
//!     let engine = ChatEngine::new(storage);
//!
//!     // Start protocol adapters
//!     server::run(engine, config).await
//! }
//! ```

// Domain types - pure Rust, no I/O
pub mod domain;

// Core business logic
pub mod core;

// Database abstraction
pub mod storage;

// Protocol adapters
pub mod adapters;

// Encryption utilities
pub mod crypto;

// Configuration
pub mod config;

// Error types
pub mod error;

// Re-exports
pub use crate::error::Error;

// ChatEngine will be re-exported once implemented in Phase 1

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;
