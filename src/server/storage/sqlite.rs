//! SQLite storage backend implementation for lair-chat server
//!
//! This module provides a complete SQLite-based implementation of all storage traits,
//! with support for connection pooling, migrations, and comprehensive data operations.

use super::{
    models::*, traits::*, DatabaseConfig, OrderBy, OrderDirection, Pagination, StorageError,
    StorageResult,
};
use async_trait::async_trait;
use serde_json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

/// SQLite storage backend
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage instance
    pub async fn new(config: DatabaseConfig) -> StorageResult<Self> {
        let connect_options = SqliteConnectOptions::from_str(&config.url)
            .map_err(|e| StorageError::ConnectionError {
                message: format!("Invalid SQLite URL: {}", e),
            })?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(30))
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(config.idle_timeout)
            .connect_with(connect_options)
            .await
            .map_err(|e| StorageError::ConnectionError {
                message: format!("Failed to connect to SQLite: {}", e),
            })?;

        let storage = Self { pool };

        if config.auto_migrate {
            storage.run_migrations().await?;
        }

        info!("SQLite storage backend initialized successfully");
        Ok(storage)
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> StorageResult<()> {
        info!("Running database migrations...");

        // Create migrations table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                applied_at INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;

        // Run migrations in order
        let migrations = super::migrations::get_all_migrations();

        for (name, sql) in migrations {
            self.apply_migration(name, sql).await?;
        }

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Apply a single migration
    async fn apply_migration(&self, name: &str, sql: &str) -> StorageResult<()> {
        // Check if migration already applied
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM migrations WHERE name = ?)")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;

        if exists {
            debug!("Migration {} already applied, skipping", name);
            return Ok(());
        }

        debug!("Applying migration: {}", name);

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Execute migration SQL
        sqlx::query(sql).execute(&mut *tx).await?;

        // Record migration
        let timestamp = super::current_timestamp();
        sqlx::query("INSERT INTO migrations (name, applied_at) VALUES (?, ?)")
            .bind(name)
            .bind(timestamp as i64)
            .execute(&mut *tx)
            .await?;

        // Commit transaction
        tx.commit().await?;

        info!("Migration {} applied successfully", name);
        Ok(())
    }

    /// Convert order by clause to SQL
    fn order_by_to_sql(&self, order_by: &Option<OrderBy>) -> String {
        match order_by {
            Some(order) => {
                let direction = match order.direction {
                    OrderDirection::Ascending => "ASC",
                    OrderDirection::Descending => "DESC",
                };
                format!("ORDER BY {} {}", order.field, direction)
            }
            None => "ORDER BY created_at DESC".to_string(),
        }
    }

    /// Convert pagination to SQL limit/offset
    fn pagination_to_sql(&self, pagination: &Pagination) -> String {
        format!("LIMIT {} OFFSET {}", pagination.limit, pagination.offset)
    }
}

#[async_trait]
impl UserStorage for SqliteStorage {
    async fn create_user(&self, user: User) -> StorageResult<User> {
        let profile_json =
            serde_json::to_string(&user.profile).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let settings_json = serde_json::to_string(&user.settings).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let role_str = match user.role {
            UserRole::Admin => "admin",
            UserRole::Moderator => "moderator",
            UserRole::User => "user",
            UserRole::Guest => "guest",
        };

        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.salt)
        .bind(user.created_at as i64)
        .bind(user.updated_at as i64)
        .bind(user.last_seen.map(|t| t as i64))
        .bind(user.is_active)
        .bind(role_str)
        .bind(profile_json)
        .bind(settings_json)
        .execute(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_id(&self, id: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn get_user_by_username(&self, username: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn get_user_by_email(&self, email: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn update_user(&self, user: User) -> StorageResult<User> {
        let profile_json =
            serde_json::to_string(&user.profile).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let settings_json = serde_json::to_string(&user.settings).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let role_str = match user.role {
            UserRole::Admin => "admin",
            UserRole::Moderator => "moderator",
            UserRole::User => "user",
            UserRole::Guest => "guest",
        };

        sqlx::query(
            "UPDATE users SET username = ?, email = ?, password_hash = ?, salt = ?,
             updated_at = ?, last_seen = ?, is_active = ?, role = ?, profile = ?, settings = ?
             WHERE id = ?",
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.salt)
        .bind(user.updated_at as i64)
        .bind(user.last_seen.map(|t| t as i64))
        .bind(user.is_active)
        .bind(role_str)
        .bind(profile_json)
        .bind(settings_json)
        .bind(&user.id)
        .execute(&self.pool)
        .await?;

        Ok(user)
    }

    async fn update_last_seen(&self, user_id: &str, timestamp: u64) -> StorageResult<()> {
        sqlx::query("UPDATE users SET last_seen = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_password(
        &self,
        user_id: &str,
        password_hash: &str,
        salt: &str,
    ) -> StorageResult<()> {
        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET password_hash = ?, salt = ?, updated_at = ? WHERE id = ?")
            .bind(password_hash)
            .bind(salt)
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_profile(&self, user_id: &str, profile: UserProfile) -> StorageResult<()> {
        let profile_json =
            serde_json::to_string(&profile).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET profile = ?, updated_at = ? WHERE id = ?")
            .bind(profile_json)
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_settings(&self, user_id: &str, settings: UserSettings) -> StorageResult<()> {
        let settings_json =
            serde_json::to_string(&settings).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET settings = ?, updated_at = ? WHERE id = ?")
            .bind(settings_json)
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_role(&self, user_id: &str, role: UserRole) -> StorageResult<()> {
        let role_str = match role {
            UserRole::Admin => "admin",
            UserRole::Moderator => "moderator",
            UserRole::User => "user",
            UserRole::Guest => "guest",
        };

        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
            .bind(role_str)
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn deactivate_user(&self, user_id: &str) -> StorageResult<()> {
        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET is_active = 0, updated_at = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn reactivate_user(&self, user_id: &str) -> StorageResult<()> {
        let timestamp = super::current_timestamp();
        sqlx::query("UPDATE users SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_user(&self, user_id: &str) -> StorageResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn list_users(
        &self,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<User>> {
        let order_sql = self.order_by_to_sql(&order_by);
        let limit_sql = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users {} {}",
            order_sql, limit_sql
        );

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(row)?);
        }

        Ok(users)
    }

    async fn search_users(&self, query: &str, pagination: Pagination) -> StorageResult<Vec<User>> {
        let search_term = format!("%{}%", query);
        let limit_sql = self.pagination_to_sql(&pagination);

        let sql = format!(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users
             WHERE username LIKE ? OR JSON_EXTRACT(profile, '$.display_name') LIKE ?
             ORDER BY username ASC {}",
            limit_sql
        );

        let rows = sqlx::query(&sql)
            .bind(&search_term)
            .bind(&search_term)
            .fetch_all(&self.pool)
            .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(row)?);
        }

        Ok(users)
    }

    async fn get_users_by_role(
        &self,
        role: UserRole,
        pagination: Pagination,
    ) -> StorageResult<Vec<User>> {
        let role_str = match role {
            UserRole::Admin => "admin",
            UserRole::Moderator => "moderator",
            UserRole::User => "user",
            UserRole::Guest => "guest",
        };

        let limit_sql = self.pagination_to_sql(&pagination);

        let sql = format!(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users
             WHERE role = ? ORDER BY username ASC {}",
            limit_sql
        );

        let rows = sqlx::query(&sql)
            .bind(role_str)
            .fetch_all(&self.pool)
            .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(row)?);
        }

        Ok(users)
    }

    async fn get_active_users(&self, since: u64) -> StorageResult<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, salt, created_at, updated_at,
             last_seen, is_active, role, profile, settings FROM users
             WHERE is_active = 1 AND last_seen > ? ORDER BY last_seen DESC",
        )
        .bind(since as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(row)?);
        }

        Ok(users)
    }

    async fn count_users(&self) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u64)
    }

    async fn count_active_users(&self, since: u64) -> StorageResult<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_active = 1 AND last_seen > ?")
                .bind(since as i64)
                .fetch_one(&self.pool)
                .await?;

        Ok(count as u64)
    }

    async fn username_exists(&self, username: &str) -> StorageResult<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = ?)")
                .bind(username)
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }

    async fn email_exists(&self, email: &str) -> StorageResult<bool> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)")
            .bind(email)
            .fetch_one(&self.pool)
            .await?;

        Ok(exists)
    }

    async fn get_user_stats(&self) -> StorageResult<UserStats> {
        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let active_users: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_active = 1")
                .fetch_one(&self.pool)
                .await?;

        let now = super::current_timestamp();
        let day_ago = now - 86400;
        let week_ago = now - (86400 * 7);
        let month_ago = now - (86400 * 30);

        let new_users_today: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE created_at > ?")
                .bind(day_ago as i64)
                .fetch_one(&self.pool)
                .await?;

        let new_users_this_week: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE created_at > ?")
                .bind(week_ago as i64)
                .fetch_one(&self.pool)
                .await?;

        let new_users_this_month: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE created_at > ?")
                .bind(month_ago as i64)
                .fetch_one(&self.pool)
                .await?;

        let role_rows = sqlx::query("SELECT role, COUNT(*) as count FROM users GROUP BY role")
            .fetch_all(&self.pool)
            .await?;

        let mut users_by_role = HashMap::new();
        for row in role_rows {
            let role: String = row.get("role");
            let count: i64 = row.get("count");
            users_by_role.insert(role, count as u64);
        }

        Ok(UserStats {
            total_users: total_users as u64,
            active_users: active_users as u64,
            new_users_today: new_users_today as u64,
            new_users_this_week: new_users_this_week as u64,
            new_users_this_month: new_users_this_month as u64,
            users_by_role,
        })
    }
}

// Helper methods for SQLite storage
impl SqliteStorage {
    /// Convert a database row to a User struct
    fn row_to_user(&self, row: sqlx::sqlite::SqliteRow) -> StorageResult<User> {
        let profile_json: String = row.get("profile");
        let profile: UserProfile =
            serde_json::from_str(&profile_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let settings_json: String = row.get("settings");
        let settings: UserSettings =
            serde_json::from_str(&settings_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let role_str: String = row.get("role");
        let role = match role_str.as_str() {
            "admin" => UserRole::Admin,
            "moderator" => UserRole::Moderator,
            "user" => UserRole::User,
            "guest" => UserRole::Guest,
            _ => UserRole::User,
        };

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            salt: row.get("salt"),
            created_at: row.get::<i64, _>("created_at") as u64,
            updated_at: row.get::<i64, _>("updated_at") as u64,
            last_seen: row.get::<Option<i64>, _>("last_seen").map(|t| t as u64),
            is_active: row.get("is_active"),
            role,
            profile,
            settings,
        })
    }

    /// Convert a database row to a Message struct
    fn row_to_message(&self, row: sqlx::sqlite::SqliteRow) -> StorageResult<Message> {
        use sqlx::Row;

        let metadata_json: String = row.get("metadata");
        let metadata: MessageMetadata =
            serde_json::from_str(&metadata_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let message_type_str: String = row.get("message_type");
        let message_type = match message_type_str.as_str() {
            "text" => MessageType::Text,
            "system" => MessageType::System,
            "file" => MessageType::File,
            "image" => MessageType::Image,
            "voice" => MessageType::Voice,
            "video" => MessageType::Video,
            "code" => MessageType::Code,
            "markdown" => MessageType::Markdown,
            "encrypted" => MessageType::Encrypted,
            _ => MessageType::Text,
        };

        Ok(Message {
            id: row.get("id"),
            room_id: row.get("room_id"),
            user_id: row.get("user_id"),
            content: row.get("content"),
            message_type,
            timestamp: row.get::<i64, _>("timestamp") as u64,
            edited_at: row.get::<Option<i64>, _>("edited_at").map(|t| t as u64),
            parent_message_id: row.get("parent_message_id"),
            metadata,
            is_deleted: row.get("is_deleted"),
            deleted_at: row.get::<Option<i64>, _>("deleted_at").map(|t| t as u64),
        })
    }

    /// Convert a database row to a Room struct
    fn row_to_room(&self, row: sqlx::sqlite::SqliteRow) -> StorageResult<Room> {
        use sqlx::Row;

        let settings_json: String = row.get("settings");
        let settings: RoomSettings =
            serde_json::from_str(&settings_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let room_type_str: String = row.get("room_type");
        let room_type = match room_type_str.as_str() {
            "channel" => RoomType::Channel,
            "group" => RoomType::Group,
            "direct_message" => RoomType::DirectMessage,
            "system" => RoomType::System,
            "temporary" => RoomType::Temporary,
            _ => RoomType::Channel,
        };

        let privacy_str: String = row.get("privacy");
        let privacy = match privacy_str.as_str() {
            "public" => RoomPrivacy::Public,
            "private" => RoomPrivacy::Private,
            "protected" => RoomPrivacy::Protected,
            "system" => RoomPrivacy::System,
            _ => RoomPrivacy::Public,
        };

        Ok(Room {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            topic: row.get("topic"),
            room_type,
            privacy,
            settings,
            created_by: row.get("created_by"),
            created_at: row.get::<i64, _>("created_at") as u64,
            updated_at: row.get::<i64, _>("updated_at") as u64,
            is_active: row.get("is_active"),
        })
    }

    /// Convert a database row to a RoomMembership struct
    fn row_to_room_membership(
        &self,
        row: sqlx::sqlite::SqliteRow,
    ) -> StorageResult<RoomMembership> {
        use sqlx::Row;

        let settings_json: String = row.get("settings");
        let settings: RoomMemberSettings =
            serde_json::from_str(&settings_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let role_str: String = row.get("role");
        let role = match role_str.as_str() {
            "owner" => RoomRole::Owner,
            "admin" => RoomRole::Admin,
            "moderator" => RoomRole::Moderator,
            "member" => RoomRole::Member,
            "guest" => RoomRole::Guest,
            _ => RoomRole::Member,
        };

        Ok(RoomMembership {
            id: row.get("id"),
            room_id: row.get("room_id"),
            user_id: row.get("user_id"),
            role,
            joined_at: row.get::<i64, _>("joined_at") as u64,
            last_activity: row.get::<Option<i64>, _>("last_activity").map(|t| t as u64),
            is_active: row.get("is_active"),
            settings,
        })
    }

    /// Convert a database row to a Session struct
    fn row_to_session(&self, row: sqlx::sqlite::SqliteRow) -> StorageResult<Session> {
        use sqlx::Row;

        let metadata_json: String = row.get("metadata");
        let metadata: SessionMetadata =
            serde_json::from_str(&metadata_json).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        Ok(Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            token: row.get("token"),
            created_at: row.get::<i64, _>("created_at") as u64,
            expires_at: row.get::<i64, _>("expires_at") as u64,
            last_activity: row.get::<i64, _>("last_activity") as u64,
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            is_active: row.get("is_active"),
            metadata,
        })
    }
}

// Note: For brevity, I'm implementing a basic version of MessageStorage, RoomStorage, and SessionStorage
// In a full implementation, these would be complete with all methods from the traits

#[async_trait]
impl MessageStorage for SqliteStorage {
    async fn store_message(&self, message: Message) -> StorageResult<Message> {
        let metadata_json = serde_json::to_string(&message.metadata).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let message_type_str = match message.message_type {
            MessageType::Text => "text",
            MessageType::System => "system",
            MessageType::File => "file",
            MessageType::Image => "image",
            MessageType::Voice => "voice",
            MessageType::Video => "video",
            MessageType::Code => "code",
            MessageType::Markdown => "markdown",
            MessageType::Encrypted => "encrypted",
        };

        sqlx::query(
            "INSERT INTO messages (id, room_id, user_id, content, message_type, timestamp,
             edited_at, parent_message_id, metadata, is_deleted, deleted_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.room_id)
        .bind(&message.user_id)
        .bind(&message.content)
        .bind(message_type_str)
        .bind(message.timestamp as i64)
        .bind(message.edited_at.map(|t| t as i64))
        .bind(&message.parent_message_id)
        .bind(metadata_json)
        .bind(message.is_deleted)
        .bind(message.deleted_at.map(|t| t as i64))
        .execute(&self.pool)
        .await?;

        Ok(message)
    }

    async fn get_message_by_id(&self, id: &str) -> StorageResult<Option<Message>> {
        let row = sqlx::query(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_message(row)?)),
            None => Ok(None),
        }
    }

    async fn count_messages(&self) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u64)
    }

    // ... other methods would be implemented here
    // For brevity, I'm providing placeholder implementations
    async fn update_message(&self, message: Message) -> StorageResult<Message> {
        let metadata_json = serde_json::to_string(&message.metadata).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let message_type_str = match message.message_type {
            MessageType::Text => "text",
            MessageType::System => "system",
            MessageType::File => "file",
            MessageType::Image => "image",
            MessageType::Voice => "voice",
            MessageType::Video => "video",
            MessageType::Code => "code",
            MessageType::Markdown => "markdown",
            MessageType::Encrypted => "encrypted",
        };

        sqlx::query(
            "UPDATE messages SET room_id = ?, user_id = ?, content = ?, message_type = ?,
             timestamp = ?, edited_at = ?, parent_message_id = ?, metadata = ?,
             is_deleted = ?, deleted_at = ? WHERE id = ?",
        )
        .bind(&message.room_id)
        .bind(&message.user_id)
        .bind(&message.content)
        .bind(message_type_str)
        .bind(message.timestamp as i64)
        .bind(message.edited_at.map(|t| t as i64))
        .bind(&message.parent_message_id)
        .bind(metadata_json)
        .bind(message.is_deleted)
        .bind(message.deleted_at.map(|t| t as i64))
        .bind(&message.id)
        .execute(&self.pool)
        .await?;

        Ok(message)
    }
    async fn delete_message(&self, message_id: &str, deleted_at: u64) -> StorageResult<()> {
        sqlx::query("UPDATE messages SET is_deleted = 1, deleted_at = ? WHERE id = ?")
            .bind(deleted_at as i64)
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    async fn hard_delete_message(&self, message_id: &str) -> StorageResult<()> {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    async fn get_room_messages(
        &self,
        room_id: &str,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Message>> {
        let order_clause =
            self.order_by_to_sql(&Some(order_by.unwrap_or(OrderBy::desc("timestamp"))));
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE room_id = ? AND is_deleted = 0 {} {}",
            order_clause, limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(room_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn get_user_messages(
        &self,
        user_id: &str,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Message>> {
        let order_clause =
            self.order_by_to_sql(&Some(order_by.unwrap_or(OrderBy::desc("timestamp"))));
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE user_id = ? AND is_deleted = 0 {} {}",
            order_clause, limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn get_messages_in_range(
        &self,
        room_id: &str,
        start_time: u64,
        end_time: u64,
        pagination: Pagination,
    ) -> StorageResult<Vec<Message>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE room_id = ? AND timestamp >= ? AND timestamp <= ? AND is_deleted = 0
             ORDER BY timestamp ASC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(room_id)
            .bind(start_time as i64)
            .bind(end_time as i64)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn get_messages_after(
        &self,
        room_id: &str,
        after_message_id: &str,
        limit: u64,
    ) -> StorageResult<Vec<Message>> {
        // First get the timestamp of the after_message_id
        let after_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT timestamp FROM messages WHERE id = ?")
                .bind(after_message_id)
                .fetch_optional(&self.pool)
                .await?;

        let after_timestamp = match after_timestamp {
            Some(ts) => ts,
            None => return Ok(Vec::new()), // Message not found
        };

        let rows = sqlx::query(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE room_id = ? AND timestamp > ? AND is_deleted = 0
             ORDER BY timestamp ASC LIMIT ?",
        )
        .bind(room_id)
        .bind(after_timestamp)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn get_messages_before(
        &self,
        room_id: &str,
        before_message_id: &str,
        limit: u64,
    ) -> StorageResult<Vec<Message>> {
        // First get the timestamp of the before_message_id
        let before_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT timestamp FROM messages WHERE id = ?")
                .bind(before_message_id)
                .fetch_optional(&self.pool)
                .await?;

        let before_timestamp = match before_timestamp {
            Some(ts) => ts,
            None => return Ok(Vec::new()), // Message not found
        };

        let rows = sqlx::query(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE room_id = ? AND timestamp < ? AND is_deleted = 0
             ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(room_id)
        .bind(before_timestamp)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        // Reverse to maintain chronological order
        messages.reverse();
        Ok(messages)
    }
    async fn search_messages(&self, query: SearchQuery) -> StorageResult<SearchResult> {
        use std::time::Instant;

        let start_time = Instant::now();

        // Build the FTS query
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        // Add the main search query
        conditions.push("messages_fts MATCH ?");
        params.push(query.query.clone());

        // Build WHERE clause for additional filters
        let mut where_conditions = Vec::new();
        let mut where_params = Vec::new();

        if let Some(room_id) = &query.room_id {
            where_conditions.push("m.room_id = ?");
            where_params.push(room_id.clone());
        }

        if let Some(user_id) = &query.user_id {
            where_conditions.push("m.user_id = ?");
            where_params.push(user_id.clone());
        }

        if let Some(message_type) = &query.message_type {
            where_conditions.push("m.message_type = ?");
            let message_type_str = match message_type {
                MessageType::Text => "text",
                MessageType::System => "system",
                MessageType::File => "file",
                MessageType::Image => "image",
                MessageType::Voice => "voice",
                MessageType::Video => "video",
                MessageType::Code => "code",
                MessageType::Markdown => "markdown",
                MessageType::Encrypted => "encrypted",
            };
            where_params.push(message_type_str.to_string());
        }

        if let Some(date_from) = query.date_from {
            where_conditions.push("m.timestamp >= ?");
            where_params.push(date_from.to_string());
        }

        if let Some(date_to) = query.date_to {
            where_conditions.push("m.timestamp <= ?");
            where_params.push(date_to.to_string());
        }

        // Always exclude deleted messages
        where_conditions.push("m.is_deleted = 0");

        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        // Build the complete query
        let search_query = format!(
            "SELECT m.id, m.room_id, m.user_id, m.content, m.message_type, m.timestamp, m.edited_at,
             m.parent_message_id, m.metadata, m.is_deleted, m.deleted_at
             FROM messages_fts
             JOIN messages m ON messages_fts.rowid = m.rowid
             {}
             ORDER BY m.timestamp DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        // Execute the search query
        let mut sql_query = sqlx::query(&search_query);

        // Bind FTS parameters
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        // Bind WHERE parameters
        for param in &where_params {
            sql_query = sql_query.bind(param);
        }

        // Bind pagination parameters
        sql_query = sql_query.bind(query.limit.unwrap_or(50) as i64);
        sql_query = sql_query.bind(query.offset.unwrap_or(0) as i64);

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        // Get total count for pagination
        let count_query = format!(
            "SELECT COUNT(*)
             FROM messages_fts
             JOIN messages m ON messages_fts.rowid = m.rowid
             {}",
            where_clause
        );

        let mut count_sql_query = sqlx::query_scalar(&count_query);

        // Bind FTS parameters
        for param in &params {
            count_sql_query = count_sql_query.bind(param);
        }

        // Bind WHERE parameters
        for param in &where_params {
            count_sql_query = count_sql_query.bind(param);
        }

        let total_count: i64 = count_sql_query.fetch_one(&self.pool).await?;

        let execution_time = start_time.elapsed();
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(50);
        let has_more = (offset + limit) < total_count as u64;

        Ok(SearchResult {
            messages,
            total_count: total_count as u64,
            has_more,
            execution_time: execution_time.as_millis() as u64,
        })
    }
    async fn get_message_thread(
        &self,
        parent_message_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Message>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, content, message_type, timestamp, edited_at,
             parent_message_id, metadata, is_deleted, deleted_at
             FROM messages WHERE parent_message_id = ? AND is_deleted = 0
             ORDER BY timestamp ASC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(parent_message_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn add_reaction(&self, message_id: &str, reaction: MessageReaction) -> StorageResult<()> {
        use uuid::Uuid;

        let reaction_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT OR REPLACE INTO message_reactions (id, message_id, user_id, reaction, timestamp)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(reaction_id)
        .bind(message_id)
        .bind(&reaction.user_id)
        .bind(&reaction.reaction)
        .bind(reaction.timestamp as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    async fn remove_reaction(
        &self,
        message_id: &str,
        user_id: &str,
        reaction: &str,
    ) -> StorageResult<()> {
        sqlx::query(
            "DELETE FROM message_reactions WHERE message_id = ? AND user_id = ? AND reaction = ?",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(reaction)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    async fn add_read_receipt(
        &self,
        message_id: &str,
        receipt: MessageReadReceipt,
    ) -> StorageResult<()> {
        use uuid::Uuid;

        let receipt_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT OR REPLACE INTO message_read_receipts (id, message_id, user_id, timestamp)
             VALUES (?, ?, ?, ?)",
        )
        .bind(receipt_id)
        .bind(message_id)
        .bind(&receipt.user_id)
        .bind(receipt.timestamp as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    async fn get_unread_messages(
        &self,
        user_id: &str,
        room_id: &str,
        since: u64,
    ) -> StorageResult<Vec<Message>> {
        // Get messages in room since the given timestamp that haven't been read by the user
        let rows = sqlx::query(
            "SELECT m.id, m.room_id, m.user_id, m.content, m.message_type, m.timestamp, m.edited_at,
             m.parent_message_id, m.metadata, m.is_deleted, m.deleted_at
             FROM messages m
             LEFT JOIN message_read_receipts r ON m.id = r.message_id AND r.user_id = ?
             WHERE m.room_id = ? AND m.timestamp > ? AND m.is_deleted = 0 AND r.id IS NULL
             ORDER BY m.timestamp ASC",
        )
        .bind(user_id)
        .bind(room_id)
        .bind(since as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }
    async fn mark_messages_read(
        &self,
        user_id: &str,
        room_id: &str,
        up_to_message_id: &str,
        timestamp: u64,
    ) -> StorageResult<()> {
        use uuid::Uuid;

        // Get the timestamp of the up_to_message_id
        let up_to_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT timestamp FROM messages WHERE id = ?")
                .bind(up_to_message_id)
                .fetch_optional(&self.pool)
                .await?;

        let up_to_timestamp = match up_to_timestamp {
            Some(ts) => ts,
            None => return Ok(()), // Message not found, nothing to mark
        };

        // Get all messages in the room up to the specified timestamp that haven't been read
        let message_ids: Vec<String> = sqlx::query_scalar(
            "SELECT m.id FROM messages m
             LEFT JOIN message_read_receipts r ON m.id = r.message_id AND r.user_id = ?
             WHERE m.room_id = ? AND m.timestamp <= ? AND m.is_deleted = 0 AND r.id IS NULL",
        )
        .bind(user_id)
        .bind(room_id)
        .bind(up_to_timestamp)
        .fetch_all(&self.pool)
        .await?;

        // Insert read receipts for all unread messages
        for message_id in message_ids {
            let receipt_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT OR IGNORE INTO message_read_receipts (id, message_id, user_id, timestamp)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(receipt_id)
            .bind(message_id)
            .bind(user_id)
            .bind(timestamp as i64)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
    async fn count_room_messages(&self, room_id: &str) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE room_id = ? AND is_deleted = 0",
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as u64)
    }

    async fn count_user_messages(&self, user_id: &str) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE user_id = ? AND is_deleted = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as u64)
    }

    async fn delete_old_messages(&self, before_timestamp: u64) -> StorageResult<u64> {
        let result =
            sqlx::query("UPDATE messages SET is_deleted = 1, deleted_at = ? WHERE timestamp < ?")
                .bind(super::current_timestamp() as i64)
                .bind(before_timestamp as i64)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    async fn get_message_stats(&self) -> StorageResult<MessageStats> {
        let total_messages: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_deleted = 0")
                .fetch_one(&self.pool)
                .await?;

        let now = super::current_timestamp();
        let day_ago = now - 86400;
        let week_ago = now - (86400 * 7);
        let month_ago = now - (86400 * 30);

        let messages_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE timestamp > ? AND is_deleted = 0",
        )
        .bind(day_ago as i64)
        .fetch_one(&self.pool)
        .await?;

        let messages_this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE timestamp > ? AND is_deleted = 0",
        )
        .bind(week_ago as i64)
        .fetch_one(&self.pool)
        .await?;

        let messages_this_month: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE timestamp > ? AND is_deleted = 0",
        )
        .bind(month_ago as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(MessageStats {
            total_messages: total_messages as u64,
            messages_today: messages_today as u64,
            messages_this_week: messages_this_week as u64,
            messages_this_month: messages_this_month as u64,
            messages_by_type: std::collections::HashMap::new(),
            most_active_rooms: Vec::new(),
            most_active_users: Vec::new(),
        })
    }
}

#[async_trait]
impl RoomStorage for SqliteStorage {
    async fn create_room(&self, room: Room) -> StorageResult<Room> {
        let settings_json = serde_json::to_string(&room.settings).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let room_type_str = match room.room_type {
            RoomType::Channel => "channel",
            RoomType::Group => "group",
            RoomType::DirectMessage => "direct_message",
            RoomType::System => "system",
            RoomType::Temporary => "temporary",
        };

        let privacy_str = match room.privacy {
            RoomPrivacy::Public => "public",
            RoomPrivacy::Private => "private",
            RoomPrivacy::Protected => "protected",
            RoomPrivacy::System => "system",
        };

        sqlx::query(
            "INSERT INTO rooms (id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&room.id)
        .bind(&room.name)
        .bind(&room.display_name)
        .bind(&room.description)
        .bind(&room.topic)
        .bind(room_type_str)
        .bind(privacy_str)
        .bind(settings_json)
        .bind(&room.created_by)
        .bind(room.created_at as i64)
        .bind(room.updated_at as i64)
        .bind(room.is_active)
        .execute(&self.pool)
        .await?;

        Ok(room)
    }

    async fn get_room_by_id(&self, id: &str) -> StorageResult<Option<Room>> {
        let row = sqlx::query(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_room(row)?)),
            None => Ok(None),
        }
    }

    async fn get_room_by_name(&self, name: &str) -> StorageResult<Option<Room>> {
        let row = sqlx::query(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_room(row)?)),
            None => Ok(None),
        }
    }

    async fn count_rooms(&self) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE is_active = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(count as u64)
    }

    async fn update_room(&self, room: Room) -> StorageResult<Room> {
        let settings_json = serde_json::to_string(&room.settings).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let room_type_str = match room.room_type {
            RoomType::Channel => "channel",
            RoomType::Group => "group",
            RoomType::DirectMessage => "direct_message",
            RoomType::System => "system",
            RoomType::Temporary => "temporary",
        };

        let privacy_str = match room.privacy {
            RoomPrivacy::Public => "public",
            RoomPrivacy::Private => "private",
            RoomPrivacy::Protected => "protected",
            RoomPrivacy::System => "system",
        };

        sqlx::query(
            "UPDATE rooms SET name = ?, display_name = ?, description = ?, topic = ?,
             room_type = ?, privacy = ?, settings = ?, updated_at = ?, is_active = ?
             WHERE id = ?",
        )
        .bind(&room.name)
        .bind(&room.display_name)
        .bind(&room.description)
        .bind(&room.topic)
        .bind(room_type_str)
        .bind(privacy_str)
        .bind(settings_json)
        .bind(room.updated_at as i64)
        .bind(room.is_active)
        .bind(&room.id)
        .execute(&self.pool)
        .await?;

        Ok(room)
    }

    async fn update_room_settings(
        &self,
        room_id: &str,
        settings: RoomSettings,
    ) -> StorageResult<()> {
        let settings_json =
            serde_json::to_string(&settings).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        let timestamp = super::current_timestamp();

        sqlx::query("UPDATE rooms SET settings = ?, updated_at = ? WHERE id = ?")
            .bind(settings_json)
            .bind(timestamp as i64)
            .bind(room_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn deactivate_room(&self, room_id: &str) -> StorageResult<()> {
        let timestamp = super::current_timestamp();

        sqlx::query("UPDATE rooms SET is_active = 0, updated_at = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(room_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn reactivate_room(&self, room_id: &str) -> StorageResult<()> {
        let timestamp = super::current_timestamp();

        sqlx::query("UPDATE rooms SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(room_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_room(&self, room_id: &str) -> StorageResult<()> {
        sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    async fn list_rooms(
        &self,
        pagination: Pagination,
        order_by: Option<OrderBy>,
    ) -> StorageResult<Vec<Room>> {
        let order_clause =
            self.order_by_to_sql(&Some(order_by.unwrap_or(OrderBy::desc("created_at"))));
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE is_active = 1 {} {}",
            order_clause, limit_offset
        );

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(self.row_to_room(row)?);
        }

        Ok(rooms)
    }

    async fn list_public_rooms(&self, pagination: Pagination) -> StorageResult<Vec<Room>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE is_active = 1 AND privacy = 'public'
             ORDER BY created_at DESC {}",
            limit_offset
        );

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(self.row_to_room(row)?);
        }

        Ok(rooms)
    }

    async fn list_rooms_by_type(
        &self,
        room_type: RoomType,
        pagination: Pagination,
    ) -> StorageResult<Vec<Room>> {
        let room_type_str = match room_type {
            RoomType::Channel => "channel",
            RoomType::Group => "group",
            RoomType::DirectMessage => "direct_message",
            RoomType::System => "system",
            RoomType::Temporary => "temporary",
        };

        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE is_active = 1 AND room_type = ?
             ORDER BY created_at DESC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(room_type_str)
            .fetch_all(&self.pool)
            .await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(self.row_to_room(row)?);
        }

        Ok(rooms)
    }

    async fn search_rooms(&self, query: &str, pagination: Pagination) -> StorageResult<Vec<Room>> {
        let limit_offset = self.pagination_to_sql(&pagination);
        let search_pattern = format!("%{}%", query);

        let sql_query = format!(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE is_active = 1 AND (
                 name LIKE ? OR display_name LIKE ? OR description LIKE ?
             ) ORDER BY created_at DESC {}",
            limit_offset
        );

        let rows = sqlx::query(&sql_query)
            .bind(&search_pattern)
            .bind(&search_pattern)
            .bind(&search_pattern)
            .fetch_all(&self.pool)
            .await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(self.row_to_room(row)?);
        }

        Ok(rooms)
    }

    async fn get_user_created_rooms(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Room>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, name, display_name, description, topic, room_type, privacy,
             settings, created_by, created_at, updated_at, is_active
             FROM rooms WHERE is_active = 1 AND created_by = ?
             ORDER BY created_at DESC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(self.row_to_room(row)?);
        }

        Ok(rooms)
    }
    async fn add_room_member(&self, membership: RoomMembership) -> StorageResult<RoomMembership> {
        let settings_json = serde_json::to_string(&membership.settings).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        let role_str = match membership.role {
            RoomRole::Owner => "owner",
            RoomRole::Admin => "admin",
            RoomRole::Moderator => "moderator",
            RoomRole::Member => "member",
            RoomRole::Guest => "guest",
        };

        sqlx::query(
            "INSERT INTO room_memberships (id, room_id, user_id, role, joined_at, last_activity, is_active, settings)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&membership.id)
        .bind(&membership.room_id)
        .bind(&membership.user_id)
        .bind(role_str)
        .bind(membership.joined_at as i64)
        .bind(membership.last_activity.map(|t| t as i64))
        .bind(membership.is_active)
        .bind(settings_json)
        .execute(&self.pool)
        .await?;

        Ok(membership)
    }

    async fn remove_room_member(&self, room_id: &str, user_id: &str) -> StorageResult<()> {
        sqlx::query("DELETE FROM room_memberships WHERE room_id = ? AND user_id = ?")
            .bind(room_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_member_role(
        &self,
        room_id: &str,
        user_id: &str,
        role: RoomRole,
    ) -> StorageResult<()> {
        let role_str = match role {
            RoomRole::Owner => "owner",
            RoomRole::Admin => "admin",
            RoomRole::Moderator => "moderator",
            RoomRole::Member => "member",
            RoomRole::Guest => "guest",
        };

        sqlx::query("UPDATE room_memberships SET role = ? WHERE room_id = ? AND user_id = ?")
            .bind(role_str)
            .bind(room_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_member_settings(
        &self,
        room_id: &str,
        user_id: &str,
        settings: RoomMemberSettings,
    ) -> StorageResult<()> {
        let settings_json =
            serde_json::to_string(&settings).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        sqlx::query("UPDATE room_memberships SET settings = ? WHERE room_id = ? AND user_id = ?")
            .bind(settings_json)
            .bind(room_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_room_membership(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> StorageResult<Option<RoomMembership>> {
        let row = sqlx::query(
            "SELECT id, room_id, user_id, role, joined_at, last_activity, is_active, settings
             FROM room_memberships WHERE room_id = ? AND user_id = ?",
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_room_membership(row)?)),
            None => Ok(None),
        }
    }
    async fn list_room_members(
        &self,
        room_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<RoomMembership>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, role, joined_at, last_activity, is_active, settings
             FROM room_memberships WHERE room_id = ? AND is_active = 1
             ORDER BY joined_at ASC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(room_id)
            .fetch_all(&self.pool)
            .await?;

        let mut memberships = Vec::new();
        for row in rows {
            memberships.push(self.row_to_room_membership(row)?);
        }

        Ok(memberships)
    }

    async fn list_user_memberships(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<RoomMembership>> {
        let limit_offset = self.pagination_to_sql(&pagination);

        let query = format!(
            "SELECT id, room_id, user_id, role, joined_at, last_activity, is_active, settings
             FROM room_memberships WHERE user_id = ? AND is_active = 1
             ORDER BY joined_at DESC {}",
            limit_offset
        );

        let rows = sqlx::query(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let mut memberships = Vec::new();
        for row in rows {
            memberships.push(self.row_to_room_membership(row)?);
        }

        Ok(memberships)
    }

    async fn get_active_room_members(
        &self,
        room_id: &str,
        since: u64,
    ) -> StorageResult<Vec<RoomMembership>> {
        let rows = sqlx::query(
            "SELECT id, room_id, user_id, role, joined_at, last_activity, is_active, settings
             FROM room_memberships WHERE room_id = ? AND is_active = 1 AND last_activity > ?
             ORDER BY last_activity DESC",
        )
        .bind(room_id)
        .bind(since as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut memberships = Vec::new();
        for row in rows {
            memberships.push(self.row_to_room_membership(row)?);
        }

        Ok(memberships)
    }

    async fn count_room_members(&self, room_id: &str) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM room_memberships WHERE room_id = ? AND is_active = 1",
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    async fn room_name_exists(&self, name: &str) -> StorageResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    async fn is_room_member(&self, room_id: &str, user_id: &str) -> StorageResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM room_memberships WHERE room_id = ? AND user_id = ? AND is_active = 1",
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }
    async fn get_room_stats(&self) -> StorageResult<RoomStats> {
        let total_rooms: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms")
            .fetch_one(&self.pool)
            .await?;

        let active_rooms: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE is_active = 1")
                .fetch_one(&self.pool)
                .await?;

        let public_rooms: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE is_active = 1 AND privacy = 'public'",
        )
        .fetch_one(&self.pool)
        .await?;

        let private_rooms: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE is_active = 1 AND privacy = 'private'",
        )
        .fetch_one(&self.pool)
        .await?;

        // Get rooms by type
        let mut rooms_by_type = std::collections::HashMap::new();

        let channel_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE is_active = 1 AND room_type = 'channel'",
        )
        .fetch_one(&self.pool)
        .await?;
        rooms_by_type.insert("channel".to_string(), channel_count as u64);

        let group_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE is_active = 1 AND room_type = 'group'",
        )
        .fetch_one(&self.pool)
        .await?;
        rooms_by_type.insert("group".to_string(), group_count as u64);

        let dm_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE is_active = 1 AND room_type = 'direct_message'",
        )
        .fetch_one(&self.pool)
        .await?;
        rooms_by_type.insert("direct_message".to_string(), dm_count as u64);

        // Calculate average members per room
        let total_memberships: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM room_memberships WHERE is_active = 1")
                .fetch_one(&self.pool)
                .await?;

        let average_members_per_room = if active_rooms > 0 {
            total_memberships as f64 / active_rooms as f64
        } else {
            0.0
        };

        // Get largest rooms
        let largest_rooms: Vec<(String, i64)> = sqlx::query_as(
            "SELECT r.name, COUNT(rm.id) as member_count
             FROM rooms r
             LEFT JOIN room_memberships rm ON r.id = rm.room_id AND rm.is_active = 1
             WHERE r.is_active = 1
             GROUP BY r.id, r.name
             ORDER BY member_count DESC
             LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await?;

        let largest_rooms: Vec<(String, u64)> = largest_rooms
            .into_iter()
            .map(|(name, count)| (name, count as u64))
            .collect();

        Ok(RoomStats {
            total_rooms: total_rooms as u64,
            active_rooms: active_rooms as u64,
            public_rooms: public_rooms as u64,
            private_rooms: private_rooms as u64,
            rooms_by_type,
            average_members_per_room,
            largest_rooms,
        })
    }
}

#[async_trait]
impl SessionStorage for SqliteStorage {
    async fn create_session(&self, session: Session) -> StorageResult<Session> {
        let metadata_json = serde_json::to_string(&session.metadata).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, user_id, token, created_at, expires_at, last_activity,
                ip_address, user_agent, is_active, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.token)
        .bind(session.created_at as i64)
        .bind(session.expires_at as i64)
        .bind(session.last_activity as i64)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.is_active)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(session)
    }

    async fn get_session_by_id(&self, id: &str) -> StorageResult<Option<Session>> {
        let row = sqlx::query("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_session(row)?)),
            None => Ok(None),
        }
    }

    async fn get_session_by_token(&self, token: &str) -> StorageResult<Option<Session>> {
        let row = sqlx::query("SELECT * FROM sessions WHERE token = ?")
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_session(row)?)),
            None => Ok(None),
        }
    }

    async fn update_session_activity(&self, session_id: &str, timestamp: u64) -> StorageResult<()> {
        sqlx::query("UPDATE sessions SET last_activity = ? WHERE id = ?")
            .bind(timestamp as i64)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_session_metadata(
        &self,
        session_id: &str,
        metadata: SessionMetadata,
    ) -> StorageResult<()> {
        let metadata_json =
            serde_json::to_string(&metadata).map_err(|e| StorageError::SerializationError {
                message: e.to_string(),
            })?;

        sqlx::query("UPDATE sessions SET metadata = ? WHERE id = ?")
            .bind(&metadata_json)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn deactivate_session(&self, session_id: &str) -> StorageResult<()> {
        sqlx::query("UPDATE sessions SET is_active = 0 WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> StorageResult<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_user_sessions(
        &self,
        user_id: &str,
        pagination: Pagination,
    ) -> StorageResult<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM sessions
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user_id)
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(self.row_to_session(row)?);
        }

        Ok(sessions)
    }

    async fn get_active_user_sessions(&self, user_id: &str) -> StorageResult<Vec<Session>> {
        let now = super::current_timestamp();
        let rows = sqlx::query(
            r#"
            SELECT * FROM sessions
            WHERE user_id = ? AND is_active = 1 AND expires_at > ?
            ORDER BY last_activity DESC
            "#,
        )
        .bind(user_id)
        .bind(now as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(self.row_to_session(row)?);
        }

        Ok(sessions)
    }

    async fn deactivate_user_sessions(&self, user_id: &str) -> StorageResult<u64> {
        let result = sqlx::query("UPDATE sessions SET is_active = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn deactivate_user_sessions_except(
        &self,
        user_id: &str,
        except_session_id: &str,
    ) -> StorageResult<u64> {
        let result = sqlx::query("UPDATE sessions SET is_active = 0 WHERE user_id = ? AND id != ?")
            .bind(user_id)
            .bind(except_session_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn update_session(&self, session: &Session) -> StorageResult<()> {
        let metadata_json = serde_json::to_string(&session.metadata).map_err(|e| {
            StorageError::SerializationError {
                message: e.to_string(),
            }
        })?;

        sqlx::query(
            r#"
            UPDATE sessions SET
                token = ?, expires_at = ?, last_activity = ?,
                ip_address = ?, user_agent = ?, is_active = ?, metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(&session.token)
        .bind(session.expires_at as i64)
        .bind(session.last_activity as i64)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.is_active)
        .bind(&metadata_json)
        .bind(&session.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> StorageResult<Option<Session>> {
        self.get_session_by_id(session_id).await
    }

    async fn cleanup_expired_sessions(&self) -> StorageResult<u64> {
        let now = super::current_timestamp();
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(now as i64)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn count_active_sessions(&self) -> StorageResult<u64> {
        let now = super::current_timestamp();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE is_active = 1 AND expires_at > ?",
        )
        .bind(now as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as u64)
    }

    async fn count_user_sessions(&self, user_id: &str) -> StorageResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count as u64)
    }

    async fn get_session_stats(&self) -> StorageResult<SessionStats> {
        let now = super::current_timestamp();
        let today_start = now - (24 * 60 * 60); // 24 hours ago
        let week_start = now - (7 * 24 * 60 * 60); // 7 days ago

        // Total sessions
        let total_sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
            .fetch_one(&self.pool)
            .await?;

        // Active sessions
        let active_sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE is_active = 1 AND expires_at > ?",
        )
        .bind(now as i64)
        .fetch_one(&self.pool)
        .await?;

        // Sessions today
        let sessions_today: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE created_at > ?")
                .bind(today_start as i64)
                .fetch_one(&self.pool)
                .await?;

        // Sessions this week
        let sessions_this_week: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE created_at > ?")
                .bind(week_start as i64)
                .fetch_one(&self.pool)
                .await?;

        // Sessions by client type
        let client_rows = sqlx::query(
            r#"
            SELECT
                COALESCE(JSON_EXTRACT(metadata, '$.client_type'), 'unknown') as client_type,
                COUNT(*) as count
            FROM sessions
            GROUP BY JSON_EXTRACT(metadata, '$.client_type')
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions_by_client = std::collections::HashMap::new();
        for row in client_rows {
            let client_type: String = row.get("client_type");
            let count: i64 = row.get("count");
            sessions_by_client.insert(client_type, count as u64);
        }

        // Average session duration (for completed sessions)
        let avg_duration: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(last_activity - created_at)
            FROM sessions
            WHERE is_active = 0 AND last_activity > created_at
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SessionStats {
            total_sessions: total_sessions as u64,
            active_sessions: active_sessions as u64,
            sessions_today: sessions_today as u64,
            sessions_this_week: sessions_this_week as u64,
            sessions_by_client,
            average_session_duration: avg_duration.unwrap_or(0.0),
        })
    }
}
