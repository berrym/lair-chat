# User Management 👥

This document provides comprehensive guidance for managing users in Lair Chat systems.

## 📋 Table of Contents

- [User Lifecycle Management](#user-lifecycle-management)
- [User Roles and Permissions](#user-roles-and-permissions)
- [User Creation and Onboarding](#user-creation-and-onboarding)
- [User Modification and Updates](#user-modification-and-updates)
- [User Deactivation and Removal](#user-deactivation-and-removal)
- [Bulk User Operations](#bulk-user-operations)
- [User Authentication Management](#user-authentication-management)
- [User Activity Monitoring](#user-activity-monitoring)
- [Troubleshooting User Issues](#troubleshooting-user-issues)

## 🔄 User Lifecycle Management

### User States
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Pending   │───▶│   Active    │───▶│  Suspended  │
│             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
       │                  │                  │
       │                  │                  │
       ▼                  ▼                  ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Expired   │◄───│  Disabled   │◄───│   Locked    │
│             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
```

### State Descriptions
- **Pending**: User account created but not yet activated
- **Active**: User can fully access the system
- **Suspended**: Temporary restriction, can be reactivated
- **Locked**: Account locked due to security concerns
- **Disabled**: Account manually disabled by admin
- **Expired**: Account expired due to inactivity or policy

## 🔐 User Roles and Permissions

### Role Hierarchy
```
         ┌─────────────────┐
         │   Super Admin   │ ← Full system access
         └─────────────────┘
                   │
         ┌─────────────────┐
         │      Admin      │ ← User & system management
         └─────────────────┘
                   │
         ┌─────────────────┐
         │   Moderator     │ ← Chat moderation & user support
         └─────────────────┘
                   │
         ┌─────────────────┐
         │      User       │ ← Basic chat functionality
         └─────────────────┘
                   │
         ┌─────────────────┐
         │     Guest       │ ← Limited access
         └─────────────────┘
```

### Permission Matrix

| Permission | Guest | User | Moderator | Admin | Super Admin |
|------------|-------|------|-----------|-------|-------------|
| **Chat Features** |
| Send messages | ❌ | ✅ | ✅ | ✅ | ✅ |
| Join public rooms | ❌ | ✅ | ✅ | ✅ | ✅ |
| Create private rooms | ❌ | ✅ | ✅ | ✅ | ✅ |
| Direct messaging | ❌ | ✅ | ✅ | ✅ | ✅ |
| File sharing | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Moderation** |
| Delete own messages | ❌ | ✅ | ✅ | ✅ | ✅ |
| Delete others' messages | ❌ | ❌ | ✅ | ✅ | ✅ |
| Kick users from rooms | ❌ | ❌ | ✅ | ✅ | ✅ |
| Ban users temporarily | ❌ | ❌ | ✅ | ✅ | ✅ |
| Ban users permanently | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Administration** |
| View user list | ❌ | ❌ | ✅ | ✅ | ✅ |
| Create users | ❌ | ❌ | ❌ | ✅ | ✅ |
| Modify user profiles | ❌ | ❌ | ❌ | ✅ | ✅ |
| Delete users | ❌ | ❌ | ❌ | ✅ | ✅ |
| View audit logs | ❌ | ❌ | ❌ | ✅ | ✅ |
| System configuration | ❌ | ❌ | ❌ | ❌ | ✅ |
| Manage admins | ❌ | ❌ | ❌ | ❌ | ✅ |

## 👤 User Creation and Onboarding

### Manual User Creation

#### CLI Method
```bash
# Create a new user
lair-chat-admin user create \
  --username "john.doe" \
  --email "john.doe@company.com" \
  --role "user" \
  --full-name "John Doe" \
  --department "Engineering"

# Create admin user
lair-chat-admin user create \
  --username "admin" \
  --email "admin@company.com" \
  --role "admin" \
  --full-name "System Administrator" \
  --password-reset-required
```

#### Web Interface
1. Navigate to Admin Dashboard → Users → Create User
2. Fill in required fields:
   - Username (unique identifier)
   - Email address
   - Full name
   - Role assignment
   - Initial password (optional)
3. Set user preferences:
   - Force password reset on first login
   - Account expiration date
   - Department/group assignment
4. Send welcome email with login instructions

### Bulk User Import

#### CSV Format
```csv
username,email,full_name,role,department,manager_email
john.doe,john@company.com,John Doe,user,Engineering,manager@company.com
jane.smith,jane@company.com,Jane Smith,moderator,Support,director@company.com
```

#### Import Process
```bash
# Validate CSV format
lair-chat-admin user validate-import users.csv

# Perform bulk import
lair-chat-admin user bulk-import users.csv --send-welcome-emails

# Check import status
lair-chat-admin user import-status --job-id 12345
```

### Self-Registration (if enabled)
```yaml
# config.toml
[user_registration]
enabled = true
require_email_verification = true
require_admin_approval = false
default_role = "user"
allowed_domains = ["company.com", "subsidiary.com"]
```

## ✏️ User Modification and Updates

### Profile Updates
```bash
# Update user information
lair-chat-admin user update john.doe \
  --email "john.doe.new@company.com" \
  --full-name "John D. Doe" \
  --department "Senior Engineering"

# Change user role
lair-chat-admin user update john.doe --role "moderator"

# Update user status
lair-chat-admin user update john.doe --status "suspended"
```

### Password Management
```bash
# Force password reset
lair-chat-admin user password-reset john.doe

# Set temporary password
lair-chat-admin user set-password john.doe --temporary

# Unlock account after failed attempts
lair-chat-admin user unlock john.doe
```

### Profile Picture Management
```bash
# Upload profile picture
lair-chat-admin user avatar john.doe --upload profile.jpg

# Remove profile picture
lair-chat-admin user avatar john.doe --remove

# Set default avatar
lair-chat-admin user avatar john.doe --default
```

## 🚫 User Deactivation and Removal

### Suspension Process
```bash
# Temporary suspension
lair-chat-admin user suspend john.doe \
  --duration "7 days" \
  --reason "Policy violation" \
  --notify-user

# Permanent suspension
lair-chat-admin user suspend john.doe \
  --permanent \
  --reason "Severe policy violation"
```

### Account Disabling
```bash
# Disable account (reversible)
lair-chat-admin user disable john.doe \
  --reason "Employee departure" \
  --preserve-data

# Re-enable account
lair-chat-admin user enable john.doe
```

### Account Deletion
```bash
# Soft delete (data preserved)
lair-chat-admin user delete john.doe \
  --soft \
  --backup-data

# Hard delete (permanent removal)
lair-chat-admin user delete john.doe \
  --hard \
  --confirm \
  --reason "GDPR request"
```

### Data Export Before Deletion
```bash
# Export user data
lair-chat-admin user export john.doe \
  --format json \
  --include-messages \
  --include-files \
  --output user_data_export.zip
```

## 📊 Bulk User Operations

### Mass Updates
```bash
# Update all users in department
lair-chat-admin user bulk-update \
  --filter "department=Marketing" \
  --set "manager_email=new.manager@company.com"

# Change role for multiple users
lair-chat-admin user bulk-update \
  --usernames "user1,user2,user3" \
  --set "role=moderator"
```

### Mass Communications
```bash
# Send announcement to all users
lair-chat-admin user broadcast \
  --message "System maintenance scheduled for tonight" \
  --priority high

# Send message to specific group
lair-chat-admin user broadcast \
  --filter "role=user" \
  --message "New features available in latest update"
```

## 🔐 User Authentication Management

### Multi-Factor Authentication (MFA)
```bash
# Enable MFA for user
lair-chat-admin user mfa enable john.doe

# Generate backup codes
lair-chat-admin user mfa backup-codes john.doe

# Reset MFA device
lair-chat-admin user mfa reset john.doe
```

### Session Management
```bash
# View active sessions
lair-chat-admin user sessions john.doe

# Terminate specific session
lair-chat-admin user session-kill john.doe --session-id abc123

# Force logout from all devices
lair-chat-admin user logout-all john.doe
```

### API Key Management
```bash
# Generate API key for user
lair-chat-admin user api-key create john.doe \
  --name "Mobile App" \
  --permissions "read,write" \
  --expires "2025-01-01"

# Revoke API key
lair-chat-admin user api-key revoke john.doe --key-id 12345
```

## 📈 User Activity Monitoring

### Activity Dashboard
```bash
# View user activity summary
lair-chat-admin user activity john.doe --days 30

# Generate activity report
lair-chat-admin user activity-report \
  --all-users \
  --format csv \
  --output monthly_activity.csv
```

### Login Analytics
```bash
# View login patterns
lair-chat-admin user login-stats john.doe

# Check for suspicious logins
lair-chat-admin user security-check john.doe \
  --check-location \
  --check-device \
  --check-timing
```

### Message Statistics
```bash
# View messaging statistics
lair-chat-admin user message-stats john.doe --period month

# Check for spam or abuse
lair-chat-admin user abuse-check john.doe \
  --check-rate-limit \
  --check-content \
  --check-reports
```

## 🔍 Troubleshooting User Issues

### Common Issues and Solutions

#### "User Cannot Login"
1. **Check account status**:
   ```bash
   lair-chat-admin user info john.doe
   ```

2. **Verify credentials**:
   ```bash
   lair-chat-admin user verify-password john.doe
   ```

3. **Check for account locks**:
   ```bash
   lair-chat-admin user security-status john.doe
   ```

4. **Reset password if needed**:
   ```bash
   lair-chat-admin user password-reset john.doe
   ```

#### "Messages Not Sending"
1. **Check user permissions**:
   ```bash
   lair-chat-admin user permissions john.doe
   ```

2. **Verify room access**:
   ```bash
   lair-chat-admin room check-access room-id john.doe
   ```

3. **Check rate limiting**:
   ```bash
   lair-chat-admin user rate-limit-status john.doe
   ```

#### "Profile Picture Not Updating"
1. **Check file permissions**:
   ```bash
   lair-chat-admin system check-storage
   ```

2. **Verify file size limits**:
   ```bash
   lair-chat-admin user upload-limits john.doe
   ```

3. **Clear avatar cache**:
   ```bash
   lair-chat-admin user avatar john.doe --clear-cache
   ```

### Diagnostic Commands
```bash
# Comprehensive user diagnostic
lair-chat-admin user diagnose john.doe

# Check user connectivity
lair-chat-admin user connection-test john.doe

# Verify user data integrity
lair-chat-admin user data-integrity john.doe
```

## 📋 User Management Checklists

### New User Onboarding
- [ ] Create user account with appropriate role
- [ ] Set up initial password or send reset link
- [ ] Add to relevant departments/groups
- [ ] Configure permissions and access levels
- [ ] Send welcome email with getting started guide
- [ ] Schedule follow-up check after first week

### User Departure
- [ ] Disable account immediately
- [ ] Transfer ownership of created rooms
- [ ] Export user data if required
- [ ] Remove from all groups and permissions
- [ ] Archive or delete account per policy
- [ ] Update emergency contact lists

### Security Incident Response
- [ ] Immediately suspend affected accounts
- [ ] Document all actions taken
- [ ] Preserve audit logs and evidence
- [ ] Reset passwords for compromised accounts
- [ ] Review and update security policies
- [ ] Conduct post-incident review

## 📊 User Management Metrics

### Key Performance Indicators
- **User Growth Rate**: New users per month
- **User Retention**: Active users over time
- **Support Ticket Volume**: User-related issues
- **Login Success Rate**: Authentication success percentage
- **Average Session Duration**: User engagement metric
- **Password Reset Frequency**: Security and usability indicator

### Monthly Reporting
```bash
# Generate monthly user report
lair-chat-admin report user-metrics \
  --month $(date +%Y-%m) \
  --format pdf \
  --output monthly_user_report.pdf
```

---

**Last Updated**: December 2024  
**Document Version**: 1.0  
**Reviewed by**: System Administration Team