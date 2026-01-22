# Architecture Decision Records

This document captures the key architectural decisions made for the Lair Chat project, along with the context and rationale behind each decision. These records serve as a reference for understanding why the system is designed the way it is.

---

## ADR-001: Protocol-Agnostic Core Architecture

### Status
Accepted

### Context
The original implementation had two separate server binaries (TCP and REST) that evolved organically. They shared a database but communicated statistics through global state (`OnceLock`). This led to:
- Duplicate logic in both servers
- Inconsistent behavior between protocols
- Difficulty maintaining feature parity
- Tight coupling between protocol handling and business logic

### Decision
Implement a **protocol-agnostic core engine** that handles all business logic, with thin protocol adapters (TCP, HTTP, WebSocket) that translate between wire formats and core commands.

```
Protocol Adapters → Core Engine → Storage Layer
```

### Consequences

**Positive:**
- Business logic written once, used by all protocols
- Easy to add new protocols (WebSocket, gRPC, etc.)
- Protocol adapters are thin and easy to test
- Core logic testable without network I/O
- Consistent behavior across all access methods

**Negative:**
- Requires more upfront design
- Some protocol-specific optimizations may be harder
- Additional abstraction layer

### Rationale
The benefits of consistency, testability, and maintainability outweigh the costs. A chat system must behave identically whether accessed via TCP or HTTP - this architecture enforces that.

---

## ADR-002: Cargo Workspace Structure

### Status
Accepted

### Context
The codebase needs to support:
- A server binary
- A TUI client binary  
- Potentially shared protocol types
- Clear separation of concerns

Options considered:
1. Single crate with multiple binaries
2. Separate repositories
3. Cargo workspace with multiple crates

### Decision
Use a **Cargo workspace** with separate crates:

```
lair-chat/
├── Cargo.toml (workspace)
├── crates/
│   ├── lair-chat-server/
│   └── lair-chat-client/
```

### Consequences

**Positive:**
- Clear separation between server and client code
- Client cannot accidentally depend on server internals
- Shared dependencies managed at workspace level
- Each crate can have its own dependencies
- Parallel compilation of independent crates
- Easy to add more crates (e.g., `lair-chat-proto` for shared types)

**Negative:**
- Slightly more complex project structure
- Need to manage inter-crate dependencies

### Rationale
The workspace structure enforces the boundary between server and client at the compiler level. The client must implement the protocol from documentation, not by importing server code. This validates that our protocol documentation is sufficient.

---

## ADR-003: Trait-Based Storage Abstraction

### Status
Accepted

### Context
The system needs to persist data (users, messages, rooms, etc.). We want to:
- Start with SQLite for simplicity
- Support PostgreSQL and MySQL in the future
- Keep business logic independent of database choice
- Enable testing with mock storage

### Decision
Define **repository traits** that the core depends on, with concrete implementations for each database backend.

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: NewUser) -> Result<User>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>>;
    // ...
}

// Core depends on the trait
pub struct ChatEngine<S: Storage> {
    storage: S,
}

// Implementations are separate
pub struct SqliteStorage { /* ... */ }
impl UserRepository for SqliteStorage { /* ... */ }
```

### Consequences

**Positive:**
- Core logic completely database-agnostic
- Easy to swap database backends
- Unit tests can use mock implementations
- Clear contract for what storage must provide
- Supports multi-database deployments

**Negative:**
- Some database-specific optimizations may be awkward
- Async traits require `async_trait` macro (until Rust stabilizes async traits)
- More boilerplate for trait definitions

### Rationale
Database independence is essential for a system that aims to be deployable in various environments. Some users want SQLite simplicity; others need PostgreSQL scalability. The trait abstraction makes this a configuration choice, not a code change.

---

## ADR-004: Domain Types as Pure Rust (No I/O)

### Status
Accepted

### Context
Domain types (User, Message, Room, etc.) are the core vocabulary of the system. They could either:
1. Be tightly coupled to database models (derive SQLx traits directly)
2. Be pure Rust types with separate mapping to/from database

### Decision
Domain types are **pure Rust structs** with no I/O dependencies. They may derive:
- `serde::Serialize` / `serde::Deserialize` for wire formats
- `Clone`, `Debug`, `PartialEq` for general use

They do NOT derive:
- `sqlx::FromRow` or similar database traits
- Any traits that tie them to a specific persistence mechanism

Mapping between domain types and database rows happens in the storage layer.

### Consequences

**Positive:**
- Domain types are simple and focused
- Can be shared between server and client (if needed)
- No database dependencies leak into core logic
- Domain validation can be enforced in constructors
- Easy to serialize for any protocol

**Negative:**
- Requires mapping code in storage layer
- Slight duplication between domain types and database models

### Rationale
Purity in the domain layer prevents accidental coupling. When domain types don't know about databases, the core logic can't accidentally do database I/O. This makes the architecture cleaner and more testable.

---

## ADR-005: Newtype Pattern for IDs

### Status
Accepted

### Context
The system has many ID types: `UserId`, `RoomId`, `MessageId`, `SessionId`, etc. These could be represented as:
1. Raw types (`String`, `Uuid`, `i64`)
2. Type aliases (`type UserId = Uuid`)
3. Newtypes (`struct UserId(Uuid)`)

### Decision
Use the **newtype pattern** for all ID types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
```

### Consequences

**Positive:**
- Type safety: can't accidentally pass `RoomId` where `UserId` expected
- Self-documenting: function signatures show exactly what's needed
- Encapsulation: internal representation can change
- Easy to add ID-specific methods

**Negative:**
- More boilerplate for each ID type
- Need conversion when interfacing with external systems

### Rationale
Type safety prevents a class of bugs at compile time. In a system with many different entity types, accidentally using the wrong ID is a real risk. The newtype pattern makes such mistakes impossible.

---

## ADR-006: Command/Event Pattern for Core Operations

### Status
Accepted

### Context
The core engine needs to handle operations from multiple protocols. Each protocol has different wire formats but the same logical operations.

Options:
1. Direct method calls on engine
2. Command objects that encapsulate operations
3. Actor model with message passing

### Decision
Use a **command pattern** where each operation is a command enum variant:

```rust
pub enum Command {
    SendMessage { session: SessionId, target: Target, content: String },
    JoinRoom { session: SessionId, room: RoomId },
    // ...
}

impl ChatEngine {
    pub async fn handle(&self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::SendMessage { session, target, content } => {
                // ...
            }
        }
    }
}
```

And an **event pattern** for broadcasting state changes:

```rust
pub enum Event {
    MessageReceived { message: Message },
    UserJoined { user: UserId, room: RoomId },
    // ...
}
```

### Consequences

**Positive:**
- Commands are data - can be logged, replayed, serialized
- Single entry point for all operations
- Easy to add middleware (auth, logging, rate limiting)
- Events decouple senders from receivers
- Protocol adapters just translate to/from commands

**Negative:**
- More indirection than direct method calls
- Command enum can grow large
- Pattern matching boilerplate

### Rationale
The command pattern provides a clean interface between protocol adapters and core logic. Every adapter does the same thing: parse wire format → create command → call engine → serialize response. This uniformity makes the system predictable and maintainable.

---

## ADR-007: Single Binary for Server

### Status
Accepted

### Context
The previous implementation had separate binaries for TCP and REST servers. This led to:
- Deployment complexity (run two processes)
- Shared state via database only
- Stats sharing through global state hacks
- Inconsistent lifecycle management

### Decision
Build a **single server binary** that runs all protocol adapters:

```rust
#[tokio::main]
async fn main() {
    let engine = ChatEngine::new(storage);
    
    // All adapters share the same engine instance
    tokio::join!(
        tcp::serve(engine.clone(), tcp_config),
        http::serve(engine.clone(), http_config),
    );
}
```

### Consequences

**Positive:**
- Single process to deploy and monitor
- Shared in-memory state (sessions, connections)
- Unified lifecycle and graceful shutdown
- Simpler configuration
- No inter-process communication needed

**Negative:**
- All protocols must be in same process
- Can't scale protocols independently
- Single point of failure

### Rationale
For most deployments, a single binary is simpler and more efficient. The previous multi-binary approach was not intentional distributed architecture—it was accidental complexity. A single binary with shared state is the right default.

For true horizontal scaling, the architecture could evolve to stateless servers with shared Redis/database state, but that's a future concern when the need arises.

---

## ADR-008: Protocol Documentation as Contract

### Status
Accepted

### Context
Clients need to communicate with the server. They could either:
1. Import shared code from the server
2. Implement the protocol from documentation

### Decision
Protocol documentation is the **contract**. Clients must be implementable from documentation alone, without any server code.

Requirements:
- TCP wire protocol fully documented (framing, message types, sequences)
- REST API documented in OpenAPI format
- Examples for every operation
- Documentation is language-agnostic

The TUI client validates this by:
- Living in a separate crate
- Not depending on any server crate
- Implementing protocol from documentation

### Consequences

**Positive:**
- Anyone can build a client in any language
- Forces complete documentation
- Server internals can change without breaking clients
- Client implementation validates documentation quality

**Negative:**
- Documentation must be maintained alongside code
- Protocol changes require documentation updates
- Potential for docs to drift from implementation

### Rationale
A professional system is one that others can integrate with. If building a client requires reading Rust source code, the system fails this standard. Protocol-first design ensures the system is truly accessible.

---

## ADR-009: No Global State

### Status
Accepted

### Context
The previous implementation used global state for sharing data between components:
- `OnceLock` for TCP stats
- Static configuration

This led to:
- Hidden dependencies
- Difficult testing
- Unclear data flow

### Decision
**No global state**. All state flows through explicit parameters:

```rust
// Bad: Global state
static STATS: OnceLock<Stats> = OnceLock::new();

// Good: Explicit state
struct ChatEngine {
    stats: Stats,
}
```

### Consequences

**Positive:**
- All dependencies explicit in function signatures
- Easy to test (inject test dependencies)
- Clear data flow through the system
- No hidden coupling between components

**Negative:**
- More parameters to pass around
- Need dependency injection patterns

### Rationale
Global state is a code smell that indicates missing architecture. If two components need to share data, that relationship should be explicit in the type system, not hidden in global variables.

---

## ADR-010: File Size Limits

### Status
Accepted

### Context
The original TCP server (`server.rs`) grew to 169KB—over 4,000 lines in a single file. This made it:
- Hard to navigate
- Difficult to understand
- Prone to merge conflicts
- Impossible to test in isolation

### Decision
Enforce a soft limit of **500 lines per file**. When a file approaches this limit, it should be split into focused modules.

### Consequences

**Positive:**
- Each file has a clear, focused purpose
- Easier to navigate and understand
- Smaller units are easier to test
- Encourages good module design

**Negative:**
- More files to manage
- Need good module organization
- Some logical units may span files

### Rationale
Large files are a symptom of unclear architecture. A file should represent a coherent concept that can be understood in isolation. The 500-line guideline encourages thoughtful decomposition.

---

## ADR-011: Error Handling Strategy

### Status
Accepted

### Context
Errors need to be:
- Informative for debugging
- Safe for users (no internal details leaked)
- Consistent across the system
- Easy to handle in calling code

### Decision
Use a **layered error approach**:

1. **Domain errors**: Business rule violations (e.g., "user not found", "permission denied")
2. **Storage errors**: Database failures (wrapped, not exposed directly)
3. **Protocol errors**: Wire format issues (e.g., "invalid JSON")
4. **System errors**: Infrastructure failures (e.g., "connection failed")

All errors implement a trait that provides:
- Error code (for programmatic handling)
- User-safe message (for display)
- Internal details (for logging, not exposed to users)

```rust
pub enum CoreError {
    NotFound { entity: &'static str, id: String },
    PermissionDenied { action: &'static str },
    ValidationFailed { field: &'static str, reason: String },
    // ...
}
```

### Consequences

**Positive:**
- Consistent error handling throughout
- Safe to return errors to users
- Rich information for debugging
- Type-safe error handling

**Negative:**
- Need to define many error variants
- Mapping between error layers

### Rationale
Good error handling is essential for a production system. Users need clear feedback, operators need debugging information, and developers need type safety. A structured approach provides all three.

---

## ADR-012: Versioning Philosophy

### Status
Accepted

### Context
Software versions communicate stability expectations. Version 1.0.0 implies "ready for production."

### Decision
Stay in **0.x.x versions** until:
- Architecture is proven and stable
- Protocol is finalized
- Documentation is comprehensive
- Real-world usage has validated the design
- We're genuinely proud to call it "released"

No artificial pressure to reach 1.0. Even v0.99.0 is acceptable if that reflects reality.

### Consequences

**Positive:**
- Freedom to make breaking changes
- Honest signaling to users
- No premature stability promises
- Version reflects actual maturity

**Negative:**
- Some users may avoid "pre-1.0" software
- Longer time to perceived stability

### Rationale
Version 1.0 is a statement, not a milestone. Reaching it prematurely creates technical debt through backward compatibility constraints. We'll get there when we're ready, not before.

---

## Summary

These decisions collectively define a system that is:

- **Clean**: Protocol-agnostic core, no global state, small files
- **Testable**: Trait-based storage, pure domain types, explicit dependencies
- **Extensible**: Workspace structure, protocol adapters, command pattern
- **Professional**: Complete documentation, type safety, thoughtful error handling
- **Honest**: Version reflects reality, not aspiration

Each decision reinforces the others. Together, they create an architecture that is maintainable, understandable, and built to last.
