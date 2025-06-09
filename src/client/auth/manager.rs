//! Authentication manager for Lair-Chat client
//! Manages authentication state and coordinates authentication operations.

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::{
    AuthError, AuthResult, AuthState, Credentials,
    Session, UserProfile,
};
use super::protocol::{AuthProtocol, AuthRequest, AuthResponse};
use crate::client::transport::Transport;

/// Manages client authentication state and operations
pub struct AuthManager {
    /// Current authentication state
    state: Arc<RwLock<AuthState>>,
    /// Transport for sending/receiving auth messages
    transport: Arc<Box<dyn Transport>>,
    /// Token storage for persistence
    token_storage: Arc<Box<dyn TokenStorage>>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(transport: Arc<Box<dyn Transport>>, token_storage: Box<dyn TokenStorage>) -> Self {
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
        match self.transport.send(&encoded).await {
            Ok(_) => {
                // Wait for response
                if let Some(response) = self.transport.receive().await? {
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
                } else {
                    let error = AuthError::ConnectionError("No response received".into());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
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
        match self.transport.send(&encoded).await {
            Ok(_) => {
                // Wait for response
                if let Some(response) = self.transport.receive().await? {
                    let auth_response = AuthProtocol::decode_response(&response)?;
                    
                    // Handle response
                    match auth_response.into_session_and_profile() {
                        Ok((session, profile)) => {
                            // Update state to authenticated
                            let mut state = self.state.write().await;
                            *state = AuthState::Authenticated {
                                session,
                                profile,
                            };
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
                } else {
                    let error = AuthError::ConnectionError("No response received".into());
                    let mut state = self.state.write().await;
                    *state = AuthState::Failed {
                        reason: error.to_string(),
                    };
                    Err(error)
                }
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
        
        match self.transport.send(&encoded).await {
            Ok(_) => {
                // Clear authentication state
                let mut state = self.state.write().await;
                *state = AuthState::Unauthenticated;
                Ok(())
            }
            Err(e) => {
                Err(AuthError::ConnectionError(e.to_string()))
            }
        }
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
        
        match self.transport.send(&encoded).await {
            Ok(_) => {
                // Wait for response
                if let Some(response) = self.transport.receive().await? {
                    let auth_response = AuthProtocol::decode_response(&response)?;
                    
                    // Handle response
                    match auth_response.into_session_and_profile() {
                        Ok((session, profile)) => {
                            // Update state with new session
                            let mut state = self.state.write().await;
                            *state = AuthState::Authenticated {
                                session,
                                profile,
                            };
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Err(AuthError::ConnectionError("No response received".into()))
                }
            }
            Err(e) => Err(AuthError::ConnectionError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use crate::client::transport::MockTransport;
    use crate::client::auth::storage::MemoryTokenStorage;
    
    async fn setup_test_manager() -> (AuthManager, mpsc::UnboundedSender<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let transport = Arc::new(Box::new(MockTransport::new(rx)));
        let storage = Box::new(MemoryTokenStorage::new());
        let manager = AuthManager::new(transport, storage);
        (manager, tx)
    }
    
    #[tokio::test]
    async fn test_login_flow() {
        let (manager, tx) = setup_test_manager().await;
        
        // Set up mock response
        let success_response = r#"{
            "type": "success",
            "user_id": "123e4567-e89b-12d3-a456-426614174000",
            "username": "testuser",
            "roles": ["user"],
            "token": "session123",
            "expires_at": 1234567890
        }"#;
        
        // Spawn task to send mock response
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(success_response.to_string()).unwrap();
        });
        
        // Attempt login
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };
        
        assert!(manager.login(credentials).await.is_ok());
        assert!(manager.is_authenticated().await);
        
        let profile = manager.get_profile().await.unwrap();
        assert_eq!(profile.username, "testuser");
    }
    
    #[tokio::test]
    async fn test_failed_login() {
        let (manager, tx) = setup_test_manager().await;
        
        // Set up mock error response
        let error_response = r#"{
            "type": "error",
            "code": "AUTH001",
            "message": "Invalid credentials"
        }"#;
        
        // Spawn task to send mock response
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(error_response.to_string()).unwrap();
        });
        
        // Attempt login
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: "wrongpassword".to_string(),
        };
        
        assert!(manager.login(credentials).await.is_err());
        assert!(!manager.is_authenticated().await);
    }
    
    #[tokio::test]
    async fn test_logout() {
        let (manager, _) = setup_test_manager().await;
        
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