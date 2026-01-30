# Manual Testing Guide

This guide documents manual testing procedures for verifying Lair Chat functionality. These steps help developers and evaluators verify the project works correctly in real-world scenarios.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Testing HTTPS/TLS](#testing-httpstls)
- [Testing TCP with E2E Encryption](#testing-tcp-with-e2e-encryption)
- [Testing the TUI Client](#testing-the-tui-client)
- [Full Integration Test](#full-integration-test)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Tools

- **Rust 1.88+**: Install via [rustup](https://rustup.rs/)
- **OpenSSL**: For generating test certificates
- **curl**: For HTTP API testing
- **netcat (nc)**: For TCP connectivity testing (optional)

### Build the Project

```bash
git clone https://github.com/berrym/lair-chat.git
cd lair-chat

# Build both server and client in release mode
cargo build --release --workspace
```

---

## Testing HTTPS/TLS

This section verifies that the HTTP API correctly serves traffic over TLS.

### Step 1: Generate Self-Signed Certificate

```bash
# Generate a self-signed certificate valid for 365 days
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -nodes \
  -subj "/CN=localhost"
```

This creates:
- `cert.pem`: The public certificate
- `key.pem`: The private key

### Step 2: Start Server with TLS Enabled

```bash
# Set environment variables and start the server
LAIR_TLS_ENABLED=true \
LAIR_TLS_CERT_PATH=./cert.pem \
LAIR_TLS_KEY_PATH=./key.pem \
LAIR_HTTP_PORT=8443 \
cargo run --release --package lair-chat-server
```

**Expected output:**
```
INFO lair_chat_server: Lair Chat Server v0.8.0 starting...
INFO lair_chat_server: Configuration loaded
INFO lair_chat_server:   TCP port: 8080
INFO lair_chat_server:   HTTP port: 8443
INFO lair_chat_server:   Database: sqlite:lair-chat.db?mode=rwc
INFO lair_chat_server:   TLS: enabled
INFO lair_chat_server:     Certificate: ./cert.pem
INFO lair_chat_server:     Private key: ./key.pem
INFO lair_chat_server::adapters::tcp::server: TCP server listening on 0.0.0.0:8080
INFO lair_chat_server::adapters::http::server: HTTPS server listening on https://0.0.0.0:8443
INFO lair_chat_server: Server ready!
```

**If you see a panic about CryptoProvider:** The rustls crypto provider fix may not be applied. Ensure you have the latest code with the `rustls::crypto::ring::default_provider().install_default()` call in `main.rs`.

### Step 3: Verify HTTPS Health Endpoint

In another terminal:

```bash
# Test health endpoint (-k skips certificate verification for self-signed certs)
curl -k https://localhost:8443/health
```

**Expected output:**
```json
{"status":"healthy","version":"0.8.0"}
```

### Step 4: Test User Registration over HTTPS

```bash
curl -k -X POST https://localhost:8443/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","email":"test@example.com","password":"SecurePass123"}'
```

**Expected output:**
```json
{
  "user": {
    "id": "...",
    "username": "testuser",
    "email": "test@example.com",
    "role": "user",
    ...
  },
  "session": {
    "id": "...",
    "expires_at": "..."
  },
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

### Step 5: Test Login over HTTPS

```bash
curl -k -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"identifier":"testuser","password":"SecurePass123"}'
```

**Expected output:** Same structure as registration, with a new JWT token.

### Step 6: Test Authenticated Endpoint

Using the token from login/registration:

```bash
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."

curl -k https://localhost:8443/api/v1/rooms \
  -H "Authorization: Bearer $TOKEN"
```

**Expected output:**
```json
{"rooms":[],"total":0,"limit":50,"offset":0}
```

### Step 7: Verify TLS Certificate Details (Optional)

```bash
# View certificate information
openssl s_client -connect localhost:8443 -showcerts </dev/null 2>/dev/null | \
  openssl x509 -noout -text | head -20
```

### Cleanup

```bash
# Stop the server (Ctrl+C)
# Remove test certificates
rm -f cert.pem key.pem

# Optionally remove test database
rm -f lair-chat.db
```

### TLS Test Checklist

- [ ] Server starts without panicking
- [ ] Logs show "TLS: enabled" and "HTTPS server listening"
- [ ] `curl -k https://localhost:8443/health` returns healthy status
- [ ] User registration works over HTTPS
- [ ] User login works over HTTPS
- [ ] JWT-authenticated requests work over HTTPS

---

## Testing TCP with E2E Encryption

The TCP protocol uses application-layer encryption (AES-256-GCM with X25519 key exchange) rather than TLS. This encrypts message content after authentication.

### Step 1: Start Server (Without TLS for Simplicity)

```bash
cargo run --release --package lair-chat-server
```

### Step 2: Connect with the TUI Client

```bash
cargo run --release --package lair-chat-client
```

### Step 3: Register/Login and Observe Logs

With `RUST_LOG=debug`:

```bash
RUST_LOG=debug cargo run --release --package lair-chat-server
```

After the client authenticates, you should see:

```
DEBUG lair_chat_server::adapters::tcp::connection: Client 127.0.0.1:xxxxx requested encryption, generating keypair
INFO lair_chat_server::adapters::tcp::connection: Encryption enabled for connection from 127.0.0.1:xxxxx
DEBUG lair_chat_server::adapters::tcp::connection: Encryption activated for subsequent messages
```

### Step 4: Verify Encrypted Communication

With encryption enabled:
1. Messages sent after key exchange are encrypted with AES-256-GCM
2. Each message has a unique nonce
3. Wire format: `[4-byte length][12-byte nonce][ciphertext][16-byte auth tag]`

### E2E Encryption Test Checklist

- [ ] Server logs show "Client requested encryption"
- [ ] Server logs show "Encryption enabled for connection"
- [ ] Messages are successfully sent/received after encryption setup
- [ ] No plaintext message content visible in network captures (if using Wireshark)

---

## Testing the TUI Client

### Basic Functionality Test

```bash
# Start server
cargo run --release --package lair-chat-server &

# Start client
cargo run --release --package lair-chat-client
```

### Client Test Checklist

- [ ] Login screen appears
- [ ] Can register a new user (type username, Tab, type email, Tab, type password, Enter)
- [ ] Can login with existing credentials
- [ ] Chat screen loads after authentication
- [ ] Press `r` to open room list
- [ ] Press `c` to create a new room
- [ ] Press `i` to enter insert mode
- [ ] Can type and send a message (Enter to send)
- [ ] Press `Esc` to exit insert mode
- [ ] Press `Tab` to switch to users panel
- [ ] Press `?` or `F1` to open help overlay
- [ ] Press `q` to quit (shows confirmation dialog)

### Client with HTTPS Server

```bash
# Start server with TLS
LAIR_TLS_ENABLED=true \
LAIR_TLS_CERT_PATH=./cert.pem \
LAIR_TLS_KEY_PATH=./key.pem \
LAIR_HTTP_PORT=8443 \
cargo run --release --package lair-chat-server &

# Start client with HTTPS URL and insecure flag (for self-signed cert)
cargo run --release --package lair-chat-client -- \
  --http-url https://localhost:8443 \
  --insecure
```

---

## Full Integration Test

This test verifies the complete flow with multiple users.

### Step 1: Start Server

```bash
cargo run --release --package lair-chat-server
```

### Step 2: Open Two Terminal Windows for Two Clients

**Terminal A:**
```bash
cargo run --release --package lair-chat-client
# Register as "alice"
```

**Terminal B:**
```bash
cargo run --release --package lair-chat-client
# Register as "bob"
```

### Step 3: Test Multi-User Features

1. **Alice creates a room:**
   - Press `r` to open room list
   - Press `c` to create room
   - Type room name, press Enter

2. **Bob joins the room:**
   - Press `r` to open room list
   - Navigate to the room with `j`/`k`
   - Press Enter to join

3. **Alice sends a message:**
   - Press `i` to enter insert mode
   - Type a message
   - Press Enter to send

4. **Bob receives the message:**
   - Message should appear in real-time (no refresh needed)

5. **Test Direct Messaging:**
   - Press `Tab` to switch to users panel
   - Navigate to the other user
   - Press Enter to start DM
   - Send a message

### Integration Test Checklist

- [ ] Multiple clients can connect simultaneously
- [ ] Users appear in each other's user lists
- [ ] Online/offline status updates in real-time
- [ ] Room creation works
- [ ] Room joining works
- [ ] Messages are delivered in real-time to all room members
- [ ] Direct messages work between two users
- [ ] User disconnect is reflected in other clients

---

## Troubleshooting

### Server Won't Start

**"Address already in use"**
```bash
# Find and kill existing process
lsof -i :8080
kill <PID>
```

**"Failed to install rustls crypto provider"**
Ensure you're running the latest code. The fix was added in commit `79de6ad`.

### TLS Certificate Errors

**"Certificate verify failed"**
Use the `-k` flag with curl or `--insecure` with the client for self-signed certificates.

**"No such file or directory" for cert/key**
Ensure the paths are correct and the files exist:
```bash
ls -la cert.pem key.pem
```

### Client Connection Issues

**"Connection refused"**
- Verify server is running: `pgrep -f lair-chat-server`
- Check ports are listening: `ss -tlnp | grep -E "8080|8082"`

**"Failed to connect"**
- Check firewall isn't blocking ports
- Verify server address matches client's `--server` argument

### Database Issues

**"Database is locked"**
SQLite doesn't support multiple writers. Ensure only one server instance is running.

**Reset database:**
```bash
rm lair-chat.db
# Server will recreate on next start
```

---

## Automated Tests

In addition to manual testing, run the automated test suite:

```bash
# Run all tests
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo test --workspace

# Run specific test
cargo test test_name

# Check test coverage
cargo tarpaulin --workspace --out Html
```

---

## Reporting Issues

If you encounter issues during manual testing:

1. Note the exact steps to reproduce
2. Capture relevant log output (`RUST_LOG=debug`)
3. Include your environment (OS, Rust version)
4. Open an issue at https://github.com/berrym/lair-chat/issues
