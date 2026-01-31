# WebSocket Protocol Specification

This document specifies the Lair Chat WebSocket protocol. The WebSocket adapter provides TCP protocol parity for web browser clients.

**Protocol Version**: 1.0

---

## Overview

The WebSocket protocol provides:
- Persistent, bidirectional connection over HTTP/HTTPS
- Real-time message delivery
- Browser compatibility (no custom framing needed)
- TLS encryption at the transport layer (when using `wss://`)

### Key Differences from TCP

| Feature | TCP | WebSocket |
|---------|-----|-----------|
| Framing | Length-prefixed (4-byte header) | Native WebSocket framing |
| Port | 8080 (default) | 8082 (shared with HTTP) |
| Encryption | Optional per-message (X25519 + AES-256-GCM) | TLS at transport layer |
| Pre-auth | Not supported | Query parameter token |
| Key Exchange | Explicit handshake step | Not needed |

### Connection URL

```
ws://server:8082/ws      # Without TLS
wss://server:8082/ws     # With TLS (recommended)
```

### Optional Pre-Authentication

You can pre-authenticate by passing a JWT token as a query parameter:

```
wss://server:8082/ws?token=eyJhbGciOiJIUzI1NiI...
```

When a valid token is provided, the connection skips the handshake/auth phases and is immediately authenticated.

---

## Connection Flow

### Standard Flow (Recommended)

```
┌────────┐              ┌──────────┐              ┌───────────┐
│ Client │              │   HTTP   │              │ WebSocket │
└───┬────┘              └────┬─────┘              └─────┬─────┘
    │                        │                          │
    │─── POST /auth/login ──▶│                          │
    │◀── JWT Token + User ───│                          │
    │                        │                          │
    │─── GET /rooms ────────▶│                          │
    │◀── Room List ──────────│                          │
    │                        │                          │
    │───────────────── WebSocket Upgrade ──────────────▶│
    │◀──────────────── ServerHello ────────────────────│
    │───────────────── ClientHello ───────────────────▶│
    │───────────────── Authenticate(token) ───────────▶│
    │◀──────────────── AuthenticateResponse ──────────│
    │                        │                          │
    │◀══════════════ Real-time Events ════════════════▶│
```

### Pre-Authenticated Flow

```
┌────────┐              ┌──────────┐              ┌───────────┐
│ Client │              │   HTTP   │              │ WebSocket │
└───┬────┘              └────┬─────┘              └─────┬─────┘
    │                        │                          │
    │─── POST /auth/login ──▶│                          │
    │◀── JWT Token + User ───│                          │
    │                        │                          │
    │───── WebSocket Upgrade: /ws?token=JWT ───────────▶│
    │◀──────────────── ServerHello ────────────────────│
    │◀──────────────── AuthenticateResponse ──────────│
    │                        │                          │
    │◀══════════════ Real-time Events ════════════════▶│
```

---

## Message Format

All messages are plain JSON (no length prefix). WebSocket handles message boundaries automatically.

### Client Messages

The message format is identical to TCP. See [TCP.md](TCP.md) for the complete message reference.

**Example - Client Hello**:
```json
{
  "type": "client_hello",
  "version": "1.0",
  "features": []
}
```

**Example - Authenticate**:
```json
{
  "type": "authenticate",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Example - Send Message**:
```json
{
  "type": "send_message",
  "target": {
    "room": "550e8400-e29b-41d4-a716-446655440000"
  },
  "content": "Hello, world!"
}
```

### Server Messages

The server sends the same message types as TCP. See [TCP.md](TCP.md) for the complete message reference.

**Example - Server Hello**:
```json
{
  "type": "server_hello",
  "version": "1.0",
  "server_name": "Lair Chat",
  "features": ["encryption"],
  "encryption_required": false
}
```

**Example - Message Received** (real-time event):
```json
{
  "type": "message_received",
  "message": {
    "id": "...",
    "author": "...",
    "target": { "room": "..." },
    "content": "Hello!",
    "created_at": "2025-01-31T12:00:00Z"
  },
  "author_username": "alice"
}
```

---

## Timeouts

| Phase | Timeout | Description |
|-------|---------|-------------|
| Handshake | 30s | Time to complete ClientHello |
| Authentication | 60s | Time to authenticate after handshake |
| Idle | 90s | Time without any message (use Ping to keep alive) |

### Keepalive

Send periodic Ping messages to prevent idle timeout:

```json
{"type": "ping"}
```

Server responds with:

```json
{"type": "pong", "server_time": "2025-01-31T12:00:00Z"}
```

---

## Encryption

Unlike TCP, WebSocket does not support in-protocol encryption negotiation. Instead:

1. **Transport encryption**: Use `wss://` (WebSocket Secure) for encrypted connections
2. **Key exchange messages are rejected**: Sending `key_exchange` returns an error

This simplifies client implementation since TLS is handled by the browser.

---

## JavaScript Client Example

```javascript
// Get JWT from HTTP API
const loginResponse = await fetch('/api/v1/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ identifier: 'alice', password: 'secret' })
});
const { token } = await loginResponse.json();

// Connect with pre-authentication
const ws = new WebSocket(`wss://server:8082/ws?token=${token}`);

ws.onopen = () => {
  console.log('Connected and authenticated');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  switch (message.type) {
    case 'message_received':
      console.log(`${message.author_username}: ${message.message.content}`);
      break;
    case 'user_joined_room':
      console.log(`${message.user.username} joined the room`);
      break;
    // ... handle other events
  }
};

// Send a message
function sendMessage(roomId, content) {
  ws.send(JSON.stringify({
    type: 'send_message',
    target: { room: roomId },
    content: content
  }));
}

// Keep alive
setInterval(() => {
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'ping' }));
  }
}, 30000);
```

---

## Error Handling

Errors are returned in the same format as TCP:

```json
{
  "type": "error",
  "code": "unauthorized",
  "message": "Must authenticate first"
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `unauthorized` | Action requires authentication |
| `invalid_message` | Malformed JSON or unknown message type |
| `version_mismatch` | Incompatible protocol version |
| `not_supported` | Feature not available (e.g., key exchange) |
| `timeout` | Connection timed out |

---

## TUI Client Usage

The Lair Chat TUI client supports WebSocket transport via the `-w` or `--websocket` flag:

```bash
# Connect using WebSocket instead of TCP
lair-chat-client -s 127.0.0.1:8080 --websocket

# With TLS (when server has HTTPS enabled)
lair-chat-client -s server.example.com:8080 --http-url https://server.example.com:8082 --websocket
```

### When to Use WebSocket

Use WebSocket transport when:
- Connecting through corporate firewalls that block non-HTTP ports
- Using HTTP-only network proxies
- You need browser compatibility for future web clients
- TLS encryption at the transport layer is sufficient

Use TCP transport when:
- You need end-to-end encryption (E2E with X25519 key exchange)
- You want lower latency (no HTTP upgrade overhead)
- Direct connections are available

---

## See Also

- [TCP Protocol](TCP.md) - Complete message type reference
- [HTTP API](HTTP.md) - REST API for authentication and CRUD operations
