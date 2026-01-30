# HTTP REST API Specification

This document specifies the Lair Chat REST API. Any client can use this API to interact with a Lair Chat server using standard HTTP requests.

**API Version**: v1
**Base URL**: `http://localhost:8082/api/v1` (or `https://` with TLS enabled)

---

## Transport Security

The HTTP API supports native TLS (HTTPS) for transport encryption. TLS is opt-in and disabled by default for development convenience.

### Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `LAIR_TLS_ENABLED` | Enable HTTPS | `false` |
| `LAIR_TLS_CERT_PATH` | Path to certificate PEM file | Required if TLS enabled |
| `LAIR_TLS_KEY_PATH` | Path to private key PEM file | Required if TLS enabled |

### Development Setup (Self-Signed Certificates)

Generate self-signed certificates for local development:

```bash
# Generate RSA key and self-signed certificate
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem \
    -days 365 -nodes -subj "/CN=localhost"

# Start server with TLS
LAIR_TLS_ENABLED=true \
LAIR_TLS_CERT_PATH=./cert.pem \
LAIR_TLS_KEY_PATH=./key.pem \
cargo run -p lair-chat-server

# Connect with client (skip verification for self-signed)
cargo run -p lair-chat-client -- --http-url https://127.0.0.1:8082 --insecure

# Test with curl
curl -k https://localhost:8082/health
```

### Production Recommendations

For production deployments:

1. **Use certificates from a trusted CA** (Let's Encrypt, etc.)
2. **Set up automatic renewal** using certbot or similar
3. **Use strong cipher suites** (rustls defaults are secure)
4. **Consider using a reverse proxy** for advanced features (rate limiting, load balancing)

### Client Connection Examples

```bash
# HTTP (development default)
curl http://localhost:8082/health

# HTTPS (production)
curl https://your-domain.com:8082/health

# HTTPS with self-signed cert (development)
curl -k https://localhost:8082/health
```

See [ADR-014](../architecture/DECISIONS.md#adr-014-native-tls-for-http-transport) for the design rationale.

---

## Overview

The REST API is the **primary protocol for authentication and CRUD operations**. Use HTTP for:
- All authentication (login, register, logout, token refresh)
- User management and queries
- Room CRUD operations
- Message history retrieval
- Invitation management
- Admin operations

For real-time messaging and presence, use the [TCP protocol](TCP.md) after authenticating via HTTP.

See [ADR-013](../architecture/DECISIONS.md#adr-013-protocol-responsibility-split) for the rationale behind this split.

The REST API provides:
- Stateless request/response communication
- JWT-based authentication
- JSON request and response bodies
- OpenAPI/Swagger documentation at `/docs`
- Rate limiting and CORS support

### Authentication

Most endpoints require authentication via JWT Bearer token:

```http
Authorization: Bearer <jwt-token>
```

Tokens are obtained from the `/auth/login` or `/auth/register` endpoints.

### Content Type

All requests with a body must use:

```http
Content-Type: application/json
```

All responses are JSON:

```http
Content-Type: application/json
```

### Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "error_code",
    "message": "Human-readable message",
    "details": { }
  }
}
```

**HTTP Status Codes:**

| Status | Meaning |
|--------|---------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request (validation error) |
| 401 | Unauthorized (missing/invalid token) |
| 403 | Forbidden (permission denied) |
| 404 | Not Found |
| 409 | Conflict |
| 422 | Unprocessable Entity |
| 429 | Rate Limited |
| 500 | Internal Server Error |

---

## Authentication Endpoints

### Register

Create a new user account.

```http
POST /auth/register
```

**Request Body:**

```json
{
  "username": "alice",
  "email": "alice@example.com",
  "password": "SecurePassword123!"
}
```

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| username | string | Yes | 3-32 chars, alphanumeric + underscore |
| email | string | Yes | Valid email format |
| password | string | Yes | Min 8 chars |

**Response (201 Created):**

```json
{
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "email": "alice@example.com",
    "role": "user",
    "created_at": "2025-01-21T12:00:00Z",
    "updated_at": "2025-01-21T12:00:00Z"
  },
  "session": {
    "id": "session-uuid",
    "expires_at": "2025-01-22T12:00:00Z"
  },
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| username_invalid | 400 | Username doesn't meet requirements |
| username_taken | 409 | Username already exists |
| email_invalid | 400 | Invalid email format |
| email_taken | 409 | Email already registered |
| password_too_weak | 400 | Password doesn't meet requirements |

---

### Login

Authenticate and get a JWT token.

```http
POST /auth/login
```

**Request Body:**

```json
{
  "identifier": "alice",
  "password": "SecurePassword123!"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| identifier | string | Yes | Username or email |
| password | string | Yes | User password |

**Response (200 OK):**

```json
{
  "user": { ... },
  "session": { ... },
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| invalid_credentials | 401 | Wrong username/email or password |
| account_locked | 403 | Too many failed attempts |
| account_banned | 403 | User is banned |

---

### Logout

Invalidate the current session.

```http
POST /auth/logout
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

### Refresh Token

Get a new JWT token before expiration.

```http
POST /auth/refresh
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2025-01-22T12:00:00Z"
}
```

---

### Change Password

Change the authenticated user's password.

```http
POST /auth/change-password
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "current_password": "OldPassword123!",
  "new_password": "NewPassword456!"
}
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

## User Endpoints

### Get Current User

Get the authenticated user's profile and stats.

```http
GET /users/me
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "username": "alice",
    "email": "alice@example.com",
    "role": "user",
    "created_at": "2025-01-21T12:00:00Z",
    "updated_at": "2025-01-21T12:00:00Z"
  },
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

### Update Profile

Update the authenticated user's profile.

```http
PATCH /users/me
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "email": "newemail@example.com"
}
```

**Response (200 OK):**

```json
{
  "user": { ... }
}
```

---

### Get User

Get a user's public profile.

```http
GET /users/{user_id}
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "user": {
    "id": "456e7890-e12b-34c5-d678-901234567890",
    "username": "bob",
    "role": "user",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "online": true
}
```

Note: Email is not included in other users' profiles.

---

### List Users

Search and list users.

```http
GET /users?search=ali&online_only=false&limit=20&offset=0
Authorization: Bearer <token>
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| search | string | - | Search by username |
| online_only | boolean | false | Only online users |
| limit | integer | 50 | Max results (1-100) |
| offset | integer | 0 | Pagination offset |

**Response (200 OK):**

```json
{
  "users": [
    {
      "user": { ... },
      "online": true
    }
  ],
  "has_more": false,
  "total_count": 5
}
```

---

## Room Endpoints

### Create Room

Create a new chat room.

```http
POST /rooms
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "name": "General Chat",
  "description": "A place for general discussion",
  "settings": {
    "public": true,
    "max_members": null,
    "moderated": false,
    "join_role": "user"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | string | Yes | 1-64 characters |
| description | string | No | Room description |
| settings | object | No | Room settings (defaults apply) |

**Response (201 Created):**

```json
{
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

---

### List Rooms

List available rooms.

```http
GET /rooms?search=general&joined_only=false&public_only=true&limit=20&offset=0
Authorization: Bearer <token>
```

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| search | string | - | Search name/description |
| joined_only | boolean | false | Only rooms user is in |
| public_only | boolean | false | Only public rooms |
| limit | integer | 50 | Max results |
| offset | integer | 0 | Pagination offset |

**Response (200 OK):**

```json
{
  "rooms": [
    {
      "room": { ... },
      "member_count": 42,
      "is_member": true
    }
  ],
  "has_more": false,
  "total_count": 10
}
```

---

### Get Room

Get room details.

```http
GET /rooms/{room_id}
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "room": { ... },
  "membership": {
    "room_id": "...",
    "user_id": "...",
    "room_role": "member",
    "joined_at": "2025-01-21T12:00:00Z"
  },
  "member_count": 42
}
```

---

### Update Room

Update room settings (owner/moderator only).

```http
PATCH /rooms/{room_id}
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "name": "New Room Name",
  "description": "Updated description",
  "settings": {
    "public": false
  }
}
```

**Response (200 OK):**

```json
{
  "room": { ... }
}
```

---

### Delete Room

Delete a room (owner only).

```http
DELETE /rooms/{room_id}
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

### Join Room

Join a room.

```http
POST /rooms/{room_id}/join
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "room": { ... },
  "membership": { ... }
}
```

---

### Leave Room

Leave a room.

```http
POST /rooms/{room_id}/leave
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

### Get Room Members

List room members. Requires room membership.

```http
GET /rooms/{room_id}/members?limit=50&offset=0
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "members": [
    {
      "user_id": "456e7890-e12b-34c5-d678-901234567890",
      "username": "alice",
      "role": "member",
      "online": true
    }
  ],
  "has_more": false
}
```

**Room Roles:**

| Role | Description |
|------|-------------|
| owner | Room creator, full control |
| moderator | Can kick members, send invitations |
| member | Regular member |

---

### Update Member Role

Change a member's role. Owner only.

```http
PUT /rooms/{room_id}/members/{user_id}/role
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "role": "moderator"
}
```

| Field | Type | Required | Valid Values |
|-------|------|----------|--------------|
| role | string | Yes | "member", "moderator" |

Note: Cannot change owner role. To transfer ownership, use room settings.

**Response (200 OK):**

```json
{
  "member": {
    "user_id": "456e7890-e12b-34c5-d678-901234567890",
    "username": "alice",
    "role": "moderator"
  }
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| permission_denied | 403 | Not the room owner |
| not_found | 404 | User is not a room member |
| invalid_role | 400 | Cannot assign owner role |

---

### Kick Member

Remove a member from the room. Owner or moderator only.

```http
DELETE /rooms/{room_id}/members/{user_id}
Authorization: Bearer <token>
```

**Permission Rules:**
- Owners can kick anyone except themselves
- Moderators can kick members, but not other moderators or owners
- Members cannot kick anyone

**Response (200 OK):**

```json
{
  "success": true
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| permission_denied | 403 | Insufficient permissions to kick target |
| not_found | 404 | User is not a room member |
| cannot_kick_self | 400 | Cannot kick yourself (use leave) |

---

## Message Endpoints

### Send Message

Send a message to a room or user.

```http
POST /messages
Authorization: Bearer <token>
```

**Request Body (room message):**

```json
{
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "content": "Hello, everyone!"
}
```

**Request Body (direct message):**

```json
{
  "target": {
    "type": "direct_message",
    "recipient": "456e7890-e12b-34c5-d678-901234567890"
  },
  "content": "Hey, how are you?"
}
```

**Response (201 Created):**

```json
{
  "message": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "author": "123e4567-e89b-12d3-a456-426614174000",
    "target": { ... },
    "content": "Hello, everyone!",
    "edited": false,
    "created_at": "2025-01-21T12:00:00Z"
  }
}
```

---

### Get Messages

Get message history.

```http
GET /messages?target_type=room&target_id={room_id}&limit=50&before=2025-01-21T12:00:00Z
Authorization: Bearer <token>
```

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| target_type | string | Yes | "room" or "direct_message" |
| target_id | string | Yes | Room ID or user ID |
| limit | integer | No | Max results (default 50) |
| before | datetime | No | Messages before this time |
| after | datetime | No | Messages after this time |

**Response (200 OK):**

```json
{
  "messages": [ ... ],
  "has_more": true,
  "total_count": 1000
}
```

---

### Edit Message

Edit a message.

```http
PATCH /messages/{message_id}
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "content": "Updated message content"
}
```

**Response (200 OK):**

```json
{
  "message": { ... }
}
```

---

### Delete Message

Delete a message.

```http
DELETE /messages/{message_id}
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

### Mark Messages Read

Mark messages as read.

```http
POST /messages/read
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "target": {
    "type": "room",
    "room_id": "789e0123-e45b-67c8-d901-234567890abc"
  },
  "up_to": "message-uuid"
}
```

**Response (200 OK):**

```json
{
  "unread_count": 5
}
```

---

## Invitation Endpoints

### Invite to Room

Invite a user to a room. **Only room owners and moderators can create invitations.**

```http
POST /invitations
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "room_id": "789e0123-e45b-67c8-d901-234567890abc",
  "user_id": "456e7890-e12b-34c5-d678-901234567890",
  "message": "Join our room!"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| room_id | string | Yes | Target room UUID |
| user_id | string | Yes | User to invite UUID |
| message | string | No | Optional invitation message |

**Response (201 Created):**

```json
{
  "invitation": {
    "id": "aaa11111-bbbb-cccc-dddd-eeeeeeeeeeee",
    "room_id": "...",
    "room_name": "General Chat",
    "inviter": "...",
    "inviter_name": "alice",
    "invitee": "...",
    "invitee_name": "bob",
    "status": "pending",
    "created_at": "2025-01-21T12:00:00Z",
    "expires_at": "2025-01-28T12:00:00Z"
  }
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| permission_denied | 403 | Not an owner or moderator |
| not_room_member | 403 | Not a member of the room |
| user_not_found | 404 | Invitee doesn't exist |
| already_member | 409 | User is already a room member |

---

### List Invitations

List pending invitations for current user.

```http
GET /invitations?limit=20&offset=0
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "invitations": [
    {
      "invitation": { ... },
      "room": { ... },
      "inviter": { ... }
    }
  ],
  "has_more": false
}
```

---

### Accept Invitation

Accept a pending invitation. Only the invitee can accept their own invitations.

```http
POST /invitations/{invitation_id}/accept
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "room": { ... },
  "membership": {
    "room_id": "...",
    "user_id": "...",
    "role": "member",
    "joined_at": "2025-01-21T12:00:00Z"
  }
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| invitation_not_found | 404 | Invitation doesn't exist |
| not_invitee | 409 | You are not the intended recipient |
| invitation_expired | 410 | Invitation has expired |

---

### Decline Invitation

Decline a pending invitation. Only the invitee can decline their own invitations.

```http
POST /invitations/{invitation_id}/decline
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

**Errors:**

| Code | Status | Condition |
|------|--------|-----------|
| invitation_not_found | 404 | Invitation doesn't exist |
| not_invitee | 409 | You are not the intended recipient |
| invitation_expired | 410 | Invitation has expired |

---

## Admin Endpoints

All admin endpoints require admin role.

### Get System Stats

```http
GET /admin/stats
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "stats": {
    "users": {
      "total": 1000,
      "online": 42,
      "created_today": 5
    },
    "rooms": {
      "total": 50,
      "public": 35,
      "active": 20
    },
    "messages": {
      "total": 100000,
      "today": 500,
      "this_hour": 50
    },
    "connections": {
      "tcp": 30,
      "http_sessions": 12,
      "websocket": 0
    },
    "uptime_seconds": 86400
  }
}
```

---

### Ban User

```http
POST /admin/users/{user_id}/ban
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "reason": "Violation of terms",
  "duration_hours": 24
}
```

**Response (200 OK):**

```json
{
  "success": true,
  "expires_at": "2025-01-22T12:00:00Z"
}
```

---

### Unban User

```http
POST /admin/users/{user_id}/unban
Authorization: Bearer <token>
```

**Response (200 OK):**

```json
{
  "success": true
}
```

---

### Delete Room (Admin)

Force-delete any room.

```http
DELETE /admin/rooms/{room_id}
Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "reason": "Violation of terms"
}
```

---

## Health Endpoints

### Health Check

```http
GET /health
```

No authentication required.

**Response (200 OK):**

```json
{
  "status": "healthy",
  "version": "0.7.0",
  "uptime_seconds": 86400
}
```

---

### Readiness Check

```http
GET /ready
```

**Response (200 OK):**

```json
{
  "ready": true,
  "database": "connected",
  "tcp_server": "running"
}
```

---

## Rate Limiting

API requests are rate limited per user:

| Category | Limit |
|----------|-------|
| Authentication | 10 per minute |
| Message sending | 30 per minute |
| General API | 100 per minute |

Rate limit headers are included in responses:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1642774800
```

When rate limited, you receive:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 30
```

```json
{
  "error": {
    "code": "rate_limited",
    "message": "Too many requests",
    "details": {
      "retry_after": 30
    }
  }
}
```

---

## Pagination

List endpoints support pagination:

```http
GET /rooms?limit=20&offset=40
```

Response includes pagination info:

```json
{
  "rooms": [ ... ],
  "has_more": true,
  "total_count": 100
}
```

---

## Filtering

List endpoints support filtering via query parameters:

```http
GET /rooms?search=general&public_only=true
GET /users?search=ali&online_only=true
GET /messages?target_type=room&target_id=...&before=2025-01-21T12:00:00Z
```

---

## Real-Time Updates

The REST API is stateless and does not support push notifications. For real-time updates:

1. **TCP Protocol (Recommended)**: After authenticating via HTTP, connect to TCP and use `authenticate` with your JWT token. TCP provides:
   - Real-time message delivery
   - Presence updates (user online/offline)
   - Typing indicators
   - Live room events

   See [TCP Protocol](TCP.md) for details.

2. **Polling**: Periodically fetch new messages via `GET /messages` (not recommended for chat)
3. **WebSocket**: Connect via `/ws` for real-time events (future)

---

## OpenAPI Specification

The full OpenAPI 3.0 specification is available at:

```
GET /docs              # Swagger UI
GET /openapi.json      # OpenAPI JSON spec
GET /openapi.yaml      # OpenAPI YAML spec
```

---

## SDK Generation

The OpenAPI spec can be used to generate client SDKs in any language:

```bash
# Generate TypeScript client
openapi-generator generate -i openapi.json -g typescript-fetch -o ./client

# Generate Python client
openapi-generator generate -i openapi.json -g python -o ./client

# Generate Go client
openapi-generator generate -i openapi.json -g go -o ./client
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v1.2 | 2025-01 | Added room member management (role update, kick), invitation RBAC |
| v1.1 | 2025-01 | Marked as primary protocol for auth/CRUD per ADR-013 |
| v1 | 2025-01 | Initial specification |
