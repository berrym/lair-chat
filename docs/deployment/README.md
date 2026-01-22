# Deployment Guide

This guide covers deploying Lair Chat in various environments, from development to production-scale deployments.

## 🚀 Quick Deployment Options

### Option 1: Single Server Deployment (Recommended for Small Teams)
```bash
# Clone and start
git clone https://github.com/your-org/lair-chat.git
cd lair-chat
./scripts/start.sh
```

### Option 2: Docker Deployment
```bash
# Using Docker Compose
docker-compose up -d

# Or manual Docker
docker build -t lair-chat .
docker run -p 8080:8080 -p 8082:8082 lair-chat
```

### Option 3: Production-Ready Deployment
See [Production Deployment](#production-deployment) section below.

## 📋 Prerequisites

### System Requirements

**Minimum Requirements:**
- **CPU**: 2 cores
- **RAM**: 2GB
- **Storage**: 10GB
- **OS**: Linux, macOS, or Windows

**Recommended for Production:**
- **CPU**: 4+ cores
- **RAM**: 8GB+
- **Storage**: 100GB+ SSD
- **OS**: Linux (Ubuntu 22.04+ or CentOS 8+)

### Software Dependencies

**Required:**
- **Rust 1.70+** - [Install from rustup.rs](https://rustup.rs/)
- **Git** - For source code
- **OpenSSL/LibSSL** - For TLS support

**Optional but Recommended:**
- **Docker & Docker Compose** - For containerized deployment
- **Nginx** - For reverse proxy and load balancing
- **PostgreSQL** - For production database (SQLite default)
- **Redis** - For session caching and rate limiting

## 🏗️ Architecture Overview

### Single Server Architecture
```
┌─────────────────────────────────────────┐
│                Server                   │
│  ┌─────────────┐  ┌─────────────────┐   │
│  │ TCP Server  │  │  REST API       │   │
│  │ Port 8080   │  │  Port 8082      │   │
│  └─────────────┘  └─────────────────┘   │
│  ┌─────────────────────────────────────┐ │
│  │        SQLite Database              │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### Production Architecture
```
┌─────────────────────────────────────────────────────────┐
│                    Load Balancer                       │
│                     (Nginx)                            │
└─────────────────────┬───────────────────────────────────┘
                      │
    ┌─────────────────┼─────────────────┐
    │                 │                 │
┌───▼────┐        ┌───▼────┐        ┌───▼────┐
│Server 1│        │Server 2│        │Server N│
│        │        │        │        │        │
│TCP+API │        │TCP+API │        │TCP+API │
└────────┘        └────────┘        └────────┘
    │                 │                 │
    └─────────────────┼─────────────────┘
                      │
              ┌───────▼────────┐
              │   PostgreSQL   │
              │   (Primary)    │
              └────────────────┘
```

## 🐳 Docker Deployment

### Quick Start with Docker Compose

1. **Create docker-compose.yml**
```yaml
version: '3.8'

services:
  lair-chat:
    build: .
    ports:
      - "8080:8080"
      - "8082:8082"
    environment:
      - DATABASE_URL=sqlite:data/lair_chat.db
      - SERVER_HOST=0.0.0.0
      - RUST_LOG=info,lair_chat=debug
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - lair-chat
    restart: unless-stopped
```

2. **Start the services**
```bash
docker-compose up -d
```

### Production Docker Setup

**Dockerfile (Multi-stage build)**
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/lair-chat-server-new /app/
COPY --from=builder /app/target/release/lair-chat-server /app/
COPY admin-dashboard ./admin-dashboard
COPY scripts ./scripts

EXPOSE 8080 8082
CMD ["./lair-chat-server-new"]
```

**Build and deploy**
```bash
# Build image
docker build -t lair-chat:latest .

# Run with production settings
docker run -d \
  --name lair-chat \
  -p 8080:8080 \
  -p 8082:8082 \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  -e DATABASE_URL=postgresql://user:pass@db:5432/lair_chat \
  -e SERVER_HOST=0.0.0.0 \
  -e RUST_LOG=info \
  lair-chat:latest
```

## 🏭 Production Deployment

### Database Setup

#### PostgreSQL (Recommended for Production)

1. **Install PostgreSQL**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install postgresql postgresql-contrib

# CentOS/RHEL
sudo dnf install postgresql-server postgresql-contrib
sudo postgresql-setup --initdb
sudo systemctl enable postgresql
sudo systemctl start postgresql
```

2. **Create database and user**
```sql
-- Connect as postgres user
sudo -u postgres psql

-- Create database and user
CREATE DATABASE lair_chat;
CREATE USER lair_chat_user WITH PASSWORD 'secure_password_here';
GRANT ALL PRIVILEGES ON DATABASE lair_chat TO lair_chat_user;

-- Exit psql
\q
```

3. **Configure connection**
```bash
# Set environment variable
export DATABASE_URL="postgresql://lair_chat_user:secure_password_here@localhost:5432/lair_chat"
```

#### SQLite (Development/Small Deployments)
```bash
# SQLite is used by default
export DATABASE_URL="sqlite:data/lair_chat.db"
```

### Reverse Proxy Setup (Nginx)

1. **Install Nginx**
```bash
# Ubuntu/Debian
sudo apt install nginx

# CentOS/RHEL
sudo dnf install nginx
```

2. **Configure Nginx**
```nginx
# /etc/nginx/sites-available/lair-chat
server {
    listen 80;
    server_name your-domain.com;

    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    # SSL Configuration
    ssl_certificate /etc/ssl/certs/your-domain.crt;
    ssl_certificate_key /etc/ssl/private/your-domain.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains";

    # API endpoints
    location /api/ {
        proxy_pass http://127.0.0.1:8082;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Admin dashboard
    location /admin/ {
        proxy_pass http://127.0.0.1:8082;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket support (future)
    location /ws {
        proxy_pass http://127.0.0.1:8082;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Static files (if any)
    location / {
        proxy_pass http://127.0.0.1:8082;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# TCP proxy for direct client connections
stream {
    upstream lair_chat_tcp {
        server 127.0.0.1:8080;
    }

    server {
        listen 8080;
        proxy_pass lair_chat_tcp;
        proxy_timeout 1s;
        proxy_responses 1;
    }
}
```

3. **Enable and start Nginx**
```bash
sudo ln -s /etc/nginx/sites-available/lair-chat /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### SSL/TLS Setup

#### Using Let's Encrypt (Recommended)
```bash
# Install Certbot
sudo apt install certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d your-domain.com

# Auto-renewal (add to crontab)
0 12 * * * /usr/bin/certbot renew --quiet
```

#### Using Custom Certificates
```bash
# Place certificates in /etc/ssl/
sudo cp your-domain.crt /etc/ssl/certs/
sudo cp your-domain.key /etc/ssl/private/
sudo chmod 600 /etc/ssl/private/your-domain.key
```

### Systemd Service Setup

1. **Create service file**
```ini
# /etc/systemd/system/lair-chat.service
[Unit]
Description=Lair Chat Server
After=network.target
Wants=network.target

[Service]
Type=simple
User=lair-chat
Group=lair-chat
WorkingDirectory=/opt/lair-chat
ExecStart=/opt/lair-chat/scripts/start.sh
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Environment
Environment=DATABASE_URL=postgresql://lair_chat_user:password@localhost:5432/lair_chat
Environment=SERVER_HOST=0.0.0.0
Environment=RUST_LOG=info,lair_chat=debug

[Install]
WantedBy=multi-user.target
```

2. **Create user and setup permissions**
```bash
# Create user
sudo useradd --system --shell /bin/false lair-chat

# Setup directories
sudo mkdir -p /opt/lair-chat
sudo chown -R lair-chat:lair-chat /opt/lair-chat

# Copy application files
sudo cp -r * /opt/lair-chat/
sudo chown -R lair-chat:lair-chat /opt/lair-chat
```

3. **Enable and start service**
```bash
sudo systemctl daemon-reload
sudo systemctl enable lair-chat
sudo systemctl start lair-chat
sudo systemctl status lair-chat
```

## 📊 Monitoring and Logging

### Log Management

**Log locations:**
- Application logs: `logs/server.log`
- System logs: `journalctl -u lair-chat`
- Nginx logs: `/var/log/nginx/`

**Log rotation setup:**
```bash
# /etc/logrotate.d/lair-chat
/opt/lair-chat/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    sharedscripts
    postrotate
        systemctl reload lair-chat
    endscript
}
```

### Health Monitoring

**Basic health check script:**
```bash
#!/bin/bash
# /opt/lair-chat/scripts/health-check.sh

API_URL="http://127.0.0.1:8082/api/v1/health"
TCP_PORT="8080"

# Check API health
if curl -f -s "$API_URL" > /dev/null; then
    echo "API: OK"
else
    echo "API: FAILED"
    exit 1
fi

# Check TCP port
if nc -z 127.0.0.1 "$TCP_PORT"; then
    echo "TCP: OK"
else
    echo "TCP: FAILED"
    exit 1
fi

echo "All services healthy"
```

**Add to crontab for monitoring:**
```bash
# Check every 5 minutes
*/5 * * * * /opt/lair-chat/scripts/health-check.sh || /opt/lair-chat/scripts/alert.sh
```

## 🔒 Security Hardening

### Firewall Configuration
```bash
# UFW (Ubuntu)
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 8080/tcp  # TCP chat (if exposing directly)
sudo ufw enable

# iptables (manual)
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

### Environment Security
```bash
# Secure environment file
sudo chmod 600 /opt/lair-chat/.env

# Strong JWT secret
export JWT_SECRET=$(openssl rand -hex 32)

# Database security
# - Use strong passwords
# - Enable SSL connections
# - Restrict network access
```

### Application Security
- **Rate limiting**: Enabled by default
- **Input validation**: Comprehensive validation on all inputs  
- **SQL injection protection**: Using SQLx parameterized queries
- **XSS protection**: Proper output encoding
- **CSRF protection**: Token-based protection

## 📈 Performance Optimization

### Database Optimization

**PostgreSQL tuning:**
```sql
-- postgresql.conf optimizations
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB
max_connections = 200
```

**Indexing:**
```sql
-- Common indexes for performance
CREATE INDEX idx_messages_room_timestamp ON messages(room_id, created_at);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_sessions_user ON sessions(user_id);
```

### Application Tuning

**Environment variables for performance:**
```bash
# Database connection pool
DATABASE_MAX_CONNECTIONS=50
DATABASE_MIN_CONNECTIONS=5

# Server settings
MAX_CONCURRENT_CONNECTIONS=10000
REQUEST_TIMEOUT_SECONDS=30
WORKER_THREADS=8

# Logging
RUST_LOG=info  # Reduce to 'warn' for production
```

### Load Balancing

**Multiple server setup:**
1. Deploy multiple Lair Chat instances
2. Use Nginx upstream configuration
3. Share database across instances
4. Use Redis for session storage

```nginx
upstream lair_chat_backend {
    server 127.0.0.1:8082;
    server 127.0.0.1:8083;
    server 127.0.0.1:8084;
}
```

## 🚀 Scaling Strategies

### Horizontal Scaling

1. **Load balancer** (Nginx/HAProxy)
2. **Multiple application servers**
3. **Shared database** (PostgreSQL cluster)
4. **Redis cluster** for sessions/caching

### Vertical Scaling

1. **Increase server resources** (CPU, RAM)
2. **SSD storage** for database
3. **Optimize database** configuration
4. **Tune application** settings

### Cloud Deployment

#### AWS Deployment
- **EC2**: Application servers
- **RDS**: PostgreSQL database
- **ELB**: Load balancing
- **ElastiCache**: Redis caching
- **CloudWatch**: Monitoring

#### DigitalOcean
- **Droplets**: Application servers
- **Managed Database**: PostgreSQL
- **Load Balancer**: Traffic distribution
- **Spaces**: File storage

#### Self-hosted
- **Bare metal**: Maximum performance
- **VPS**: Cost-effective solution
- **Docker Swarm**: Container orchestration
- **Kubernetes**: Advanced orchestration

## 🔄 Backup and Recovery

### Database Backup

**PostgreSQL backup:**
```bash
# Daily backup script
#!/bin/bash
BACKUP_DIR="/opt/backups/lair-chat"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

pg_dump -h localhost -U lair_chat_user -d lair_chat \
    | gzip > "$BACKUP_DIR/lair_chat_$DATE.sql.gz"

# Keep only last 30 days
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +30 -delete
```

**SQLite backup:**
```bash
# Backup SQLite database
cp data/lair_chat.db "backups/lair_chat_$(date +%Y%m%d_%H%M%S).db"
```

### Application Backup
```bash
# Backup configuration and logs
tar -czf "backup_$(date +%Y%m%d).tar.gz" \
    .env \
    admin-dashboard/ \
    logs/ \
    data/
```

### Recovery Procedures

1. **Stop services**
2. **Restore database** from backup
3. **Restore application** files
4. **Update configuration** if needed
5. **Start services**
6. **Verify functionality**

## 🧪 Testing Deployment

### Automated Testing
```bash
# Run UAT tests against deployment
./scripts/uat-test.sh

# Load testing
./scripts/load-test.sh --clients 1000 --duration 300

# Security testing
./scripts/security-test.sh
```

### Manual Verification
1. **API health check**: `curl https://your-domain.com/api/v1/health`
2. **Admin dashboard**: Visit `https://your-domain.com/admin/`
3. **TUI client**: Connect to TCP port
4. **Load testing**: Simulate multiple users

## 🆘 Troubleshooting

### Common Issues

**Service won't start:**
```bash
# Check logs
sudo journalctl -u lair-chat -f

# Check configuration
sudo -u lair-chat /opt/lair-chat/scripts/start.sh --validate
```

**Database connection failed:**
```bash
# Test database connectivity
psql -h localhost -U lair_chat_user -d lair_chat

# Check environment variables
env | grep DATABASE_URL
```

**High memory usage:**
```bash
# Check process memory
ps aux | grep lair-chat

# Monitor with top/htop
top -p $(pgrep lair-chat)
```

**Performance issues:**
```bash
# Check database performance
EXPLAIN ANALYZE SELECT * FROM messages WHERE room_id = 'room_id';

# Monitor connections
netstat -an | grep :8082 | wc -l
```

### Debug Mode
```bash
# Enable debug logging
export RUST_LOG=debug,lair_chat=trace

# Start with debug
RUST_BACKTRACE=1 ./scripts/start.sh
```

## 📞 Support

- **Documentation**: [docs/](../README.md)
- **Issues**: [GitHub Issues](https://github.com/your-org/lair-chat/issues)
- **Security**: security@lair-chat.org
- **Commercial Support**: Available for enterprise deployments

---

**Ready for production? Your Lair Chat deployment awaits! 🚀**