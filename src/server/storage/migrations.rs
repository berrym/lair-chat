//! Database migrations for lair-chat server
//!
//! This module contains SQL migration scripts for creating and updating
//! the database schema. Migrations are applied automatically on server startup
//! when auto_migrate is enabled in the configuration.

/// Migration 001: Create users table
pub const MIGRATION_001_USERS: &str = r#"
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_seen INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    role TEXT NOT NULL DEFAULT 'user',
    profile TEXT NOT NULL DEFAULT '{}',
    settings TEXT NOT NULL DEFAULT '{}'
);
"#;

/// Migration 002: Create rooms table
pub const MIGRATION_002_ROOMS: &str = r#"
CREATE TABLE rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    topic TEXT,
    room_type TEXT NOT NULL DEFAULT 'channel',
    privacy TEXT NOT NULL DEFAULT 'public',
    settings TEXT NOT NULL DEFAULT '{}',
    created_by TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);
"#;

/// Migration 003: Create messages table
pub const MIGRATION_003_MESSAGES: &str = r#"
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL DEFAULT 'text',
    timestamp INTEGER NOT NULL,
    edited_at INTEGER,
    parent_message_id TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    is_deleted BOOLEAN NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_message_id) REFERENCES messages(id) ON DELETE SET NULL
);
"#;

/// Migration 004: Create sessions table
pub const MIGRATION_004_SESSIONS: &str = r#"
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    last_activity INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
"#;

/// Migration 005: Create room memberships table
pub const MIGRATION_005_ROOM_MEMBERSHIPS: &str = r#"
CREATE TABLE room_memberships (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at INTEGER NOT NULL,
    last_activity INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    settings TEXT NOT NULL DEFAULT '{}',
    UNIQUE(room_id, user_id),
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
"#;

/// Migration 006: Create indexes for performance
pub const MIGRATION_006_INDEXES: &str = r#"
-- User indexes
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_is_active ON users(is_active);
CREATE INDEX idx_users_last_seen ON users(last_seen);
CREATE INDEX idx_users_created_at ON users(created_at);

-- Room indexes
CREATE INDEX idx_rooms_name ON rooms(name);
CREATE INDEX idx_rooms_type ON rooms(room_type);
CREATE INDEX idx_rooms_privacy ON rooms(privacy);
CREATE INDEX idx_rooms_created_by ON rooms(created_by);
CREATE INDEX idx_rooms_created_at ON rooms(created_at);
CREATE INDEX idx_rooms_is_active ON rooms(is_active);

-- Message indexes
CREATE INDEX idx_messages_room_id ON messages(room_id);
CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
CREATE INDEX idx_messages_parent_id ON messages(parent_message_id);
CREATE INDEX idx_messages_is_deleted ON messages(is_deleted);
CREATE INDEX idx_messages_room_timestamp ON messages(room_id, timestamp);
CREATE INDEX idx_messages_user_timestamp ON messages(user_id, timestamp);

-- Session indexes
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity);
CREATE INDEX idx_sessions_is_active ON sessions(is_active);

-- Room membership indexes
CREATE INDEX idx_memberships_room_id ON room_memberships(room_id);
CREATE INDEX idx_memberships_user_id ON room_memberships(user_id);
CREATE INDEX idx_memberships_role ON room_memberships(role);
CREATE INDEX idx_memberships_joined_at ON room_memberships(joined_at);
CREATE INDEX idx_memberships_is_active ON room_memberships(is_active);

-- Full-text search index for messages (SQLite FTS5)
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content='messages',
    content_rowid='rowid'
);

-- Triggers to maintain FTS index
CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

CREATE TRIGGER messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;
"#;

/// Migration 007: Add file attachments table
pub const MIGRATION_007_FILE_ATTACHMENTS: &str = r#"
CREATE TABLE file_attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    original_name TEXT NOT NULL,
    size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    hash TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    uploaded_at INTEGER NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_attachments_message_id ON file_attachments(message_id);
CREATE INDEX idx_attachments_hash ON file_attachments(hash);
CREATE INDEX idx_attachments_uploaded_at ON file_attachments(uploaded_at);
"#;

/// Migration 008: Add message reactions table
pub const MIGRATION_008_MESSAGE_REACTIONS: &str = r#"
CREATE TABLE message_reactions (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    reaction TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    UNIQUE(message_id, user_id, reaction),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_reactions_message_id ON message_reactions(message_id);
CREATE INDEX idx_reactions_user_id ON message_reactions(user_id);
CREATE INDEX idx_reactions_timestamp ON message_reactions(timestamp);
"#;

/// Migration 009: Add message read receipts table
pub const MIGRATION_009_READ_RECEIPTS: &str = r#"
CREATE TABLE message_read_receipts (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    UNIQUE(message_id, user_id),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_read_receipts_message_id ON message_read_receipts(message_id);
CREATE INDEX idx_read_receipts_user_id ON message_read_receipts(user_id);
CREATE INDEX idx_read_receipts_timestamp ON message_read_receipts(timestamp);
"#;

/// Migration 010: Add audit logs table for admin actions
pub const MIGRATION_010_AUDIT_LOG: &str = r#"
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    admin_user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target_id TEXT,
    target_type TEXT NOT NULL,
    description TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    timestamp INTEGER NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (admin_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_audit_logs_admin_user_id ON audit_logs(admin_user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_target_type ON audit_logs(target_type);
CREATE INDEX idx_audit_logs_target_id ON audit_logs(target_id);
CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX idx_audit_logs_admin_timestamp ON audit_logs(admin_user_id, timestamp);
"#;

/// Migration 011: Add user login attempts tracking
pub const MIGRATION_011_LOGIN_ATTEMPTS: &str = r#"
CREATE TABLE login_attempts (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    timestamp INTEGER NOT NULL,
    user_agent TEXT,
    failure_reason TEXT
);

CREATE INDEX idx_login_attempts_username ON login_attempts(username);
CREATE INDEX idx_login_attempts_ip_address ON login_attempts(ip_address);
CREATE INDEX idx_login_attempts_success ON login_attempts(success);
CREATE INDEX idx_login_attempts_timestamp ON login_attempts(timestamp);
CREATE INDEX idx_login_attempts_username_timestamp ON login_attempts(username, timestamp);
CREATE INDEX idx_login_attempts_ip_timestamp ON login_attempts(ip_address, timestamp);
"#;

/// Migration 012: Add server configuration table
pub const MIGRATION_012_SERVER_CONFIG: &str = r#"
CREATE TABLE server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string',
    description TEXT,
    is_sensitive BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_server_config_value_type ON server_config(value_type);
CREATE INDEX idx_server_config_is_sensitive ON server_config(is_sensitive);
CREATE INDEX idx_server_config_updated_at ON server_config(updated_at);
"#;

/// Migration 013: Add room invites table
pub const MIGRATION_013_ROOM_INVITES: &str = r#"
CREATE TABLE room_invites (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    invited_by TEXT NOT NULL,
    invited_user TEXT,
    invite_code TEXT UNIQUE,
    email TEXT,
    expires_at INTEGER,
    max_uses INTEGER DEFAULT 1,
    use_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    FOREIGN KEY (invited_by) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (invited_user) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_room_invites_room_id ON room_invites(room_id);
CREATE INDEX idx_room_invites_invited_by ON room_invites(invited_by);
CREATE INDEX idx_room_invites_invited_user ON room_invites(invited_user);
CREATE INDEX idx_room_invites_invite_code ON room_invites(invite_code);
CREATE INDEX idx_room_invites_email ON room_invites(email);
CREATE INDEX idx_room_invites_expires_at ON room_invites(expires_at);
CREATE INDEX idx_room_invites_is_active ON room_invites(is_active);
"#;

/// Migration 014: Add user ban/mute table
pub const MIGRATION_014_USER_MODERATION: &str = r#"
CREATE TABLE user_moderation (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    room_id TEXT,
    moderator_id TEXT NOT NULL,
    action TEXT NOT NULL, -- 'ban', 'mute', 'kick'
    reason TEXT,
    expires_at INTEGER,
    created_at INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    FOREIGN KEY (moderator_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_moderation_user_id ON user_moderation(user_id);
CREATE INDEX idx_user_moderation_room_id ON user_moderation(room_id);
CREATE INDEX idx_user_moderation_moderator_id ON user_moderation(moderator_id);
CREATE INDEX idx_user_moderation_action ON user_moderation(action);
CREATE INDEX idx_user_moderation_expires_at ON user_moderation(expires_at);
CREATE INDEX idx_user_moderation_is_active ON user_moderation(is_active);
"#;

/// Migration 015: Add typing indicators table
pub const MIGRATION_015_TYPING_INDICATORS: &str = r#"
CREATE TABLE typing_indicators (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    last_update INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    UNIQUE(room_id, user_id),
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_typing_indicators_room_id ON typing_indicators(room_id);
CREATE INDEX idx_typing_indicators_user_id ON typing_indicators(user_id);
CREATE INDEX idx_typing_indicators_expires_at ON typing_indicators(expires_at);
"#;

/// Get all migrations in order
pub fn get_all_migrations() -> Vec<(&'static str, &'static str)> {
    vec![
        ("001_create_users_table", MIGRATION_001_USERS),
        ("002_create_rooms_table", MIGRATION_002_ROOMS),
        ("003_create_messages_table", MIGRATION_003_MESSAGES),
        ("004_create_sessions_table", MIGRATION_004_SESSIONS),
        (
            "005_create_room_memberships_table",
            MIGRATION_005_ROOM_MEMBERSHIPS,
        ),
        ("006_create_indexes", MIGRATION_006_INDEXES),
        (
            "007_create_file_attachments_table",
            MIGRATION_007_FILE_ATTACHMENTS,
        ),
        (
            "008_create_message_reactions_table",
            MIGRATION_008_MESSAGE_REACTIONS,
        ),
        (
            "009_create_read_receipts_table",
            MIGRATION_009_READ_RECEIPTS,
        ),
        ("010_create_audit_logs_table", MIGRATION_010_AUDIT_LOG),
        (
            "011_create_login_attempts_table",
            MIGRATION_011_LOGIN_ATTEMPTS,
        ),
        (
            "012_create_server_config_table",
            MIGRATION_012_SERVER_CONFIG,
        ),
        ("013_create_room_invites_table", MIGRATION_013_ROOM_INVITES),
        (
            "014_create_user_moderation_table",
            MIGRATION_014_USER_MODERATION,
        ),
        (
            "015_create_typing_indicators_table",
            MIGRATION_015_TYPING_INDICATORS,
        ),
    ]
}

/// Check if a migration is reversible
pub fn is_migration_reversible(name: &str) -> bool {
    // Most migrations are not easily reversible in SQLite
    // This would be more sophisticated in a real implementation
    match name {
        "006_create_indexes" => true,
        "007_create_file_attachments_table" => true,
        "008_create_message_reactions_table" => true,
        "009_create_read_receipts_table" => true,
        "010_create_audit_logs_table" => true,
        "011_create_login_attempts_table" => true,
        "012_create_server_config_table" => true,
        "013_create_room_invites_table" => true,
        "014_create_user_moderation_table" => true,
        "015_create_typing_indicators_table" => true,
        _ => false,
    }
}

/// Get rollback SQL for a migration (if supported)
pub fn get_migration_rollback(name: &str) -> Option<&'static str> {
    match name {
        "015_create_typing_indicators_table" => Some("DROP TABLE typing_indicators;"),
        "014_create_user_moderation_table" => Some("DROP TABLE user_moderation;"),
        "013_create_room_invites_table" => Some("DROP TABLE room_invites;"),
        "012_create_server_config_table" => Some("DROP TABLE server_config;"),
        "011_create_login_attempts_table" => Some("DROP TABLE login_attempts;"),
        "010_create_audit_logs_table" => Some("DROP TABLE audit_logs;"),
        "009_create_read_receipts_table" => Some("DROP TABLE message_read_receipts;"),
        "008_create_message_reactions_table" => Some("DROP TABLE message_reactions;"),
        "007_create_file_attachments_table" => Some("DROP TABLE file_attachments;"),
        _ => None,
    }
}
