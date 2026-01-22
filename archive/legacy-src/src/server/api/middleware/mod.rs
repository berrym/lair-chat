//! API Middleware Module
//!
//! This module provides middleware components for the REST API, including
//! authentication, authorization, rate limiting, request logging, and
//! error handling middleware.
//!
//! # Architecture
//!
//! The middleware stack is applied in the following order:
//! 1. Request tracing and logging
//! 2. Rate limiting (per-IP and per-user)
//! 3. CORS handling
//! 4. Request body size limits
//! 5. JWT authentication (for protected routes)
//! 6. Authorization checks (role-based)
//! 7. Request timeout
//!
//! # Authentication Flow
//!
//! The JWT authentication middleware:
//! 1. Extracts Bearer token from Authorization header
//! 2. Validates JWT signature and expiration
//! 3. Extracts user claims and session information
//! 4. Validates session is still active
//! 5. Adds user context to request extensions

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::server::api::models::common::ApiError;
use crate::server::api::{models::auth::JwtClaims, ApiState};

pub mod auth;
pub mod rate_limit;
pub mod request_id;
pub mod tracing_middleware;

// Re-export commonly used middleware
pub use auth::*;
pub use rate_limit::*;
pub use request_id::*;
pub use tracing_middleware::*;

/// User context extracted from JWT token
#[derive(Debug, Clone)]
pub struct UserContext {
    /// User ID
    pub user_id: Uuid,

    /// Username
    pub username: String,

    /// User role
    pub role: crate::server::api::models::auth::UserRole,

    /// Session ID
    pub session_id: Uuid,

    /// Token expiration timestamp
    pub expires_at: chrono::DateTime<Utc>,

    /// JWT ID for revocation tracking
    pub jti: String,
}

impl UserContext {
    /// Check if the user has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self.role, crate::server::api::models::auth::UserRole::Admin)
    }

    /// Check if the user has moderator privileges or higher
    pub fn is_moderator_or_higher(&self) -> bool {
        matches!(
            self.role,
            crate::server::api::models::auth::UserRole::Admin
                | crate::server::api::models::auth::UserRole::Moderator
        )
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// JWT authentication middleware
pub async fn jwt_auth_middleware(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").unwrap_or("")
        }
        _ => {
            return Err(ApiError::auth_error(
                "Missing or invalid Authorization header",
            ));
        }
    };

    if token.is_empty() {
        return Err(ApiError::auth_error("Empty bearer token"));
    }

    // Validate JWT token
    let user_context = validate_jwt_token(token, &state).await?;

    // Check if token is expired
    if user_context.is_expired() {
        return Err(ApiError::auth_error("Token has expired"));
    }

    // Validate session is still active
    match state
        .storage
        .sessions()
        .get_session(&user_context.session_id.to_string())
        .await
    {
        Ok(Some(session)) => {
            if !session.is_active {
                return Err(ApiError::auth_error("Session is no longer active"));
            }
            if session.expires_at < Utc::now().timestamp() as u64 {
                return Err(ApiError::auth_error("Session has expired"));
            }
        }
        Ok(None) => {
            return Err(ApiError::auth_error("Session not found"));
        }
        Err(e) => {
            warn!("Failed to validate session: {}", e);
            return Err(ApiError::internal_error("Session validation failed"));
        }
    }

    // Add user context to request extensions
    request.extensions_mut().insert(user_context);

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Optional JWT authentication middleware (doesn't fail if no token)
pub async fn optional_jwt_auth_middleware(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token, but don't fail if missing
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if !token.is_empty() {
                match validate_jwt_token(token, &state).await {
                    Ok(user_context) if !user_context.is_expired() => {
                        // Validate session if token is valid
                        if let Ok(Some(session)) = state
                            .storage
                            .sessions()
                            .get_session(&user_context.session_id.to_string())
                            .await
                        {
                            if session.is_active
                                && session.expires_at >= Utc::now().timestamp() as u64
                            {
                                request.extensions_mut().insert(user_context);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Optional auth failed: {}", e.message);
                    }
                    Ok(_) => {
                        debug!("Optional auth failed: token expired");
                    }
                }
            }
        }
    }

    next.run(request).await
}

/// Admin-only authorization middleware
pub async fn admin_auth_middleware(request: Request, next: Next) -> Result<Response, ApiError> {
    let user_context = request
        .extensions()
        .get::<UserContext>()
        .ok_or_else(|| ApiError::auth_error("Authentication required"))?;

    if !user_context.is_admin() {
        return Err(ApiError::forbidden_error(
            "Admin privileges required for this operation",
        ));
    }

    Ok(next.run(request).await)
}

/// Moderator or admin authorization middleware
pub async fn moderator_auth_middleware(request: Request, next: Next) -> Result<Response, ApiError> {
    let user_context = request
        .extensions()
        .get::<UserContext>()
        .ok_or_else(|| ApiError::auth_error("Authentication required"))?;

    if !user_context.is_moderator_or_higher() {
        return Err(ApiError::forbidden_error(
            "Moderator or admin privileges required for this operation",
        ));
    }

    Ok(next.run(request).await)
}

/// Validate JWT token and extract user context
async fn validate_jwt_token(token: &str, state: &ApiState) -> Result<UserContext, ApiError> {
    // Decode JWT token
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        warn!("JWT validation failed: {}", e);
        ApiError::auth_error("Invalid token")
    })?;

    let claims = token_data.claims;

    // Parse user ID
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        warn!("Invalid user ID in JWT claims: {}", claims.sub);
        ApiError::auth_error("Invalid token claims")
    })?;

    // Parse session ID
    let session_id = Uuid::parse_str(&claims.session_id).map_err(|_| {
        warn!("Invalid session ID in JWT claims: {}", claims.session_id);
        ApiError::auth_error("Invalid token claims")
    })?;

    // Convert expiration timestamp
    let expires_at = chrono::DateTime::from_timestamp(claims.exp, 0)
        .ok_or_else(|| ApiError::auth_error("Invalid token expiration"))?;

    // Get username from user storage (for fresh data)
    let user = state
        .storage
        .users()
        .get_user_by_id(&user_id.to_string())
        .await
        .map_err(|e| {
            warn!("Failed to get user for token validation: {}", e);
            ApiError::auth_error("User not found")
        })?
        .ok_or_else(|| ApiError::auth_error("User not found"))?;

    Ok(UserContext {
        user_id,
        username: user.username,
        role: claims.role,
        session_id,
        expires_at,
        jti: claims.jti,
    })
}

/// Extract user context from request extensions
pub fn get_user_context(request: &Request) -> Option<&UserContext> {
    request.extensions().get::<UserContext>()
}

/// Extract user context from request extensions (required)
pub fn require_user_context(request: &Request) -> Result<&UserContext, ApiError> {
    get_user_context(request).ok_or_else(|| ApiError::auth_error("Authentication required"))
}

/// Request ID middleware for tracing
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();

    // Add request ID to request extensions
    request.extensions_mut().insert(request_id.clone());

    // Continue to next middleware
    let mut response = next.run(request).await;

    // Add request ID to response headers
    response
        .headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::models::auth::{TokenType, UserRole, UserStatus};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::collections::HashMap;

    fn create_test_claims(user_id: Uuid, session_id: Uuid, exp_offset_seconds: i64) -> JwtClaims {
        let now = Utc::now().timestamp();

        JwtClaims {
            sub: user_id.to_string(),
            iat: now,
            exp: now + exp_offset_seconds,
            iss: "lair-chat".to_string(),
            aud: "lair-chat-api".to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
            role: UserRole::User,
            session_id: session_id.to_string(),
            custom: HashMap::new(),
        }
    }

    #[test]
    fn test_user_context_role_checks() {
        let admin_context = UserContext {
            user_id: Uuid::new_v4(),
            username: "admin".to_string(),
            role: UserRole::Admin,
            session_id: Uuid::new_v4(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            jti: "test".to_string(),
        };

        let moderator_context = UserContext {
            role: UserRole::Moderator,
            ..admin_context.clone()
        };

        let user_context = UserContext {
            role: UserRole::User,
            ..admin_context.clone()
        };

        assert!(admin_context.is_admin());
        assert!(admin_context.is_moderator_or_higher());

        assert!(!moderator_context.is_admin());
        assert!(moderator_context.is_moderator_or_higher());

        assert!(!user_context.is_admin());
        assert!(!user_context.is_moderator_or_higher());
    }

    #[test]
    fn test_user_context_expiration() {
        let expired_context = UserContext {
            user_id: Uuid::new_v4(),
            username: "user".to_string(),
            role: UserRole::User,
            session_id: Uuid::new_v4(),
            expires_at: Utc::now() - chrono::Duration::hours(1),
            jti: "test".to_string(),
        };

        let valid_context = UserContext {
            expires_at: Utc::now() + chrono::Duration::hours(1),
            ..expired_context.clone()
        };

        assert!(expired_context.is_expired());
        assert!(!valid_context.is_expired());
    }

    #[tokio::test]
    async fn test_jwt_token_creation_and_validation() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let jwt_secret = "test_secret";

        let claims = create_test_claims(user_id, session_id, 3600);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .unwrap();

        let validation = Validation::new(Algorithm::HS256);
        let decoded = decode::<JwtClaims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        )
        .unwrap();

        assert_eq!(decoded.claims.sub, user_id.to_string());
        assert_eq!(decoded.claims.session_id, session_id.to_string());
    }
}
