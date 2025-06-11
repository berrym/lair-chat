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
        // Ensure we're not already authenticated
        if self.is_authenticated().await {
            return Err(AuthError::InternalError("Already authenticated".into()));
        }

        // Update state to authenticating
        {
            let mut state = self.state.write().await;
            *state = AuthState::Authenticating;
        }

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
                    let auth_response = AuthProtocol::decode_response(&response)?;

                    // Handle response
                    match auth_response.into_session_and_profile() {
                        Ok((session, profile)) => {
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
                        }
                        Err(e) => {
                            // Update state to failed
                            let mut state = self.state.write().await;
                            *state = AuthState::Failed {
                                reason: e.to_string(),
                            };
                            Err(e)
                        }
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
        // Ensure we're not already authenticated
        if self.is_authenticated().await {
            return Err(AuthError::InternalError("Already authenticated".into()));
        }

        // Update state to authenticating
        {
            let mut state = self.state.write().await;
            *state = AuthState::Authenticating;
        }

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
                    // Try to parse as JSON auth response first
                    match AuthProtocol::decode_response(&response) {
                        Ok(auth_response) => {
                            // Handle structured JSON response
                            match auth_response.into_session_and_profile() {
                                Ok((session, profile)) => {
                                    // Update state to authenticated
                                    let mut state = self.state.write().await;
                                    *state = AuthState::Authenticated { session, profile };
                                    Ok(())
                                }
                                Err(e) => {
                                    // Update state to failed
                                    let mut state = self.state.write().await;
                                    *state = AuthState::Failed {
                                        reason: e.to_string(),
                                    };
                                    Err(e)
                                }
                            }
                        }
                        Err(_) => {
                            // Response is not JSON, check if it's a success message
                            if response.contains("Welcome") || response.contains("successful") {
                                // Server sent plain text success message, create default session
                                use std::time::{SystemTime, UNIX_EPOCH};
                                use uuid::Uuid;

                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();

                                let session = crate::auth::Session {
                                    id: Uuid::new_v4(),
                                    token: format!("session_{}", now),
                                    created_at: now,
                                    expires_at: now + 3600, // 1 hour
                                };

                                let profile = crate::auth::UserProfile {
                                    id: Uuid::new_v4(),
                                    username: credentials.username.clone(),
                                    roles: vec!["user".to_string()],
                                };

                                // Update state to authenticated
                                let mut state = self.state.write().await;
                                *state = AuthState::Authenticated { session, profile };
                                Ok(())
                            } else {
                                // Treat other non-JSON responses as errors
                                let error = AuthError::ProtocolError(format!(
                                    "Authentication failed: {}",
                                    response
                                ));
                                let mut state = self.state.write().await;
                                *state = AuthState::Failed {
                                    reason: error.to_string(),
                                };
                                Err(error)
                            }
                        }
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

        // Send logout request
        let request = AuthRequest::logout(token);
        let encoded = AuthProtocol::encode_request(&request)?;

        {
            let mut transport = self.transport.lock().await;
            if let Err(e) = transport.send(&encoded).await {
                return Err(AuthError::ConnectionError(e.to_string()));
            }
        }

        // Clear authentication state
        let mut state = self.state.write().await;
        *state = AuthState::Unauthenticated;
        Ok(())
    }

    /// Refresh the current session
    pub async fn refresh_session(&self) -> AuthResult<()> {
        // Get current session token
        let token = match self.get_session().await {
            Some(session) => session.token,
            None => return Err(AuthError::SessionExpired),
        };

        // Send refresh request
        let request = AuthRequest::refresh(token);
        let encoded = AuthProtocol::encode_request(&request)?;

        {
            let mut transport = self.transport.lock().await;
            if let Err(e) = transport.send(&encoded).await {
                return Err(AuthError::ConnectionError(e.to_string()));
            }

            // Wait for response
            match transport.receive().await {
                Ok(Some(response)) => {
                    let auth_response = AuthProtocol::decode_response(&response)?;

                    // Handle response
                    match auth_response.into_session_and_profile() {
                        Ok((session, profile)) => {
                            // Update state with new session
                            let mut state = self.state.write().await;
                            *state = AuthState::Authenticated { session, profile };
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                Ok(None) => Err(AuthError::ConnectionError("No response received".into())),
                Err(e) => Err(AuthError::ConnectionError(e.to_string())),
            }
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
