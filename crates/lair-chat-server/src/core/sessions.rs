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
