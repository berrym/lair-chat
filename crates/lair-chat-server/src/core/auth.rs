//! Authentication service - user registration, login, password management.
//!
//! This service handles all authentication-related operations:
//! - User registration with password hashing
//! - Login with credential verification
//! - Password changes
//! - JWT token generation and validation

use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use super::jwt::JwtService;
use crate::domain::{
    Email, Pagination, Protocol, Role, Session, SessionId, User, UserId, Username,
};
use crate::storage::{SessionRepository, Storage, UserRepository};
use crate::{Error, Result};

// ============================================================================
// AuthService
// ============================================================================

/// Service for authentication operations.
pub struct AuthService<S: Storage> {
    storage: Arc<S>,
    jwt_service: JwtService,
}

impl<S: Storage + 'static> AuthService<S> {
    /// Create a new authentication service with JWT support.
    pub fn new(storage: Arc<S>, jwt_secret: &str) -> Self {
        Self {
            storage,
            jwt_service: JwtService::new(jwt_secret),
        }
    }

    /// Get a reference to the JWT service for token validation.
    pub fn jwt_service(&self) -> &JwtService {
        &self.jwt_service
    }

    /// Register a new user account.
    ///
    /// Creates the user, hashes the password, and automatically logs them in.
    ///
    /// # Errors
    ///
    /// - `UsernameInvalid` - Username doesn't meet requirements
    /// - `UsernameTaken` - Username already exists
    /// - `EmailInvalid` - Email doesn't meet requirements
    /// - `EmailTaken` - Email already registered
    /// - `PasswordTooWeak` - Password doesn't meet requirements
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(User, Session, String)> {
        // Validate username
        let username = Username::new(username).map_err(|e| Error::UsernameInvalid {
            reason: e.to_string(),
        })?;

        // Validate email
        let email = Email::new(email).map_err(|e| Error::EmailInvalid {
            reason: e.to_string(),
        })?;

        // Validate password
        Self::validate_password(password)?;

        // Check username availability
        if UserRepository::username_exists(&*self.storage, username.as_str()).await? {
            return Err(Error::UsernameTaken);
        }

        // Check email availability
        if UserRepository::email_exists(&*self.storage, email.as_str()).await? {
            return Err(Error::EmailTaken);
        }

        // Hash password
        let password_hash = Self::hash_password(password)?;

        // Create user (new users are regular users by default)
        let user = User::new(username, email, Role::User);
        UserRepository::create(&*self.storage, &user, &password_hash).await?;

        // Create session (auto-login)
        let session = Session::new(user.id, Protocol::Http);
        SessionRepository::create(&*self.storage, &session).await?;

        // Generate JWT token
        let token = self.generate_token(&user, &session)?;

        Ok((user, session, token))
    }

    /// Authenticate a user and create a session.
    ///
    /// The identifier can be either a username or email.
    ///
    /// # Errors
    ///
    /// - `InvalidCredentials` - Username/email not found or password incorrect
    pub async fn login(&self, identifier: &str, password: &str) -> Result<(User, Session, String)> {
        // Find user by username or email
        let user = UserRepository::find_by_username(&*self.storage, identifier)
            .await?
            .or(UserRepository::find_by_email(&*self.storage, identifier).await?)
            .ok_or(Error::InvalidCredentials)?;

        // Get password hash
        let stored_hash = UserRepository::get_password_hash(&*self.storage, user.id)
            .await?
            .ok_or(Error::InvalidCredentials)?;

        // Verify password
        Self::verify_password(password, &stored_hash)?;

        // Create session
        let session = Session::new(user.id, Protocol::Http);
        SessionRepository::create(&*self.storage, &session).await?;

        // Generate JWT token
        let token = self.generate_token(&user, &session)?;

        Ok((user, session, token))
    }

    /// Refresh a session token.
    pub async fn refresh_token(&self, session_id: SessionId) -> Result<String> {
        let session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        if session.is_expired() {
            return Err(Error::SessionExpired);
        }

        // Get the user for the JWT claims
        let user = UserRepository::find_by_id(&*self.storage, session.user_id)
            .await?
            .ok_or(Error::UserNotFound)?;

        self.generate_token(&user, &session)
    }

    /// Change a user's password.
    ///
    /// # Errors
    ///
    /// - `IncorrectPassword` - Current password is wrong
    /// - `PasswordTooWeak` - New password doesn't meet requirements
    pub async fn change_password(
        &self,
        user_id: UserId,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        // Verify current password
        let stored_hash = UserRepository::get_password_hash(&*self.storage, user_id)
            .await?
            .ok_or(Error::UserNotFound)?;

        Self::verify_password(current_password, &stored_hash)
            .map_err(|_| Error::IncorrectPassword)?;

        // Validate new password
        Self::validate_password(new_password)?;

        // Hash and store new password
        let new_hash = Self::hash_password(new_password)?;
        UserRepository::update_password_hash(&*self.storage, user_id, &new_hash).await?;

        Ok(())
    }

    /// Get a user by their ID.
    pub async fn get_user(&self, user_id: UserId) -> Result<Option<User>> {
        UserRepository::find_by_id(&*self.storage, user_id).await
    }

    /// List users with pagination.
    pub async fn list_users(&self, pagination: Pagination) -> Result<Vec<User>> {
        UserRepository::list(&*self.storage, pagination).await
    }

    // ========================================================================
    // Password Helpers
    // ========================================================================

    /// Hash a password using Argon2.
    fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(format!("password hashing failed: {}", e)))?;
        Ok(hash.to_string())
    }

    /// Verify a password against a stored hash.
    fn verify_password(password: &str, hash: &str) -> Result<()> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|_| Error::Internal("invalid hash format".into()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| Error::InvalidCredentials)
    }

    /// Validate password strength.
    fn validate_password(password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(Error::PasswordTooWeak {
                reason: "password must be at least 8 characters".into(),
            });
        }

        // Could add more complexity requirements here
        // For now, just length requirement

        Ok(())
    }

    /// Generate a JWT token for a user session.
    fn generate_token(&self, user: &User, session: &Session) -> Result<String> {
        self.jwt_service.generate(user, session)
    }

    /// Validate a JWT token and return the claims.
    ///
    /// This can be used by other services (like TCP) to validate tokens.
    pub fn validate_token(&self, token: &str) -> Result<(UserId, SessionId, Role)> {
        self.jwt_service.validate_and_extract(token)
    }

    /// Validate a token and verify the session is still active in the database.
    ///
    /// This performs full validation: JWT signature + session exists + session not expired.
    pub async fn validate_token_full(&self, token: &str) -> Result<(User, Session)> {
        let (user_id, session_id, _role) = self.jwt_service.validate_and_extract(token)?;

        // Verify session exists and is valid
        let session = SessionRepository::find_by_id(&*self.storage, session_id)
            .await?
            .ok_or(Error::SessionNotFound)?;

        if session.is_expired() {
            return Err(Error::SessionExpired);
        }

        // Verify user exists
        let user = UserRepository::find_by_id(&*self.storage, user_id)
            .await?
            .ok_or(Error::UserNotFound)?;

        Ok((user, session))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::sqlite::SqliteStorage;

    const TEST_JWT_SECRET: &str = "test_secret_key_for_jwt_testing_32bytes!";

    async fn create_test_storage() -> Arc<SqliteStorage> {
        Arc::new(SqliteStorage::in_memory().await.unwrap())
    }

    fn create_auth_service(storage: Arc<SqliteStorage>) -> AuthService<SqliteStorage> {
        AuthService::new(storage, TEST_JWT_SECRET)
    }

    // ========================================================================
    // Password validation tests
    // ========================================================================

    #[test]
    fn test_validate_password_success() {
        let result = AuthService::<SqliteStorage>::validate_password("validpassword123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        let result = AuthService::<SqliteStorage>::validate_password("short");
        assert!(matches!(result, Err(Error::PasswordTooWeak { .. })));
    }

    #[test]
    fn test_validate_password_exactly_8_chars() {
        let result = AuthService::<SqliteStorage>::validate_password("exactly8");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_password_7_chars() {
        let result = AuthService::<SqliteStorage>::validate_password("seven77");
        assert!(matches!(result, Err(Error::PasswordTooWeak { .. })));
    }

    #[test]
    fn test_hash_password_success() {
        let result = AuthService::<SqliteStorage>::hash_password("testpassword");
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_hash_produces_different_hashes() {
        let hash1 = AuthService::<SqliteStorage>::hash_password("testpassword").unwrap();
        let hash2 = AuthService::<SqliteStorage>::hash_password("testpassword").unwrap();
        // Same password should produce different hashes due to salt
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_success() {
        let hash = AuthService::<SqliteStorage>::hash_password("testpassword").unwrap();
        let result = AuthService::<SqliteStorage>::verify_password("testpassword", &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_password_wrong_password() {
        let hash = AuthService::<SqliteStorage>::hash_password("testpassword").unwrap();
        let result = AuthService::<SqliteStorage>::verify_password("wrongpassword", &hash);
        assert!(matches!(result, Err(Error::InvalidCredentials)));
    }

    // ========================================================================
    // Registration tests
    // ========================================================================

    #[tokio::test]
    async fn test_register_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.register("newuser", "new@example.com", "password123").await;

        assert!(result.is_ok());
        let (user, session, token) = result.unwrap();
        assert_eq!(user.username.as_str(), "newuser");
        assert_eq!(user.email.as_str(), "new@example.com");
        assert!(!token.is_empty());
        assert_eq!(session.user_id, user.id);
    }

    #[tokio::test]
    async fn test_register_invalid_username_too_short() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.register("ab", "valid@example.com", "password123").await;

        assert!(matches!(result, Err(Error::UsernameInvalid { .. })));
    }

    #[tokio::test]
    async fn test_register_invalid_email() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.register("validuser", "notanemail", "password123").await;

        assert!(matches!(result, Err(Error::EmailInvalid { .. })));
    }

    #[tokio::test]
    async fn test_register_weak_password() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.register("validuser", "valid@example.com", "short").await;

        assert!(matches!(result, Err(Error::PasswordTooWeak { .. })));
    }

    #[tokio::test]
    async fn test_register_username_taken() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register first user
        auth.register("takenuser", "first@example.com", "password123")
            .await
            .unwrap();

        // Try to register with same username
        let result = auth
            .register("takenuser", "second@example.com", "password123")
            .await;

        assert!(matches!(result, Err(Error::UsernameTaken)));
    }

    #[tokio::test]
    async fn test_register_email_taken() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register first user
        auth.register("firstuser", "taken@example.com", "password123")
            .await
            .unwrap();

        // Try to register with same email
        let result = auth
            .register("seconduser", "taken@example.com", "password123")
            .await;

        assert!(matches!(result, Err(Error::EmailTaken)));
    }

    // ========================================================================
    // Login tests
    // ========================================================================

    #[tokio::test]
    async fn test_login_with_username() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register user first
        let (original_user, _, _) = auth
            .register("loginuser", "login@example.com", "password123")
            .await
            .unwrap();

        // Login with username
        let result = auth.login("loginuser", "password123").await;

        assert!(result.is_ok());
        let (user, session, token) = result.unwrap();
        assert_eq!(user.id, original_user.id);
        assert!(!token.is_empty());
        assert_eq!(session.user_id, user.id);
    }

    #[tokio::test]
    async fn test_login_with_email() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register user first
        let (original_user, _, _) = auth
            .register("emaillogin", "emaillogin@example.com", "password123")
            .await
            .unwrap();

        // Login with email
        let result = auth.login("emaillogin@example.com", "password123").await;

        assert!(result.is_ok());
        let (user, _, _) = result.unwrap();
        assert_eq!(user.id, original_user.id);
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register user first
        auth.register("wrongpwuser", "wrongpw@example.com", "password123")
            .await
            .unwrap();

        // Login with wrong password
        let result = auth.login("wrongpwuser", "wrongpassword").await;

        assert!(matches!(result, Err(Error::InvalidCredentials)));
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.login("nonexistent", "password123").await;

        assert!(matches!(result, Err(Error::InvalidCredentials)));
    }

    // ========================================================================
    // Token validation tests
    // ========================================================================

    #[tokio::test]
    async fn test_validate_token_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (user, _session, token) = auth
            .register("tokenuser", "token@example.com", "password123")
            .await
            .unwrap();

        let result = auth.validate_token(&token);

        assert!(result.is_ok());
        let (user_id, _session_id, role) = result.unwrap();
        assert_eq!(user_id, user.id);
        assert_eq!(role, Role::User);
    }

    #[tokio::test]
    async fn test_validate_token_invalid() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.validate_token("invalid_token");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_full_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (original_user, original_session, token) = auth
            .register("fulluser", "full@example.com", "password123")
            .await
            .unwrap();

        let result = auth.validate_token_full(&token).await;

        assert!(result.is_ok());
        let (user, session) = result.unwrap();
        assert_eq!(user.id, original_user.id);
        assert_eq!(session.id, original_session.id);
    }

    // ========================================================================
    // Refresh token tests
    // ========================================================================

    #[tokio::test]
    async fn test_refresh_token_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (_user, session, _token) = auth
            .register("refreshuser", "refresh@example.com", "password123")
            .await
            .unwrap();

        let result = auth.refresh_token(session.id).await;

        assert!(result.is_ok());
        let new_token = result.unwrap();
        assert!(!new_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_session_not_found() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.refresh_token(SessionId::new()).await;

        assert!(matches!(result, Err(Error::SessionNotFound)));
    }

    // ========================================================================
    // Change password tests
    // ========================================================================

    #[tokio::test]
    async fn test_change_password_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (user, _, _) = auth
            .register("pwchangeuser", "pwchange@example.com", "oldpassword1")
            .await
            .unwrap();

        let result = auth
            .change_password(user.id, "oldpassword1", "newpassword1")
            .await;

        assert!(result.is_ok());

        // Should be able to login with new password
        let login_result = auth.login("pwchangeuser", "newpassword1").await;
        assert!(login_result.is_ok());
    }

    #[tokio::test]
    async fn test_change_password_wrong_current() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (user, _, _) = auth
            .register("wrongcurrent", "wrongcurrent@example.com", "realpassword")
            .await
            .unwrap();

        let result = auth
            .change_password(user.id, "wrongpassword", "newpassword1")
            .await;

        assert!(matches!(result, Err(Error::IncorrectPassword)));
    }

    #[tokio::test]
    async fn test_change_password_weak_new_password() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (user, _, _) = auth
            .register("weaknewpw", "weaknewpw@example.com", "goodpassword")
            .await
            .unwrap();

        let result = auth.change_password(user.id, "goodpassword", "short").await;

        assert!(matches!(result, Err(Error::PasswordTooWeak { .. })));
    }

    // ========================================================================
    // Get user tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_user_success() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let (original_user, _, _) = auth
            .register("getuser", "getuser@example.com", "password123")
            .await
            .unwrap();

        let result = auth.get_user(original_user.id).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().id, original_user.id);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        let result = auth.get_user(UserId::new()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // ========================================================================
    // List users tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_users() {
        let storage = create_test_storage().await;
        let auth = create_auth_service(storage);

        // Register multiple users
        auth.register("listuser1", "list1@example.com", "password123")
            .await
            .unwrap();
        auth.register("listuser2", "list2@example.com", "password123")
            .await
            .unwrap();

        let result = auth.list_users(Pagination::default()).await;

        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 2);
    }
}
