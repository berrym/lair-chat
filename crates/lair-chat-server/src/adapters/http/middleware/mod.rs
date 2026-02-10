//! HTTP middleware.
//!
//! This module provides middleware for the HTTP adapter:
//! - `auth`: JWT-based authentication with extractors
//! - `metrics`: Per-request counters and latency histograms
//! - `rate_limit`: Per-IP rate limiting

pub mod auth;
pub mod metrics;
pub mod rate_limit;

// Re-export commonly used types
pub use auth::{jwt_service_layer, AdminUser, AuthError, AuthUser, OptionalAuthUser};
pub use metrics::metrics_middleware;
pub use rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter};
