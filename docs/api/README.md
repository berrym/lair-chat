# API Documentation 🔌

This document provides comprehensive documentation for the Lair Chat REST API and WebSocket interfaces.

## 📋 Table of Contents

- [API Overview](#api-overview)
- [Authentication](#authentication)
- [REST API Endpoints](#rest-api-endpoints)
- [WebSocket API](#websocket-api)
- [Admin API](#admin-api)
- [Rate Limiting](#rate-limiting)
- [Error Handling](#error-handling)
- [SDKs and Libraries](#sdks-and-libraries)
- [Examples and Tutorials](#examples-and-tutorials)

## 🌐 API Overview

Lair Chat provides two main API interfaces:
- **REST API**: HTTP-based API for user management, configuration, and data retrieval
- **WebSocket API**: Real-time messaging and live updates

### Base URLs
- **REST API**: `https://your-server.com/api/v1`
- **WebSocket**: `wss://your-server.com/ws`
- **Admin API**: `https://your-server.com/api/v1/admin`

### API Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                       Client Applications                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Web App   │  │ Mobile App  │  │   Desktop Client    │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────┬───────────────┬───────────────┬───────────────┘
              │               │               │
              ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                      API Gateway                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │    REST     │  │  WebSocket  │  │      Admin API      │ │
│  │   Handler   │  │   Handler   │  │      Handler        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────┬───────────────┬───────────────┬───────────────┘
              │               │               │
              ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Core Application                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │    Auth     │  │    Chat     │  │      Storage        │ │
│  │   Service   │  │   Service   │  │      Service        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🔐 Authentication

### Authentication Methods
1. **API Key Authentication** (for service accounts)
2. **JWT Token Authentication** (for user sessions)
3. **Session Authentication** (for web applications)

### API Key Authentication
```http
GET /api/v1/users/me HTTP/1.1
Host: your-server.com
Authorization: Bearer sk_live_1234567890abcdef
Content-Type: application/json
```

### JWT Token Authentication
```http
POST /api/v1/auth/login HTTP/1.1
Host: your-server.com
Content-Type: application/json

{
  "username": "john.doe",
  "password": "secure_password123"
}
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "user_123",
    "username": "john.doe",
    "email": "john.doe@company.com",
    "role": "user"
  }
}
```

### Token Refresh
```http
POST /api/v1/auth/refresh HTTP/1.1
Host: your-server.com
Content-Type: application/json

{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

## 🛠️ REST API Endpoints

### Authentication Endpoints

#### Login
```http
POST /api/v1/auth/login
```

**Request Body:**
```json
{
  "username": "string",
  "password": "string",
  "remember_me": "boolean (optional)"
}
```

**Response:**
```json
{
  "access_token": "string",
  "refresh_token": "string",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "string",
    "username": "string",
    "email": "string",
    "role": "string",
    "created_at": "string (ISO 8601)",
    "last_login": "string (ISO 8601)"
  }
}
```

#### Logout
```http
POST /api/v1/auth/logout
Authorization: Bearer {token}
```

#### Password Reset
```http
POST /api/v1/auth/password-reset
```

**Request Body:**
```json
{
  "email": "string"
}
```

### User Management Endpoints

#### Get Current User
```http
GET /api/v1/users/me
Authorization: Bearer {token}
```

**Response:**
```json
{
  "id": "user_123",
  "username": "john.doe",
  "email": "john.doe@company.com",
  "full_name": "John Doe",
  "role": "user",
  "avatar_url": "https://example.com/avatar.jpg",
  "status": "active",
  "created_at": "2024-01-15T10:30:00Z",
  "last_login": "2024-12-21T09:15:00Z",
  "preferences": {
    "theme": "dark",
    "notifications": true,
    "language": "en"
  }
}
```

#### Update User Profile
```http
PUT /api/v1/users/me
Authorization: Bearer {token}
```

**Request Body:**
```json
{
  "full_name": "John D. Doe",
  "email": "john.new@company.com",
  "preferences": {
    "theme": "light",
    "notifications": false
  }
}
```

#### Upload Avatar
```http
POST /api/v1/users/me/avatar
Authorization: Bearer {token}
Content-Type: multipart/form-data

avatar: [file]
```

### Room Management Endpoints

#### List Rooms
```http
GET /api/v1/rooms
Authorization: Bearer {token}
```

**Query Parameters:**
- `limit`: Number of rooms to return (default: 50, max: 100)
- `offset`: Number of rooms to skip (default: 0)
- `type`: Filter by room type (`public`, `private`, `direct`)
- `search`: Search rooms by name

**Response:**
```json
{
  "rooms": [
    {
      "id": "room_123",
      "name": "General Discussion",
      "description": "Main chat room for general topics",
      "type": "public",
      "created_by": "user_456",
      "created_at": "2024-01-15T10:30:00Z",
      "participant_count": 42,
      "last_message": {
        "id": "msg_789",
        "content": "Hello everyone!",
        "sender": {
          "id": "user_456",
          "username": "jane.doe"
        },
        "timestamp": "2024-12-21T09:15:00Z"
      }
    }
  ],
  "total": 15,
  "limit": 50,
  "offset": 0,
  "has_more": false
}
```

#### Create Room
```http
POST /api/v1/rooms
Authorization: Bearer {token}
```

**Request Body:**
```json
{
  "name": "Project Alpha",
  "description": "Discussion room for Project Alpha",
  "type": "private",
  "participants": ["user_123", "user_456"]
}
```

#### Get Room Details
```http
GET /api/v1/rooms/{room_id}
Authorization: Bearer {token}
```

#### Join Room
```http
POST /api/v1/rooms/{room_id}/join
Authorization: Bearer {token}
```

#### Leave Room
```http
POST /api/v1/rooms/{room_id}/leave
Authorization: Bearer {token}
```

### Message Endpoints

#### Get Messages
```http
GET /api/v1/rooms/{room_id}/messages
Authorization: Bearer {token}
```

**Query Parameters:**
- `limit`: Number of messages (default: 50, max: 100)
- `before`: Get messages before this message ID
- `after`: Get messages after this message ID
- `search`: Search message content

**Response:**
```json
{
  "messages": [
    {
      "id": "msg_123",
      "content": "Hello, how is everyone doing?",
      "type": "text",
      "sender": {
        "id": "user_456",
        "username": "jane.doe",
        "avatar_url": "https://example.com/avatar.jpg"
      },
      "room_id": "room_789",
      "timestamp": "2024-12-21T09:15:00Z",
      "edited_at": null,
      "reactions": [
        {
          "emoji": "👍",
          "count": 3,
          "users": ["user_123", "user_456", "user_789"]
        }
      ],
      "thread_count": 2
    }
  ],
  "has_more": true,
  "next_cursor": "msg_122"
}
```

#### Send Message
```http
POST /api/v1/rooms/{room_id}/messages
Authorization: Bearer {token}
```

**Request Body:**
```json
{
  "content": "Hello everyone!",
  "type": "text",
  "reply_to": "msg_456",
  "attachments": [
    {
      "type": "file",
      "url": "https://example.com/file.pdf",
      "filename": "document.pdf",
      "size": 1024000
    }
  ]
}
```

#### Edit Message
```http
PUT /api/v1/messages/{message_id}
Authorization: Bearer {token}
```

#### Delete Message
```http
DELETE /api/v1/messages/{message_id}
Authorization: Bearer {token}
```

#### Add Reaction
```http
POST /api/v1/messages/{message_id}/reactions
Authorization: Bearer {token}
```

**Request Body:**
```json
{
  "emoji": "👍"
}
```

### File Upload Endpoints

#### Upload File
```http
POST /api/v1/files/upload
Authorization: Bearer {token}
Content-Type: multipart/form-data

file: [file]
room_id: "room_123"
```

**Response:**
```json
{
  "id": "file_123",
  "filename": "document.pdf",
  "size": 1024000,
  "mime_type": "application/pdf",
  "url": "https://example.com/files/file_123",
  "thumbnail_url": "https://example.com/thumbnails/file_123",
  "uploaded_by": "user_456",
  "uploaded_at": "2024-12-21T09:15:00Z"
}
```

## 🔌 WebSocket API

### Connection
```javascript
const ws = new WebSocket('wss://your-server.com/ws?token=' + accessToken);

ws.onopen = function(event) {
    console.log('Connected to Lair Chat');
};

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    handleMessage(message);
};
```

### Message Types

#### Authentication
```json
{
  "type": "auth",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

#### Join Room
```json
{
  "type": "join_room",
  "room_id": "room_123"
}
```

#### Send Message
```json
{
  "type": "message",
  "room_id": "room_123",
  "content": "Hello everyone!",
  "message_type": "text"
}
```

#### Receive Message
```json
{
  "type": "message",
  "message": {
    "id": "msg_123",
    "content": "Hello everyone!",
    "sender": {
      "id": "user_456",
      "username": "jane.doe"
    },
    "room_id": "room_123",
    "timestamp": "2024-12-21T09:15:00Z"
  }
}
```

#### User Status Updates
```json
{
  "type": "user_status",
  "user_id": "user_123",
  "status": "online",
  "last_seen": "2024-12-21T09:15:00Z"
}
```

#### Typing Indicators
```json
{
  "type": "typing",
  "room_id": "room_123",
  "user_id": "user_456",
  "typing": true
}
```

### WebSocket Events Flow
```
Client                                Server
  │                                     │
  ├─── auth ────────────────────────────▶│
  │◄─── auth_success ───────────────────┤
  │                                     │
  ├─── join_room ───────────────────────▶│
  │◄─── room_joined ────────────────────┤
  │                                     │
  ├─── message ─────────────────────────▶│
  │◄─── message_sent ───────────────────┤
  │◄─── message (broadcast) ────────────┤
  │                                     │
  ├─── typing ──────────────────────────▶│
  │◄─── typing (broadcast) ─────────────┤
  │                                     │
```

## 🛡️ Admin API

### User Management

#### List All Users
```http
GET /api/v1/admin/users
Authorization: Bearer {admin_token}
```

**Query Parameters:**
- `limit`: Number of users (default: 50)
- `offset`: Pagination offset
- `role`: Filter by role
- `status`: Filter by status
- `search`: Search by username or email

#### Create User
```http
POST /api/v1/admin/users
Authorization: Bearer {admin_token}
```

**Request Body:**
```json
{
  "username": "new.user",
  "email": "new.user@company.com",
  "full_name": "New User",
  "role": "user",
  "password": "temporary_password",
  "force_password_reset": true
}
```

#### Update User
```http
PUT /api/v1/admin/users/{user_id}
Authorization: Bearer {admin_token}
```

#### Suspend User
```http
POST /api/v1/admin/users/{user_id}/suspend
Authorization: Bearer {admin_token}
```

**Request Body:**
```json
{
  "reason": "Policy violation",
  "duration": "7d",
  "notify_user": true
}
```

### System Monitoring

#### Get System Metrics
```http
GET /api/v1/admin/metrics
Authorization: Bearer {admin_token}
```

**Response:**
```json
{
  "timestamp": "2024-12-21T09:15:00Z",
  "system": {
    "cpu_usage_percent": 25.3,
    "memory_usage_bytes": 1073741824,
    "memory_total_bytes": 4294967296,
    "disk_usage_bytes": 48318382080,
    "disk_total_bytes": 107374182400
  },
  "application": {
    "active_connections": 142,
    "total_users": 1250,
    "online_users": 89,
    "total_rooms": 45,
    "messages_per_minute": 23.5
  },
  "database": {
    "active_connections": 12,
    "total_connections": 20,
    "query_time_avg_ms": 15.2
  }
}
```

#### Get Audit Logs
```http
GET /api/v1/admin/audit-logs
Authorization: Bearer {admin_token}
```

**Query Parameters:**
- `start_date`: Start date (ISO 8601)
- `end_date`: End date (ISO 8601)
- `user_id`: Filter by user
- `action`: Filter by action type
- `limit`: Number of logs

## ⚡ Rate Limiting

### Rate Limit Headers
```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

### Rate Limit Tiers
- **Free Tier**: 100 requests/minute
- **Basic Tier**: 1,000 requests/minute
- **Premium Tier**: 10,000 requests/minute
- **Enterprise**: Custom limits

### Rate Limit Exceeded Response
```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "API rate limit exceeded",
    "details": {
      "limit": 1000,
      "reset_at": "2024-12-21T10:00:00Z"
    }
  }
}
```

## ❌ Error Handling

### Error Response Format
```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message",
    "details": {
      "field": "Additional error details"
    },
    "request_id": "req_123456789"
  }
}
```

### Common Error Codes

| HTTP Status | Error Code | Description |
|-------------|------------|-------------|
| 400 | `INVALID_REQUEST` | Malformed request |
| 401 | `UNAUTHORIZED` | Invalid or missing authentication |
| 403 | `FORBIDDEN` | Insufficient permissions |
| 404 | `NOT_FOUND` | Resource not found |
| 409 | `CONFLICT` | Resource already exists |
| 422 | `VALIDATION_ERROR` | Request validation failed |
| 429 | `RATE_LIMIT_EXCEEDED` | Too many requests |
| 500 | `INTERNAL_ERROR` | Server error |
| 503 | `SERVICE_UNAVAILABLE` | Service temporarily unavailable |

### Validation Errors
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Request validation failed",
    "details": {
      "username": ["Username is required", "Username must be at least 3 characters"],
      "email": ["Invalid email format"]
    }
  }
}
```

## 📚 SDKs and Libraries

### Official SDKs

#### JavaScript/TypeScript
```bash
npm install @lair-chat/sdk
```

```javascript
import LairChat from '@lair-chat/sdk';

const client = new LairChat({
  apiKey: 'your-api-key',
  baseUrl: 'https://your-server.com'
});

// Send a message
await client.messages.send('room_123', {
  content: 'Hello world!',
  type: 'text'
});

// Listen for messages
client.on('message', (message) => {
  console.log('New message:', message);
});
```

#### Python
```bash
pip install lair-chat-sdk
```

```python
from lair_chat import LairChatClient

client = LairChatClient(
    api_key='your-api-key',
    base_url='https://your-server.com'
)

# Send a message
message = client.messages.send('room_123', {
    'content': 'Hello world!',
    'type': 'text'
})

# Get messages
messages = client.messages.list('room_123', limit=50)
```

#### Rust
```toml
[dependencies]
lair-chat-sdk = "0.1.0"
```

```rust
use lair_chat_sdk::{Client, MessageBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("your-api-key", "https://your-server.com");
    
    // Send a message
    let message = MessageBuilder::new()
        .content("Hello world!")
        .message_type("text")
        .build();
    
    client.messages().send("room_123", message).await?;
    
    Ok(())
}
```

## 💡 Examples and Tutorials

### Basic Chat Application
```javascript
// Simple chat client example
class SimpleChatClient {
    constructor(apiKey, baseUrl) {
        this.apiKey = apiKey;
        this.baseUrl = baseUrl;
        this.ws = null;
    }
    
    async connect() {
        // Get authentication token
        const response = await fetch(`${this.baseUrl}/api/v1/auth/login`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${this.apiKey}`
            },
            body: JSON.stringify({
                username: 'your-username',
                password: 'your-password'
            })
        });
        
        const { access_token } = await response.json();
        
        // Connect WebSocket
        this.ws = new WebSocket(`${this.baseUrl.replace('http', 'ws')}/ws?token=${access_token}`);
        
        this.ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            this.handleMessage(message);
        };
    }
    
    joinRoom(roomId) {
        this.ws.send(JSON.stringify({
            type: 'join_room',
            room_id: roomId
        }));
    }
    
    sendMessage(roomId, content) {
        this.ws.send(JSON.stringify({
            type: 'message',
            room_id: roomId,
            content: content,
            message_type: 'text'
        }));
    }
    
    handleMessage(message) {
        switch (message.type) {
            case 'message':
                console.log(`New message: ${message.message.content}`);
                break;
            case 'user_status':
                console.log(`User ${message.user_id} is now ${message.status}`);
                break;
        }
    }
}
```

### File Upload Example
```javascript
async function uploadFile(file, roomId, accessToken) {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('room_id', roomId);
    
    const response = await fetch('/api/v1/files/upload', {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${accessToken}`
        },
        body: formData
    });
    
    if (!response.ok) {
        throw new Error('Upload failed');
    }
    
    const fileData = await response.json();
    
    // Send message with file attachment
    await fetch(`/api/v1/rooms/${roomId}/messages`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${accessToken}`
        },
        body: JSON.stringify({
            content: `File shared: ${fileData.filename}`,
            type: 'file',
            attachments: [fileData]
        })
    });
}
```

### Pagination Example
```javascript
async function getAllMessages(roomId, accessToken) {
    const messages = [];
    let cursor = null;
    
    do {
        const url = new URL(`/api/v1/rooms/${roomId}/messages`, window.location.origin);
        if (cursor) {
            url.searchParams.set('before', cursor);
        }
        url.searchParams.set('limit', '100');
        
        const response = await fetch(url, {
            headers: {
                'Authorization': `Bearer ${accessToken}`
            }
        });
        
        const data = await response.json();
        messages.push(...data.messages);
        
        cursor = data.has_more ? data.next_cursor : null;
        
    } while (cursor);
    
    return messages;
}
```

### Error Handling Best Practices
```javascript
async function apiRequest(url, options = {}) {
    try {
        const response = await fetch(url, {
            headers: {
                'Content-Type': 'application/json',
                ...options.headers
            },
            ...options
        });
        
        if (!response.ok) {
            const error = await response.json();
            
            switch (response.status) {
                case 401:
                    // Redirect to login
                    window.location.href = '/login';
                    break;
                case 429:
                    // Rate limited - retry after delay
                    const retryAfter = response.headers.get('Retry-After') || 60;
                    await new Promise(resolve => setTimeout(resolve, retryAfter * 1000));
                    return apiRequest(url, options);
                case 500:
                    // Server error - show user-friendly message
                    throw new Error('Server temporarily unavailable. Please try again later.');
                default:
                    throw new Error(error.error?.message || 'Request failed');
            }
        }
        
        return await response.json();
        
    } catch (error) {
        console.error('API request failed:', error);
        throw error;
    }
}
```

## 🔧 Testing the API

### Using cURL
```bash
# Login
curl -X POST https://your-server.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "test", "password": "password"}'

# Get user profile
curl -X GET https://your-server.com/api/v1/users/me \
  -H "Authorization: Bearer your_token_here"

# Send message
curl -X POST https://your-server.com/api/v1/rooms/room_123/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{"content": "Hello API!", "type": "text"}'
```

### Using Postman
Import the [Postman Collection](../examples/postman/lair-chat-api.json) for ready-to-use API requests.

### API Testing Checklist
- [ ] Authentication flows work correctly
- [ ] All endpoints return expected response formats
- [ ] Error handling works as documented
- [ ] Rate limiting is enforced
- [ ] WebSocket connections are stable
- [ ] File uploads work for various file types
- [ ] Pagination works correctly

---

**API Version**: v1  
**Last Updated**: December 2024  
**OpenAPI Specification**: [Download](openapi.yaml)