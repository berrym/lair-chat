//! Database migrations for SQLite.
//!
//! Migrations are applied in order based on their numeric prefix.
//! Once applied, a migration is never re-run (tracked in `_migrations` table).

/// Get all migrations in order.
///
/// Returns a list of (name, sql) tuples.
pub fn all() -> Vec<(&'static str, &'static str)> {
    vec![
        ("001_initial_schema", MIGRATION_001),
        ("002_add_indexes", MIGRATION_002),
    ]
}

/// Initial schema: users, rooms, memberships, messages, sessions, invitations.
const MIGRATION_001: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_seen_at INTEGER
);

-- Rooms table
CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_private INTEGER NOT NULL DEFAULT 0,
    max_members INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Room memberships
CREATE TABLE IF NOT EXISTS room_memberships (
    room_id TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (room_id, user_id)
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY NOT NULL,
    author_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    content TEXT NOT NULL,
    is_edited INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    protocol TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL
);

-- Invitations table
CREATE TABLE IF NOT EXISTS invitations (
    id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    inviter_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    message TEXT,
    created_at INTEGER NOT NULL,
    responded_at INTEGER,
    expires_at INTEGER NOT NULL
)
"#;

/// Indexes for performance.
const MIGRATION_002: &str = r#"
-- User indexes
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- Room indexes
CREATE INDEX IF NOT EXISTS idx_rooms_name ON rooms(name);
CREATE INDEX IF NOT EXISTS idx_rooms_owner_id ON rooms(owner_id);
CREATE INDEX IF NOT EXISTS idx_rooms_is_private ON rooms(is_private);

-- Membership indexes
CREATE INDEX IF NOT EXISTS idx_memberships_user_id ON room_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_memberships_room_id ON room_memberships(room_id);

-- Message indexes
CREATE INDEX IF NOT EXISTS idx_messages_author_id ON messages(author_id);
CREATE INDEX IF NOT EXISTS idx_messages_target ON messages(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);

-- Session indexes
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Invitation indexes
CREATE INDEX IF NOT EXISTS idx_invitations_room_id ON invitations(room_id);
CREATE INDEX IF NOT EXISTS idx_invitations_invitee_id ON invitations(invitee_id);
CREATE INDEX IF NOT EXISTS idx_invitations_inviter_id ON invitations(inviter_id);
CREATE INDEX IF NOT EXISTS idx_invitations_status ON invitations(status)
"#;
