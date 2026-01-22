//! # Lair Chat Server
//!
//! Unified server binary that runs TCP and HTTP protocol adapters
//! with a shared core engine.
//!
//! ## Usage
//!
//! ```bash
//! lair-chat-server --config config.toml
//! ```
//!
//! ## Current Status
//!
//! This is a placeholder for Phase 0 (Architecture Foundation).
//! The actual implementation will be built in subsequent phases:
//!
//! - Phase 1: Domain types and Core engine
//! - Phase 2: Storage implementation
//! - Phase 3: Protocol adapters
//! - Phase 4: Unified binary
//!
//! See docs/architecture/OVERVIEW.md for the full plan.

fn main() {
    println!("Lair Chat Server v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("This is the Phase 0 skeleton.");
    println!("Implementation begins in Phase 1.");
    println!();
    println!("See docs/architecture/ for the architecture documentation:");
    println!("  - OVERVIEW.md      High-level architecture");
    println!("  - DOMAIN_MODEL.md  Entity definitions");
    println!("  - COMMANDS.md      All operations");
    println!("  - EVENTS.md        Real-time events");
    println!("  - DECISIONS.md     Architecture decisions");
    println!();
    println!("See docs/protocols/ for wire format specifications:");
    println!("  - TCP.md           TCP protocol specification");
    println!("  - HTTP.md          REST API specification");
}
