//! # Core Engine
//!
//! Protocol-agnostic business logic for Lair Chat.
//!
//! The core engine handles all business operations regardless of
//! which protocol (TCP, HTTP, WebSocket) the request came from.
//!
//! ## Components
//!
//! - `engine`: Main `ChatEngine` that coordinates all operations
//! - `auth`: Authentication and password management
//! - `messaging`: Message sending, editing, deletion
//! - `rooms`: Room creation, membership, settings
//! - `sessions`: Session lifecycle management
//! - `events`: Event dispatching to connected clients
//!
//! ## Design
//!
//! All operations flow through the `ChatEngine`:
//!
//! ```text
//! Protocol Adapter → ChatEngine → Storage Layer
//!                        ↓
//!                  EventDispatcher → Connected Clients
//! ```
//!
//! The engine:
//! 1. Validates inputs
//! 2. Enforces business rules
//! 3. Coordinates with storage
//! 4. Emits events for state changes

pub mod auth;
pub mod engine;
pub mod events;
pub mod jwt;
pub mod messaging;
pub mod rooms;
pub mod sessions;

// Re-export main types
pub use auth::AuthService;
pub use engine::{ChatEngine, SystemStats};
pub use events::{should_receive_event, EventDispatcher};
pub use jwt::{Claims, JwtService};
pub use messaging::MessagingService;
pub use rooms::RoomService;
pub use sessions::SessionManager;
