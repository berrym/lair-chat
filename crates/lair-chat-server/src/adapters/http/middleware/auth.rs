//! Authentication middleware for HTTP handlers.
//!
//! This module provides middleware for JWT authentication on protected routes.
//!
//! ## Usage
//!
//! ```ignore
//! // In your routes setup, add the AuthUser extractor to protected handlers
//! async fn protected_handler(auth: AuthUser) -> impl IntoResponse {
//!     // auth.user_id, auth.session_id, auth.role are available
//! }
//! ```

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::core::jwt::JwtService;
use crate::domain::{Role, SessionId, UserId};
use crate::Error;

// ============================================================================
// Auth Extractor
// ============================================================================

/// Authenticated user information extracted from JWT.
///
/// Use this as an extractor in handlers that require authentication.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// The authenticated user's ID.
    pub user_id: UserId,
    /// The session ID.
    pub session_id: SessionId,
    /// The user's role.
    pub role: Role,
    /// The username (from JWT claims).
    pub username: String,
}

impl AuthUser {
    /// Check if the user is an admin.
    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// Check if the user is at least a moderator.
    pub fn is_moderator(&self) -> bool {
        self.role.is_moderator()
    }

    /// Check if the user has the required permission level.
    pub fn has_permission(&self, required: Role) -> bool {
        self.role.has_permission(required)
    }

    /// Extract from request parts (for use in other extractors).
    async fn extract(parts: &mut Parts) -> Result<Self, AuthError> {
        // Get the JWT service from extensions (must be added by middleware layer)
        let jwt_service = parts
            .extensions
            .get::<Arc<JwtService>>()
            .ok_or(AuthError::InternalError)?;

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::MissingToken)?;

        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidFormat)?;

        // Validate JWT
        let claims = jwt_service.validate(token).map_err(|e| match e {
            Error::TokenExpired => AuthError::TokenExpired,
            Error::InvalidToken { reason } => AuthError::InvalidToken(reason),
            _ => AuthError::InvalidToken("validation failed".into()),
        })?;

        // Extract user info from claims
        let user_id = claims.user_id().map_err(|_| AuthError::InvalidClaims)?;
        let session_id = claims.session_id().map_err(|_| AuthError::InvalidClaims)?;

        Ok(AuthUser {
            user_id,
            session_id,
            role: claims.role(),
            username: claims.username,
        })
    }
}

/// Extract authentication from JWT token in Authorization header.
///
/// This extractor will reject requests without a valid JWT token.
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::extract(parts).await
    }
}

// ============================================================================
// Optional Auth Extractor
// ============================================================================

/// Optional authentication - doesn't reject if no token present.
///
/// Use this for endpoints that have different behavior for authenticated
/// vs anonymous users.
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuthUser(AuthUser::extract(parts).await.ok()))
    }
}

// ============================================================================
// Admin Auth Extractor
// ============================================================================

/// Admin-only authentication.
///
/// Use this for endpoints that require admin privileges.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::extract(parts).await?;
        if !auth.is_admin() {
            return Err(AuthError::PermissionDenied);
        }
        Ok(AdminUser(auth))
    }
}

// ============================================================================
// Auth Errors
// ============================================================================

/// Authentication errors.
#[derive(Debug)]
pub enum AuthError {
    /// No Authorization header present.
    MissingToken,
    /// Authorization header format is invalid.
    InvalidFormat,
    /// JWT token is invalid.
    InvalidToken(String),
    /// JWT token has expired.
    TokenExpired,
    /// Claims in token are invalid.
    InvalidClaims,
    /// User doesn't have required permissions.
    PermissionDenied,
    /// Internal server error (JWT service not available).
    InternalError,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "missing_token",
                "Authorization header required".to_string(),
            ),
            AuthError::InvalidFormat => (
                StatusCode::UNAUTHORIZED,
                "invalid_format",
                "Authorization header must be 'Bearer <token>'".to_string(),
            ),
            AuthError::InvalidToken(reason) => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                format!("Invalid token: {}", reason),
            ),
            AuthError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                "token_expired",
                "Token has expired, please login again".to_string(),
            ),
            AuthError::InvalidClaims => (
                StatusCode::UNAUTHORIZED,
                "invalid_claims",
                "Token claims are invalid".to_string(),
            ),
            AuthError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "permission_denied",
                "You don't have permission to access this resource".to_string(),
            ),
            AuthError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Authentication service unavailable".to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            error: ErrorDetail { code, message },
        });

        (status, body).into_response()
    }
}

// ============================================================================
// JWT Service Extension Layer
// ============================================================================

use axum::extract::Request;
use axum::middleware::Next;

/// Middleware layer that adds JWT service to request extensions.
///
/// This must be applied to routes that use the AuthUser extractor.
pub async fn jwt_service_layer(
    jwt_service: Arc<JwtService>,
    mut request: Request,
    next: Next,
) -> Response {
    request.extensions_mut().insert(jwt_service);
    next.run(request).await
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract claims from a bearer token.
///
/// Useful for handlers that need direct access to claims.
pub fn extract_token_from_header(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    // ========================================================================
    // AuthUser tests
    // ========================================================================

    #[test]
    fn test_auth_user_is_admin() {
        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::Admin,
            username: "admin".to_string(),
        };
        assert!(auth.is_admin());

        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::User,
            username: "user".to_string(),
        };
        assert!(!auth.is_admin());
    }

    #[test]
    fn test_auth_user_is_moderator() {
        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::Admin,
            username: "admin".to_string(),
        };
        assert!(auth.is_moderator());

        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::Moderator,
            username: "mod".to_string(),
        };
        assert!(auth.is_moderator());

        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::User,
            username: "user".to_string(),
        };
        assert!(!auth.is_moderator());
    }

    #[test]
    fn test_auth_user_has_permission() {
        let admin = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::Admin,
            username: "admin".to_string(),
        };

        // Admin has all permissions
        assert!(admin.has_permission(Role::User));
        assert!(admin.has_permission(Role::Moderator));
        assert!(admin.has_permission(Role::Admin));

        let user = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::User,
            username: "user".to_string(),
        };

        // User only has user-level permission
        assert!(user.has_permission(Role::User));
        assert!(!user.has_permission(Role::Moderator));
        assert!(!user.has_permission(Role::Admin));
    }

    // ========================================================================
    // AuthError tests
    // ========================================================================

    #[test]
    fn test_auth_error_debug() {
        // Test Debug trait implementation
        let error = AuthError::MissingToken;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MissingToken"));

        let error = AuthError::InvalidToken("test reason".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidToken"));
        assert!(debug_str.contains("test reason"));
    }

    #[test]
    fn test_auth_error_into_response_missing_token() {
        let error = AuthError::MissingToken;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_into_response_invalid_format() {
        let error = AuthError::InvalidFormat;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_into_response_invalid_token() {
        let error = AuthError::InvalidToken("bad signature".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_into_response_token_expired() {
        let error = AuthError::TokenExpired;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_into_response_invalid_claims() {
        let error = AuthError::InvalidClaims;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_error_into_response_permission_denied() {
        let error = AuthError::PermissionDenied;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_auth_error_into_response_internal_error() {
        let error = AuthError::InternalError;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ========================================================================
    // extract_token_from_header tests
    // ========================================================================

    #[test]
    fn test_extract_token_from_header_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer my_jwt_token_here".parse().unwrap(),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some("my_jwt_token_here"));
    }

    #[test]
    fn test_extract_token_from_header_missing() {
        let headers = HeaderMap::new();
        let token = extract_token_from_header(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_token_from_header_wrong_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Basic dXNlcjpwYXNz".parse().unwrap(),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_token_from_header_empty_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer ".parse().unwrap());

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some(""));
    }

    #[test]
    fn test_extract_token_from_header_no_space_after_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearertoken".parse().unwrap());

        let token = extract_token_from_header(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_token_from_header_case_sensitive() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "bearer token".parse().unwrap());

        // "Bearer" is case sensitive per spec
        let token = extract_token_from_header(&headers);
        assert_eq!(token, None);
    }

    // ========================================================================
    // OptionalAuthUser tests
    // ========================================================================

    #[test]
    fn test_optional_auth_user_clone() {
        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::User,
            username: "test".to_string(),
        };
        let optional = OptionalAuthUser(Some(auth));
        let cloned = optional.clone();
        assert!(cloned.0.is_some());
    }

    #[test]
    fn test_optional_auth_user_none() {
        let optional = OptionalAuthUser(None);
        assert!(optional.0.is_none());
    }

    // ========================================================================
    // AdminUser tests
    // ========================================================================

    #[test]
    fn test_admin_user_clone() {
        let auth = AuthUser {
            user_id: UserId::new(),
            session_id: SessionId::new(),
            role: Role::Admin,
            username: "admin".to_string(),
        };
        let admin = AdminUser(auth);
        let cloned = admin.clone();
        assert!(cloned.0.is_admin());
    }
}
