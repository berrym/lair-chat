# Real-World Testing Guide 🚀

This comprehensive guide provides step-by-step instructions for running and testing Lair Chat with multiple clients in real-world scenarios.

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Environment Setup](#environment-setup)
- [Single Machine Testing](#single-machine-testing)
- [Network Testing](#network-testing)
- [Production-Like Testing](#production-like-testing)
- [Load Testing](#load-testing)
- [Security Testing](#security-testing)
- [Troubleshooting](#troubleshooting)

## ⚡ Quick Start

### Prerequisites
- **Rust**: 1.70.0 or later
- **Terminal**: Multiple terminal windows/tabs
- **Network**: Local network access for multi-machine testing

### 1-Minute Test Setup
```bash
# Clone and build
git clone <repository-url>
cd lair-chat
cargo build --release

# Start server (Terminal 1)
./target/release/lair-chat-server

# Start client 1 (Terminal 2)
./target/release/lair-chat-client

# Start client 2 (Terminal 3)
./target/release/lair-chat-client
```

## 🛠️ Environment Setup

### Development Environment
```bash
# 1. Create environment file
cp .env.example .env

# 2. Edit configuration for testing
nano .env
```

### Basic Testing Configuration
```env
# Basic server settings
LAIR_CHAT_SERVER_HOST=127.0.0.1
LAIR_CHAT_SERVER_PORT=8080
LAIR_CHAT_SERVER_MAX_CONNECTIONS=100

# Use in-memory database for testing
LAIR_CHAT_DATABASE_URL=:memory:

# Enable debug logging
LAIR_CHAT_LOGGING_LEVEL=debug
LAIR_CHAT_LOGGING_FORMAT=pretty

# Relaxed security for testing
LAIR_CHAT_SECURITY_PASSWORD_MIN_LENGTH=4
LAIR_CHAT_SECURITY_PASSWORD_REQUIRE_SPECIAL=false
LAIR_CHAT_SECURITY_MAX_LOGIN_ATTEMPTS=10

# Enable all features
LAIR_CHAT_FEATURES_ENABLE_DIRECT_MESSAGES=true
LAIR_CHAT_FEATURES_ENABLE_USER_ROOM_CREATION=true
LAIR_CHAT_FEATURES_ENABLE_MESSAGE_HISTORY=true
LAIR_CHAT_FEATURES_ENABLE_TYPING_INDICATORS=true
```

### Build the Project
```bash
# Debug build for development testing
cargo build

# Release build for performance testing
cargo build --release

# Verify binaries exist
ls -la target/release/lair-chat-*
```

## 🖥️ Single Machine Testing

### Basic Multi-Client Test

#### Step 1: Start the Server
```bash
# Terminal 1 - Server
cd lair-chat
RUST_LOG=debug ./target/release/lair-chat-server

# Expected output:
# [INFO] Lair Chat Server starting...
# [INFO] Server listening on 127.0.0.1:8080
# [INFO] Ready to accept connections
```

#### Step 2: Start First Client
```bash
# Terminal 2 - Client 1
cd lair-chat
./target/release/lair-chat-client

# Follow prompts:
# Username: alice
# Password: test123
# Server: 127.0.0.1:8080
```

#### Step 3: Start Second Client
```bash
# Terminal 3 - Client 2
cd lair-chat
./target/release/lair-chat-client

# Follow prompts:
# Username: bob
# Password: test456
# Server: 127.0.0.1:8080
```

#### Step 4: Test Basic Chat
1. **In Alice's client**: Type a message and press Enter
2. **In Bob's client**: Verify message appears
3. **Switch perspective**: Send message from Bob to Alice
4. **Test commands**: Try `/help`, `/users`, `/rooms`

### Advanced Single Machine Testing

#### Test with Multiple Rooms
```bash
# In Alice's client
/create-room general
/join general
Hello everyone in general!

# In Bob's client
/join general
Hi Alice! I'm in general too.

# In a third client (Charlie)
/create-room dev-team
/join dev-team
Private discussion in dev-team
```

#### Test Direct Messages
```bash
# In Alice's client
/dm bob Hey Bob, private message!

# In Bob's client
/dm alice Got your message Alice!

# Verify other users don't see DMs
```

#### Test User Management
```bash
# Test registration
# Start new client, choose "Register" instead of login
Username: dave
Password: newuser123
Email: dave@example.com

# Test user list
/users
/who

# Test presence
# Close and reopen client, verify user status changes
```

## 🌐 Network Testing

### Multi-Machine Setup

#### Server Configuration
```bash
# On server machine - Edit .env
LAIR_CHAT_SERVER_HOST=0.0.0.0  # Bind to all interfaces
LAIR_CHAT_SERVER_PORT=8080

# Start server
./target/release/lair-chat-server
```

#### Client Configuration
```bash
# On each client machine
./target/release/lair-chat-client

# When prompted for server:
# Enter: <server-ip>:8080
# Example: 192.168.1.100:8080
```

#### Firewall Configuration
```bash
# On server machine (Linux)
sudo ufw allow 8080/tcp
sudo ufw reload

# On server machine (Windows)
# Add firewall rule for port 8080 in Windows Defender
```

### Network Test Scenarios

#### Basic Network Test
1. **Setup**: Server on Machine A, clients on Machines B and C
2. **Test**: Basic chat functionality across network
3. **Verify**: Messages route properly between machines
4. **Check**: Network latency and connection stability

#### Network Interruption Test
1. **Setup**: Establish chat session across network
2. **Interrupt**: Briefly disconnect client machine from network
3. **Reconnect**: Restore network connection
4. **Verify**: Client reconnects and catches up on messages

#### Cross-Subnet Test
1. **Setup**: Place server and clients on different subnets
2. **Configure**: Ensure routing between subnets
3. **Test**: Full chat functionality across subnet boundaries

## 🏭 Production-Like Testing

### Docker-Based Testing

#### Docker Compose Setup
```yaml
# docker-compose.test.yml
version: '3.8'

services:
  lair-chat-server:
    build: .
    ports:
      - "8080:8080"
      - "8081:8081"  # Admin port
    environment:
      - LAIR_CHAT_SERVER_HOST=0.0.0.0
      - LAIR_CHAT_DATABASE_URL=postgresql://lair_chat:password@postgres:5432/lair_chat
      - LAIR_CHAT_LOGGING_LEVEL=info
    depends_on:
      - postgres
    volumes:
      - ./logs:/app/logs

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=lair_chat
      - POSTGRES_USER=lair_chat
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  postgres_data:
```

#### Start Production-Like Environment
```bash
# Build and start
docker-compose -f docker-compose.test.yml up --build

# Connect clients
./target/release/lair-chat-client
# Server: localhost:8080
```

### Database Persistence Testing
```bash
# 1. Start server with database
LAIR_CHAT_DATABASE_URL=data/test.db ./target/release/lair-chat-server

# 2. Create users and send messages
# 3. Stop server
# 4. Restart server
# 5. Verify data persistence

# Check database
sqlite3 data/test.db
.tables
SELECT * FROM users;
SELECT * FROM messages;
```

### SSL/TLS Testing
```bash
# Generate test certificates
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Configure server for TLS
LAIR_CHAT_SERVER_ENABLE_TLS=true
LAIR_CHAT_SERVER_TLS_CERT_PATH=cert.pem
LAIR_CHAT_SERVER_TLS_KEY_PATH=key.pem

# Test with TLS-enabled clients
```

## ⚡ Load Testing

### Automated Load Testing

#### Simple Load Test Script
```bash
#!/bin/bash
# load_test.sh

SERVER_HOST="127.0.0.1"
SERVER_PORT="8080"
NUM_CLIENTS=10
DURATION=60  # seconds

echo "Starting load test with $NUM_CLIENTS clients for $DURATION seconds"

for i in $(seq 1 $NUM_CLIENTS); do
    {
        # Create a simple client session
        echo "user$i"
        echo "password$i"
        sleep 1
        echo "Hello from user $i"
        sleep $DURATION
    } | ./target/release/lair-chat-client &
done

wait
echo "Load test completed"
```

#### Run Load Test
```bash
chmod +x load_test.sh
./load_test.sh
```

### Performance Monitoring

#### Server Metrics
```bash
# Monitor server performance during load test
# Terminal 1 - Server with debug logging
RUST_LOG=info ./target/release/lair-chat-server

# Terminal 2 - System monitoring
top -p $(pgrep lair-chat-server)

# Terminal 3 - Network monitoring
netstat -an | grep :8080

# Terminal 4 - Memory monitoring
watch "ps aux | grep lair-chat"
```

#### Connection Limits Testing
```bash
# Test maximum connections
# Modify .env
LAIR_CHAT_SERVER_MAX_CONNECTIONS=50

# Run load test with more clients than limit
# Verify proper connection handling
```

### Message Throughput Testing
```bash
# Test script for message throughput
#!/bin/bash
# throughput_test.sh

MESSAGES_PER_CLIENT=100
NUM_CLIENTS=5

for client in $(seq 1 $NUM_CLIENTS); do
    {
        echo "throughput_user_$client"
        echo "test123"
        sleep 2
        for msg in $(seq 1 $MESSAGES_PER_CLIENT); do
            echo "Message $msg from client $client"
            sleep 0.1
        done
    } | ./target/release/lair-chat-client &
done

wait
```

## 🔒 Security Testing

### Authentication Testing

#### Valid Authentication
```bash
# Test successful login
./target/release/lair-chat-client
# Username: testuser
# Password: validpassword
# Expected: Successful connection
```

#### Invalid Authentication
```bash
# Test failed login
./target/release/lair-chat-client
# Username: testuser
# Password: wrongpassword
# Expected: Authentication failure
```

#### Brute Force Protection
```bash
# Test rate limiting
for i in {1..10}; do
    echo -e "testuser\nwrongpassword" | ./target/release/lair-chat-client
    sleep 1
done
# Expected: Account lockout after configured attempts
```

### Encryption Testing

#### Message Encryption Verification
1. **Setup**: Enable debug logging
2. **Send**: Message between clients
3. **Monitor**: Server logs for encrypted content
4. **Verify**: Plain text not visible in logs

#### Key Exchange Testing
1. **Start**: Two clients
2. **Monitor**: Key exchange process in logs
3. **Verify**: Unique session keys generated
4. **Test**: Message decryption with wrong keys fails

### Input Validation Testing

#### Message Content Testing
```bash
# Test various message types
echo "Normal message"
echo "Message with special chars: !@#$%^&*()"
echo "Very long message: $(yes 'a' | head -5000 | tr -d '\n')"
echo "Unicode message: 🚀 Hello 世界"
echo "SQL injection attempt: '; DROP TABLE users; --"
echo "XSS attempt: <script>alert('xss')</script>"
```

#### Username Validation
```bash
# Test invalid usernames
./target/release/lair-chat-client
# Try: admin, root, system, very-long-username-over-limit
```

## 🐛 Troubleshooting

### Common Issues

#### Connection Refused
```bash
# Check if server is running
ps aux | grep lair-chat-server

# Check port availability
netstat -an | grep :8080

# Check firewall
sudo ufw status | grep 8080
```

#### Database Connection Issues
```bash
# Check database file permissions
ls -la data/lair-chat.db

# Check database URL
echo $LAIR_CHAT_DATABASE_URL

# Test database connection
sqlite3 data/lair-chat.db ".tables"
```

#### High Memory Usage
```bash
# Monitor memory usage
top -p $(pgrep lair-chat-server)

# Check for memory leaks
valgrind --tool=memcheck ./target/debug/lair-chat-server
```

### Debug Mode Testing

#### Enable Detailed Logging
```bash
# Maximum verbosity
RUST_LOG=trace ./target/release/lair-chat-server

# Specific module logging
RUST_LOG=lair_chat::server=debug ./target/release/lair-chat-server
```

#### Network Debugging
```bash
# Monitor network traffic
sudo tcpdump -i lo port 8080

# Check connection states
ss -tuln | grep :8080
```

### Performance Issues

#### Slow Message Delivery
1. **Check**: Network latency with `ping`
2. **Monitor**: Server CPU usage
3. **Verify**: Message queue lengths
4. **Test**: With fewer concurrent clients

#### Connection Drops
1. **Check**: Network stability
2. **Monitor**: Server error logs
3. **Verify**: Client reconnection logic
4. **Test**: With keep-alive settings

## 📊 Test Scenarios Matrix

### Basic Functionality Tests
| Scenario | Client Count | Expected Result |
|----------|--------------|-----------------|
| Single user chat | 1 | Messages echo back |
| Two-user chat | 2 | Messages exchange |
| Multi-user room | 5 | All see messages |
| Direct messages | 3 | Only recipient sees |
| User join/leave | 4 | Notifications work |

### Stress Tests
| Scenario | Load | Duration | Metrics |
|----------|------|----------|---------|
| Connection flood | 100 clients | 30s | Connection success rate |
| Message spam | 10 clients, 10 msg/s | 60s | Message delivery rate |
| Long session | 5 clients | 30 min | Memory usage growth |
| Reconnection storm | 20 clients | 10 min | Reconnection success |

### Security Tests
| Test | Method | Expected Result |
|------|--------|-----------------|
| Invalid login | Wrong credentials | Authentication failure |
| SQL injection | Malformed input | Input sanitized |
| Buffer overflow | Oversized messages | Graceful handling |
| DoS attempt | Connection flooding | Rate limiting active |

## 🚀 Automated Testing Scripts

### Complete Test Suite
```bash
#!/bin/bash
# full_test_suite.sh

set -e

echo "🚀 Starting Lair Chat Test Suite"

# 1. Build project
echo "Building project..."
cargo build --release

# 2. Start server in background
echo "Starting server..."
./target/release/lair-chat-server &
SERVER_PID=$!
sleep 3

# 3. Run basic functionality tests
echo "Running basic tests..."
./scripts/basic_test.sh

# 4. Run load tests
echo "Running load tests..."
./scripts/load_test.sh

# 5. Run security tests
echo "Running security tests..."
./scripts/security_test.sh

# 6. Cleanup
echo "Cleaning up..."
kill $SERVER_PID

echo "✅ All tests completed successfully!"
```

### Continuous Integration Testing
```yaml
# .github/workflows/integration-test.yml
name: Integration Tests

on: [push, pull_request]

jobs:
  integration-test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable
      
    - name: Build project
      run: cargo build --release
      
    - name: Run integration tests
      run: |
        ./target/release/lair-chat-server &
        sleep 5
        ./scripts/full_test_suite.sh
```

## 📝 Testing Checklist

### Pre-Test Setup
- [ ] Environment configured (.env file)
- [ ] Project built successfully
- [ ] Network ports available
- [ ] Required dependencies installed

### Basic Functionality
- [ ] Server starts without errors
- [ ] Single client connects successfully
- [ ] Multiple clients connect simultaneously
- [ ] Messages sent and received correctly
- [ ] User authentication works
- [ ] Room creation and joining works
- [ ] Direct messages function properly

### Network Testing
- [ ] Cross-machine communication works
- [ ] Network interruption handling
- [ ] Firewall configuration correct
- [ ] SSL/TLS encryption (if enabled)

### Performance Testing
- [ ] Load testing with multiple clients
- [ ] Memory usage monitoring
- [ ] Connection limit testing
- [ ] Message throughput testing

### Security Testing
- [ ] Authentication validation
- [ ] Input sanitization
- [ ] Rate limiting
- [ ] Encryption verification

### Production Readiness
- [ ] Database persistence
- [ ] Log file rotation
- [ ] Error handling
- [ ] Resource cleanup

---

**Guide Version**: 1.0  
**Last Updated**: December 2024  
**Compatible With**: Lair Chat v0.6.3+  
**Testing Environment**: Development and Production

For additional help, see [Development Guide](development/DEVELOPMENT_GUIDE.md) and [Troubleshooting](TROUBLESHOOTING.md).