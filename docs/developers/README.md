# Developer Guide

Welcome to Lair Chat! This guide will help you get started with developing clients, contributing to the codebase, and understanding the system architecture.

## 🚀 Quick Start for Developers

### Prerequisites
- **Rust 1.70+** - [Install from rustup.rs](https://rustup.rs/)
- **Git** - For version control
- **curl** & **jq** - For API testing
- **Docker** (optional) - For containerized development

### 1-Minute Setup
```bash
# Clone and start development environment
git clone https://github.com/your-org/lair-chat.git
cd lair-chat
./scripts/dev.sh
```

That's it! Your development environment is now running with hot-reload enabled.

## 🏗️ System Architecture

Lair Chat consists of several key components:

```
┌─────────────────────────────────────────────────────────┐
│                   Lair Chat System                     │
├─────────────────────────────────────────────────────────┤
│  🔌 TCP Server     │  🌐 REST API     │  📊 Admin UI   │
│  (Port 8080)       │  (Port 8082)     │  (/admin/)     │
├─────────────────────────────────────────────────────────┤
│  🛡️ Authentication & Authorization Layer              │
│     • JWT tokens • Role-based access • Sessions       │
├─────────────────────────────────────────────────────────┤
│  💾 Storage Layer                                      │
│     • SQLite/PostgreSQL/MySQL • Connection pooling    │
└─────────────────────────────────────────────────────────┘
```

### Core Components

1. **TCP Chat Server** (`src/bin/server.rs`)
   - Real-time messaging over TCP
   - Terminal client support
   - Binary protocol with encryption

2. **REST API Server** (`src/bin/server_new.rs`)
   - HTTP/JSON API
   - JWT authentication
   - OpenAPI documentation

3. **TUI Client** (`src/bin/client.rs`)
   - Terminal-based user interface
   - Built with Ratatui
   - Cross-platform support

4. **Web Admin Dashboard** (`admin-dashboard/`)
   - System administration
   - User management
   - Real-time monitoring

## 🔌 Developing Clients

### REST API Client Development

The REST API is the recommended way to build new clients. All functionality is available through standardized HTTP endpoints.

#### Base Configuration
```bash
# API Base URL
API_BASE="http://127.0.0.1:8082/api/v1"

# Authentication required for most endpoints
Authorization: Bearer <JWT_TOKEN>
```

#### Authentication Flow
```bash
# 1. Register a new user
curl -X POST $API_BASE/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com","password":"SecurePass123!"}'

# 2. Login to get JWT token
curl -X POST $API_BASE/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"alice","password":"SecurePass123!"}'

# Response includes access_token for future requests
```

#### Core API Endpoints

| Category | Method | Endpoint | Description |
|----------|--------|----------|-------------|
| **Auth** | POST | `/auth/register` | Register new user |
| **Auth** | POST | `/auth/login` | Login user |
| **Auth** | POST | `/auth/refresh` | Refresh token |
| **Users** | GET | `/users/profile` | Get user profile |
| **Users** | PUT | `/users/profile` | Update profile |
| **Rooms** | GET | `/rooms` | List user's rooms |
| **Rooms** | POST | `/rooms` | Create new room |
| **Rooms** | GET | `/rooms/{id}` | Get room details |
| **Messages** | GET | `/messages?room_id={id}` | Get room messages |
| **Messages** | POST | `/messages` | Send message |
| **Admin** | GET | `/admin/stats` | System statistics |
| **Admin** | GET | `/admin/health` | System health |

### Example Client Implementations

#### Python Client Example
```python
import requests
import json

class LairChatClient:
    def __init__(self, base_url="http://127.0.0.1:8082/api/v1"):
        self.base_url = base_url
        self.token = None
    
    def login(self, username, password):
        response = requests.post(f"{self.base_url}/auth/login", json={
            "identifier": username,
            "password": password
        })
        data = response.json()
        self.token = data.get("access_token")
        return self.token is not None
    
    def get_rooms(self):
        headers = {"Authorization": f"Bearer {self.token}"}
        response = requests.get(f"{self.base_url}/rooms", headers=headers)
        return response.json()
    
    def send_message(self, room_id, content):
        headers = {"Authorization": f"Bearer {self.token}"}
        response = requests.post(f"{self.base_url}/messages", 
            headers=headers,
            json={"room_id": room_id, "content": content}
        )
        return response.json()

# Usage
client = LairChatClient()
client.login("alice", "SecurePass123!")
rooms = client.get_rooms()
client.send_message(rooms["data"][0]["id"], "Hello, world!")
```

#### JavaScript/Node.js Client Example
```javascript
class LairChatClient {
    constructor(baseUrl = "http://127.0.0.1:8082/api/v1") {
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
        return !!this.token;
    }

    async getRooms() {
        const response = await fetch(`${this.baseUrl}/rooms`, {
            headers: { 'Authorization': `Bearer ${this.token}` }
        });
        return response.json();
    }

    async sendMessage(roomId, content) {
        const response = await fetch(`${this.baseUrl}/messages`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.token}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ room_id: roomId, content })
        });
        return response.json();
    }
}

// Usage
const client = new LairChatClient();
await client.login("alice", "SecurePass123!");
const rooms = await client.getRooms();
await client.sendMessage(rooms.data[0].id, "Hello from JavaScript!");
```

### WebSocket Support (Coming Soon)
```javascript
// Real-time messaging via WebSocket
const ws = new WebSocket('ws://127.0.0.1:8082/ws');
ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    console.log('New message:', message);
};
```

## 🧪 Testing Your Client

### API Testing with curl
```bash
# Source the test environment
source .env.dev

# Get admin token for testing
ADMIN_TOKEN=$(curl -s -X POST $API_BASE/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"admin","password":"DevPassword123!"}' | jq -r '.access_token')

# Test room creation
curl -X POST $API_BASE/rooms \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"test-room","description":"Test room for client development"}'

# Test message sending
curl -X POST $API_BASE/messages \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"room_id":"ROOM_ID_HERE","content":"Hello from my client!"}'
```

### Load Testing
```bash
# Test your client under load
./scripts/load-test.sh --clients 10 --duration 60
```

## 📚 API Documentation

### Interactive Documentation
- **Swagger UI**: http://127.0.0.1:8082/docs
- **OpenAPI Spec**: http://127.0.0.1:8082/api/v1/openapi.json

### Response Format
All API responses follow this structure:
```json
{
  "success": true,
  "data": { /* response data */ },
  "message": "Operation completed successfully",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Error responses:
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input data",
    "details": { /* error details */ }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Authentication
JWT tokens are required for most endpoints. Include in header:
```
Authorization: Bearer <your_jwt_token>
```

Token payload contains:
```json
{
  "sub": "user_id",
  "username": "alice",
  "role": "User",
  "session_id": "session_uuid",
  "exp": 1642248600
}
```

## 🛠️ Development Environment

### Directory Structure
```
lair-chat/
├── src/
│   ├── bin/                    # Executable binaries
│   │   ├── client.rs          # TUI client
│   │   ├── server.rs          # TCP server
│   │   └── server_new.rs      # REST API server
│   ├── client/                # Client implementation
│   ├── server/                # Server implementation
│   │   ├── api/               # REST API layer
│   │   ├── storage/           # Database layer
│   │   └── config/            # Configuration
│   └── shared_types/          # Common types
├── admin-dashboard/           # Web admin UI
├── docs/                      # Documentation
├── examples/                  # Example clients
└── scripts/                   # Development scripts
```

### Development Scripts
```bash
# Start development environment with hot-reload
./scripts/dev.sh

# Run production environment
./scripts/start.sh

# Run comprehensive UAT tests
./scripts/uat-test.sh

# Load testing
./scripts/load-test.sh
```

### Environment Variables
```bash
# Development environment (.env.dev)
DATABASE_URL=sqlite:data/lair_chat_dev.db
SERVER_HOST=127.0.0.1
SERVER_PORT=8082
TCP_PORT=8080
RUST_LOG=debug,lair_chat=trace
CORS_ALLOW_ALL=true
RATE_LIMIT_DISABLED=true
```

## 🔐 Security Considerations

### Authentication
- JWT tokens expire in 24 hours (configurable)
- Session management with database backing
- Role-based access control (Admin, Moderator, User)

### Best Practices for Client Development
1. **Store tokens securely** - Use secure storage, not localStorage
2. **Handle token expiry** - Implement refresh token logic
3. **Validate user input** - Client-side validation for UX
4. **Rate limiting** - Respect API rate limits
5. **Error handling** - Handle network errors gracefully

### Security Headers
Include these headers in requests:
```
Content-Type: application/json
User-Agent: YourClient/1.0
X-Client-Version: 1.0.0
```

## 🚀 Deployment

### Client Distribution
Package your client for distribution:

```bash
# Rust client
cargo build --release

# Python client
pip install -r requirements.txt
python setup.py bdist_wheel

# Node.js client
npm install
npm run build
npm pack
```

### Configuration
Clients should support configuration:
```json
{
  "server": {
    "api_url": "https://your-lair-chat.com/api/v1",
    "websocket_url": "wss://your-lair-chat.com/ws"
  },
  "auth": {
    "token_storage": "secure_storage",
    "auto_refresh": true
  },
  "ui": {
    "theme": "dark",
    "notifications": true
  }
}
```

## 🧪 Testing

### Unit Testing
```bash
# Run all tests
cargo test

# Run specific test module
cargo test client::tests

# Run with debug output
cargo test -- --nocapture
```

### Integration Testing
```bash
# Run integration tests
cargo test --test integration

# Test specific client functionality
cargo test --test client_integration
```

### Performance Testing
```bash
# Benchmark your client
cargo bench

# Load test the API
./scripts/load-test.sh --clients 100
```

## 🤝 Contributing

### Code Style
- Follow Rust standard formatting: `cargo fmt`
- Run linting: `cargo clippy`
- Ensure tests pass: `cargo test`

### Pull Request Process
1. Fork the repository
2. Create feature branch: `git checkout -b feature/my-client`
3. Implement your client
4. Add tests and documentation
5. Submit pull request

### Example Contributions
- Mobile clients (React Native, Flutter)
- Desktop clients (Electron, Tauri)
- CLI clients
- Bot frameworks
- Language bindings (Python, Go, etc.)

## 📞 Support

### Getting Help
- **Documentation**: [docs/](../README.md)
- **API Reference**: http://127.0.0.1:8082/docs
- **Issues**: [GitHub Issues](https://github.com/your-org/lair-chat/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/lair-chat/discussions)

### Common Issues

**Q: API returns 401 Unauthorized**
A: Check your JWT token is valid and included in Authorization header

**Q: Connection refused to 127.0.0.1:8082**
A: Ensure the server is running with `./scripts/start.sh` or `./scripts/dev.sh`

**Q: Messages not appearing in real-time**
A: Use WebSocket connection for real-time updates (HTTP API is request/response only)

### Development Tips
1. Use the development environment (`./scripts/dev.sh`) for faster iteration
2. Check logs in `dev-logs/` for debugging
3. Use the admin dashboard to monitor your client's behavior
4. Test with multiple users using different browsers/clients
5. Use curl for quick API testing before implementing in your client

---

**Happy coding! Build amazing clients with Lair Chat! 🚀**