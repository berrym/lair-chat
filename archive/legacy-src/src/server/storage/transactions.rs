//! Database Transaction Management System
//!
//! This module provides comprehensive transaction management capabilities
//! for the lair-chat server, including atomic operations, rollback mechanisms,
//! and integration with the existing storage and error handling systems.

use super::{models::*, traits::*, StorageError, StorageResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, Pool, Sqlite, Transaction as SqlxTransaction};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Transaction-specific error types
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Transaction failed to begin: {message}")]
    BeginFailed { message: String },

    #[error("Transaction failed to commit: {message}")]
    CommitFailed { message: String },

    #[error("Transaction failed to rollback: {message}")]
    RollbackFailed { message: String },

    #[error("Transaction timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Transaction deadlock detected: {message}")]
    Deadlock { message: String },

    #[error("Transaction already completed")]
    AlreadyCompleted,

    #[error("Transaction not found: {id}")]
    NotFound { id: String },

    #[error("Concurrent transaction conflict: {message}")]
    ConcurrentConflict { message: String },

    #[error("Storage error in transaction: {0}")]
    StorageError(#[from] StorageError),
}

/// Transaction result type
pub type TransactionResult<T> = Result<T, TransactionError>;

/// Transaction state tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionState {
    Active,
    Committed,
    RolledBack,
    Failed,
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub id: String,
    pub state: TransactionState,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub timeout_at: u64,
    pub operations: Vec<String>,
    pub error: Option<String>,
}

/// Database transaction wrapper
pub struct Transaction<'a> {
    inner: SqlxTransaction<'a, Sqlite>,
    metadata: TransactionMetadata,
}

impl<'a> Transaction<'a> {
    /// Create a new transaction
    pub fn new(inner: SqlxTransaction<'a, Sqlite>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let metadata = TransactionMetadata {
            id: Uuid::new_v4().to_string(),
            state: TransactionState::Active,
            created_at: now,
            completed_at: None,
            timeout_at: now + 30, // 30 second timeout
            operations: Vec::new(),
            error: None,
        };

        Self { inner, metadata }
    }

    /// Get transaction ID
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Get transaction state
    pub fn state(&self) -> &TransactionState {
        &self.metadata.state
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        matches!(self.metadata.state, TransactionState::Active)
    }

    /// Check if transaction has timed out
    pub fn is_timed_out(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.metadata.timeout_at
    }

    /// Record an operation in the transaction
    pub fn record_operation(&mut self, operation: &str) {
        self.metadata.operations.push(operation.to_string());
    }

    /// Get transaction metadata
    pub fn metadata(&self) -> &TransactionMetadata {
        &self.metadata
    }

    /// Get mutable reference to inner transaction
    pub fn inner_mut(&mut self) -> &mut SqlxTransaction<'a, Sqlite> {
        &mut self.inner
    }

    /// Get reference to inner transaction
    pub fn inner(&self) -> &SqlxTransaction<'a, Sqlite> {
        &self.inner
    }

    /// Get mutable reference to inner transaction as executor
    pub fn as_executor(&mut self) -> &mut SqlxTransaction<'a, Sqlite> {
        &mut self.inner
    }
}

/// Transaction manager trait
#[async_trait]
pub trait TransactionManager: Send + Sync {
    /// Begin a new transaction
    async fn begin_transaction(&self) -> TransactionResult<Transaction<'_>>;

    /// Commit a transaction
    async fn commit_transaction(&self, transaction: Transaction<'_>) -> TransactionResult<()>;

    /// Rollback a transaction
    async fn rollback_transaction(&self, transaction: Transaction<'_>) -> TransactionResult<()>;

    /// Get transaction statistics
    async fn get_transaction_stats(&self) -> TransactionResult<TransactionStats>;

    /// Cleanup timed out transactions
    async fn cleanup_timeout_transactions(&self) -> TransactionResult<u64>;
}

/// Transaction operations trait for atomic operations
#[async_trait]
pub trait TransactionOperations: Send + Sync {
    /// Create invitation with membership atomically
    async fn create_invitation_with_membership(
        &self,
        transaction: &mut Transaction<'_>,
        invitation: Invitation,
        membership: RoomMembership,
    ) -> TransactionResult<(Invitation, RoomMembership)>;

    /// Update invitation and membership atomically
    async fn update_invitation_and_membership(
        &self,
        transaction: &mut Transaction<'_>,
        invitation_id: &str,
        new_status: InvitationStatus,
        membership: RoomMembership,
    ) -> TransactionResult<(Invitation, RoomMembership)>;

    /// Perform batch room operations atomically
    async fn batch_room_operations(
        &self,
        transaction: &mut Transaction<'_>,
        operations: Vec<RoomOperation>,
    ) -> TransactionResult<Vec<RoomOperationResult>>;

    /// Create user with session atomically
    async fn user_registration_transaction(
        &self,
        transaction: &mut Transaction<'_>,
        user: User,
        session: Session,
    ) -> TransactionResult<(User, Session)>;

    /// Delete user and cleanup all related data atomically
    async fn user_deletion_transaction(
        &self,
        transaction: &mut Transaction<'_>,
        user_id: &str,
    ) -> TransactionResult<UserDeletionResult>;

    /// Create room with initial membership atomically
    async fn create_room_with_membership(
        &self,
        transaction: &mut Transaction<'_>,
        room: Room,
        creator_membership: RoomMembership,
    ) -> TransactionResult<(Room, RoomMembership)>;
}

/// Room operation types for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomOperation {
    CreateRoom(Room),
    UpdateRoom(Room),
    DeleteRoom(String),
    AddMember(RoomMembership),
    RemoveMember(String, String),               // room_id, user_id
    UpdateMemberRole(String, String, RoomRole), // room_id, user_id, new_role
}

/// Room operation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomOperationResult {
    RoomCreated(Room),
    RoomUpdated(Room),
    RoomDeleted(String),
    MemberAdded(RoomMembership),
    MemberRemoved(String, String),
    MemberRoleUpdated(RoomMembership),
}

/// User deletion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeletionResult {
    pub user_id: String,
    pub deleted_sessions: u64,
    pub deleted_messages: u64,
    pub removed_from_rooms: u64,
    pub deleted_invitations: u64,
}

/// Transaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub total_transactions: u64,
    pub active_transactions: u64,
    pub committed_transactions: u64,
    pub rolled_back_transactions: u64,
    pub failed_transactions: u64,
    pub timed_out_transactions: u64,
    pub average_duration: Duration,
    pub operations_by_type: HashMap<String, u64>,
}

/// Transaction manager implementation
pub struct DatabaseTransactionManager {
    pool: Arc<Pool<Sqlite>>,
    active_transactions: Arc<RwLock<HashMap<String, TransactionMetadata>>>,
    stats: Arc<RwLock<TransactionStats>>,
    config: TransactionConfig,
}

/// Transaction configuration
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub default_timeout: Duration,
    pub max_concurrent_transactions: usize,
    pub cleanup_interval: Duration,
    pub enable_deadlock_detection: bool,
    pub retry_on_deadlock: bool,
    pub max_retry_attempts: u32,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_concurrent_transactions: 100,
            cleanup_interval: Duration::from_secs(60),
            enable_deadlock_detection: true,
            retry_on_deadlock: true,
            max_retry_attempts: 3,
        }
    }
}

impl DatabaseTransactionManager {
    /// Create a new transaction manager
    pub fn new(pool: Arc<Pool<Sqlite>>, config: TransactionConfig) -> Self {
        Self {
            pool,
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TransactionStats::default())),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(pool: Arc<Pool<Sqlite>>) -> Self {
        Self::new(pool, TransactionConfig::default())
    }

    /// Update transaction metadata
    async fn update_transaction_metadata(
        &self,
        tx_id: &str,
        state: TransactionState,
        error: Option<String>,
    ) -> TransactionResult<()> {
        let mut active_txs = self.active_transactions.write().await;
        if let Some(metadata) = active_txs.get_mut(tx_id) {
            metadata.state = state.clone();
            metadata.error = error;

            if matches!(
                state,
                TransactionState::Committed
                    | TransactionState::RolledBack
                    | TransactionState::Failed
            ) {
                metadata.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );
            }
        }
        Ok(())
    }

    /// Update transaction statistics
    async fn update_stats(&self, operation: &str) -> TransactionResult<()> {
        let mut stats = self.stats.write().await;
        match operation {
            "begin" => stats.total_transactions += 1,
            "commit" => stats.committed_transactions += 1,
            "rollback" => stats.rolled_back_transactions += 1,
            "failed" => stats.failed_transactions += 1,
            "timeout" => stats.timed_out_transactions += 1,
            _ => {}
        }
        Ok(())
    }

    /// Check for deadlocks
    async fn check_deadlock(&self, error: &TransactionError) -> bool {
        if !self.config.enable_deadlock_detection {
            return false;
        }

        match error {
            TransactionError::StorageError(StorageError::QueryError { message }) => {
                message.to_lowercase().contains("deadlock")
                    || message.to_lowercase().contains("database is locked")
            }
            TransactionError::Deadlock { .. } => true,
            _ => false,
        }
    }
}

#[async_trait]
impl TransactionManager for DatabaseTransactionManager {
    async fn begin_transaction(&self) -> TransactionResult<Transaction<'_>> {
        // Check concurrent transaction limit
        let active_count = self.active_transactions.read().await.len();
        if active_count >= self.config.max_concurrent_transactions {
            return Err(TransactionError::ConcurrentConflict {
                message: "Maximum concurrent transactions reached".to_string(),
            });
        }

        // Begin database transaction
        let sqlx_tx = self
            .pool
            .begin()
            .await
            .map_err(|e| TransactionError::BeginFailed {
                message: format!("Failed to begin transaction: {}", e),
            })?;

        let mut transaction = Transaction::new(sqlx_tx);

        // Store transaction metadata
        let mut active_txs = self.active_transactions.write().await;
        active_txs.insert(transaction.id().to_string(), transaction.metadata().clone());

        self.update_stats("begin").await?;

        Ok(transaction)
    }

    async fn commit_transaction(&self, mut transaction: Transaction<'_>) -> TransactionResult<()> {
        if !transaction.is_active() {
            return Err(TransactionError::AlreadyCompleted);
        }

        if transaction.is_timed_out() {
            self.rollback_transaction(transaction).await?;
            return Err(TransactionError::Timeout {
                duration: self.config.default_timeout,
            });
        }

        let tx_id = transaction.id().to_string();

        // Commit the transaction
        let result = transaction.inner.commit().await;

        match result {
            Ok(()) => {
                self.update_transaction_metadata(&tx_id, TransactionState::Committed, None)
                    .await?;
                self.update_stats("commit").await?;

                // Remove from active transactions
                let mut active_txs = self.active_transactions.write().await;
                active_txs.remove(&tx_id);

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Commit failed: {}", e);
                self.update_transaction_metadata(
                    &tx_id,
                    TransactionState::Failed,
                    Some(error_msg.clone()),
                )
                .await?;
                self.update_stats("failed").await?;

                Err(TransactionError::CommitFailed { message: error_msg })
            }
        }
    }

    async fn rollback_transaction(
        &self,
        mut transaction: Transaction<'_>,
    ) -> TransactionResult<()> {
        if !transaction.is_active() {
            return Err(TransactionError::AlreadyCompleted);
        }

        let tx_id = transaction.id().to_string();

        // Rollback the transaction
        let result = transaction.inner.rollback().await;

        match result {
            Ok(()) => {
                self.update_transaction_metadata(&tx_id, TransactionState::RolledBack, None)
                    .await?;
                self.update_stats("rollback").await?;

                // Remove from active transactions
                let mut active_txs = self.active_transactions.write().await;
                active_txs.remove(&tx_id);

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Rollback failed: {}", e);
                self.update_transaction_metadata(
                    &tx_id,
                    TransactionState::Failed,
                    Some(error_msg.clone()),
                )
                .await?;
                self.update_stats("failed").await?;

                Err(TransactionError::RollbackFailed { message: error_msg })
            }
        }
    }

    async fn get_transaction_stats(&self) -> TransactionResult<TransactionStats> {
        let stats = self.stats.read().await;
        let active_count = self.active_transactions.read().await.len() as u64;

        let mut result = stats.clone();
        result.active_transactions = active_count;

        Ok(result)
    }

    async fn cleanup_timeout_transactions(&self) -> TransactionResult<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut active_txs = self.active_transactions.write().await;
        let mut timed_out = Vec::new();

        for (id, metadata) in active_txs.iter() {
            if metadata.state == TransactionState::Active && now > metadata.timeout_at {
                timed_out.push(id.clone());
            }
        }

        let count = timed_out.len() as u64;

        for id in timed_out {
            if let Some(mut metadata) = active_txs.remove(&id) {
                metadata.state = TransactionState::Failed;
                metadata.error = Some("Transaction timed out".to_string());
                metadata.completed_at = Some(now);
            }
        }

        if count > 0 {
            self.update_stats("timeout").await?;
        }

        Ok(count)
    }
}

impl Default for TransactionStats {
    fn default() -> Self {
        Self {
            total_transactions: 0,
            active_transactions: 0,
            committed_transactions: 0,
            rolled_back_transactions: 0,
            failed_transactions: 0,
            timed_out_transactions: 0,
            average_duration: Duration::from_millis(0),
            operations_by_type: HashMap::new(),
        }
    }
}

/// Transaction executor with retry logic
pub struct TransactionExecutor {
    manager: Arc<dyn TransactionManager>,
    config: TransactionConfig,
}

impl TransactionExecutor {
    pub fn new(manager: Arc<dyn TransactionManager>, config: TransactionConfig) -> Self {
        Self { manager, config }
    }

    /// Execute operation with transaction and retry logic
    pub async fn execute_with_retry<F, T>(&self, operation: F) -> TransactionResult<T>
    where
        F: Fn(&mut Transaction<'_>) -> TransactionResult<T> + Send + Sync,
        T: Send + Sync,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.config.max_retry_attempts {
            let mut transaction = self.manager.begin_transaction().await?;

            match operation(&mut transaction) {
                Ok(result) => {
                    match self.manager.commit_transaction(transaction).await {
                        Ok(()) => return Ok(result),
                        Err(e) => {
                            last_error = Some(e);
                            attempts += 1;

                            // If it's a deadlock and we should retry, continue
                            if self.config.retry_on_deadlock
                                && attempts < self.config.max_retry_attempts
                            {
                                // Exponential backoff
                                let delay =
                                    Duration::from_millis(100 * (2_u64.pow(attempts as u32)));
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = self.manager.rollback_transaction(transaction).await;
                    last_error = Some(e);
                    attempts += 1;

                    // If it's a deadlock and we should retry, continue
                    if self.config.retry_on_deadlock && attempts < self.config.max_retry_attempts {
                        let delay = Duration::from_millis(100 * (2_u64.pow(attempts as u32)));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                }
            }

            break;
        }

        Err(
            last_error.unwrap_or_else(|| TransactionError::CommitFailed {
                message: "Max retry attempts exceeded".to_string(),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::time::Duration;

    async fn create_test_pool() -> Pool<Sqlite> {
        SqlitePool::connect(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_transaction_manager_creation() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool);

        let stats = manager.get_transaction_stats().await.unwrap();
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.active_transactions, 0);
    }

    #[tokio::test]
    async fn test_transaction_lifecycle() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool);

        // Begin transaction
        let transaction = manager.begin_transaction().await.unwrap();
        assert!(transaction.is_active());
        assert!(!transaction.is_timed_out());

        let stats = manager.get_transaction_stats().await.unwrap();
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.active_transactions, 1);

        // Commit transaction
        manager.commit_transaction(transaction).await.unwrap();

        let stats = manager.get_transaction_stats().await.unwrap();
        assert_eq!(stats.committed_transactions, 1);
        assert_eq!(stats.active_transactions, 0);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool);

        let transaction = manager.begin_transaction().await.unwrap();
        manager.rollback_transaction(transaction).await.unwrap();

        let stats = manager.get_transaction_stats().await.unwrap();
        assert_eq!(stats.rolled_back_transactions, 1);
        assert_eq!(stats.active_transactions, 0);
    }

    #[tokio::test]
    async fn test_transaction_timeout_cleanup() {
        let pool = Arc::new(create_test_pool().await);
        let mut config = TransactionConfig::default();
        config.default_timeout = Duration::from_millis(1); // Very short timeout

        let manager = DatabaseTransactionManager::new(pool, config);

        let _transaction = manager.begin_transaction().await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        let cleaned = manager.cleanup_timeout_transactions().await.unwrap();
        assert_eq!(cleaned, 1);
    }

    #[tokio::test]
    async fn test_transaction_executor() {
        let pool = Arc::new(create_test_pool().await);
        let manager = Arc::new(DatabaseTransactionManager::with_defaults(pool));
        let executor = TransactionExecutor::new(manager, TransactionConfig::default());

        let result = executor.execute_with_retry(|_tx| Ok(42)).await.unwrap();

        assert_eq!(result, 42);
    }
}
