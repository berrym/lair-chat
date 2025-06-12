//! Authentication manager for Lair-Chat client
//! Manages authentication state and coordinates authentication operations.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::protocol::{AuthProtocol, AuthRequest};
use super::storage::{StoredAuth, TokenStorage};
use super::types::{AuthError, AuthResult, AuthState, Credentials, Session, UserProfile};
use crate::transport::Transport;

/// Manages client authentication state and operations
pub struct AuthManager {
    /// Current authentication state
    state: Arc<RwLock<AuthState>>,
    /// Transport for sending/receiving auth messages
    transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
    /// Token storage for persistence
    token_storage: Arc<Box<dyn TokenStorage>>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(
        transport: Arc<Mutex<Box<dyn Transport + Send + Sync>>>,
        token_storage: Box<dyn TokenStorage>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthState::Unauthenticated)),
            transport,
            token_storage: Arc::new(token_storage),
        }
    }

    /// Create a new authentication manager without transport (for testing or delayed initialization)
    pub fn new_without_transport(token_storage: Box<dyn TokenStorage>) -> Self {
        // Create a dummy transport that will panic if used
        let transport = Arc::new(Mutex::new(
            Box::new(DummyTransport) as Box<dyn Transport + Send + Sync>
        ));
        Self {
            state: Arc::new(RwLock::new(AuthState::Unauthenticated)),
            transport,
            token_storage: Arc::new(token_storage),
        }
    }

    /// Initialize the auth manager and attempt to restore previous session
    pub async fn initialize(&self) -> AuthResult<()> {
        if let Some(stored_auth) = self.token_storage.load_auth().await? {
            if !stored_auth.session.is_expired() {
                let mut state = self.state.write().await;
                *state = AuthState::Authenticated {
                    profile: stored_auth.profile,
                    session: stored_auth.session,
                };
                return Ok(());
            }
        }
        Ok(())
    }

    /// Get the current authentication state
    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }

    /// Check if currently authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.state.read().await.is_authenticated()
    }

    /// Get the current session if authenticated
    pub async fn get_session(&self) -> Option<Session> {
        self.state.read().await.session().cloned()
    }

    /// Get the current user profile if authenticated
    pub async fn get_profile(&self) -> Option<UserProfile> {
        self.state.read().await.profile().cloned()
    }

    /// Register a new user account
    pub async fn register(&self, credentials: Credentials) -> AuthResult<()> {
        // Clear any existing authentication state before registering
        if self.is_authenticated().await {
            let mut state = self.state.write().await;
            *state = AuthState::Unauthenticated;
        }

        // Update state to authenticating
        {
            let mut state = self.state.write().await;
            *state = AuthState::Authenticating;
        }

        // Clone credentials before moving into request
        let username = credentials.username.clone();

        // Create and send registration request
        let request = AuthRequest::register(credentials);
        let encoded = AuthProtocol::encode_request(&request)?;

        // Send request and await response
        {
            let mut transport = self.transport.lock().await;
            if let Err(e) = transport.send(&encoded).await {
                let error = AuthError::ConnectionError(e.to_string());
                let mut state = self.state.write().await;
                *state = AuthState::Failed {
                    reason: error.to_string(),
                };
                return Err(error);
            }

            // Wait for response
            match transport.receive().await {
                Ok(Some(response)) => {
                    // Check if response indicates success or failure
                    if response.starts_with("Welcome back,")
                        || response.contains("successful")
                        || response.contains("Welcome")
                    {
                        // Create successful auth state
                        let session = Session {
                            id: uuid::Uuid::new_v4(),
                            token: format!("session_{}", username),
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            expires_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                + 3600, // 1 hour expiration
                        };

                        let profile = UserProfile {
                            id: uuid::Uuid::new_v4(),
                            username: username.clone(),
                            roles: vec!["user".to_string()],
                        };

                        // Update state to authenticated and persist
                        {
                            let mut state = self.state.write().await;
                            *state = AuthState::Authenticated {
                                session: session.clone(),
                                profile: profile.clone(),
                            };
                        }

                        // Store authentication data
                        let stored_auth = StoredAuth {
                            profile,
                            session,
                            stored_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        self.token_storage.save_auth(stored_auth).await?;
                        Ok(())
                    } else {
                        // Response indicates failure
                        let error_msg = if response.starts_with("Authentication failed:") {
                            response
                                .trim_start_matches("Authentication failed:")
                                .trim()
                                .to_string()
                        } else {
                            response
                        };

                        let error = AuthError::AuthenticationFailed(error_msg);
                        let mut state = self.state.write().await;
                        *state = AuthState::Failed {
                            reason: error.to_string(),
                        };
                        Err(error)
                    }
                }
                Ok(None) => {
                    let error = AuthError::ConnectionError("No response received".into());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
                Err(e) => {
                    let error = AuthError::ConnectionError(e.to_string());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
            }
        }
    }

    /// Login with existing credentials
    pub async fn login(&self, credentials: Credentials) -> AuthResult<()> {
        // Clear any existing authentication state before logging in
        if self.is_authenticated().await {
            let mut state = self.state.write().await;
            *state = AuthState::Unauthenticated;
        }

        // Update state to authenticating
        {
            let mut state = self.state.write().await;
            *state = AuthState::Authenticating;
        }

        // Clone credentials before moving into request
        let username = credentials.username.clone();

        // Create and send login request
        let request = AuthRequest::login(credentials);
        let encoded = AuthProtocol::encode_request(&request)?;

        // Send request and await response
        {
            let mut transport = self.transport.lock().await;
            if let Err(e) = transport.send(&encoded).await {
                let error = AuthError::ConnectionError(e.to_string());
                let mut state = self.state.write().await;
                *state = AuthState::Failed {
                    reason: error.to_string(),
                };
                return Err(error);
            }

            // Wait for response
            match transport.receive().await {
                Ok(Some(response)) => {
                    // Check if response indicates success or failure
                    if response.starts_with("Welcome back,")
                        || response.contains("successful")
                        || response.contains("Welcome")
                    {
                        // Create successful auth state
                        let session = Session {
                            id: uuid::Uuid::new_v4(),
                            token: format!("session_{}", username),
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            expires_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                + 3600, // 1 hour expiration
                        };

                        let profile = UserProfile {
                            id: uuid::Uuid::new_v4(),
                            username: username.clone(),
                            roles: vec!["user".to_string()],
                        };

                        // Update state to authenticated and persist
                        {
                            let mut state = self.state.write().await;
                            *state = AuthState::Authenticated {
                                session: session.clone(),
                                profile: profile.clone(),
                            };
                        }

                        // Store authentication data
                        let stored_auth = StoredAuth {
                            profile,
                            session,
                            stored_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        self.token_storage.save_auth(stored_auth).await?;
                        Ok(())
                    } else {
                        // Response indicates failure
                        let error_msg = if response.starts_with("Authentication failed:") {
                            response
                                .trim_start_matches("Authentication failed:")
                                .trim()
                                .to_string()
                        } else {
                            response
                        };

                        let error = AuthError::AuthenticationFailed(error_msg);
                        let mut state = self.state.write().await;
                        *state = AuthState::Failed {
                            reason: error.to_string(),
                        };
                        Err(error)
                    }
                }
                Ok(None) => {
                    let error = AuthError::ConnectionError("No response received".into());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
                Err(e) => {
                    let error = AuthError::ConnectionError(e.to_string());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
            }
        }
    }

    /// Logout and clear session
    pub async fn logout(&self) -> AuthResult<()> {
        // Get current session token
        let token = match self.get_session().await {
            Some(session) => session.token,
            None => return Ok(()),
        };

        // Clear stored authentication
        self.token_storage.clear_auth().await?;

        // Clear authentication state (server doesn't support logout protocol)
        let mut state = self.state.write().await;
        *state = AuthState::Unauthenticated;
        Ok(())
    }

    /// Refresh the current session
    pub async fn refresh_session(&self) -> AuthResult<()> {
        // Check if current session is still valid
        match self.get_session().await {
            Some(session) => {
                // Check if session is expired
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if session.expires_at > now {
                    // Session is still valid
                    Ok(())
                } else {
                    // Session expired, clear state
                    let mut state = self.state.write().await;
                    *state = AuthState::Unauthenticated;
                    Err(AuthError::SessionExpired)
                }
            }
            None => Err(AuthError::SessionExpired),
        }
    }
}

/// Dummy transport implementation for AuthManager that doesn't need transport
struct DummyTransport;

#[async_trait::async_trait]
impl Transport for DummyTransport {
    async fn connect(&mut self) -> Result<(), crate::transport::TransportError> {
        Err(crate::transport::TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::NotConnected, "No transport configured"),
        ))
    }

    async fn send(&mut self, _data: &str) -> Result<(), crate::transport::TransportError> {
        Err(crate::transport::TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::NotConnected, "No transport configured"),
        ))
    }

    async fn receive(&mut self) -> Result<Option<String>, crate::transport::TransportError> {
        Err(crate::transport::TransportError::ConnectionError(
            std::io::Error::new(std::io::ErrorKind::NotConnected, "No transport configured"),
        ))
    }

    async fn close(&mut self) -> Result<(), crate::transport::TransportError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::storage::MemoryTokenStorage;
    use uuid::Uuid;

    async fn setup_test_manager() -> AuthManager {
        let storage = Box::new(MemoryTokenStorage::new());
        AuthManager::new_without_transport(storage)
    }

    #[tokio::test]
    async fn test_basic_auth_manager() {
        let manager = setup_test_manager().await;

        // Test initial state
        assert!(matches!(
            manager.get_state().await,
            AuthState::Unauthenticated
        ));

        // Test that we can check authentication status
        assert!(!manager.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_auth_manager_initialization() {
        let manager = setup_test_manager().await;

        // Test that manager starts in unauthenticated state
        assert!(matches!(
            manager.get_state().await,
            AuthState::Unauthenticated
        ));

        // Test that we can create a manager without errors
        assert!(!manager.is_authenticated().await);
        assert!(!manager.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_logout() {
        let manager = setup_test_manager().await;

        // Set initial authenticated state
        {
            let mut state = manager.state.write().await;
            *state = AuthState::Authenticated {
                profile: UserProfile {
                    id: Uuid::new_v4(),
                    username: "testuser".to_string(),
                    roles: vec!["user".to_string()],
                },
                session: Session {
                    id: Uuid::new_v4(),
                    token: "session123".to_string(),
                    created_at: 0,
                    expires_at: u64::MAX,
                },
            };
        }

        assert!(manager.is_authenticated().await);
        assert!(manager.logout().await.is_ok());
        assert!(!manager.is_authenticated().await);
    }
}
