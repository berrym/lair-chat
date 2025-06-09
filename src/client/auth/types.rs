//! Client-side authentication types for Lair-Chat
//! Handles user authentication state and credentials management.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use thiserror::Error;

/// Authentication-related errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

/// User credentials for authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// User session information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub token: String,
    pub created_at: u64,
    pub expires_at: u64,
}

impl Session {
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.expires_at < now
    }
    
    /// Time until session expiration in seconds
    pub fn time_until_expiry(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.expires_at as i64 - now as i64
    }
}

/// User profile information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub roles: Vec<String>,
}

/// Authentication state
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuthState {
    /// Not authenticated
    Unauthenticated,
    
    /// Authentication in progress
    Authenticating,
    
    /// Successfully authenticated
    Authenticated {
        profile: UserProfile,
        session: Session,
    },
    
    /// Authentication failed
    Failed {
        reason: String,
    },
}

impl AuthState {
    /// Check if currently authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self, AuthState::Authenticated { .. })
    }
    
    /// Get the current session if authenticated
    pub fn session(&self) -> Option<&Session> {
        match self {
            AuthState::Authenticated { session, .. } => Some(session),
            _ => None,
        }
    }
    
    /// Get the user profile if authenticated
    pub fn profile(&self) -> Option<&UserProfile> {
        match self {
            AuthState::Authenticated { profile, .. } => Some(profile),
            _ => None,
        }
    }
}

/// Client device information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub fingerprint: String,
    pub name: String,
    pub os: String,
    pub version: String,
}

impl DeviceInfo {
    /// Create device info for the current system
    pub fn current() -> Self {
        Self {
            fingerprint: uuid::Uuid::new_v4().to_string(),
            name: std::env::var("HOSTNAME")
                .unwrap_or_else(|_| "unknown".to_string()),
            os: std::env::consts::OS.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_expiry() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let expired_session = Session {
            id: Uuid::new_v4(),
            token: "test".to_string(),
            created_at: now - 3600,
            expires_at: now - 1800,
        };
        
        assert!(expired_session.is_expired());
        assert!(expired_session.time_until_expiry() < 0);
        
        let valid_session = Session {
            id: Uuid::new_v4(),
            token: "test".to_string(),
            created_at: now,
            expires_at: now + 3600,
        };
        
        assert!(!valid_session.is_expired());
        assert!(valid_session.time_until_expiry() > 0);
    }
    
    #[test]
    fn test_auth_state() {
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            roles: vec!["user".to_string()],
        };
        
        let session = Session {
            id: Uuid::new_v4(),
            token: "test".to_string(),
            created_at: 0,
            expires_at: u64::MAX,
        };
        
        let state = AuthState::Authenticated {
            profile: profile.clone(),
            session: session.clone(),
        };
        
        assert!(state.is_authenticated());
        assert_eq!(state.session().unwrap().token, session.token);
        assert_eq!(state.profile().unwrap().username, profile.username);
        
        let unauthenticated = AuthState::Unauthenticated;
        assert!(!unauthenticated.is_authenticated());
        assert!(unauthenticated.session().is_none());
        assert!(unauthenticated.profile().is_none());
    }
    
    #[test]
    fn test_device_info() {
        let device = DeviceInfo::current();
        assert!(!device.fingerprint.is_empty());
        assert!(!device.os.is_empty());
        assert!(!device.version.is_empty());
    }
}