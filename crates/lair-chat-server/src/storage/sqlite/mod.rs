//! SQLite storage implementation.
//!
//! This module provides a complete SQLite-based implementation of all storage traits.
//! It uses SQLx for async database access with connection pooling.
//!
//! ## Features
//!
//! - Connection pooling via SQLx
//! - WAL mode for better concurrent read/write performance
//! - Automatic migrations
//! - Foreign key enforcement
//!
//! ## Configuration
//!
//! The `SqliteStorage` is created with a database URL:
//!
//! ```rust,ignore
//! let storage = SqliteStorage::new("sqlite:data.db?mode=rwc").await?;
//! ```
//!
//! Options:
//! - `mode=rwc`: Read-write-create (creates file if missing)
//! - `mode=ro`: Read-only
//! - `:memory:`: In-memory database (for testing)

mod invitations;
mod messages;
mod migrations;
mod rooms;
mod sessions;
mod users;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

use crate::Result;

/// SQLite-based storage implementation.
///
/// Implements all repository traits for a complete storage solution.
/// Uses connection pooling for efficient database access.
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

/// Configuration for SQLite storage.
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Database URL (e.g., "sqlite:data.db?mode=rwc")
    pub url: String,
    /// Maximum connections in the pool
    pub max_connections: u32,
    /// Minimum connections to keep open
    pub min_connections: u32,
    /// Timeout for acquiring a connection
    pub acquire_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Run migrations on startup
    pub auto_migrate: bool,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:lair-chat.db?mode=rwc".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            auto_migrate: true,
        }
    }
}

impl SqliteConfig {
    /// Create config for an in-memory database (useful for testing).
    pub fn in_memory() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 1, // In-memory requires single connection
            min_connections: 1,
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
            auto_migrate: true,
        }
    }

    /// Create config from a database path.
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            url: format!("sqlite:{}?mode=rwc", path.into()),
            ..Default::default()
        }
    }
}

impl SqliteStorage {
    /// Create a new SQLite storage instance with default configuration.
    pub async fn new(url: &str) -> Result<Self> {
        let config = SqliteConfig {
            url: url.to_string(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create a new SQLite storage instance with custom configuration.
    pub async fn with_config(config: SqliteConfig) -> Result<Self> {
        info!("Initializing SQLite storage: {}", config.url);

        let connect_options = SqliteConnectOptions::from_str(&config.url)
            .map_err(|e| crate::Error::Config(format!("Invalid SQLite URL: {e}")))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(30))
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .connect_with(connect_options)
            .await?;

        let storage = Self { pool };

        if config.auto_migrate {
            storage.run_migrations().await?;
        }

        info!("SQLite storage initialized successfully");
        Ok(storage)
    }

    /// Create an in-memory database (useful for testing).
    pub async fn in_memory() -> Result<Self> {
        Self::with_config(SqliteConfig::in_memory()).await
    }

    /// Run database migrations.
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Create migrations tracking table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                applied_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Get all migrations
        let migrations = migrations::all();

        for (name, sql) in migrations {
            self.apply_migration(name, sql).await?;
        }

        info!("Database migrations completed");
        Ok(())
    }

    /// Apply a single migration if not already applied.
    async fn apply_migration(&self, name: &str, sql: &str) -> Result<()> {
        // Check if already applied
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?)")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;

        if exists {
            debug!("Migration {} already applied, skipping", name);
            return Ok(());
        }

        debug!("Applying migration: {}", name);

        // Execute in a transaction
        let mut tx = self.pool.begin().await?;

        // Split SQL into statements and execute each
        for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(statement).execute(&mut *tx).await?;
        }

        // Record migration
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO _migrations (name, applied_at) VALUES (?, ?)")
            .bind(name)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Migration {} applied successfully", name);
        Ok(())
    }

    /// Get the underlying connection pool.
    ///
    /// Useful for advanced operations or testing.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the storage connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Check if the database is healthy.
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

// Note: SqliteStorage automatically implements the Storage trait via the blanket impl
// because it implements all the individual repository traits.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_in_memory() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        storage.health_check().await.unwrap();
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let storage = SqliteStorage::in_memory().await.unwrap();

        // Run migrations again - should not fail
        storage.run_migrations().await.unwrap();
        storage.run_migrations().await.unwrap();
    }
}
