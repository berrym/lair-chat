//! Authentication service for Lair-Chat
//! Coordinates user authentication, session management, and rate limiting.

use super::storage::{SessionStorage, UserStorage};
use super::types::{AuthError, AuthRequest, AuthResponse, AuthResult, Session, User, UserStatus};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum login attempts per window
    pub max_attempts: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Lockout duration in seconds
    pub lockout_duration: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            window_seconds: 300,   // 5 minutes
            lockout_duration: 900, // 15 minutes
        }
    }
}

/// Tracks failed login attempts
#[derive(Debug, Clone)]
struct RateLimit {
    attempts: u32,
    window_start: u64,
    lockout_until: Option<u64>,
}

impl RateLimit {
    fn new() -> Self {
        Self {
            attempts: 0,
            window_start: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            lockout_until: None,
        }
    }

    fn check(&mut self, config: &RateLimitConfig) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check lockout
        if let Some(lockout_until) = self.lockout_until {
            if now < lockout_until {
                return false;
            }
            self.lockout_until = None;
        }

        // Reset window if expired
        if now - self.window_start >= config.window_seconds {
            self.attempts = 0;
            self.window_start = now;
        }

        // Check attempts
        if self.attempts >= config.max_attempts {
            self.lockout_until = Some(now + config.lockout_duration);
            return false;
        }

        true
    }

    fn record_failure(&mut self) {
        self.attempts += 1;
    }

    fn reset(&mut self) {
        self.attempts = 0;
        self.lockout_until = None;
    }
}

/// Authentication service
pub struct AuthService {
    user_storage: Arc<dyn UserStorage>,
    session_storage: Arc<dyn SessionStorage>,
    rate_limits: Arc<RwLock<im::HashMap<String, RateLimit>>>,
    config: RateLimitConfig,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(
        user_storage: Arc<dyn UserStorage>,
        session_storage: Arc<dyn SessionStorage>,
        config: Option<RateLimitConfig>,
    ) -> Self {
        Self {
            user_storage,
            session_storage,
            rate_limits: Arc::new(RwLock::new(im::HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }

    /// Register a new user
    pub async fn register(&self, username: String, password: &str) -> AuthResult<User> {
        // Create new user
        let user = User::new(username, password)?;

        // Store user
        self.user_storage.create_user(user).await
    }

    /// Authenticate a user and create a session
    pub async fn login(&self, request: AuthRequest) -> AuthResult<AuthResponse> {
        // Check rate limit
        self.check_rate_limit(&request.username).await?;

        // Get user
        let mut user = match self
            .user_storage
            .get_user_by_username(&request.username)
            .await
        {
            Ok(user) => user,
            Err(e) => {
                self.record_failed_attempt(&request.username).await;
                return Err(e);
            }
        };

        // Verify password
        if !user.verify_password(&request.password)? {
            self.record_failed_attempt(&request.username).await;
            return Err(AuthError::InvalidCredentials);
        }

        // Check account status
        if user.status != UserStatus::Active {
            return Err(AuthError::InvalidCredentials);
        }

        // Update last login
        user.last_login = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.user_storage.update_user(&user).await?;

        // Create session
        let session = Session::new(user.id, request.fingerprint);
        let session = self.session_storage.create_session(session).await?;

        // Reset rate limit on successful login
        self.reset_rate_limit(&request.username).await;

        Ok(AuthResponse { user, session })
    }

    /// Validate a session token
    pub async fn validate_session(&self, token: &str) -> AuthResult<Session> {
        let session = self.session_storage.get_session_by_token(token).await?;

        if session.is_expired() {
            self.session_storage.delete_session(session.id).await?;
            return Err(AuthError::SessionExpired);
        }

        Ok(session)
    }

    /// Refresh a session
    pub async fn refresh_session(&self, token: &str) -> AuthResult<Session> {
        let mut session = self.validate_session(token).await?;

        // Extend session by 24 hours
        session.extend(86400);
        self.session_storage.update_session(&session).await?;

        Ok(session)
    }

    /// Logout and invalidate session
    pub async fn logout(&self, token: &str) -> AuthResult<()> {
        let session = self.session_storage.get_session_by_token(token).await?;
        self.session_storage.delete_session(session.id).await
    }

    /// Clean up expired sessions
    pub async fn cleanup_sessions(&self) -> AuthResult<u64> {
        self.session_storage.cleanup_expired().await
    }

    // Rate limiting methods
    async fn check_rate_limit(&self, username: &str) -> AuthResult<()> {
        let mut limits = self.rate_limits.write().await;
        let limit = limits
            .entry(username.to_string())
            .or_insert_with(RateLimit::new);

        if !limit.check(&self.config) {
            Err(AuthError::RateLimitExceeded)
        } else {
            Ok(())
        }
    }

    async fn record_failed_attempt(&self, username: &str) {
        let mut limits = self.rate_limits.write().await;
        let limit = limits
            .entry(username.to_string())
            .or_insert_with(RateLimit::new);
        limit.record_failure();
    }

    async fn reset_rate_limit(&self, username: &str) {
        let mut limits = self.rate_limits.write().await;
        if let Some(limit) = limits.get_mut(username) {
            limit.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::storage::{MemorySessionStorage, MemoryUserStorage};
    use super::*;

    async fn create_test_service() -> AuthService {
        AuthService::new(
            Arc::new(MemoryUserStorage::new()),
            Arc::new(MemorySessionStorage::new()),
            None,
        )
    }

    #[tokio::test]
    async fn test_user_registration_and_login() {
        let service = create_test_service().await;

        // Register user
        let username = "testuser".to_string();
        let password = "password123";
        let user = service.register(username.clone(), password).await.unwrap();

        // Login
        let request = AuthRequest {
            username: username.clone(),
            password: password.to_string(),
            fingerprint: "test_device".to_string(),
            is_registration: false,
        };

        let response = service.login(request).await.unwrap();
        assert_eq!(response.user.id, user.id);
        assert!(!response.session.is_expired());
    }

    #[tokio::test]
    async fn test_failed_login_rate_limiting() {
        let service = create_test_service().await;

        // Register user
        let username = "testuser".to_string();
        let password = "password123";
        service.register(username.clone(), password).await.unwrap();

        // Attempt multiple failed logins
        let request = AuthRequest {
            username: username.clone(),
            password: "wrong_password".to_string(),
            fingerprint: "test_device".to_string(),
            is_registration: false,
        };

        for _ in 0..5 {
            assert!(service.login(request.clone()).await.is_err());
        }

        // Next attempt should be rate limited
        assert!(matches!(
            service.login(request.clone()).await,
            Err(AuthError::RateLimitExceeded)
        ));
    }

    #[tokio::test]
    async fn test_session_management() {
        let service = create_test_service().await;

        // Create user and login
        let username = "testuser".to_string();
        let password = "password123";
        service.register(username.clone(), password).await.unwrap();

        let request = AuthRequest {
            username,
            password: password.to_string(),
            fingerprint: "test_device".to_string(),
            is_registration: false,
        };

        let response = service.login(request).await.unwrap();
        let token = response.session.token;

        // Validate session
        let session = service.validate_session(&token).await.unwrap();
        assert_eq!(session.id, response.session.id);

        // Refresh session
        let refreshed = service.refresh_session(&token).await.unwrap();
        assert_eq!(refreshed.id, session.id);
        assert!(refreshed.expires_at > session.expires_at);

        // Logout
        service.logout(&token).await.unwrap();
        assert!(service.validate_session(&token).await.is_err());
    }
}
