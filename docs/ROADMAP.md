# Lair Chat Roadmap

This document outlines the development roadmap for Lair Chat, organized into phases with clear priorities and success criteria.

---

## Current Status

**Version**: 0.8.0
**Architecture**: Stable, protocol-agnostic core
**Protocols**: HTTP (with TLS), TCP (with E2E encryption), WebSocket
**Storage**: SQLite
**Client**: TUI (ratatui)

---

## Phase 1: Production Hardening (In Progress)

**Goal**: Address immediate gaps for production readiness.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Emit InvitationReceived event | High | **Done** | Real-time notification when users receive room invitations. Event emitted in `RoomService::invite()`, dispatched to TCP and WebSocket clients. |
| Expose online status to HTTP | High | **Done** | `EventDispatcher` tracks per-user connection counts across all adapters. `GET /api/v1/users` returns `UserWithStatus` with `online` field and supports `online_only` filter. |
| Configure rate limiting | Medium | **Done** | Custom middleware with auth (10/60s) and general (100/60s) tiers. Per-IP tracking, proper `429` responses with `Retry-After` header, auto-cleanup. Messaging tier (30/60s) defined but not yet deployed. |
| Connection limits | Medium | **Done** | TCP adapter enforces configurable `max_connections` (default 10,000) via `LAIR_TCP_MAX_CONNECTIONS` env var. Atomic counter with active enforcement and logging. |
| Add observability/metrics | High | **Partial** | `/metrics` endpoint serves Prometheus format with 3 gauges (`lair_chat_online_users`, `lair_chat_total_users`, `lair_chat_total_rooms`). Missing: request counters, latency histograms, per-endpoint instrumentation. |
| Add profile update endpoint | Medium | **Stub** | Route exists at `PATCH /api/v1/users/me` but handler returns "Not implemented". Request struct only has `email` field — needs `display_name` support. |

### Remaining Work

- [ ] Expand `/metrics` with request counters and latency histograms per endpoint
- [ ] Deploy messaging rate limit tier to message endpoints
- [ ] Implement profile update handler with `display_name` and `email` support
- [ ] Add integration tests for InvitationReceived event delivery to connected clients
- [ ] All existing tests pass

---

## Phase 2: WebSocket Adapter (Done)

**Goal**: Enable browser-based clients via WebSocket protocol.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| WebSocket server implementation | High | **Done** | 815-line handler at `/ws` endpoint with split sink/stream architecture |
| Protocol adaptation | High | **Done** | JSON frames over WebSocket, all TCP message types supported (27 types) |
| Authentication flow | High | **Done** | JWT authentication post-connect and pre-auth via `?token=` query parameter |
| E2E encryption support | Medium | **N/A** | Intentionally omitted — WebSocket relies on TLS at the transport layer (`wss://`). `key_exchange` messages return "not_supported" error. |
| Connection lifecycle | High | **Done** | State machine (AwaitingHandshake → AwaitingAuth → Authenticated → Closing) with configurable timeouts (handshake: 30s, auth: 60s, idle: 90s) |
| Integration tests | High | **Done** | 27 integration tests covering auth, rooms, messages, invitations, presence, error handling |
| Protocol documentation | High | **Done** | 298-line `docs/protocols/WEBSOCKET.md` with connection flows, message format, JS client example |

### Architecture

The WebSocket adapter:
- Shares `ChatEngine` with HTTP and TCP adapters
- Reuses existing protocol message types from TCP
- Supports TLS when HTTP TLS is enabled
- Integrates with `EventDispatcher` for 13 real-time event types

---

## Phase 3: Web Client (Next)

**Goal**: Browser-based chat client using WebSocket adapter.

| Task | Priority | Status | Description |
|------|----------|--------|-------------|
| Client framework selection | High | Pending | Evaluate Leptos, Yew, or vanilla JS/TS |
| Authentication UI | High | Pending | Login, register, logout flows |
| Room management | High | Pending | Create, join, leave rooms |
| Real-time messaging | High | Pending | Send/receive messages, typing indicators |
| User experience | Medium | Pending | Notifications, unread counts, presence |

*WebSocket adapter is complete — this phase can begin.*

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

### Existing Infrastructure

Health endpoints already exist (`/health`, `/ready`) but readiness check does not currently validate database connectivity.

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
| 2.0 | February 2026 | Updated Phase 1 and 2 status to reflect implemented features |

---

## Contributing

When working on roadmap items:

1. Create a feature branch from `main`
2. Follow existing code patterns and ADRs
3. Add tests for new functionality
4. Update relevant documentation
5. Submit PR with clear description

See [DECISIONS.md](architecture/DECISIONS.md) for architectural guidance.
