# Lair Chat REST API: Complete Usage Guide

This guide explains the real-world purpose, functionality, and practical usage of Lair Chat's REST API server.

## 🎯 What is the REST API Server?

The REST API server is the **HTTP-based backend service** that provides:

### **Core Purpose:**
- **Data Management**: CRUD operations for users, rooms, messages, and settings
- **Authentication**: Secure login/logout, token management, and authorization
- **Administration**: User management, system monitoring, and configuration
- **Integration**: Allows external applications to interact with Lair Chat
- **Web Interface**: Powers web-based clients and admin dashboards

### **Why Use REST API vs Direct TCP Chat?**
- **TCP Chat**: Real-time messaging, low latency, terminal-based
- **REST API**: Data management, web integration, mobile apps, administration

## 🏗️ Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Client    │    │   Mobile App    │    │  Admin Panel    │
│  (Browser/JS)   │    │ (iOS/Android)   │    │   (Dashboard)   │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────┴─────────────┐
                    │     REST API Server       │
                    │    (Port 8082)           │
                    │                          │
                    │  • Authentication        │
                    │  • User Management       │
                    │  • Room Management       │
                    │  • Message History       │
                    │  • Admin Functions       │
                    └─────────────┬─────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │      Storage Layer        │
                    │    (SQLite Database)      │
                    └───────────────────────────┘
```

## 🚀 Getting Started

### 1. Start the REST API Server

```bash
# Method 1: Using the fixed startup script (recommended)
./start_server_fixed.sh

# Method 2: Manual start with correct database
DATABASE_URL="sqlite:data/lair-chat.db" ./target/release/lair-chat-server

# Method 3: Development mode
DATABASE_URL="sqlite:data/lair-chat.db" cargo run --bin lair-chat-server
```

### 2. Verify Server is Running

```bash
# Health check
curl http://localhost:8082/api/v1/health

# Expected response:
{
  "database": "healthy",
  "service": "lair-chat-api", 
  "status": "ok",
  "timestamp": "2025-06-18T01:07:02Z",
  "version": "0.6.3"
}
```

### 3. Access Points

- **REST API**: `http://localhost:8082/api/v1/`
- **Admin Dashboard**: `http://localhost:8082/admin/`
- **API Documentation**: `http://localhost:8082/docs`
- **Server Info**: `http://localhost:8082/`

## 🔐 Authentication Workflow

### Step 1: Login to Get Token

```bash
curl -X POST http://localhost:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "identifier": "admin",
    "password": "AdminPassword123!",
    "remember_me": true
  }'
```

**Response:**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "b3140c06-532a-4320-8711-de93f3499b3c",
    "username": "admin",
    "email": "admin@example.com",
    "display_name": "Administrator admin",
    "role": "admin",
    "status": "active",
    "created_at": "2025-06-18T00:50:08Z"
  },
  "session": {
    "id": "2b6fb601-c543-4412-80c8-7c899cc6d840",
    "created_at": "2025-06-18T01:07:02Z",
    "expires_at": "2025-07-18T01:07:02Z"
  }
}
```

### Step 2: Use Token for Authenticated Requests

```bash
# Save token for convenience
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."

# Make authenticated requests
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/users/me
```

## 📚 Core API Endpoints

### 🧑‍💼 User Management

#### Get Current User Info
```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/users/me
```

#### Register New User
```bash
curl -X POST http://localhost:8082/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "email": "user@example.com",
    "password": "SecurePass123!",
    "full_name": "New User"
  }'
```

#### List All Users (Admin Only)
```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/users
```

### 🏠 Room Management

#### List User's Rooms
```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/rooms
```

#### Create New Room
```bash
curl -X POST http://localhost:8082/api/v1/rooms \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "General Discussion",
    "description": "Main chat room for everyone",
    "is_private": false
  }'
```

#### Get Room Details
```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/rooms/{room_id}
```

### 💬 Message Management

#### Get Message History
```bash
curl -H "Authorization: Bearer $TOKEN" \
     "http://localhost:8082/api/v1/rooms/{room_id}/messages?limit=50&before=2024-01-01T00:00:00Z"
```

#### Send Message
```bash
curl -X POST http://localhost:8082/api/v1/rooms/{room_id}/messages \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Hello everyone!",
    "message_type": "text"
  }'
```

### 👥 Session Management

#### List Active Sessions
```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/sessions
```

#### Logout (Invalidate Session)
```bash
curl -X POST http://localhost:8082/api/v1/auth/logout \
  -H "Authorization: Bearer $TOKEN"
```

## 🛡️ Admin API Functions

### System Health Monitoring

```bash
# Get system metrics
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/metrics

# Get audit logs
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/audit-logs
```

### User Administration

```bash
# Get admin user info
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/users/{user_id}

# Update user role
curl -X PATCH http://localhost:8082/api/v1/admin/users/{user_id}/role \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role": "moderator"}'

# Deactivate user
curl -X POST http://localhost:8082/api/v1/admin/users/{user_id}/deactivate \
  -H "Authorization: Bearer $TOKEN"
```

## 🔧 Practical Use Cases

### 1. Building a Web Chat Application

```javascript
class LairChatClient {
  constructor(baseUrl) {
    this.baseUrl = baseUrl;
    this.token = null;
  }

  async login(username, password) {
    const response = await fetch(`${this.baseUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ identifier: username, password })
    });
    
    const data = await response.json();
    this.token = data.access_token;
    return data.user;
  }

  async getRooms() {
    const response = await fetch(`${this.baseUrl}/rooms`, {
      headers: { 'Authorization': `Bearer ${this.token}` }
    });
    return response.json();
  }

  async sendMessage(roomId, content) {
    const response = await fetch(`${this.baseUrl}/rooms/${roomId}/messages`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ content, message_type: 'text' })
    });
    return response.json();
  }
}

// Usage
const client = new LairChatClient('http://localhost:8082/api/v1');
await client.login('admin', 'AdminPassword123!');
const rooms = await client.getRooms();
await client.sendMessage(rooms[0].id, 'Hello from web app!');
```

### 2. Mobile App Integration

```swift
// iOS Swift example
class LairChatAPI {
    private let baseURL = "http://localhost:8082/api/v1"
    private var token: String?
    
    func login(username: String, password: String) async throws -> User {
        let url = URL(string: "\(baseURL)/auth/login")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = ["identifier": username, "password": password]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (data, _) = try await URLSession.shared.data(for: request)
        let response = try JSONDecoder().decode(LoginResponse.self, from: data)
        
        self.token = response.access_token
        return response.user
    }
    
    func getRooms() async throws -> [Room] {
        let url = URL(string: "\(baseURL)/rooms")!
        var request = URLRequest(url: url)
        request.setValue("Bearer \(token!)", forHTTPHeaderField: "Authorization")
        
        let (data, _) = try await URLSession.shared.data(for: request)
        return try JSONDecoder().decode([Room].self, from: data)
    }
}
```

### 3. Administrative Scripts

```python
# Python admin script
import requests
import json

class LairChatAdmin:
    def __init__(self, base_url):
        self.base_url = base_url
        self.token = None
    
    def login(self, username, password):
        response = requests.post(f"{self.base_url}/auth/login", json={
            "identifier": username,
            "password": password
        })
        data = response.json()
        self.token = data["access_token"]
        return data["user"]
    
    def get_all_users(self):
        headers = {"Authorization": f"Bearer {self.token}"}
        response = requests.get(f"{self.base_url}/admin/users", headers=headers)
        return response.json()
    
    def create_user(self, username, email, password, role="user"):
        headers = {"Authorization": f"Bearer {self.token}"}
        data = {
            "username": username,
            "email": email,
            "password": password,
            "role": role
        }
        response = requests.post(f"{self.base_url}/admin/users", 
                               headers=headers, json=data)
        return response.json()

# Usage
admin = LairChatAdmin("http://localhost:8082/api/v1")
admin.login("admin", "AdminPassword123!")

# Bulk user creation
users_to_create = [
    ("alice", "alice@company.com", "TempPass123!", "user"),
    ("bob", "bob@company.com", "TempPass123!", "moderator"),
]

for username, email, password, role in users_to_create:
    user = admin.create_user(username, email, password, role)
    print(f"Created user: {user['username']} with role: {user['role']}")
```

### 4. Data Export/Backup

```bash
#!/bin/bash
# Backup script using REST API

TOKEN=$(curl -s -X POST http://localhost:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"admin","password":"AdminPassword123!"}' \
  | jq -r '.access_token')

# Export all users
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/users > backup_users.json

# Export all rooms
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/rooms > backup_rooms.json

# Export system metrics
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/metrics > backup_metrics.json

echo "Backup completed: backup_*.json files created"
```

## 🔍 Monitoring and Debugging

### API Logs
```bash
# Watch server logs
tail -f logs/server.log

# Filter for API requests
tail -f logs/server.log | grep "HTTP"

# Filter for authentication
tail -f logs/server.log | grep "auth"
```

### Health Checks
```bash
# Simple health check
curl http://localhost:8082/api/v1/health

# Detailed system status (admin required)
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8082/api/v1/admin/status
```

### Testing API Endpoints
```bash
# Test script for all major endpoints
#!/bin/bash
set -e

BASE_URL="http://localhost:8082/api/v1"

echo "Testing Lair Chat REST API..."

# 1. Health check
echo "1. Health check..."
curl -s "$BASE_URL/health" | jq .

# 2. Login
echo "2. Testing login..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"identifier":"admin","password":"AdminPassword123!"}')

TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')

# 3. Get user info
echo "3. Getting user info..."
curl -s -H "Authorization: Bearer $TOKEN" "$BASE_URL/users/me" | jq .

# 4. List rooms
echo "4. Listing rooms..."
curl -s -H "Authorization: Bearer $TOKEN" "$BASE_URL/rooms" | jq .

# 5. Admin functions
echo "5. Testing admin functions..."
curl -s -H "Authorization: Bearer $TOKEN" "$BASE_URL/admin/users" | jq .

echo "All tests completed successfully!"
```

## 🎯 Benefits and Use Cases

### **When to Use REST API:**

1. **Web Applications**: Browser-based chat interfaces
2. **Mobile Apps**: iOS/Android applications
3. **Integrations**: Connect with other services/platforms
4. **Administration**: User management, system monitoring
5. **Data Analysis**: Export chat data, generate reports
6. **Automation**: Automated user creation, bulk operations
7. **External Systems**: CRM integration, notification systems

### **Key Benefits:**

- **Stateless**: Easy to scale and cache
- **Standard HTTP**: Works with any HTTP client/library
- **JSON Format**: Easy to parse in any language
- **Authentication**: Secure token-based access
- **Documentation**: Self-documenting with OpenAPI/Swagger
- **Cross-Platform**: Works on web, mobile, desktop, servers

## 🚨 Security Best Practices

1. **Always use HTTPS in production**
2. **Store tokens securely** (not in localStorage for sensitive apps)
3. **Implement token refresh** before expiration
4. **Validate all input** on both client and server
5. **Use proper CORS settings** for web applications
6. **Monitor and log** API usage for security
7. **Rate limit** API calls to prevent abuse

## 🔗 Integration Examples

The REST API enables Lair Chat to integrate with:
- **Slack/Discord bots** (bridge messages)
- **Customer support systems** (embed chat)
- **Business applications** (notifications, team chat)
- **Analytics platforms** (message data export)
- **Authentication systems** (SSO integration)
- **Mobile applications** (native chat apps)

This REST API server transforms Lair Chat from a simple terminal application into a full-featured, integration-ready chat platform suitable for modern web and mobile applications.