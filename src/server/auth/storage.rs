//! User storage interface and implementations for Lair-Chat
//! Provides persistent storage for user data and sessions.

use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::{AuthError, AuthResult, Session, User};

/// Interface for user data storage
#[async_trait::async_trait]
pub trait UserStorage: Send + Sync {
    /// Create a new user
    async fn create_user(&self, user: User) -> AuthResult<User>;

    /// Get user by ID
    async fn get_user(&self, id: Uuid) -> AuthResult<User>;

    /// Get user by username
    async fn get_user_by_username(&self, username: &str) -> AuthResult<User>;

    /// Update user data
    async fn update_user(&self, user: &User) -> AuthResult<()>;

    /// Delete user
    async fn delete_user(&self, id: Uuid) -> AuthResult<()>;

    /// List all users
    async fn list_users(&self) -> AuthResult<Vec<User>>;
}

/// Interface for session storage
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    /// Store a new session
    async fn create_session(&self, session: Session) -> AuthResult<Session>;

    /// Get session by ID
    async fn get_session(&self, id: Uuid) -> AuthResult<Session>;

    /// Get session by token
    async fn get_session_by_token(&self, token: &str) -> AuthResult<Session>;

    /// Update session data
    async fn update_session(&self, session: &Session) -> AuthResult<()>;

    /// Delete session
    async fn delete_session(&self, id: Uuid) -> AuthResult<()>;

    /// Delete all sessions for a user
    async fn delete_user_sessions(&self, user_id: Uuid) -> AuthResult<()>;

    /// Clean up expired sessions
    async fn cleanup_expired(&self) -> AuthResult<u64>;
}

/// In-memory implementation of UserStorage
pub struct MemoryUserStorage {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    usernames: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl MemoryUserStorage {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            usernames: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl UserStorage for MemoryUserStorage {
    async fn create_user(&self, user: User) -> AuthResult<User> {
        let mut users = self.users.write().await;
        let mut usernames = self.usernames.write().await;

        // Check if username is taken
        if usernames.contains_key(&user.username) {
            return Err(AuthError::UsernameTaken);
        }

        usernames.insert(user.username.clone(), user.id);
        users.insert(user.id, user.clone());

        Ok(user)
    }

    async fn get_user(&self, id: Uuid) -> AuthResult<User> {
        let users = self.users.read().await;
        users.get(&id).cloned().ok_or(AuthError::UserNotFound)
    }

    async fn get_user_by_username(&self, username: &str) -> AuthResult<User> {
        let usernames = self.usernames.read().await;
        let users = self.users.read().await;

        let user_id = usernames.get(username).ok_or(AuthError::UserNotFound)?;

        users.get(user_id).cloned().ok_or(AuthError::UserNotFound)
    }

    async fn update_user(&self, user: &User) -> AuthResult<()> {
        let mut users = self.users.write().await;

        if users.contains_key(&user.id) {
            users.insert(user.id, user.clone());
            Ok(())
        } else {
            Err(AuthError::UserNotFound)
        }
    }

    async fn delete_user(&self, id: Uuid) -> AuthResult<()> {
        let mut users = self.users.write().await;
        let mut usernames = self.usernames.write().await;

        if let Some(user) = users.remove(&id) {
            usernames.remove(&user.username);
            Ok(())
        } else {
            Err(AuthError::UserNotFound)
        }
    }

    async fn list_users(&self) -> AuthResult<Vec<User>> {
        let users = self.users.read().await;
        Ok(users.values().cloned().collect())
    }
}

/// In-memory implementation of SessionStorage
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    tokens: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl MemorySessionStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn create_session(&self, session: Session) -> AuthResult<Session> {
        let mut sessions = self.sessions.write().await;
        let mut tokens = self.tokens.write().await;

        tokens.insert(session.token.clone(), session.id);
        sessions.insert(session.id, session.clone());

        Ok(session)
    }

    async fn get_session(&self, id: Uuid) -> AuthResult<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(&id).cloned().ok_or(AuthError::InvalidToken)
    }

    async fn get_session_by_token(&self, token: &str) -> AuthResult<Session> {
        let tokens = self.tokens.read().await;
        let sessions = self.sessions.read().await;

        let session_id = tokens.get(token).ok_or(AuthError::InvalidToken)?;

        sessions
            .get(session_id)
            .cloned()
            .ok_or(AuthError::InvalidToken)
    }

    async fn update_session(&self, session: &Session) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;

        if sessions.contains_key(&session.id) {
            sessions.insert(session.id, session.clone());
            Ok(())
        } else {
            Err(AuthError::InvalidToken)
        }
    }

    async fn delete_session(&self, id: Uuid) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        let mut tokens = self.tokens.write().await;

        if let Some(session) = sessions.remove(&id) {
            tokens.remove(&session.token);
            Ok(())
        } else {
            Err(AuthError::InvalidToken)
        }
    }

    async fn delete_user_sessions(&self, user_id: Uuid) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        let mut tokens = self.tokens.write().await;

        // Find all sessions for the user
        let session_ids: Vec<_> = sessions
            .iter()
            .filter(|(_, session)| session.user_id == user_id)
            .map(|(id, session)| (*id, session.token.clone()))
            .collect();

        // Remove the sessions and their tokens
        for (id, token) in session_ids {
            sessions.remove(&id);
            tokens.remove(&token);
        }

        Ok(())
    }

    async fn cleanup_expired(&self) -> AuthResult<u64> {
        let mut sessions = self.sessions.write().await;
        let mut tokens = self.tokens.write().await;

        let expired: Vec<_> = sessions
            .iter()
            .filter(|(_, session)| session.is_expired())
            .map(|(id, session)| (*id, session.token.clone()))
            .collect();

        for (id, token) in expired.iter() {
            sessions.remove(id);
            tokens.remove(token);
        }

        Ok(expired.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::types::User;
    use super::*;

    #[tokio::test]
    async fn test_user_storage() {
        let storage = MemoryUserStorage::new();

        // Create a test user
        let user = User::new("testuser".to_string(), "password123").unwrap();
        let created = storage.create_user(user.clone()).await.unwrap();

        // Verify user retrieval
        let retrieved = storage.get_user(created.id).await.unwrap();
        assert_eq!(created.id, retrieved.id);
        assert_eq!(created.username, retrieved.username);

        // Verify username lookup
        let by_username = storage.get_user_by_username("testuser").await.unwrap();
        assert_eq!(created.id, by_username.id);

        // Test duplicate username
        let duplicate = User::new("testuser".to_string(), "password456").unwrap();
        assert!(storage.create_user(duplicate).await.is_err());

        // Test user deletion
        storage.delete_user(created.id).await.unwrap();
        assert!(storage.get_user(created.id).await.is_err());
    }

    #[tokio::test]
    async fn test_session_storage() {
        let storage = MemorySessionStorage::new();

        // Create a test session
        let user_id = Uuid::new_v4();
        let session = Session::new(user_id, "test_fingerprint".to_string());
        let created = storage.create_session(session.clone()).await.unwrap();

        // Verify session retrieval
        let retrieved = storage.get_session(created.id).await.unwrap();
        assert_eq!(created.id, retrieved.id);

        // Verify token lookup
        let by_token = storage.get_session_by_token(&created.token).await.unwrap();
        assert_eq!(created.id, by_token.id);

        // Test session deletion
        storage.delete_session(created.id).await.unwrap();
        assert!(storage.get_session(created.id).await.is_err());

        // Test user sessions cleanup
        let session1 = storage
            .create_session(Session::new(user_id, "fp1".to_string()))
            .await
            .unwrap();
        let session2 = storage
            .create_session(Session::new(user_id, "fp2".to_string()))
            .await
            .unwrap();

        storage.delete_user_sessions(user_id).await.unwrap();
        assert!(storage.get_session(session1.id).await.is_err());
        assert!(storage.get_session(session2.id).await.is_err());
    }
}
