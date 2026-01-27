//! Rate limiting middleware for HTTP handlers.
//!
//! Provides per-user and per-IP rate limiting to prevent abuse.
//!
//! ## Rate Limit Categories
//!
//! - **Authentication**: 10 requests per minute (login, register)
//! - **Messaging**: 30 requests per minute (send, edit, delete messages)
//! - **General**: 100 requests per minute (all other endpoints)

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ============================================================================
// Configuration
// ============================================================================

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Time window duration.
    pub window: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit config.
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Authentication rate limit (10/minute).
    pub fn auth() -> Self {
        Self::new(10, 60)
    }

    /// Messaging rate limit (30/minute).
    pub fn messaging() -> Self {
        Self::new(30, 60)
    }

    /// General rate limit (100/minute).
    pub fn general() -> Self {
        Self::new(100, 60)
    }
}

// ============================================================================
// Rate Limiter State
// ============================================================================

/// Tracks request counts for rate limiting.
#[derive(Debug)]
struct RequestTracker {
    /// Request count in current window.
    count: u32,
    /// When the current window started.
    window_start: Instant,
}

impl RequestTracker {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }

    /// Check and update the request count.
    /// Returns true if the request is allowed, false if rate limited.
    fn check_and_update(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();

        // Reset window if expired
        if now.duration_since(self.window_start) >= config.window {
            self.count = 0;
            self.window_start = now;
        }

        // Check if within limit
        if self.count >= config.max_requests {
            return false;
        }

        self.count += 1;
        true
    }

    /// Get remaining requests in current window.
    fn remaining(&self, config: &RateLimitConfig) -> u32 {
        config.max_requests.saturating_sub(self.count)
    }

    /// Get seconds until window reset.
    fn reset_in(&self, config: &RateLimitConfig) -> u64 {
        let elapsed = Instant::now().duration_since(self.window_start);
        config.window.saturating_sub(elapsed).as_secs()
    }
}

/// Shared rate limiter state.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Per-IP request trackers.
    trackers: Arc<RwLock<HashMap<String, RequestTracker>>>,
    /// Rate limit configuration.
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with the given config.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            trackers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a rate limiter for authentication endpoints.
    pub fn auth() -> Self {
        Self::new(RateLimitConfig::auth())
    }

    /// Create a rate limiter for messaging endpoints.
    pub fn messaging() -> Self {
        Self::new(RateLimitConfig::messaging())
    }

    /// Create a rate limiter for general endpoints.
    pub fn general() -> Self {
        Self::new(RateLimitConfig::general())
    }

    /// Check if a request from the given key is allowed.
    pub async fn check(&self, key: &str) -> RateLimitResult {
        let mut trackers = self.trackers.write().await;

        let tracker = trackers
            .entry(key.to_string())
            .or_insert_with(RequestTracker::new);

        let allowed = tracker.check_and_update(&self.config);
        let remaining = tracker.remaining(&self.config);
        let reset_in = tracker.reset_in(&self.config);

        RateLimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_in,
        }
    }

    /// Clean up expired trackers to prevent memory growth.
    pub async fn cleanup(&self) {
        let mut trackers = self.trackers.write().await;
        let now = Instant::now();

        trackers.retain(|_, tracker| {
            now.duration_since(tracker.window_start) < self.config.window * 2
        });
    }
}

/// Result of a rate limit check.
#[derive(Debug)]
pub struct RateLimitResult {
    /// Whether the request is allowed.
    pub allowed: bool,
    /// Maximum requests allowed per window.
    pub limit: u32,
    /// Remaining requests in current window.
    pub remaining: u32,
    /// Seconds until window resets.
    pub reset_in: u64,
}

// ============================================================================
// Middleware
// ============================================================================

/// Rate limiting middleware function.
///
/// Use with `axum::middleware::from_fn_with_state`.
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    // Get client identifier (prefer IP, fallback to "unknown")
    let client_key = extract_client_key(&request);

    // Check rate limit
    let result = limiter.check(&client_key).await;

    if !result.allowed {
        return RateLimitError {
            retry_after: result.reset_in as u32,
            limit: result.limit,
        }
        .into_response();
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        "X-RateLimit-Limit",
        result.limit.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        result.remaining.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Reset",
        result.reset_in.to_string().parse().unwrap(),
    );

    response
}

/// Extract a client identifier for rate limiting.
fn extract_client_key(request: &Request) -> String {
    // Try X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
    {
        // Take the first IP in the chain
        if let Some(ip) = forwarded.split(',').next() {
            return ip.trim().to_string();
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request
        .headers()
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
    {
        return real_ip.to_string();
    }

    // Fallback to connection info (not always available)
    "unknown".to_string()
}

// ============================================================================
// Error Response
// ============================================================================

/// Rate limit exceeded error.
#[derive(Debug)]
pub struct RateLimitError {
    /// Seconds until the client can retry.
    pub retry_after: u32,
    /// The rate limit that was exceeded.
    pub limit: u32,
}

#[derive(Serialize)]
struct RateLimitErrorResponse {
    error: RateLimitErrorDetail,
}

#[derive(Serialize)]
struct RateLimitErrorDetail {
    code: &'static str,
    message: String,
    details: RateLimitDetails,
}

#[derive(Serialize)]
struct RateLimitDetails {
    retry_after: u32,
    limit: u32,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let body = Json(RateLimitErrorResponse {
            error: RateLimitErrorDetail {
                code: "rate_limited",
                message: "Too many requests".to_string(),
                details: RateLimitDetails {
                    retry_after: self.retry_after,
                    limit: self.limit,
                },
            },
        });

        let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
        response.headers_mut().insert(
            "Retry-After",
            self.retry_after.to_string().parse().unwrap(),
        );

        response
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::new(3, 60));

        for _ in 0..3 {
            let result = limiter.check("test-client").await;
            assert!(result.allowed);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig::new(2, 60));

        // First two requests allowed
        assert!(limiter.check("test-client").await.allowed);
        assert!(limiter.check("test-client").await.allowed);

        // Third request blocked
        let result = limiter.check("test-client").await;
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(RateLimitConfig::new(1, 60));

        // Each key has its own limit
        assert!(limiter.check("client-1").await.allowed);
        assert!(limiter.check("client-2").await.allowed);

        // Second request for same key is blocked
        assert!(!limiter.check("client-1").await.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_headers() {
        let limiter = RateLimiter::new(RateLimitConfig::new(10, 60));

        let result = limiter.check("test").await;
        assert_eq!(result.limit, 10);
        assert_eq!(result.remaining, 9); // After first request
    }
}
