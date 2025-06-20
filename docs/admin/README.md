# Admin Documentation 🛡️

This section provides comprehensive documentation for system administrators managing Lair Chat deployments.

## 📋 Table of Contents

| Document | Description |
|----------|-------------|
| [**System Administration**](SYSTEM_ADMIN.md) | Complete system setup and management guide |
| [**User Management**](USER_MANAGEMENT.md) | User administration and access control |
| [**Monitoring & Logging**](MONITORING.md) | System monitoring, metrics, and log management |
| [**Security Administration**](SECURITY.md) | Security policies, audit logs, and compliance |
| [**Database Management**](DATABASE.md) | Database administration and maintenance |
| [**Backup & Recovery**](BACKUP_RECOVERY.md) | Data backup strategies and disaster recovery |
| [**Performance Tuning**](PERFORMANCE.md) | System optimization and scaling |
| [**Troubleshooting**](TROUBLESHOOTING.md) | Common issues and diagnostic procedures |

## 🚀 Quick Admin Setup

### Prerequisites
- Root or sudo access to the server
- Rust toolchain (1.70+)
- Database server (PostgreSQL recommended)
- SSL certificate for production

### Initial Setup
```bash
# 1. Install system dependencies
sudo apt update
sudo apt install -y postgresql postgresql-contrib nginx certbot

# 2. Create dedicated user
sudo useradd -r -s /bin/false lair-chat
sudo mkdir -p /opt/lair-chat
sudo chown lair-chat:lair-chat /opt/lair-chat

# 3. Deploy application
sudo -u lair-chat git clone <repository> /opt/lair-chat/app
cd /opt/lair-chat/app
sudo -u lair-chat cargo build --release

# 4. Configure database
sudo -u postgres createdb lair_chat
sudo -u postgres createuser lair_chat_user

# 5. Set up systemd service
sudo cp scripts/lair-chat-server.service /etc/systemd/system/
sudo systemctl enable lair-chat-server
sudo systemctl start lair-chat-server
```

## 🏗️ System Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Load Balancer                        │
│                      (nginx/haproxy)                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────┴───────────────────────────────────────┐
│                   Lair Chat Server                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │    HTTP     │  │    Chat     │  │       Admin         │ │
│  │   Server    │  │   Server    │  │      Dashboard      │ │
│  │  (Port 80)  │  │ (Port 8080) │  │    (Port 9090)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────┴───────────────────────────────────────┐
│                      Storage Layer                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ PostgreSQL  │  │   Redis     │  │    File System      │ │
│  │ (Messages)  │  │  (Cache)    │  │    (Logs/Config)    │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Administrative Interfaces

### 1. Command Line Interface (CLI)
```bash
# Server management
lair-chat-server --admin-mode
lair-chat-server --config /etc/lair-chat/config.toml

# User management
lair-chat-admin user list
lair-chat-admin user create --username admin --role admin
lair-chat-admin user disable --username user123

# System diagnostics
lair-chat-admin system status
lair-chat-admin system metrics
lair-chat-admin logs --tail 100
```

### 2. Web Admin Dashboard
Access at: `https://your-server:9090/admin`

Features:
- Real-time system metrics
- User management interface
- Chat room administration
- Log viewer and search
- Configuration management
- System health monitoring

### 3. REST API
Base URL: `https://your-server:9090/api/v1/admin`

Key endpoints:
- `GET /users` - List all users
- `POST /users` - Create new user
- `GET /metrics` - System metrics
- `GET /logs` - System logs
- `POST /maintenance` - Maintenance mode

## 📊 Key Metrics to Monitor

### System Health
- **CPU Usage**: Keep below 80%
- **Memory Usage**: Monitor for leaks
- **Disk Space**: Alert at 85% full
- **Network I/O**: Track bandwidth usage

### Application Metrics
- **Active Connections**: Current user count
- **Message Throughput**: Messages per second
- **Response Time**: API latency
- **Error Rate**: Failed requests percentage

### Security Metrics
- **Failed Login Attempts**: Monitor for brute force
- **Privilege Escalations**: Admin access attempts
- **Suspicious Activity**: Pattern detection
- **Certificate Expiry**: SSL/TLS monitoring

## 🚨 Critical Alerts

### Immediate Response Required
- Server crash or unresponsive
- Database connection failure
- Security breach detected
- Disk space critical (>95%)
- Memory exhaustion

### Investigation Required
- High error rate (>5%)
- Slow response times (>1s)
- Unusual traffic patterns
- Failed backup jobs
- Certificate expiring (<30 days)

## 🔐 Security Considerations

### Access Control
- Use strong, unique passwords for all admin accounts
- Enable two-factor authentication where possible
- Implement IP whitelisting for admin interfaces
- Regular security audits and penetration testing

### Data Protection
- Encrypt all data at rest and in transit
- Regular security updates and patches
- Secure key management and rotation
- Compliance with data protection regulations (GDPR, etc.)

## 📝 Compliance & Auditing

### Audit Log Categories
- **Authentication Events**: Login/logout, failed attempts
- **Administrative Actions**: User management, configuration changes
- **Data Access**: Message access, export operations
- **System Events**: Startup/shutdown, errors, maintenance

### Retention Policies
- **Audit Logs**: 7 years minimum
- **System Logs**: 1 year
- **Message Data**: Per compliance requirements
- **User Data**: Per privacy policy

## 🆘 Emergency Procedures

### Service Outage
1. Check system status: `systemctl status lair-chat-server`
2. Review recent logs: `journalctl -u lair-chat-server --since "1 hour ago"`
3. Attempt service restart: `systemctl restart lair-chat-server`
4. If persistent, check database connectivity and disk space
5. Escalate to development team if needed

### Security Incident
1. **Immediate**: Isolate affected systems
2. **Document**: Preserve logs and evidence
3. **Assess**: Determine scope and impact
4. **Contain**: Stop ongoing attack
5. **Recover**: Restore services safely
6. **Report**: Notify stakeholders and authorities if required

## 📞 Support Contacts

- **System Administrator**: admin@your-domain.com
- **Security Team**: security@your-domain.com
- **Development Team**: dev@your-domain.com
- **24/7 Emergency**: +1-XXX-XXX-XXXX

---

**Last Updated**: December 2024  
**Document Version**: 1.0  
**Reviewed by**: System Administration Team