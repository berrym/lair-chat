//! JWT (JSON Web Token) service for authentication.
//!
//! This module provides JWT generation and validation for the Lair Chat server.
//! Tokens are used to authenticate users across both HTTP and TCP protocols.
//!
//! ## Token Structure
//!
//! The JWT contains the following claims:
//! - `sub`: User ID (UUID)
//! - `sid`: Session ID (UUID)
//! - `username`: User's username
//! - `role`: User's role (user, moderator, admin)
//! - `exp`: Expiration timestamp
//! - `iat`: Issued at timestamp
//!
//! ## Usage
//!
//! ```ignore
//! let jwt_service = JwtService::new("your-secret-key");
//!
//! // Generate a token after login
//! let token = jwt_service.generate(&user, &session)?;
//!
//! // Validate a token from a request
//! let claims = jwt_service.validate(&token)?;
//! ```

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::domain::{Role, Session, SessionId, User, UserId};
use crate::{Error, Result};

// ============================================================================
// Configuration
// ============================================================================

/// Default token expiration duration (24 hours).
pub const DEFAULT_TOKEN_DURATION: Duration = Duration::hours(24);

/// Algorithm used for JWT signing.
const JWT_ALGORITHM: Algorithm = Algorithm::HS256;

// ============================================================================
// Claims
// ============================================================================

/// JWT claims structure.
///
/// These are the data embedded in each token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject - the user ID.
    pub sub: String,
    /// Session ID.
    pub sid: String,
    /// Username for quick access without database lookup.
    pub username: String,
    /// User role.
    pub role: String,
    /// Expiration time (Unix timestamp).
    pub exp: usize,
    /// Issued at time (Unix timestamp).
    pub iat: usize,
}

impl Claims {
    /// Get the user ID from claims.
    pub fn user_id(&self) -> Result<UserId> {
        UserId::parse(&self.sub).map_err(|e| Error::InvalidToken {
            reason: format!("invalid user ID in token: {}", e),
        })
    }

    /// Get the session ID from claims.
    pub fn session_id(&self) -> Result<SessionId> {
        SessionId::parse(&self.sid).map_err(|e| Error::InvalidToken {
            reason: format!("invalid session ID in token: {}", e),
        })
    }

    /// Get the role from claims.
    pub fn role(&self) -> Role {
        Role::parse(&self.role)
    }

    /// Check if the token has expired.
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp() as usize;
        now > self.exp
    }
}

// ============================================================================
// JwtService
// ============================================================================

/// Service for JWT token generation and validation.
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtService {
    /// Create a new JWT service with the given secret.
    ///
    /// The secret should be a cryptographically secure random string
    /// of at least 32 bytes.
    pub fn new(secret: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::new(JWT_ALGORITHM);
        // We'll handle expiration checking ourselves for better error messages
        validation.validate_exp = true;
        validation.leeway = 0; // No leeway for expiration

        Self {
            encoding_key,
            decoding_key,
            validation,
        }
    }

    /// Generate a JWT token for a user session.
    ///
    /// The token encodes the user's ID, session ID, username, and role.
    /// It expires based on the session's expiration time.
    pub fn generate(&self, user: &User, session: &Session) -> Result<String> {
        let now = Utc::now();
        let exp = session.expires_at.timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = Claims {
            sub: user.id.to_string(),
            sid: session.id.to_string(),
            username: user.username.to_string(),
            role: user.role.as_str().to_string(),
            exp,
            iat,
        };

        let header = Header::new(JWT_ALGORITHM);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| Error::Internal(format!("failed to generate JWT: {}", e)))
    }

    /// Generate a token with custom expiration duration.
    pub fn generate_with_duration(
        &self,
        user: &User,
        session: &Session,
        duration: Duration,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = (now + duration).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = Claims {
            sub: user.id.to_string(),
            sid: session.id.to_string(),
            username: user.username.to_string(),
            role: user.role.as_str().to_string(),
            exp,
            iat,
        };

        let header = Header::new(JWT_ALGORITHM);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| Error::Internal(format!("failed to generate JWT: {}", e)))
    }

    /// Validate a JWT token and extract its claims.
    ///
    /// # Errors
    ///
    /// - `InvalidToken` - Token is malformed or signature is invalid
    /// - `TokenExpired` - Token has expired
    pub fn validate(&self, token: &str) -> Result<Claims> {
        let token_data =
            decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => Error::TokenExpired,
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => Error::InvalidToken {
                        reason: "invalid signature".into(),
                    },
                    jsonwebtoken::errors::ErrorKind::InvalidToken => Error::InvalidToken {
                        reason: "malformed token".into(),
                    },
                    _ => Error::InvalidToken {
                        reason: e.to_string(),
                    },
                }
            })?;

        Ok(token_data.claims)
    }

    /// Validate a token and return the user and session IDs if valid.
    ///
    /// This is a convenience method for common authentication checks.
    pub fn validate_and_extract(&self, token: &str) -> Result<(UserId, SessionId, Role)> {
        let claims = self.validate(token)?;
        let user_id = claims.user_id()?;
        let session_id = claims.session_id()?;
        let role = claims.role();
        Ok((user_id, session_id, role))
    }
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("algorithm", &JWT_ALGORITHM)
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, Protocol, Username};

    fn create_test_user() -> User {
        User::new(
            Username::new("testuser").unwrap(),
            Email::new("test@example.com").unwrap(),
            Role::User,
        )
    }

    fn create_test_session(user: &User) -> Session {
        Session::new(user.id, Protocol::Http)
    }

    #[test]
    fn test_generate_and_validate() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");
        let user = create_test_user();
        let session = create_test_session(&user);

        let token = service.generate(&user, &session).unwrap();
        assert!(!token.is_empty());

        let claims = service.validate(&token).unwrap();
        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.sid, session.id.to_string());
        assert_eq!(claims.username, user.username.to_string());
        assert_eq!(claims.role, "user");
    }

    #[test]
    fn test_validate_and_extract() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");
        let user = create_test_user();
        let session = create_test_session(&user);

        let token = service.generate(&user, &session).unwrap();
        let (user_id, session_id, role) = service.validate_and_extract(&token).unwrap();

        assert_eq!(user_id, user.id);
        assert_eq!(session_id, session.id);
        assert_eq!(role, Role::User);
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");

        let result = service.validate("not-a-valid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let service1 = JwtService::new("secret-key-one-at-least-32-bytes");
        let service2 = JwtService::new("secret-key-two-at-least-32-bytes");

        let user = create_test_user();
        let session = create_test_session(&user);

        let token = service1.generate(&user, &session).unwrap();

        // Token signed with different secret should fail validation
        let result = service2.validate(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");
        let user = create_test_user();

        // Create session that's already expired
        let expired_session =
            Session::with_duration(user.id, Protocol::Http, Duration::seconds(-1));

        let token = service.generate(&user, &expired_session).unwrap();
        let result = service.validate(&token);

        assert!(matches!(result, Err(Error::TokenExpired)));
    }

    #[test]
    fn test_admin_role() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");
        let user = User::new_admin(
            Username::new("admin").unwrap(),
            Email::new("admin@example.com").unwrap(),
        );
        let session = create_test_session(&user);

        let token = service.generate(&user, &session).unwrap();
        let claims = service.validate(&token).unwrap();

        assert_eq!(claims.role, "admin");
        assert_eq!(claims.role(), Role::Admin);
    }

    #[test]
    fn test_claims_methods() {
        let service = JwtService::new("test-secret-key-at-least-32-bytes-long");
        let user = create_test_user();
        let session = create_test_session(&user);

        let token = service.generate(&user, &session).unwrap();
        let claims = service.validate(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user.id);
        assert_eq!(claims.session_id().unwrap(), session.id);
        assert!(!claims.is_expired());
    }
}
