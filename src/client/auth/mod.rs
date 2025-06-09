//! Client authentication module for Lair-Chat
//! Provides client-side authentication functionality and state management.

mod types;
mod protocol;
mod manager;

pub use types::{
    AuthError,
    AuthResult,
    AuthState,
    Credentials,
    Session,
    UserProfile,
};

pub use protocol::{
    AuthProtocol,
    AuthRequest,
    AuthResponse,
};

pub use manager::AuthManager;

/// Default session refresh interval in seconds
pub const DEFAULT_REFRESH_INTERVAL: u64 = 300; // 5 minutes

/// Default session expiry buffer in seconds
/// Session refresh will be attempted when remaining time is less than this
pub const DEFAULT_EXPIRY_BUFFER: u64 = 600; // 10 minutes

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    /// Create a test user profile
    pub fn create_test_profile() -> UserProfile {
        UserProfile {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            roles: vec!["user".to_string()],
        }
    }

    /// Create a test session
    pub fn create_test_session() -> Session {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Session {
            id: Uuid::new_v4(),
            token: "test_session_token".to_string(),
            created_at: now,
            expires_at: now + 3600,
        }
    }

    /// Create test credentials
    pub fn create_test_credentials() -> Credentials {
        Credentials {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        }
    }

    /// Create authenticated state for testing
    pub fn create_authenticated_state() -> AuthState {
        AuthState::Authenticated {
            profile: create_test_profile(),
            session: create_test_session(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::testing::*;

    #[test]
    fn test_auth_test_utilities() {
        let profile = create_test_profile();
        assert_eq!(profile.username, "testuser");
        assert!(profile.roles.contains(&"user".to_string()));

        let session = create_test_session();
        assert!(!session.is_expired());
        assert!(!session.token.is_empty());

        let credentials = create_test_credentials();
        assert_eq!(credentials.username, "testuser");
        assert_eq!(credentials.password, "password123");

        let state = create_authenticated_state();
        assert!(matches!(state, AuthState::Authenticated { .. }));
    }
}