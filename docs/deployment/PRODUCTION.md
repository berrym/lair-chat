# Production Deployment Guide

This guide covers deploying Lair Chat in production environments, including platform support, security configuration, and operational considerations.

## Table of Contents

- [Platform Support](#platform-support)
- [Getting Binaries](#getting-binaries)
- [Configuration Reference](#configuration-reference)
- [Linux Deployment](#linux-deployment)
- [Windows Deployment](#windows-deployment)
- [macOS Deployment](#macos-deployment)
- [Docker Deployment](#alternative-docker-deployment)
- [Reverse Proxy Setup](#alternative-reverse-proxy-setup)
- [Security Considerations](#security-considerations)
- [Monitoring and Operations](#monitoring-and-operations)
- [Production Readiness Checklist](#production-readiness-checklist)
- [Current Limitations](#current-limitations)
- [Roadmap](#roadmap)

---

## Platform Support

Lair Chat is built with cross-platform compatibility as a goal. The codebase uses pure Rust dependencies with minimal platform-specific code.

### Supported Platforms

| Platform | Architecture | Status | Notes |
|----------|--------------|--------|-------|
| **Linux** | x86_64 | Fully Supported | Primary development platform |
| **Linux** | x86_64 (musl) | Fully Supported | Static binary, runs on any Linux |
| **Linux** | ARM64 | Fully Supported | Raspberry Pi 4+, ARM servers |
| **macOS** | x86_64 (Intel) | Fully Supported | macOS 10.15+ |
| **macOS** | ARM64 (Apple Silicon) | Fully Supported | M1/M2/M3 Macs |
| **Windows** | x86_64 | Fully Supported | Windows 10+ |
| **FreeBSD** | x86_64 | Should Work | Not officially tested |

### Why Cross-Platform Works

Lair Chat avoids platform-specific dependencies:

- **TLS**: Uses `rustls` (pure Rust) instead of OpenSSL
- **Cryptography**: Pure Rust implementations (`x25519-dalek`, `aes-gcm`, `argon2`)
- **Terminal UI**: `crossterm` + `ratatui` - explicitly cross-platform
- **HTTP Client**: `reqwest` with `rustls-tls` feature
- **Async Runtime**: Tokio - cross-platform

The only platform-specific code is signal handling in the server:
- Unix: Handles both SIGTERM and Ctrl+C
- Windows: Handles Ctrl+C only (graceful fallback)

### Linux Distribution Compatibility

For Linux, two build variants are provided:

| Build | libc | Compatibility | File Size |
|-------|------|---------------|-----------|
| `linux-x86_64` | glibc (dynamic) | Ubuntu 22.04+, Fedora 38+, Debian 12+ | Smaller |
| `linux-x86_64-musl` | musl (static) | Any Linux with kernel 2.6.39+ | Larger |

**Recommendation**: Use the `musl` build for maximum compatibility, especially when deploying to containers or unknown Linux distributions.

### Client Platform Notes

**Clipboard Support** (`arboard` crate):
- **Linux/X11**: Uses X11 clipboard
- **Linux/Wayland**: Uses `wl-copy`/`wl-paste` (requires `wl-clipboard` installed)
- **macOS**: Uses native `NSPasteboard`
- **Windows**: Uses native clipboard API

---

## Getting Binaries

### Option 1: GitHub Releases (Recommended for Production)

Pre-built binaries are available for all supported platforms at:
https://github.com/berrym/lair-chat/releases

Each release includes:
```
lair-chat-linux-x86_64.tar.gz      # Linux (glibc)
lair-chat-linux-x86_64-musl.tar.gz # Linux (static)
lair-chat-linux-aarch64.tar.gz     # Linux ARM64
lair-chat-macos-x86_64.tar.gz      # macOS Intel
lair-chat-macos-aarch64.tar.gz     # macOS Apple Silicon
lair-chat-windows-x86_64.zip       # Windows
```

### Option 2: Build from Source

Requirements:
- Rust 1.88+ ([rustup.rs](https://rustup.rs/))

```bash
# Clone repository
git clone https://github.com/berrym/lair-chat.git
cd lair-chat

# Build release binaries
cargo build --release --workspace

# Binaries are in:
# target/release/lair-chat-server
# target/release/lair-chat-client
```

### Option 3: Cross-Compile

To build for other platforms:

```bash
# Add target
rustup target add x86_64-unknown-linux-musl

# Build for target
cargo build --release --target x86_64-unknown-linux-musl
```

Available targets:
- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

---

## Configuration Reference

Lair Chat is configured via environment variables. There is no configuration file (yet).

### Required for Production

| Variable | Description | Example |
|----------|-------------|---------|
| `LAIR_JWT_SECRET` | **Critical**: JWT signing secret. Must be set for persistent sessions. | `openssl rand -base64 32` |
| `LAIR_TLS_ENABLED` | Enable TLS for HTTP API | `true` |
| `LAIR_TLS_CERT_PATH` | Path to TLS certificate | `/etc/letsencrypt/live/chat.example.com/fullchain.pem` |
| `LAIR_TLS_KEY_PATH` | Path to TLS private key | `/etc/letsencrypt/live/chat.example.com/privkey.pem` |

### Optional Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `LAIR_TCP_PORT` | `8080` | TCP server port (real-time messaging) |
| `LAIR_HTTP_PORT` | `8082` | HTTP server port (auth, CRUD, queries) |
| `LAIR_DATABASE_URL` | `sqlite:lair-chat.db?mode=rwc` | SQLite database path |
| `RUST_LOG` | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |

### Database URLs

SQLite (default):
```bash
LAIR_DATABASE_URL=sqlite:/var/lib/lair-chat/data.db?mode=rwc
```

PostgreSQL (supported but requires schema adaptation):
```bash
LAIR_DATABASE_URL=postgres://user:pass@localhost/lair_chat
```

MySQL (supported but requires schema adaptation):
```bash
LAIR_DATABASE_URL=mysql://user:pass@localhost/lair_chat
```

---

## Linux Deployment

### Network Architecture

```
                    Internet
                        │
                        ▼
              ┌─────────────────┐
              │    Firewall     │
              │  (ports 443,    │
              │   8080 open)    │
              └────────┬────────┘
                       │
         ┌─────────────┴─────────────┐
         │                           │
         ▼                           ▼
   ┌───────────┐              ┌───────────┐
   │  HTTPS    │              │    TCP    │
   │  :443     │              │   :8080   │
   │ (TLS API) │              │ (realtime)│
   └─────┬─────┘              └─────┬─────┘
         │                          │
         └──────────┬───────────────┘
                    │
                    ▼
           ┌───────────────┐
           │  lair-chat-   │
           │    server     │
           └───────┬───────┘
                   │
                   ▼
           ┌───────────────┐
           │    SQLite     │
           │   Database    │
           └───────────────┘
```

### Step 1: Create System User

```bash
# Create dedicated user (no login shell)
sudo useradd --system --no-create-home --shell /usr/sbin/nologin lair-chat

# Create data directory
sudo mkdir -p /var/lib/lair-chat
sudo chown lair-chat:lair-chat /var/lib/lair-chat
sudo chmod 750 /var/lib/lair-chat
```

### Step 2: Install Binary

```bash
# Download latest release (example for Linux x86_64)
curl -LO https://github.com/berrym/lair-chat/releases/latest/download/lair-chat-linux-x86_64.tar.gz

# Extract and install
tar xzf lair-chat-linux-x86_64.tar.gz
sudo mv lair-chat-server /usr/local/bin/
sudo chmod +x /usr/local/bin/lair-chat-server
```

### Step 3: TLS Certificate

**Option A: Let's Encrypt (Recommended)**

```bash
# Install certbot
sudo apt install certbot  # Debian/Ubuntu
sudo dnf install certbot  # Fedora

# Get certificate (stop any service on port 80 first)
sudo certbot certonly --standalone -d chat.yourdomain.com

# Certificate files will be at:
# /etc/letsencrypt/live/chat.yourdomain.com/fullchain.pem
# /etc/letsencrypt/live/chat.yourdomain.com/privkey.pem

# Set up auto-renewal
sudo systemctl enable certbot-renew.timer
```

**Option B: Self-Signed (Development Only)**

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost"
```

### Step 4: Create Systemd Service

Create `/etc/systemd/system/lair-chat.service`:

```ini
[Unit]
Description=Lair Chat Server
Documentation=https://github.com/berrym/lair-chat
After=network.target

[Service]
Type=simple
User=lair-chat
Group=lair-chat
WorkingDirectory=/var/lib/lair-chat

# Configuration
Environment=LAIR_TCP_PORT=8080
Environment=LAIR_HTTP_PORT=443
Environment=LAIR_DATABASE_URL=sqlite:/var/lib/lair-chat/lair-chat.db?mode=rwc
Environment=LAIR_TLS_ENABLED=true
Environment=LAIR_TLS_CERT_PATH=/etc/letsencrypt/live/chat.yourdomain.com/fullchain.pem
Environment=LAIR_TLS_KEY_PATH=/etc/letsencrypt/live/chat.yourdomain.com/privkey.pem
Environment=RUST_LOG=info

# Load JWT secret from file (more secure than inline)
EnvironmentFile=/etc/lair-chat/secrets

ExecStart=/usr/local/bin/lair-chat-server
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/lair-chat

# Allow binding to privileged ports
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

Create `/etc/lair-chat/secrets`:

```bash
sudo mkdir -p /etc/lair-chat
echo "LAIR_JWT_SECRET=$(openssl rand -base64 32)" | sudo tee /etc/lair-chat/secrets
sudo chmod 600 /etc/lair-chat/secrets
sudo chown root:lair-chat /etc/lair-chat/secrets
```

### Step 5: Configure Firewall

**UFW (Ubuntu/Debian)**:
```bash
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 8080/tcp  # TCP real-time
sudo ufw reload
```

**firewalld (Fedora/RHEL)**:
```bash
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload
```

### Step 6: Start Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable lair-chat
sudo systemctl start lair-chat

# Check status
sudo systemctl status lair-chat

# View logs
sudo journalctl -u lair-chat -f
```

---

## Windows Deployment

### Step 1: Install Binary

Download the Windows release from GitHub:
```powershell
# Download latest release
Invoke-WebRequest -Uri "https://github.com/berrym/lair-chat/releases/latest/download/lair-chat-windows-x86_64.zip" -OutFile "lair-chat.zip"

# Extract
Expand-Archive -Path "lair-chat.zip" -DestinationPath "C:\Program Files\LairChat"
```

### Step 2: Create Data Directory

```powershell
# Create data directory
New-Item -ItemType Directory -Force -Path "C:\ProgramData\LairChat"
```

### Step 3: Configure Environment Variables

**Option A: System Environment Variables (Recommended for Services)**

```powershell
# Set environment variables (run as Administrator)
[System.Environment]::SetEnvironmentVariable("LAIR_TCP_PORT", "8080", "Machine")
[System.Environment]::SetEnvironmentVariable("LAIR_HTTP_PORT", "443", "Machine")
[System.Environment]::SetEnvironmentVariable("LAIR_DATABASE_URL", "sqlite:C:\ProgramData\LairChat\lair-chat.db?mode=rwc", "Machine")
[System.Environment]::SetEnvironmentVariable("LAIR_TLS_ENABLED", "true", "Machine")
[System.Environment]::SetEnvironmentVariable("LAIR_TLS_CERT_PATH", "C:\ProgramData\LairChat\cert.pem", "Machine")
[System.Environment]::SetEnvironmentVariable("LAIR_TLS_KEY_PATH", "C:\ProgramData\LairChat\key.pem", "Machine")

# Generate and set JWT secret
$secret = [Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Maximum 256 }))
[System.Environment]::SetEnvironmentVariable("LAIR_JWT_SECRET", $secret, "Machine")
```

**Option B: Environment File with Wrapper Script**

Create `C:\Program Files\LairChat\start-server.ps1`:
```powershell
$env:LAIR_TCP_PORT = "8080"
$env:LAIR_HTTP_PORT = "443"
$env:LAIR_DATABASE_URL = "sqlite:C:\ProgramData\LairChat\lair-chat.db?mode=rwc"
$env:LAIR_JWT_SECRET = "your-secret-here"
$env:LAIR_TLS_ENABLED = "true"
$env:LAIR_TLS_CERT_PATH = "C:\ProgramData\LairChat\cert.pem"
$env:LAIR_TLS_KEY_PATH = "C:\ProgramData\LairChat\key.pem"

& "C:\Program Files\LairChat\lair-chat-server.exe"
```

### Step 4: TLS Certificate

**Option A: Use win-acme for Let's Encrypt**

Download win-acme from https://www.win-acme.com/:
```powershell
# Run win-acme to obtain certificate
wacs.exe --target manual --host chat.yourdomain.com --store pemfiles --pemfilespath "C:\ProgramData\LairChat"
```

**Option B: Self-Signed (Development Only)**

```powershell
# Generate self-signed certificate using OpenSSL (install via winget or chocolatey)
openssl req -x509 -newkey rsa:4096 -keyout "C:\ProgramData\LairChat\key.pem" -out "C:\ProgramData\LairChat\cert.pem" -days 365 -nodes -subj "/CN=localhost"
```

### Step 5: Install as Windows Service

**Using NSSM (Non-Sucking Service Manager) - Recommended**

Download NSSM from https://nssm.cc/:
```powershell
# Install as service
nssm install LairChat "C:\Program Files\LairChat\lair-chat-server.exe"

# Configure service
nssm set LairChat AppDirectory "C:\ProgramData\LairChat"
nssm set LairChat DisplayName "Lair Chat Server"
nssm set LairChat Description "Secure chat server with real-time messaging"
nssm set LairChat Start SERVICE_AUTO_START

# Set environment variables for the service
nssm set LairChat AppEnvironmentExtra "LAIR_TCP_PORT=8080" "LAIR_HTTP_PORT=443" "LAIR_DATABASE_URL=sqlite:C:\ProgramData\LairChat\lair-chat.db?mode=rwc"

# Start the service
nssm start LairChat
```

**Using sc.exe (Built-in)**

Create a wrapper batch file `C:\Program Files\LairChat\run-server.bat`:
```batch
@echo off
set LAIR_TCP_PORT=8080
set LAIR_HTTP_PORT=443
set LAIR_DATABASE_URL=sqlite:C:\ProgramData\LairChat\lair-chat.db?mode=rwc
set LAIR_JWT_SECRET=your-secret-here
"C:\Program Files\LairChat\lair-chat-server.exe"
```

Then use a service wrapper like WinSW or run with Task Scheduler.

### Step 6: Configure Windows Firewall

```powershell
# Run as Administrator
New-NetFirewallRule -DisplayName "Lair Chat HTTPS" -Direction Inbound -Protocol TCP -LocalPort 443 -Action Allow
New-NetFirewallRule -DisplayName "Lair Chat TCP" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow
```

Or via GUI:
1. Open Windows Defender Firewall with Advanced Security
2. Click "Inbound Rules" → "New Rule"
3. Select "Port" → TCP → Specific ports: 443, 8080
4. Allow the connection
5. Apply to Domain, Private, and Public as appropriate

### Step 7: Manage the Service

```powershell
# Check status
Get-Service LairChat

# Stop service
Stop-Service LairChat

# Start service
Start-Service LairChat

# View logs (if using NSSM with logging enabled)
Get-Content "C:\ProgramData\LairChat\logs\service.log" -Tail 50

# Or check Windows Event Viewer for service events
```

---

## macOS Deployment

### Step 1: Install Binary

Download the macOS release from GitHub:
```bash
# For Apple Silicon (M1/M2/M3)
curl -LO https://github.com/berrym/lair-chat/releases/latest/download/lair-chat-macos-aarch64.tar.gz
tar xzf lair-chat-macos-aarch64.tar.gz

# For Intel Macs
curl -LO https://github.com/berrym/lair-chat/releases/latest/download/lair-chat-macos-x86_64.tar.gz
tar xzf lair-chat-macos-x86_64.tar.gz

# Install
sudo mv lair-chat-server /usr/local/bin/
sudo chmod +x /usr/local/bin/lair-chat-server
```

### Step 2: Create Data Directory

```bash
# Create directories
sudo mkdir -p /usr/local/var/lair-chat
sudo mkdir -p /usr/local/etc/lair-chat

# Create dedicated user (optional but recommended)
sudo dscl . -create /Users/lair-chat
sudo dscl . -create /Users/lair-chat UserShell /usr/bin/false
sudo dscl . -create /Users/lair-chat UniqueID 599
sudo dscl . -create /Users/lair-chat PrimaryGroupID 20
sudo dscl . -create /Users/lair-chat NFSHomeDirectory /var/empty

# Set ownership
sudo chown -R lair-chat:staff /usr/local/var/lair-chat
```

### Step 3: TLS Certificate

**Option A: Let's Encrypt with Certbot**

```bash
# Install certbot via Homebrew
brew install certbot

# Obtain certificate
sudo certbot certonly --standalone -d chat.yourdomain.com

# Certificates will be in /etc/letsencrypt/live/chat.yourdomain.com/
```

**Option B: Self-Signed (Development Only)**

```bash
openssl req -x509 -newkey rsa:4096 \
  -keyout /usr/local/etc/lair-chat/key.pem \
  -out /usr/local/etc/lair-chat/cert.pem \
  -days 365 -nodes -subj "/CN=localhost"
```

### Step 4: Create Environment File

Create `/usr/local/etc/lair-chat/environment`:
```bash
LAIR_TCP_PORT=8080
LAIR_HTTP_PORT=443
LAIR_DATABASE_URL=sqlite:/usr/local/var/lair-chat/lair-chat.db?mode=rwc
LAIR_JWT_SECRET=your-generated-secret-here
LAIR_TLS_ENABLED=true
LAIR_TLS_CERT_PATH=/etc/letsencrypt/live/chat.yourdomain.com/fullchain.pem
LAIR_TLS_KEY_PATH=/etc/letsencrypt/live/chat.yourdomain.com/privkey.pem
RUST_LOG=info
```

Generate JWT secret:
```bash
echo "LAIR_JWT_SECRET=$(openssl rand -base64 32)" | sudo tee -a /usr/local/etc/lair-chat/environment
```

### Step 5: Create launchd Service

Create `/Library/LaunchDaemons/com.lair-chat.server.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.lair-chat.server</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/lair-chat-server</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>LAIR_TCP_PORT</key>
        <string>8080</string>
        <key>LAIR_HTTP_PORT</key>
        <string>443</string>
        <key>LAIR_DATABASE_URL</key>
        <string>sqlite:/usr/local/var/lair-chat/lair-chat.db?mode=rwc</string>
        <key>LAIR_JWT_SECRET</key>
        <string>your-generated-secret-here</string>
        <key>LAIR_TLS_ENABLED</key>
        <string>true</string>
        <key>LAIR_TLS_CERT_PATH</key>
        <string>/etc/letsencrypt/live/chat.yourdomain.com/fullchain.pem</string>
        <key>LAIR_TLS_KEY_PATH</key>
        <string>/etc/letsencrypt/live/chat.yourdomain.com/privkey.pem</string>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>

    <key>UserName</key>
    <string>lair-chat</string>

    <key>WorkingDirectory</key>
    <string>/usr/local/var/lair-chat</string>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/usr/local/var/lair-chat/lair-chat.log</string>

    <key>StandardErrorPath</key>
    <string>/usr/local/var/lair-chat/lair-chat.error.log</string>
</dict>
</plist>
```

Set permissions:
```bash
sudo chown root:wheel /Library/LaunchDaemons/com.lair-chat.server.plist
sudo chmod 644 /Library/LaunchDaemons/com.lair-chat.server.plist
```

### Step 6: Configure macOS Firewall

**Option A: Using pfctl (Packet Filter)**

Create `/etc/pf.anchors/com.lair-chat`:
```
pass in proto tcp from any to any port 443
pass in proto tcp from any to any port 8080
```

Add to `/etc/pf.conf`:
```
anchor "com.lair-chat"
load anchor "com.lair-chat" from "/etc/pf.anchors/com.lair-chat"
```

Reload:
```bash
sudo pfctl -f /etc/pf.conf
```

**Option B: Using macOS Firewall GUI**

1. System Preferences → Security & Privacy → Firewall
2. Click "Firewall Options"
3. Add `/usr/local/bin/lair-chat-server`
4. Set to "Allow incoming connections"

**Note**: macOS firewall is application-based by default, so adding the binary should suffice.

### Step 7: Start and Manage the Service

```bash
# Load and start the service
sudo launchctl load /Library/LaunchDaemons/com.lair-chat.server.plist

# Check if running
sudo launchctl list | grep lair-chat

# Stop the service
sudo launchctl unload /Library/LaunchDaemons/com.lair-chat.server.plist

# View logs
tail -f /usr/local/var/lair-chat/lair-chat.log

# Restart after config changes
sudo launchctl unload /Library/LaunchDaemons/com.lair-chat.server.plist
sudo launchctl load /Library/LaunchDaemons/com.lair-chat.server.plist
```

### macOS-Specific Notes

1. **Privileged Ports**: Binding to port 443 requires root or the service to run as root. The launchd plist runs as the `lair-chat` user, so you may need to use a higher port (e.g., 8443) and set up port forwarding, or run as root.

2. **Gatekeeper**: If downloading binaries directly, you may need to remove the quarantine attribute:
   ```bash
   xattr -d com.apple.quarantine /usr/local/bin/lair-chat-server
   ```

3. **Code Signing**: For distribution, consider signing the binary with an Apple Developer certificate to avoid Gatekeeper warnings.

---

## Alternative: Docker Deployment

> **Note**: Docker support is planned but not yet implemented. This section shows the intended approach.

Future `Dockerfile`:

```dockerfile
FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --package lair-chat-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/lair-chat-server /usr/local/bin/
EXPOSE 8080 8082
CMD ["lair-chat-server"]
```

Future `docker-compose.yml`:

```yaml
version: '3.8'
services:
  lair-chat:
    build: .
    ports:
      - "8080:8080"
      - "443:8082"
    environment:
      - LAIR_JWT_SECRET=${LAIR_JWT_SECRET}
      - LAIR_TLS_ENABLED=true
      - LAIR_TLS_CERT_PATH=/certs/fullchain.pem
      - LAIR_TLS_KEY_PATH=/certs/privkey.pem
    volumes:
      - ./data:/var/lib/lair-chat
      - /etc/letsencrypt/live/chat.example.com:/certs:ro
    restart: unless-stopped
```

---

## Alternative: Reverse Proxy Setup

If you prefer handling TLS at a reverse proxy (nginx, Caddy, Traefik):

### Nginx Configuration

```nginx
# /etc/nginx/sites-available/lair-chat

# HTTP API (proxied with TLS termination)
server {
    listen 443 ssl http2;
    server_name chat.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/chat.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/chat.yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8082;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# TCP is NOT proxied - clients connect directly
# Ensure port 8080 is open in firewall
```

With this setup:
- HTTP API: `https://chat.yourdomain.com` (nginx handles TLS)
- TCP: `chat.yourdomain.com:8080` (direct connection)

Set `LAIR_TLS_ENABLED=false` when using a reverse proxy for TLS.

### Caddy Configuration

```caddyfile
chat.yourdomain.com {
    reverse_proxy localhost:8082
}
```

Caddy automatically provisions Let's Encrypt certificates.

---

## Security Considerations

### Production Security Checklist

- [ ] **TLS Enabled**: Never run HTTP in production
- [ ] **Strong JWT Secret**: Use `openssl rand -base64 32`, never use defaults
- [ ] **Firewall Configured**: Only expose necessary ports (443, 8080)
- [ ] **Dedicated User**: Run as non-root with minimal permissions
- [ ] **Database Secured**: SQLite file permissions 600, owned by service user
- [ ] **Logs Secured**: Ensure log files don't expose sensitive data
- [ ] **Updates**: Keep Rust dependencies updated for security patches

### Password Security

Lair Chat uses Argon2id for password hashing with secure defaults:
- Memory: 19 MiB
- Iterations: 2
- Parallelism: 1

These parameters are resistant to GPU cracking attacks.

### JWT Security

- Tokens expire after 24 hours by default
- Tokens are signed with HS256
- **Critical**: The JWT secret must be:
  - At least 32 bytes of random data
  - Kept secret (never commit to version control)
  - Consistent across restarts (for session persistence)

### Database Security

SQLite:
- Store database file outside web-accessible directories
- Set file permissions to 600
- Regular backups (the database is a single file)

### Rate Limiting

Built-in rate limiting protects against abuse:
- Registration: 5 requests/minute per IP
- Login: 10 requests/minute per IP
- General API: 100 requests/minute per IP

---

## Monitoring and Operations

### Health Check

```bash
# HTTP health endpoint
curl https://chat.yourdomain.com/health
# Response: {"status":"ok"}
```

### Logging

Logs are written to stdout/stderr (captured by systemd/Docker).

Log levels:
- `error`: Failures requiring attention
- `warn`: Potential issues
- `info`: Normal operations (default)
- `debug`: Detailed debugging
- `trace`: Very verbose (not for production)

View logs:
```bash
# Systemd
sudo journalctl -u lair-chat -f

# With timestamps
sudo journalctl -u lair-chat --since "1 hour ago"

# Filter by level
sudo journalctl -u lair-chat | grep ERROR
```

### Backup

SQLite database backup:
```bash
# Online backup (safe while server is running)
sqlite3 /var/lib/lair-chat/lair-chat.db ".backup /backup/lair-chat-$(date +%Y%m%d).db"

# Or simply copy (stop server first for consistency)
sudo systemctl stop lair-chat
cp /var/lib/lair-chat/lair-chat.db /backup/
sudo systemctl start lair-chat
```

Automated backup script:
```bash
#!/bin/bash
BACKUP_DIR=/backup/lair-chat
mkdir -p $BACKUP_DIR
sqlite3 /var/lib/lair-chat/lair-chat.db ".backup $BACKUP_DIR/lair-chat-$(date +%Y%m%d-%H%M%S).db"
# Keep last 7 days
find $BACKUP_DIR -name "*.db" -mtime +7 -delete
```

### Metrics (Planned)

Prometheus metrics endpoint is planned for future releases:
- Active connections
- Messages per second
- Request latencies
- Error rates

---

## Production Readiness Checklist

### What's Implemented

- [x] Core messaging functionality (rooms, DMs, real-time events)
- [x] User authentication (register, login, JWT sessions)
- [x] TLS/HTTPS support (rustls)
- [x] SQLite storage with migrations
- [x] Password hashing (Argon2id)
- [x] Rate limiting middleware
- [x] Input validation
- [x] Cross-platform builds (CI/CD)
- [x] Graceful shutdown handling
- [x] Comprehensive test suite (800+ tests)

### What's Missing for Production

| Feature | Priority | Status |
|---------|----------|--------|
| Bind address configuration | Medium | Hardcoded to `0.0.0.0` |
| Configuration file support | Medium | Environment variables only |
| Docker support | Medium | Not implemented |
| Health check with details | Low | Basic `/health` only |
| Prometheus metrics | Low | Not implemented |
| Admin dashboard | Low | Not implemented |
| Email verification | Low | Not implemented |
| Password reset | Low | Not implemented |

---

## Current Limitations

### Known Issues

1. **No TCP TLS**: The TCP protocol (port 8080) does not support TLS. Real-time messages are sent in plaintext unless tunneled through a VPN or SSH.

2. **No Configuration File**: All configuration is via environment variables. A TOML/YAML config file would be more convenient.

3. **Single Server Only**: No clustering or horizontal scaling support. The event dispatcher uses in-memory broadcast channels.

4. **SQLite Limitations**: While SQLite works well for small-medium deployments, high-write scenarios may benefit from PostgreSQL.

### Performance Expectations

Based on the architecture:
- **Concurrent Connections**: Tested up to 1,000 simultaneous TCP connections
- **Message Throughput**: ~10,000 messages/second on modern hardware
- **Latency**: Sub-millisecond for local, network-dependent otherwise

SQLite can handle:
- Small deployment: 1-50 users, no issues
- Medium deployment: 50-500 users, should work
- Large deployment: 500+ users, consider PostgreSQL

---

## Roadmap

Planned improvements for production deployments:

### Short Term
- [ ] TCP TLS support (encrypted real-time connections)
- [ ] Configurable bind address (`LAIR_BIND_ADDRESS`)
- [ ] Docker and docker-compose support
- [ ] Health check endpoint with database status

### Medium Term
- [ ] TOML/YAML configuration file
- [ ] Prometheus metrics endpoint
- [ ] PostgreSQL optimization and testing
- [ ] WebSocket support as alternative to TCP

### Long Term
- [ ] Horizontal scaling with Redis pub/sub
- [ ] Admin web dashboard
- [ ] Email integration (verification, notifications)
- [ ] S3/object storage for file uploads

---

## Getting Help

- **Issues**: https://github.com/berrym/lair-chat/issues
- **Discussions**: https://github.com/berrym/lair-chat/discussions
- **Protocol Docs**: See [TCP Protocol](../protocols/TCP.md) and [HTTP API](../protocols/HTTP.md)

---

## Appendix: Quick Reference

### Minimum Production Configuration

```bash
export LAIR_JWT_SECRET="$(openssl rand -base64 32)"
export LAIR_TLS_ENABLED=true
export LAIR_TLS_CERT_PATH=/path/to/cert.pem
export LAIR_TLS_KEY_PATH=/path/to/key.pem
export LAIR_HTTP_PORT=443
export LAIR_DATABASE_URL=sqlite:/var/lib/lair-chat/data.db?mode=rwc

/usr/local/bin/lair-chat-server
```

### Client Connection

```bash
# Connect to production server
lair-chat-client --server chat.yourdomain.com:8080 --http-url https://chat.yourdomain.com
```

### Verify Installation

```bash
# Check server is responding
curl -k https://chat.yourdomain.com/health

# Check TCP port is open
nc -zv chat.yourdomain.com 8080
```
