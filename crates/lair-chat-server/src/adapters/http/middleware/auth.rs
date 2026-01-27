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
