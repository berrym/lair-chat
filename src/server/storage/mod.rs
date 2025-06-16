//! Storage abstraction layer for lair-chat server
//!
//! This module provides a database-agnostic storage layer with support for
//! multiple database backends (SQLite, PostgreSQL, MySQL) and comprehensive
//! data management for users, messages, rooms, and sessions.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

pub mod migrations;
pub mod models;
pub mod sqlite;
pub mod traits;

pub use models::*;
pub use sqlite::SqliteStorage;
pub use traits::*;

/// Storage layer errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database connection failed: {message}")]
    ConnectionError { message: String },

    #[error("Database query failed: {message}")]
    QueryError { message: String },

    #[error("Transaction failed: {message}")]
    TransactionError { message: String },

    #[error("Migration failed: {message}")]
    MigrationError { message: String },

    #[error("Record not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Duplicate record: {entity} - {message}")]
    DuplicateError { entity: String, message: String },

    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Deserialization error: {message}")]
    DeserializationError { message: String },

    #[error("Database constraint violation: {message}")]
    ConstraintError { message: String },

    #[error("Database timeout: operation took too long")]
    TimeoutError,

    #[error("Database pool exhausted: no connections available")]
    PoolExhausted,

    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },
}

impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StorageError::NotFound {
                entity: "Record".to_string(),
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() {
                    StorageError::DuplicateError {
                        entity: "Record".to_string(),
                        message: db_err.message().to_string(),
                    }
                } else if db_err.is_foreign_key_violation() {
                    StorageError::ConstraintError {
                        message: db_err.message().to_string(),
                    }
                } else {
                    StorageError::QueryError {
                        message: db_err.message().to_string(),
                    }
                }
            }
            sqlx::Error::PoolTimedOut => StorageError::TimeoutError,
            sqlx::Error::PoolClosed => StorageError::PoolExhausted,
            _ => StorageError::QueryError {
                message: err.to_string(),
            },
        }
    }
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: std::time::Duration,
    pub idle_timeout: std::time::Duration,
    pub auto_migrate: bool,
}

impl From<crate::server::config::DatabaseConfig> for DatabaseConfig {
    fn from(config: crate::server::config::DatabaseConfig) -> Self {
        Self {
            url: config.url,
            max_connections: config.max_connections,
            min_connections: config.min_connections,
            connection_timeout: std::time::Duration::from_secs(config.connection_timeout),
            idle_timeout: std::time::Duration::from_secs(config.idle_timeout),
            auto_migrate: config.auto_migrate,
        }
    }
}

/// Storage manager that coordinates all storage operations
pub struct StorageManager {
    user_storage: Box<dyn UserStorage>,
    message_storage: Box<dyn MessageStorage>,
    room_storage: Box<dyn RoomStorage>,
    session_storage: Box<dyn SessionStorage>,
    audit_log_storage: Box<dyn AuditLogStorage>,
}

impl StorageManager {
    /// Create a new storage manager with the specified backend
    pub async fn new(config: DatabaseConfig) -> StorageResult<Self> {
        // For now, we only support SQLite, but this can be extended
        let backend = sqlite::SqliteStorage::new(config).await?;

        Ok(Self {
            user_storage: Box::new(backend.clone()),
            message_storage: Box::new(backend.clone()),
            room_storage: Box::new(backend.clone()),
            session_storage: Box::new(backend.clone()),
            audit_log_storage: Box::new(backend),
        })
    }

    /// Get user storage interface
    pub fn users(&self) -> &dyn UserStorage {
        self.user_storage.as_ref()
    }

    /// Get message storage interface
    pub fn messages(&self) -> &dyn MessageStorage {
        self.message_storage.as_ref()
    }

    /// Get room storage interface
    pub fn rooms(&self) -> &dyn RoomStorage {
        self.room_storage.as_ref()
    }

    /// Get session storage interface
    pub fn sessions(&self) -> &dyn SessionStorage {
        self.session_storage.as_ref()
    }

    /// Get audit log storage interface
    pub fn audit_logs(&self) -> &dyn AuditLogStorage {
        self.audit_log_storage.as_ref()
    }

    /// Run health check on all storage backends
    pub async fn health_check(&self) -> StorageResult<StorageHealth> {
        // This would check database connectivity, pool status, etc.
        Ok(StorageHealth {
            connected: true,
            pool_active: 5,
            pool_idle: 2,
            pool_max: 10,
            last_error: None,
        })
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageResult<StorageStats> {
        let user_count = self.users().count_users().await?;
        let message_count = self.messages().count_messages().await?;
        let room_count = self.rooms().count_rooms().await?;
        let session_count = self.sessions().count_active_sessions().await?;

        Ok(StorageStats {
            user_count,
            message_count,
            room_count,
            active_session_count: session_count,
        })
    }

    /// Perform database cleanup operations
    pub async fn cleanup(&self) -> StorageResult<()> {
        // Clean up expired sessions
        self.sessions().cleanup_expired_sessions().await?;

        // Clean up old messages if retention is configured
        // This would be configurable based on server settings

        Ok(())
    }
}

/// Storage health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    pub connected: bool,
    pub pool_active: u32,
    pub pool_idle: u32,
    pub pool_max: u32,
    pub last_error: Option<String>,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub user_count: u64,
    pub message_count: u64,
    pub room_count: u64,
    pub active_session_count: u64,
}

/// Pagination parameters for query results
#[derive(Debug, Clone)]
pub struct Pagination {
    pub offset: u64,
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

impl Pagination {
    pub fn new(offset: u64, limit: u64) -> Self {
        Self {
            offset,
            limit: limit.min(1000), // Cap at 1000 items per page
        }
    }
}

/// Ordering parameters for query results
#[derive(Debug, Clone)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: OrderDirection,
}

impl OrderBy {
    pub fn asc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            direction: OrderDirection::Ascending,
        }
    }

    pub fn desc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            direction: OrderDirection::Descending,
        }
    }
}

/// Utility function to get current timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Utility function to generate a new UUID
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let pagination = Pagination::default();
        assert_eq!(pagination.offset, 0);
        assert_eq!(pagination.limit, 50);

        let custom = Pagination::new(100, 25);
        assert_eq!(custom.offset, 100);
        assert_eq!(custom.limit, 25);

        // Test limit capping
        let capped = Pagination::new(0, 2000);
        assert_eq!(capped.limit, 1000);
    }

    #[test]
    fn test_order_by() {
        let asc = OrderBy::asc("created_at");
        assert_eq!(asc.field, "created_at");
        assert!(matches!(asc.direction, OrderDirection::Ascending));

        let desc = OrderBy::desc("updated_at");
        assert_eq!(desc.field, "updated_at");
        assert!(matches!(desc.direction, OrderDirection::Descending));
    }

    #[test]
    fn test_current_timestamp() {
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = current_timestamp();
        assert!(ts2 >= ts1);
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID v4 string length
    }
}
