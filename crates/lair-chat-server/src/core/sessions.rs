//! Session management - lifecycle and validation of user sessions.
//!
//! This service handles:
//! - Session validation
//! - Session refresh/touch
//! - Logout (session invalidation)
//! - Session cleanup (expired sessions)

use std::sync::Arc;

use crate::domain::{Session, SessionId, User, UserId};
use crate::storage::{SessionRepository, Storage, UserRepository};
use crate::{Error, Result};

// ============================================================================
// SessionManager
// ============================================================================

/// Service for session lifecycle management.
pub struct SessionManager<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage + 'static> SessionManager<S> {
    /// Create a new session manager.
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Validate a session and return the session with its associated user.
    ///
    /// # Errors
    ///
    /// - `SessionNotFound` - Session doesn't exist
    /// - `SessionExpired` - Session has expired
    /// - `UserNotFound` - Associated user doesn't exist (shouldn't happen)
    pub async fn validate(&self, session_id: SessionId) -> Result<(Session, User)> {
        // Find session
        let session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        // Check expiration
        if session.is_expired() {
            // Clean up expired session
            let _ = SessionRepository::delete(&*self.storage, session_id).await;
            return Err(Error::SessionExpired);
        }

        // Find associated user
        let user = UserRepository::find_by_id(&*self.storage, session.user_id)
            .await?
            .ok_or_else(|| Error::Internal("session references nonexistent user".into()))?;

        Ok((session, user))
    }

    /// Check if a session is valid without returning user data.
    pub async fn is_valid(&self, session_id: SessionId) -> Result<bool> {
        SessionRepository::is_valid(&*self.storage, session_id).await
    }

    /// End a session (logout).
    pub async fn logout(&self, session_id: SessionId) -> Result<()> {
        // Verify session exists
        let session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        // Delete the session
        SessionRepository::delete(&*self.storage, session.id).await?;

        Ok(())
    }

    /// Update a session's last activity timestamp.
    pub async fn touch(&self, session_id: SessionId) -> Result<()> {
        let mut session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        session.touch();
        SessionRepository::update(&*self.storage, &session).await?;

        Ok(())
    }

    /// Refresh a session, extending its expiration.
    pub async fn refresh(&self, session_id: SessionId) -> Result<Session> {
        let mut session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        if session.is_expired() {
            return Err(Error::SessionExpired);
        }

        session.refresh();
        SessionRepository::update(&*self.storage, &session).await?;

        Ok(session)
    }

    /// List all active sessions for a user.
    pub async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Session>> {
        SessionRepository::list_by_user(&*self.storage, user_id).await
    }

    /// Count active sessions for a user.
    pub async fn count_for_user(&self, user_id: UserId) -> Result<u32> {
        SessionRepository::count_by_user(&*self.storage, user_id).await
    }

    /// Logout all sessions for a user (e.g., on password change).
    pub async fn logout_all(&self, user_id: UserId) -> Result<u64> {
        SessionRepository::delete_by_user(&*self.storage, user_id).await
    }

    /// Clean up expired sessions (maintenance task).
    pub async fn cleanup_expired(&self) -> Result<u64> {
        SessionRepository::delete_expired(&*self.storage).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::Protocol;
    use crate::domain::user::{Email, Role, Username};
    use crate::storage::sqlite::SqliteStorage;

    async fn create_test_storage() -> Arc<SqliteStorage> {
        Arc::new(SqliteStorage::in_memory().await.unwrap())
    }

    async fn create_test_user(storage: &SqliteStorage) -> User {
        let username = Username::new("testuser").unwrap();
        let email = Email::new("test@example.com").unwrap();
        let user = User::new(username, email, Role::User);
        UserRepository::create(storage, &user, "hashed_password").await.unwrap();
        user
    }

    async fn create_test_session(storage: &SqliteStorage, user_id: UserId) -> Session {
        let session = Session::new(user_id, Protocol::Tcp);
        SessionRepository::create(storage, &session).await.unwrap();
        session
    }

    #[tokio::test]
    async fn test_session_manager_new() {
        let storage = create_test_storage().await;
        let _manager = SessionManager::new(storage);
    }

    #[tokio::test]
    async fn test_validate_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let session = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage);

        let (validated_session, validated_user) = manager.validate(session.id).await.unwrap();

        assert_eq!(validated_session.id, session.id);
        assert_eq!(validated_user.id, user.id);
    }

    #[tokio::test]
    async fn test_validate_session_not_found() {
        let storage = create_test_storage().await;
        let manager = SessionManager::new(storage);

        let result = manager.validate(SessionId::new()).await;

        assert!(matches!(result, Err(Error::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_validate_session_expired() {
        use chrono::{Duration, Utc};

        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;

        // Create an expired session manually
        let mut session = Session::new(user.id, Protocol::Tcp);
        session.expires_at = Utc::now() - Duration::hours(1);
        SessionRepository::create(&*storage, &session).await.unwrap();

        let manager = SessionManager::new(storage);

        let result = manager.validate(session.id).await;

        assert!(matches!(result, Err(Error::SessionExpired)));
    }

    #[tokio::test]
    async fn test_is_valid_true() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let session = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage);

        let is_valid = manager.is_valid(session.id).await.unwrap();

        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_is_valid_false_not_found() {
        let storage = create_test_storage().await;
        let manager = SessionManager::new(storage);

        let is_valid = manager.is_valid(SessionId::new()).await.unwrap();

        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_logout_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let session = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage.clone());

        manager.logout(session.id).await.unwrap();

        // Verify session is deleted
        let found = SessionRepository::find_by_id(&*storage, session.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_logout_session_not_found() {
        let storage = create_test_storage().await;
        let manager = SessionManager::new(storage);

        let result = manager.logout(SessionId::new()).await;

        assert!(matches!(result, Err(Error::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_touch_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let session = create_test_session(&storage, user.id).await;

        let manager = SessionManager::new(storage.clone());

        // Touch should succeed without error
        let result = manager.touch(session.id).await;
        assert!(result.is_ok());

        // Verify session still exists and is valid
        let updated = SessionRepository::find_by_id(&*storage, session.id)
            .await
            .unwrap();
        assert!(updated.is_some());
    }

    #[tokio::test]
    async fn test_touch_session_not_found() {
        let storage = create_test_storage().await;
        let manager = SessionManager::new(storage);

        let result = manager.touch(SessionId::new()).await;

        assert!(matches!(result, Err(Error::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_refresh_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let session = create_test_session(&storage, user.id).await;
        let original_expiry = session.expires_at;

        let manager = SessionManager::new(storage);
        let refreshed = manager.refresh(session.id).await.unwrap();

        assert!(refreshed.expires_at >= original_expiry);
    }

    #[tokio::test]
    async fn test_refresh_session_not_found() {
        let storage = create_test_storage().await;
        let manager = SessionManager::new(storage);

        let result = manager.refresh(SessionId::new()).await;

        assert!(matches!(result, Err(Error::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_refresh_session_expired() {
        use chrono::{Duration, Utc};

        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;

        // Create an expired session manually
        let mut session = Session::new(user.id, Protocol::Tcp);
        session.expires_at = Utc::now() - Duration::hours(1);
        SessionRepository::create(&*storage, &session).await.unwrap();

        let manager = SessionManager::new(storage);

        let result = manager.refresh(session.id).await;

        assert!(matches!(result, Err(Error::SessionExpired)));
    }

    #[tokio::test]
    async fn test_list_for_user() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let _session1 = create_test_session(&storage, user.id).await;
        let _session2 = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage);

        let sessions = manager.list_for_user(user.id).await.unwrap();

        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_list_for_user_empty() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let manager = SessionManager::new(storage);

        let sessions = manager.list_for_user(user.id).await.unwrap();

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_count_for_user() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let _session1 = create_test_session(&storage, user.id).await;
        let _session2 = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage);

        let count = manager.count_for_user(user.id).await.unwrap();

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_logout_all() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;
        let _session1 = create_test_session(&storage, user.id).await;
        let _session2 = create_test_session(&storage, user.id).await;
        let manager = SessionManager::new(storage.clone());

        let deleted = manager.logout_all(user.id).await.unwrap();

        assert_eq!(deleted, 2);

        // Verify sessions are deleted
        let sessions = SessionRepository::list_by_user(&*storage, user.id).await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        use chrono::{Duration, Utc};

        let storage = create_test_storage().await;
        let user = create_test_user(&storage).await;

        // Create a valid session
        let _valid_session = create_test_session(&storage, user.id).await;

        // Create an expired session
        let mut expired_session = Session::new(user.id, Protocol::Tcp);
        expired_session.expires_at = Utc::now() - Duration::hours(1);
        SessionRepository::create(&*storage, &expired_session).await.unwrap();

        let manager = SessionManager::new(storage.clone());
        let deleted = manager.cleanup_expired().await.unwrap();

        assert_eq!(deleted, 1);

        // Valid session should still exist
        let sessions = SessionRepository::list_by_user(&*storage, user.id).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }
}
