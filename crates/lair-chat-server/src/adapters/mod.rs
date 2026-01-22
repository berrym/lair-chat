//! # Protocol Adapters
//!
//! Thin layers that translate between wire protocols and the core engine.
//!
//! Each adapter:
//! 1. Accepts connections/requests in its wire format
//! 2. Parses incoming data into Commands
//! 3. Sends Commands to the Core Engine
//! 4. Serializes responses back to wire format
//! 5. Pushes Events to connected clients
//!
//! ## Adapters
//!
//! - `tcp`: TCP socket protocol for persistent connections
//! - `http`: REST API for stateless requests
//! - `ws`: WebSocket for real-time web clients (future)
//!
//! ## Design Principles
//!
//! - **Thin**: Minimal logic, just translation
//! - **Protocol-specific**: Handle wire format concerns
//! - **Shared engine**: All adapters use the same ChatEngine
//!
//! ## Example Flow
//!
//! ```text
//! TCP Client                TCP Adapter              Core Engine
//!     │                         │                        │
//!     │── JSON message ────────▶│                        │
//!     │                         │── Command ────────────▶│
//!     │                         │◀── Response ───────────│
//!     │◀── JSON response ───────│                        │
//!     │                         │                        │
//!     │                         │◀── Event ──────────────│
//!     │◀── JSON event ──────────│                        │
//! ```

pub mod http;
pub mod tcp;
pub mod ws;
