//! Core authentication types and traits for Lair-Chat
//! This module defines the fundamental types used in the authentication system.

use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use thiserror::Error;

/// Authentication-related errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("Username already taken")]
    UsernameTaken,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Password hashing error: {0}")]
    HashingError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

/// Represents a user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// Username (unique)
    pub username: String,
    
    /// Hashed password using Argon2id
    #[serde(skip_serializing)]
    pub password_hash: String,
    
    /// User's roles/permissions
    pub roles: Vec<Role>,
    
    /// Account creation timestamp
    pub created_at: u64,
    
    /// Last login timestamp
    pub last_login: u64,
    
    /// Account status
    pub status: UserStatus,
}

impl User {
    /// Create a new user with the given username and password
    pub fn new(username: String, password: &str) -> AuthResult<Self> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        // Hash the password
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::HashingError(e.to_string()))?
            .to_string();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Ok(Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            roles: vec![Role::User],
            created_at: now,
            last_login: now,
            status: UserStatus::Active,
        })
    }
    
    /// Verify a password against this user's stored hash
    pub fn verify_password(&self, password: &str) -> AuthResult<bool> {
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| AuthError::HashingError(e.to_string()))?;
            
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
    
    /// Update the user's password
    pub fn update_password(&mut self, password: &str) -> AuthResult<()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        self.password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::HashingError(e.to_string()))?
            .to_string();
            
        Ok(())
    }
}

/// User account status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Locked,
}

/// User roles for permission management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Moderator,
    User,
    Guest,
}

/// Session information for authenticated users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,
    
    /// Associated user ID
    pub user_id: Uuid,
    
    /// Session creation timestamp
    pub created_at: u64,
    
    /// Session expiration timestamp
    pub expires_at: u64,
    
    /// Session token for authentication
    pub token: String,
    
    /// Client fingerprint for security
    pub fingerprint: String,
}

impl Session {
    /// Create a new session for a user
    pub fn new(user_id: Uuid, fingerprint: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Self {
            id: Uuid::new_v4(),
            user_id,
            created_at: now,
            // Set expiration to 24 hours from creation
            expires_at: now + 86400,
            token: Uuid::new_v4().to_string(),
            fingerprint,
        }
    }
    
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.expires_at < now
    }
    
    /// Extend the session duration
    pub fn extend(&mut self, duration_secs: u64) {
        self.expires_at += duration_secs;
    }
}

/// Authentication request for login
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub fingerprint: String,
}

/// Authentication response containing session information
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub session: Session,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_password_verification() {
        let password = "secure_password_123";
        let user = User::new("testuser".to_string(), password).unwrap();
        
        assert!(user.verify_password(password).unwrap());
        assert!(!user.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_user_password_update() {
        let mut user = User::new("testuser".to_string(), "old_password").unwrap();
        let new_password = "new_secure_password";
        
        user.update_password(new_password).unwrap();
        
        assert!(user.verify_password(new_password).unwrap());
        assert!(!user.verify_password("old_password").unwrap());
    }

    #[test]
    fn test_session_expiration() {
        let session = Session::new(Uuid::new_v4(), "test_fingerprint".to_string());
        assert!(!session.is_expired());
        
        let mut expired_session = session;
        expired_session.expires_at = 0;
        assert!(expired_session.is_expired());
    }

    #[test]
    fn test_session_extension() {
        let mut session = Session::new(Uuid::new_v4(), "test_fingerprint".to_string());
        let original_expiry = session.expires_at;
        
        session.extend(3600); // Extend by 1 hour
        assert_eq!(session.expires_at, original_expiry + 3600);
    }
}