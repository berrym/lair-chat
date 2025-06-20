# Lair Chat Admin System Guide

## 🎯 Overview

The Lair Chat Admin System is a comprehensive enterprise-grade administration interface that leverages the existing REST API infrastructure to provide powerful management capabilities. This system includes:

- **Web-based Admin Dashboard** - Modern, responsive interface
- **REST API Backend** - Full-featured API with JWT authentication
- **Role-based Access Control** - Admin, Moderator, User, Guest roles
- **Real-time Monitoring** - System health and performance metrics
- **Database Management** - SQLite with 15+ specialized tables
- **Security Features** - JWT tokens, session management, audit logging

## 🏗️ Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Admin Dashboard                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ User Mgmt   │  │ System Mon  │  │ API Tools   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                            │
                     ┌──────▼──────┐
                     │  REST API   │
                     │  (Port 8082)│
                     └──────┬──────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
    ┌───────▼──────┐ ┌──────▼──────┐ ┌─────▼─────┐
    │ JWT Auth     │ │  Storage    │ │ WebSocket │
    │ Middleware   │ │  Layer      │ │ Support   │
    └──────────────┘ └─────────────┘ └───────────┘
                            │
                     ┌──────▼──────┐
                     │ SQLite DB   │
                     │ 15+ Tables  │
                     └─────────────┘
```

### Database Schema

The system uses a comprehensive SQLite database with the following key tables:

- `users` - User accounts and profiles
- `sessions` - JWT session management
- `rooms` - Chat room definitions
- `messages` - Message storage and history
- `room_memberships` - User-room relationships
- `file_attachments` - File upload metadata
- `message_reactions` - Message reactions/emojis
- `read_receipts` - Message read status
- `audit_logs` - System activity logging
- `login_attempts` - Security monitoring
- `user_settings` - Individual user preferences
- `room_settings` - Room-specific configurations
- `invitations` - Room invitation system
- `notifications` - User notification queue
- `blocked_users` - User blocking relationships

## 🚀 Quick Start

### 1. Automated Setup

Run the comprehensive setup script:

```bash
./setup_admin_system.sh
```

This will:
- ✅ Check prerequisites
- ✅ Build all components
- ✅ Create admin user
- ✅ Initialize database
- ✅ Set up environment
- ✅ Create monitoring scripts
- ✅ Verify system integrity

### 2. Manual Setup

If you prefer manual setup:

```bash
# 1. Build the project
cargo build --release --bin lair-chat-server-new --bin create_admin_user

# 2. Create admin user
cargo run --bin create_admin_user

# 3. Start the server
cargo run --bin lair-chat-server-new

# 4. Serve the dashboard (separate terminal)
cd admin-dashboard
python3 -m http.server 8083
```

### 3. Quick Start Scripts

After setup, use these convenient scripts:

```bash
./quick_start.sh      # Start all services
./dev_start.sh        # Start in development mode
./test_api.sh         # Test API endpoints
```

## 🔐 Authentication & Security

### Admin User Creation

The system creates a default admin user:
- **Username**: `admin`
- **Password**: `AdminPassword123!`
- **Email**: `admin@example.com`

### JWT Token System

The authentication system uses JWT tokens with:
- **Access Tokens**: Short-lived (1 hour), contain user role
- **Refresh Tokens**: Long-lived (30 days), for token renewal
- **Session Validation**: Database-backed session verification
- **Role-based Access**: Admin, Moderator, User, Guest levels

### Security Features

- ✅ **Password Hashing**: Argon2 algorithm
- ✅ **Session Management**: Database-backed with expiration
- ✅ **Rate Limiting**: Configurable per endpoint
- ✅ **CORS Protection**: Configurable origins
- ✅ **Audit Logging**: All admin actions logged
- ✅ **Token Revocation**: Session invalidation support

## 📊 Admin Dashboard Features

### Overview Tab
- **Server Statistics**: User counts, room counts, message metrics
- **System Health**: Database status, memory usage, uptime
- **Real-time Updates**: Live metrics with WebSocket support

### User Management Tab
- **User List**: Paginated user directory
- **Role Management**: Promote/demote users
- **Account Status**: Activate/suspend accounts
- **User Details**: Comprehensive user profiles

### Room Management Tab
- **Room Directory**: All chat rooms with metadata
- **Room Settings**: Configure room properties
- **Membership Management**: Add/remove users from rooms
- **Room Analytics**: Usage statistics and metrics

### System Tab
- **Maintenance Tools**: Database cleanup, cache clearing
- **Audit Logs**: Searchable system activity logs
- **Health Monitoring**: Component status and diagnostics
- **Configuration**: Runtime system settings

### Tools Tab
- **API Testing**: Interactive endpoint testing
- **Database Queries**: Direct database inspection
- **Log Viewer**: Real-time log monitoring
- **Export Tools**: Data export capabilities

## 🛠️ API Endpoints

### Authentication Endpoints
```
POST /api/v1/auth/register    - User registration
POST /api/v1/auth/login       - User login
POST /api/v1/auth/refresh     - Token refresh
POST /api/v1/auth/logout      - User logout
```

### Admin Endpoints (Requires Admin Role)
```
GET  /api/v1/admin/stats      - Server statistics
GET  /api/v1/admin/health     - System health
GET  /api/v1/admin/users      - User management
PUT  /api/v1/admin/users/:id/role   - Update user role
PUT  /api/v1/admin/users/:id/status - Update user status
GET  /api/v1/admin/rooms      - Room management
GET  /api/v1/admin/audit      - Audit logs
POST /api/v1/admin/maintenance - System maintenance
```

### User Endpoints (Requires Authentication)
```
GET  /api/v1/users/profile    - Get user profile
PUT  /api/v1/users/profile    - Update user profile
GET  /api/v1/users/settings   - Get user settings
PUT  /api/v1/users/settings   - Update user settings
```

### Room Endpoints
```
GET  /api/v1/rooms           - List rooms
POST /api/v1/rooms           - Create room
GET  /api/v1/rooms/:id       - Get room details
PUT  /api/v1/rooms/:id       - Update room
DELETE /api/v1/rooms/:id     - Delete room
```

### Message Endpoints
```
GET  /api/v1/messages        - Get messages
POST /api/v1/messages        - Send message
PUT  /api/v1/messages/:id    - Edit message
DELETE /api/v1/messages/:id  - Delete message
```

## 🔧 Configuration

### Environment Variables

The system uses a `.env` file for configuration:

```bash
# Database
DATABASE_URL=sqlite:data/lair_chat.db

# JWT Authentication
JWT_SECRET=your_secure_jwt_secret_here

# Server Settings
SERVER_HOST=127.0.0.1
SERVER_PORT=8082

# Logging
RUST_LOG=info,lair_chat=debug

# Security
BCRYPT_COST=12
SESSION_TIMEOUT_HOURS=24
REFRESH_TOKEN_ROTATION=true

# Features
ENABLE_ADMIN_API=true
ENABLE_WEBSOCKETS=true
ENABLE_FILE_UPLOADS=true
ENABLE_AUDIT_LOGGING=true
```

### Advanced Configuration

For production deployments, consider:

```bash
# Production Security
JWT_SECRET=$(openssl rand -hex 32)
BCRYPT_COST=14
SESSION_TIMEOUT_HOURS=4

# Database
DATABASE_URL=postgresql://user:pass@localhost/lair_chat

# TLS/SSL
ENABLE_TLS=true
TLS_CERT_PATH=/path/to/cert.pem
TLS_KEY_PATH=/path/to/key.pem

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=30
RATE_LIMIT_BURST=5

# File Uploads
MAX_FILE_SIZE=5242880  # 5MB
ALLOWED_FILE_TYPES=txt,md,json,png,jpg
```

## 🔍 Monitoring & Debugging

### Health Monitoring

The system provides comprehensive health checks:

```bash
# Manual health check
curl http://127.0.0.1:8082/api/v1/health

# Automated monitoring
./scripts/health_check.sh
```

### Debugging Tools

```bash
# JWT Authentication Debug
cargo run --bin debug_jwt_auth

# API Testing
./test_api.sh

# Log Analysis
tail -f logs/server.log
tail -f logs/jwt_debug.log
```

### Common Issues & Solutions

#### JWT Authentication Failures
```bash
# Check JWT secret configuration
grep JWT_SECRET .env

# Verify admin user exists
sqlite3 data/lair_chat.db "SELECT username, role FROM users WHERE role = 'Admin';"

# Debug JWT token validation
cargo run --bin debug_jwt_auth
```

#### Database Connection Issues
```bash
# Check database file permissions
ls -la data/lair_chat.db

# Verify database schema
sqlite3 data/lair_chat.db ".schema"

# Check connection string
grep DATABASE_URL .env
```

#### API Endpoint Access Issues
```bash
# Test basic connectivity
curl -I http://127.0.0.1:8082/api/v1/health

# Check server logs
tail -f logs/server.log

# Verify authentication
curl -H "Authorization: Bearer YOUR_TOKEN" http://127.0.0.1:8082/api/v1/admin/stats
```

## 📈 Performance & Scaling

### Database Optimization

```sql
-- Create indexes for performance
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_messages_room_id ON messages(room_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
```

### Caching Strategy

The system implements several caching layers:
- **JWT Token Caching**: In-memory token validation cache
- **Session Caching**: Database session cache with TTL
- **User Profile Caching**: Frequently accessed user data
- **Room Metadata Caching**: Room configuration cache

### Load Balancing

For high-traffic deployments:
- Use multiple server instances behind a load balancer
- Implement Redis for shared session storage
- Use PostgreSQL cluster for database scaling
- Enable CDN for static assets

## 🔌 WebSocket Integration

The system supports real-time features via WebSocket:

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://127.0.0.1:8082/ws');

// Handle real-time updates
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    switch (data.type) {
        case 'user_count_update':
            updateUserCount(data.count);
            break;
        case 'new_message':
            handleNewMessage(data);
            break;
        case 'system_alert':
            showAlert(data.message);
            break;
    }
};
```

## 🛡️ Security Best Practices

### Production Deployment
1. **Change Default Credentials** - Never use default admin password
2. **Secure JWT Secret** - Use cryptographically secure random keys
3. **Enable HTTPS** - Always use TLS in production
4. **Rate Limiting** - Implement aggressive rate limiting
5. **Audit Logging** - Enable comprehensive audit trails
6. **Database Security** - Use strong database credentials
7. **File Upload Security** - Restrict file types and sizes
8. **CORS Configuration** - Limit allowed origins
9. **Session Management** - Implement short session timeouts
10. **Regular Updates** - Keep dependencies updated

### Security Headers
```rust
// Recommended security headers
app.layer(
    ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
);
```

## 📋 Maintenance Tasks

### Regular Maintenance
```bash
# Log rotation (run daily)
./scripts/rotate_logs.sh

# Database vacuum (run weekly)
sqlite3 data/lair_chat.db "VACUUM;"

# Clean old sessions (run daily)
sqlite3 data/lair_chat.db "DELETE FROM sessions WHERE expires_at < strftime('%s', 'now');"

# Audit log cleanup (run monthly)
sqlite3 data/lair_chat.db "DELETE FROM audit_logs WHERE created_at < strftime('%s', 'now', '-30 days');"
```

### Backup Strategy
```bash
# Database backup
sqlite3 data/lair_chat.db ".backup backup/lair_chat_$(date +%Y%m%d).db"

# Configuration backup
tar -czf backup/config_$(date +%Y%m%d).tar.gz .env config/

# Log archival
tar -czf backup/logs_$(date +%Y%m%d).tar.gz logs/
```

## 🚀 Advanced Features

### Custom Middleware
```rust
// Example: Custom audit logging middleware
pub async fn audit_middleware(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status();
    
    // Log audit entry
    let _ = state.storage.audit_logs().create_log(AuditLog {
        user_id: user_context.user_id,
        action: format!("{} {}", method, uri),
        status: status.as_u16(),
        duration_ms: duration.as_millis() as u32,
        timestamp: Utc::now(),
    }).await;
    
    response
}
```

### Plugin System (Future Enhancement)
```rust
// Plugin trait for extensibility
pub trait AdminPlugin {
    fn name(&self) -> &'static str;
    fn routes(&self) -> Vec<Router<ApiState>>;
    fn middleware(&self) -> Vec<BoxedMiddleware>;
    fn dashboard_components(&self) -> Vec<DashboardComponent>;
}
```

## 📚 API Documentation

### Interactive Documentation
- **Swagger UI**: http://127.0.0.1:8082/swagger-ui
- **OpenAPI Spec**: http://127.0.0.1:8082/api/v1/openapi.json
- **Redoc**: http://127.0.0.1:8082/redoc

### Example API Calls

#### Authentication Flow
```bash
# 1. Login
curl -X POST http://127.0.0.1:8082/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"admin","password":"AdminPassword123!"}'

# 2. Use token for protected endpoints
TOKEN="your_jwt_token_here"
curl -H "Authorization: Bearer $TOKEN" \
  http://127.0.0.1:8082/api/v1/admin/stats
```

#### User Management
```bash
# Get all users (admin only)
curl -H "Authorization: Bearer $TOKEN" \
  http://127.0.0.1:8082/api/v1/admin/users

# Update user role
curl -X PUT http://127.0.0.1:8082/api/v1/admin/users/USER_ID/role \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role":"moderator"}'
```

## 📊 Metrics & Analytics

### Built-in Metrics
- User registration rates
- Login success/failure rates
- Message volume per room
- Session duration statistics
- API endpoint usage
- Error rates by endpoint
- Database query performance
- Memory usage patterns

### Custom Metrics
```rust
// Example: Custom metrics collection
pub struct MetricsCollector {
    user_login_counter: Counter,
    message_send_histogram: Histogram,
    active_sessions_gauge: Gauge,
}

impl MetricsCollector {
    pub fn record_login(&self, success: bool) {
        self.user_login_counter
            .with_label_values(&[if success { "success" } else { "failure" }])
            .inc();
    }
    
    pub fn record_message_send_duration(&self, duration: Duration) {
        self.message_send_histogram.observe(duration.as_secs_f64());
    }
}
```

## 🔮 Future Enhancements

### Planned Features
- [ ] **Multi-tenancy Support** - Multiple isolated instances
- [ ] **Advanced Analytics** - User behavior analysis
- [ ] **Plugin Architecture** - Extensible functionality
- [ ] **Mobile App API** - React Native/Flutter support
- [ ] **Advanced Search** - Full-text search across messages
- [ ] **File Sharing** - Enhanced file upload/sharing
- [ ] **Video Chat Integration** - WebRTC support
- [ ] **Bot Framework** - Automated chat bots
- [ ] **Backup & Restore** - Automated backup system
- [ ] **High Availability** - Multi-instance clustering

### Integration Possibilities
- **LDAP/Active Directory** - Enterprise authentication
- **Slack/Discord** - Bridge integrations
- **Prometheus/Grafana** - Advanced monitoring
- **Elasticsearch** - Full-text search
- **Redis** - Distributed caching
- **Docker/Kubernetes** - Container deployment
- **AWS/GCP/Azure** - Cloud deployment

## 🤝 Contributing

### Development Setup
```bash
# Clone and setup
git clone <repository>
cd lair-chat
./setup_admin_system.sh

# Development mode
./dev_start.sh

# Run tests
cargo test
./test_api.sh
```

### Code Structure
```
lair-chat/
├── src/
│   ├── server/
│   │   ├── api/           # REST API implementation
│   │   ├── storage/       # Database layer
│   │   └── config/        # Configuration management
│   └── bin/               # Binary executables
├── admin-dashboard/       # Web dashboard
├── scripts/              # Utility scripts
├── logs/                 # Application logs
└── data/                 # Database files
```

## 📞 Support & Troubleshooting

### Common Solutions

**Problem**: "Permission denied" on database
```bash
chmod 664 data/lair_chat.db
chown $(whoami):$(whoami) data/lair_chat.db
```

**Problem**: JWT tokens not working
```bash
# Verify secret is set
grep JWT_SECRET .env

# Check token expiration
cargo run --bin debug_jwt_auth
```

**Problem**: API endpoints returning 404
```bash
# Verify server is running on correct port
netstat -tlnp | grep 8082

# Check server logs
tail -f logs/server.log
```

### Getting Help
- Check the logs in `logs/` directory
- Run `./test_api.sh` to verify system health
- Use `cargo run --bin debug_jwt_auth` for auth issues
- Review the API documentation at `/swagger-ui`

---

## 🎉 Conclusion

The Lair Chat Admin System provides a comprehensive, enterprise-grade administration interface that leverages powerful existing infrastructure. With features like JWT authentication, role-based access control, real-time monitoring, and an intuitive web dashboard, it offers everything needed for effective chat system administration.

Whether you're managing a small team chat or a large-scale communication platform, this system provides the tools and flexibility needed for success.

**Happy administering!** 🚀