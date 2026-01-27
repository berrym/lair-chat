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
