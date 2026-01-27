# TCP Protocol Specification

This document specifies the Lair Chat TCP wire protocol. Any client in any programming language can implement this protocol to connect to a Lair Chat server.

**Protocol Version**: 1.1

---

## Overview

The TCP protocol provides:
- Persistent, bidirectional connection
- Real-time message delivery
- End-to-end encryption (optional)
- Low latency communication

> **Note**: As of v1.1, TCP focuses on real-time operations only. Authentication and CRUD operations should use the HTTP API. See [ADR-013](../architecture/DECISIONS.md#adr-013-protocol-responsibility-split) for rationale.

### Protocol Responsibilities

| TCP (Real-time) | HTTP (Auth & CRUD) |
|-----------------|-------------------|
| Token auth (`Authenticate`) | Login, Register, Logout |
| Send/Edit/Delete messages | Message history retrieval |
| Join/Leave rooms | Room CRUD operations |
| Accept/Decline invitations | Create invitations, list invitations |
| Typing indicators | User queries |
| Real-time events | Admin operations |

### Connection Flow (Recommended)

```
┌────────┐              ┌──────────┐              ┌────────┐
│ Client │              │   HTTP   │              │  TCP   │
└───┬────┘              └────┬─────┘              └───┬────┘
    │                        │                        │
    │─── POST /auth/login ──▶│                        │
    │◀── JWT Token + User ───│                        │
    │                        │                        │
    │─── GET /rooms ────────▶│                        │
    │◀── Room List ──────────│                        │
    │                        │                        │
    │───────────────── TCP Connect ──────────────────▶│
    │◀──────────────── ServerHello ──────────────────│
    │───────────────── ClientHello ─────────────────▶│
    │          [Optional: Key Exchange]               │
    │───────────────── Authenticate(token) ─────────▶│
    │◀──────────────── AuthenticateResponse ─────────│
    │                        │                        │
    │◀══════════════ Real-time Events ══════════════▶│
```

### Legacy Connection Flow (Deprecated)

The following flow still works for backward compatibility but is deprecated:

```
┌────────┐                                    ┌────────┐
│ Client │                                    │ Server │
└───┬────┘                                    └───┬────┘
    │                                             │
    │─────────── TCP Connect ────────────────────▶│
    │◀──────── ServerHello (version, features) ──│
    │─────────── ClientHello (version) ─────────▶│
    │          [Optional: Key Exchange]           │
    │─────────── Login/Register ────────────────▶│  ⚠️ DEPRECATED
    │◀──────── AuthSuccess (session, user) ──────│
    │◀═══════ Bidirectional Messages ═══════════▶│
```

---

## Message Framing

All messages are framed using length-prefixed encoding:

```
┌──────────────────┬─────────────────────────────┐
│ Length (4 bytes) │ JSON Payload (N bytes)      │
│ Big-endian u32   │ UTF-8 encoded               │
└──────────────────┴─────────────────────────────┘
```

### Frame Structure

1. **Length**: 4-byte unsigned integer in big-endian (network) byte order
   - Represents the length of the JSON payload only (not including the 4-byte length field)
   - Maximum value: 1,048,576 (1 MB) - messages larger than this are rejected

2. **Payload**: UTF-8 encoded JSON object

### Example

Message: `{"type":"ping"}`

```
Bytes (hex): 00 00 00 0F 7B 22 74 79 70 65 22 3A 22 70 69 6E 67 22 7D
             └─────┬────┘ └──────────────────┬──────────────────────┘
             Length: 15    JSON: {"type":"ping"}
```

### Pseudocode: Reading a Message

```python
def read_message(socket):
    # Read 4-byte length
    length_bytes = socket.read_exact(4)
    length = int.from_bytes(length_bytes, byteorder='big')
    
    # Validate length
    if length > 1_048_576:
        raise Error("Message too large")
    
    # Read payload
    payload_bytes = socket.read_exact(length)
    payload = payload_bytes.decode('utf-8')
    
    # Parse JSON
    return json.loads(payload)
```

### Pseudocode: Writing a Message

```python
def write_message(socket, message):
    # Serialize to JSON
    payload = json.dumps(message).encode('utf-8')
    
    # Check size
    if len(payload) > 1_048_576:
        raise Error("Message too large")
    
    # Write length prefix
    socket.write(len(payload).to_bytes(4, byteorder='big'))
    
    # Write payload
    socket.write(payload)
```

---

## Message Structure

All messages are JSON objects with at least a `type` field:

```json
{
  "type": "message_type",
  "request_id": "optional-client-generated-id",
  // ... type-specific fields
}
```

### Request/Response Pattern

Client requests include an optional `request_id`. Server responses include the same `request_id` to correlate responses:

```json
// Client request
{
  "type": "send_message",
  "request_id": "abc123",
  "target": { "type": "room", "room_id": "..." },
  "content": "Hello!"
}

// Server response
{
  "type": "send_message_response",
  "request_id": "abc123",
  "success": true,
  "message": { ... }
}
```

### Server-Initiated Messages

Events pushed by the server have no `request_id`:

```json
{
  "type": "message_received",
  "message": { ... }
}
```

---

## Connection Handshake

### Step 1: TCP Connection

Connect to the server's TCP port (default: 8080).

### Step 2: Server Hello

Server immediately sends:

```json
{
  "type": "server_hello",
  "version": "1.0",
  "server_name": "Lair Chat",
  "features": ["encryption", "compression"],
  "encryption_required": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| version | string | Protocol version |
| server_name | string | Server identification |
| features | array | Supported optional features |
| encryption_required | bool | Whether encryption is mandatory |

### Step 3: Client Hello

Client responds:

```json
{
  "type": "client_hello",
  "version": "1.0",
  "client_name": "My Chat Client",
  "features": ["encryption"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| version | string | Protocol version client supports |
| client_name | string | Client identification |
| features | array | Features client wants to use |

### Step 4: Version Negotiation

If versions are incompatible, server sends:

```json
{
  "type": "error",
  "code": "version_mismatch",
  "message": "Unsupported protocol version",
  "supported_versions": ["1.0"]
}
```

And closes the connection.

---

## Optional: Encryption

If encryption is enabled, a key exchange occurs after the handshake.

### Key Exchange (X25519)

**Client generates keypair and sends public key:**

```json
{
  "type": "key_exchange",
  "public_key": "base64-encoded-32-bytes"
}
```

**Server responds with its public key:**

```json
{
  "type": "key_exchange_response",
  "public_key": "base64-encoded-32-bytes"
}
```

**Both sides derive shared secret using X25519.**

### Encrypted Messages

After key exchange, all subsequent messages are encrypted:

```
┌──────────────────┬──────────────────┬─────────────────────────────┐
│ Length (4 bytes) │ Nonce (12 bytes) │ Ciphertext (N bytes)        │
│ Big-endian u32   │ Random           │ AES-256-GCM encrypted       │
└──────────────────┴──────────────────┴─────────────────────────────┘
```

- **Encryption**: AES-256-GCM
- **Nonce**: 12 random bytes, unique per message
- **Ciphertext**: Encrypted JSON payload + 16-byte auth tag

---

## Authentication

After handshake (and optional encryption), the client must authenticate.

### Authenticate (Recommended)

Use a JWT token obtained from the HTTP API to authenticate the TCP connection.

```json
{
  "type": "authenticate",
  "request_id": "auth-1",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Success response:**

```json
{
  "type": "authenticate_response",
  "request_id": "auth-1",
  "success": true,
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "email": "alice@example.com",
    "role": "user",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "session": {
    "id": "session-uuid",
    "expires_at": "2025-01-22T12:00:00Z"
  }
}
```

**Error response:**

```json
{
  "type": "authenticate_response",
  "request_id": "auth-1",
  "success": false,
  "error": {
    "code": "invalid_token",
    "message": "Token is invalid or expired"
  }
}
```

| Error Code | Meaning |
|------------|---------|
| `invalid_token` | Token is malformed or signature invalid |
| `token_expired` | Token has expired, get a new one via HTTP |
| `session_revoked` | Session was explicitly logged out |

---

### Login (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `POST /auth/login` to obtain a JWT token, then use `authenticate` above. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "login",
  "request_id": "req-1",
  "identifier": "username_or_email",
  "password": "user_password"
}
```

**Success response:**

```json
{
  "type": "login_response",
  "request_id": "req-1",
  "success": true,
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "email": "alice@example.com",
    "role": "user",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "session": {
    "id": "session-uuid",
    "expires_at": "2025-01-22T12:00:00Z"
  },
  "token": "jwt-token-string"
}
```

**Error response:**

```json
{
  "type": "login_response",
  "request_id": "req-1",
  "success": false,
  "error": {
    "code": "invalid_credentials",
    "message": "Invalid username or password"
  }
}
```

### Register (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `POST /auth/register` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "register",
  "request_id": "req-2",
  "username": "newuser",
  "email": "newuser@example.com",
  "password": "SecurePassword123!"
}
```

**Success response:**

```json
{
  "type": "register_response",
  "request_id": "req-2",
  "success": true,
  "user": { ... },
  "session": { ... },
  "token": "jwt-token-string"
}
```

**Error response:**

```json
{
  "type": "register_response",
  "request_id": "req-2",
  "success": false,
  "error": {
    "code": "username_taken",
    "message": "Username is already in use"
  }
}
```

### Logout

```json
{
  "type": "logout",
  "request_id": "req-3"
}
```

**Response:**

```json
{
  "type": "logout_response",
  "request_id": "req-3",
  "success": true
}
```

Connection is closed after logout response.

---

## Messaging

### Send Message

```json
{
  "type": "send_message",
  "request_id": "msg-1",
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "content": "Hello, everyone!"
}
```

Or for direct message:

```json
{
  "type": "send_message",
  "request_id": "msg-2",
  "target": {
    "type": "direct_message",
    "recipient": "user-uuid"
  },
  "content": "Hey, how are you?"
}
```

**Response:**

```json
{
  "type": "send_message_response",
  "request_id": "msg-1",
  "success": true,
  "message": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "author": "123e4567-e89b-12d3-a456-426614174000",
    "target": { "type": "room", "room_id": "..." },
    "content": "Hello, everyone!",
    "edited": false,
    "created_at": "2025-01-21T12:00:00Z"
  }
}
```

### Edit Message

```json
{
  "type": "edit_message",
  "request_id": "edit-1",
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "content": "Hello, everyone! (edited)"
}
```

**Response:**

```json
{
  "type": "edit_message_response",
  "request_id": "edit-1",
  "success": true,
  "message": { ... }
}
```

### Delete Message

```json
{
  "type": "delete_message",
  "request_id": "del-1",
  "message_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response:**

```json
{
  "type": "delete_message_response",
  "request_id": "del-1",
  "success": true
}
```

### Get Message History (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `GET /messages?target_type=room&target_id=...` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "get_messages",
  "request_id": "hist-1",
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "limit": 50,
  "before": "2025-01-21T12:00:00Z"
}
```

**Response:**

```json
{
  "type": "get_messages_response",
  "request_id": "hist-1",
  "success": true,
  "messages": [ ... ],
  "has_more": true
}
```

---

## Room Operations

### Create Room (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `POST /rooms` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "create_room",
  "request_id": "room-1",
  "name": "General Chat",
  "description": "A place for general discussion",
  "settings": {
    "public": true,
    "max_members": null,
    "moderated": false
  }
}
```

**Response:**

```json
{
  "type": "create_room_response",
  "request_id": "room-1",
  "success": true,
  "room": {
    "id": "789e0123-e45b-67c8-d901-234567890abc",
    "name": "General Chat",
    "description": "A place for general discussion",
    "owner": "123e4567-e89b-12d3-a456-426614174000",
    "settings": { ... },
    "created_at": "2025-01-21T12:00:00Z"
  }
}
```

### Join Room

```json
{
  "type": "join_room",
  "request_id": "join-1",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc"
}
```

**Response:**

```json
{
  "type": "join_room_response",
  "request_id": "join-1",
  "success": true,
  "room": { ... },
  "membership": {
    "room_id": "...",
    "user_id": "...",
    "room_role": "member",
    "joined_at": "2025-01-21T12:00:00Z"
  }
}
```

### Leave Room

```json
{
  "type": "leave_room",
  "request_id": "leave-1",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc"
}
```

**Response:**

```json
{
  "type": "leave_room_response",
  "request_id": "leave-1",
  "success": true
}
```

### List Rooms (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `GET /rooms` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "list_rooms",
  "request_id": "list-1",
  "filter": {
    "search": "general",
    "joined_only": false,
    "public_only": true
  },
  "limit": 20,
  "offset": 0
}
```

**Response:**

```json
{
  "type": "list_rooms_response",
  "request_id": "list-1",
  "success": true,
  "rooms": [
    {
      "room": { ... },
      "member_count": 42,
      "is_member": true
    }
  ],
  "has_more": false,
  "total_count": 5
}
```

### Get Room (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `GET /rooms/{room_id}` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "get_room",
  "request_id": "get-1",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc"
}
```

**Response:**

```json
{
  "type": "get_room_response",
  "request_id": "get-1",
  "success": true,
  "room": { ... },
  "membership": { ... },
  "member_count": 42,
  "members": [ ... ]
}
```

---

## Invitations

### Invite to Room (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `POST /invitations` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "invite_to_room",
  "request_id": "inv-1",
  "room_id": "789e0123-e45b-67c8-d901-234567890abc",
  "user_id": "456e7890-e12b-34c5-d678-901234567890",
  "message": "Join our room!"
}
```

**Response:**

```json
{
  "type": "invite_to_room_response",
  "request_id": "inv-1",
  "success": true,
  "invitation": { ... }
}
```

### Accept Invitation

```json
{
  "type": "accept_invitation",
  "request_id": "acc-1",
  "invitation_id": "aaa11111-bbbb-cccc-dddd-eeeeeeeeeeee"
}
```

### Decline Invitation

```json
{
  "type": "decline_invitation",
  "request_id": "dec-1",
  "invitation_id": "aaa11111-bbbb-cccc-dddd-eeeeeeeeeeee"
}
```

### List Invitations (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `GET /invitations` instead. This command remains for backward compatibility but will log a warning.

```json
{
  "type": "list_invitations",
  "request_id": "listinv-1"
}
```

---

## User Operations (Deprecated)

> ⚠️ **Deprecated**: Use HTTP `/users` endpoints instead. These commands remain for backward compatibility but will log warnings.

### Get User (Deprecated)

Use HTTP `GET /users/{user_id}` instead.

```json
{
  "type": "get_user",
  "request_id": "user-1",
  "user_id": "456e7890-e12b-34c5-d678-901234567890"
}
```

### List Users (Deprecated)

Use HTTP `GET /users` instead.

```json
{
  "type": "list_users",
  "request_id": "users-1",
  "filter": {
    "search": "ali",
    "online_only": false
  },
  "limit": 20,
  "offset": 0
}
```

### Get Current User (Deprecated)

Use HTTP `GET /users/me` instead.

```json
{
  "type": "get_current_user",
  "request_id": "me-1"
}
```

**Response includes unread counts:**

```json
{
  "type": "get_current_user_response",
  "request_id": "me-1",
  "success": true,
  "user": { ... },
  "unread_counts": {
    "total": 15,
    "by_room": {
      "room-uuid-1": 10,
      "room-uuid-2": 5
    },
    "by_dm": {
      "user-uuid-1": 3
    }
  }
}
```

---

## Typing Indicator

```json
{
  "type": "typing",
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  }
}
```

No response. Server broadcasts `user_typing` event to other participants.

**Rate limit**: Maximum once per 3 seconds per target.

---

## Server-Pushed Events

Events are sent by the server without a request. See [EVENTS.md](../architecture/EVENTS.md) for full details.

### Message Events

```json
{
  "type": "message_received",
  "message": { ... }
}
```

```json
{
  "type": "message_edited",
  "message": { ... },
  "previous_content": "..."
}
```

```json
{
  "type": "message_deleted",
  "message_id": "...",
  "target": { ... },
  "deleted_by": "..."
}
```

### Room Events

```json
{
  "type": "user_joined_room",
  "room_id": "...",
  "user": { ... },
  "membership": { ... }
}
```

```json
{
  "type": "user_left_room",
  "room_id": "...",
  "user_id": "...",
  "reason": "voluntary"
}
```

```json
{
  "type": "room_updated",
  "room": { ... },
  "changed_by": "...",
  "changes": [ ... ]
}
```

```json
{
  "type": "room_deleted",
  "room_id": "...",
  "room_name": "...",
  "deleted_by": "..."
}
```

### Presence Events

```json
{
  "type": "user_online",
  "user_id": "...",
  "username": "..."
}
```

```json
{
  "type": "user_offline",
  "user_id": "...",
  "username": "..."
}
```

```json
{
  "type": "user_typing",
  "user_id": "...",
  "target": { ... }
}
```

### System Events

```json
{
  "type": "server_notice",
  "message": "Server restarting in 5 minutes",
  "severity": "warning"
}
```

```json
{
  "type": "session_expiring",
  "session_id": "...",
  "expires_at": "..."
}
```

---

## Keepalive

### Ping

Client should send ping every 30 seconds:

```json
{
  "type": "ping"
}
```

**Response:**

```json
{
  "type": "pong",
  "server_time": "2025-01-21T12:00:00Z"
}
```

### Connection Timeout

Server closes connections that:
- Don't complete handshake within 30 seconds
- Don't authenticate within 60 seconds
- Don't send ping/message within 90 seconds

---

## Error Handling

### Error Response Format

```json
{
  "type": "error",
  "request_id": "original-request-id",
  "code": "error_code",
  "message": "Human-readable error message",
  "details": { }
}
```

### Error Codes

| Code | Meaning |
|------|---------|
| `version_mismatch` | Protocol version not supported |
| `invalid_message` | Malformed JSON or missing required fields |
| `unauthorized` | Not authenticated or session expired |
| `permission_denied` | Action not allowed |
| `not_found` | Requested resource doesn't exist |
| `validation_failed` | Input validation failed |
| `rate_limited` | Too many requests |
| `conflict` | Action conflicts with current state |
| `internal_error` | Server error |

### Rate Limit Error

```json
{
  "type": "error",
  "code": "rate_limited",
  "message": "Too many requests",
  "details": {
    "retry_after": 30,
    "limit": "30 messages per minute"
  }
}
```

---

## Complete Session Example (Recommended)

Using HTTP for auth, TCP for real-time:

```
# Step 1: Authenticate via HTTP
HTTP POST /auth/login {"identifier":"alice","password":"secret123"}
HTTP Response: {"user":{...},"session":{...},"token":"eyJ..."}

# Step 2: Get room list via HTTP
HTTP GET /rooms?joined_only=true
HTTP Response: {"rooms":[...],"has_more":false}

# Step 3: Connect TCP for real-time
Client: [TCP Connect to server:8080]

Server: {"type":"server_hello","version":"1.1","server_name":"Lair Chat","features":["encryption"],"encryption_required":false}

Client: {"type":"client_hello","version":"1.1","client_name":"My Client","features":[]}

Client: {"type":"authenticate","request_id":"1","token":"eyJ..."}

Server: {"type":"authenticate_response","request_id":"1","success":true,"user":{...},"session":{...}}

Client: {"type":"join_room","request_id":"2","room_id":"..."}

Server: {"type":"join_room_response","request_id":"2","success":true,"room":{...},"membership":{...}}

Client: {"type":"send_message","request_id":"3","target":{"type":"room","room_id":"..."},"content":"Hello!"}

Server: {"type":"send_message_response","request_id":"3","success":true,"message":{...}}

Server: {"type":"message_received","message":{...}}  // Also received by others

Server: {"type":"user_typing","user_id":"...","target":{"type":"room","room_id":"..."}}

Client: {"type":"ping"}

Server: {"type":"pong","server_time":"2025-01-21T12:00:00Z"}

Client: {"type":"logout","request_id":"4"}

Server: {"type":"logout_response","request_id":"4","success":true}

Server: [Connection closed]
```

## Complete Session Example (Legacy - Deprecated)

Using TCP for everything (backward compatible):

```
Client: [TCP Connect to server:8080]

Server: {"type":"server_hello","version":"1.1","server_name":"Lair Chat","features":["encryption"],"encryption_required":false}

Client: {"type":"client_hello","version":"1.1","client_name":"My Client","features":[]}

Client: {"type":"login","request_id":"1","identifier":"alice","password":"secret123"}  ⚠️ DEPRECATED

Server: {"type":"login_response","request_id":"1","success":true,"user":{...},"session":{...},"token":"..."}

Client: {"type":"list_rooms","request_id":"2","filter":{"joined_only":true},"limit":20,"offset":0}  ⚠️ DEPRECATED

Server: {"type":"list_rooms_response","request_id":"2","success":true,"rooms":[...],"has_more":false}

Client: {"type":"send_message","request_id":"3","target":{"type":"room","room_id":"..."},"content":"Hello!"}

Server: {"type":"send_message_response","request_id":"3","success":true,"message":{...}}

Server: {"type":"message_received","message":{...}}

Client: {"type":"ping"}

Server: {"type":"pong","server_time":"2025-01-21T12:00:00Z"}

Client: {"type":"logout","request_id":"4"}

Server: {"type":"logout_response","request_id":"4","success":true}

Server: [Connection closed]
```

---

## Implementation Notes

### For Client Implementers

1. **Handle partial reads**: TCP is a stream, not message-based. Always read exact byte counts.
2. **Buffer incoming data**: Messages may arrive split across multiple TCP packets.
3. **Implement reconnection**: Handle network interruptions gracefully.
4. **Track request IDs**: Correlate responses with requests for async handling.
5. **Handle events**: Server can push events at any time between request/response.
6. **Send pings**: Maintain connection with regular keepalive messages.

### For Server Implementers

1. **Validate message size**: Reject messages over 1 MB immediately.
2. **Validate JSON**: Return clear errors for malformed messages.
3. **Handle slow clients**: Don't let one slow client block others.
4. **Clean up on disconnect**: Remove sessions, notify room members.
5. **Rate limit**: Protect against abuse.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2025-01 | Protocol responsibility split (ADR-013): Added `authenticate` command, deprecated auth/CRUD commands |
| 1.0 | 2025-01 | Initial specification |
