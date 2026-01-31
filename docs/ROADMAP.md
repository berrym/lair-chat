# Lair Chat Roadmap

This document outlines the development roadmap for Lair Chat, organized into phases with clear priorities and success criteria.

---

## Current Status

**Version**: 0.8.0
**Architecture**: Stable, protocol-agnostic core
**Protocols**: HTTP (with TLS), TCP (with E2E encryption)
**Storage**: SQLite
**Client**: TUI (ratatui)

---

## Phase 1: Production Hardening (Now)

**Goal**: Address immediate gaps for production readiness.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Emit InvitationReceived event | High | Pending | Wire up real-time notification when users receive room invitations |
| Expose online status to HTTP | High | Pending | Share TCP connection tracking with HTTP handlers for accurate user status |
| Add profile update endpoint | Medium | Pending | HTTP endpoint for users to update display name and profile |
| Add observability/metrics | High | Pending | Prometheus-compatible metrics for requests, connections, latencies |
| Configure rate limiting | Medium | Pending | Tune rate limiting middleware for auth, messaging, and registration |
| Connection limits | Medium | Pending | Add max connection tracking to TCP adapter |

### Success Criteria

- [ ] Invitations trigger real-time events
- [ ] `/api/v1/users` returns accurate online status
- [ ] `/metrics` endpoint exposes Prometheus metrics
- [ ] Rate limits prevent abuse (login: 5/min, messages: 60/min)
- [ ] All existing tests pass

---

## Phase 2: WebSocket Adapter (Next)

**Goal**: Enable browser-based clients via WebSocket protocol.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| WebSocket server implementation | High | Pending | Accept WebSocket upgrades on HTTP port, handle connections |
| Protocol adaptation | High | Pending | Translate TCP protocol to WebSocket frames (no length prefix needed) |
| Authentication flow | High | Pending | JWT authentication over WebSocket matching TCP behavior |
| E2E encryption support | Medium | Pending | Optional encryption negotiation for WebSocket connections |
| Connection lifecycle | High | Pending | Handshake, authentication, operational phases with proper timeouts |
| Integration tests | High | Pending | Comprehensive tests for WebSocket adapter |
| Protocol documentation | High | Pending | docs/protocols/WEBSOCKET.md specification |

### Architecture Fit

The WebSocket adapter will:
- Share `ChatEngine` with HTTP and TCP adapters
- Reuse existing protocol message types from TCP
- Support TLS when HTTP TLS is enabled
- Integrate with `EventDispatcher` for real-time events

```
adapters/
  ws/
    mod.rs          # Module exports
    server.rs       # WebSocket upgrade handling
    connection.rs   # Single connection lifecycle
    protocol.rs     # Frame serialization (JSON, no length prefix)
```

### Success Criteria

- [ ] WebSocket connections authenticate with JWT
- [ ] All TCP message types work over WebSocket
- [ ] Events push to WebSocket clients in real-time
- [ ] Optional encryption negotiation works
- [ ] Integration tests cover happy path and error cases
- [ ] Protocol documentation complete

---

## Phase 3: Web Client (Future)

**Goal**: Browser-based chat client using WebSocket adapter.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Client framework selection | High | Pending | Evaluate Leptos, Yew, or vanilla JS/TS |
| Authentication UI | High | Pending | Login, register, logout flows |
| Room management | High | Pending | Create, join, leave rooms |
| Real-time messaging | High | Pending | Send/receive messages, typing indicators |
| User experience | Medium | Pending | Notifications, unread counts, presence |

*Detailed planning deferred until WebSocket adapter is complete.*

---

## Phase 4: Horizontal Scaling (Later)

**Goal**: Support multiple server instances behind load balancer.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Redis session store | High | Pending | Shared session storage across instances |
| Redis event pub/sub | High | Pending | Cross-instance event broadcast |
| PostgreSQL migration | Medium | Pending | Production-grade database backend |
| Health check improvements | Medium | Pending | Readiness checks for load balancer integration |

### Architecture Changes

```rust
// Session store trait for pluggable backends
trait SessionStore: Send + Sync {
    async fn create(&self, session: Session) -> Result<()>;
    async fn get(&self, id: SessionId) -> Result<Option<Session>>;
    async fn revoke(&self, id: SessionId) -> Result<()>;
}

// Event dispatcher with Redis pub/sub
trait EventBroadcaster: Send + Sync {
    async fn publish(&self, event: Event) -> Result<()>;
    fn subscribe(&self) -> impl Stream<Item = Event>;
}
```

---

## Phase 5: Advanced Security (Eventually)

**Goal**: True end-to-end encryption with zero-knowledge design.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Client identity keys | High | Pending | Long-term identity keypairs for users |
| Per-message encryption | High | Pending | Messages encrypted to recipient public keys |
| Key distribution | High | Pending | X3DH or similar for key exchange |
| Key verification | Medium | Pending | Out-of-band verification UI |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | January 2025 | Initial roadmap |

---

## Contributing

When working on roadmap items:

1. Create a feature branch from `main`
2. Follow existing code patterns and ADRs
3. Add tests for new functionality
4. Update relevant documentation
5. Submit PR with clear description

See [DECISIONS.md](architecture/DECISIONS.md) for architectural guidance.
