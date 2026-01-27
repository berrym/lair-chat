//! HTTP middleware.
//!
//! This module provides middleware for the HTTP adapter:
//! - `auth`: JWT-based authentication with extractors
//! - `rate_limit`: Per-IP rate limiting

pub mod auth;
pub mod rate_limit;

// Re-export commonly used types
pub use auth::{jwt_service_layer, AdminUser, AuthError, AuthUser, OptionalAuthUser};
pub use rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter};
