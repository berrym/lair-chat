//! Rate limiting middleware for API endpoints
//!
//! This module provides rate limiting functionality to prevent abuse and
//! brute force attacks on the API. It supports both IP-based and user-based
//! rate limiting with configurable limits and time windows.
//!
//! # Rate Limiting Strategy
//!
//! - **IP-based limiting**: Applied to all requests from a single IP
//! - **User-based limiting**: Applied to authenticated requests per user
//! - **Endpoint-specific limits**: Different limits for different endpoints
//! - **Sliding window**: Uses a sliding window algorithm for smooth rate limiting
//!
//! # Configuration
//!
//! Rate limits can be configured per endpoint type:
//! - Authentication endpoints: Stricter limits to prevent brute force
//! - General API endpoints: Moderate limits for normal usage
//! - Admin endpoints: Lower limits for sensitive operations

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tracing::{debug, warn};

use crate::server::api::{middleware::UserContext, models::common::ApiError};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per time window
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Burst allowance (requests that can exceed the limit temporarily)
    pub burst_allowance: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            burst_allowance: 10,
        }
    }
}

impl RateLimitConfig {
    /// Configuration for authentication endpoints (stricter limits)
    pub fn auth_endpoints() -> Self {
        Self {
            max_requests: 5,
            window_duration: Duration::from_secs(60),
            burst_allowance: 2,
        }
    }

    /// Configuration for general API endpoints
    pub fn general_endpoints() -> Self {
        Self {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            burst_allowance: 50,
        }
    }

    /// Configuration for admin endpoints (very strict limits)
    pub fn admin_endpoints() -> Self {
        Self {
            max_requests: 20,
            window_duration: Duration::from_secs(60),
            burst_allowance: 5,
        }
    }

    /// Configuration for message endpoints (moderate limits)
    pub fn message_endpoints() -> Self {
        Self {
            max_requests: 200,
            window_duration: Duration::from_secs(60),
            burst_allowance: 20,
        }
    }
}

/// Rate limiter entry tracking requests for a specific key
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Request timestamps in the current window
    requests: Vec<Instant>,
    /// First request timestamp in current window
    window_start: Instant,
    /// Number of burst requests used
    burst_used: u32,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            window_start: Instant::now(),
            burst_used: 0,
        }
    }

    /// Check if a request should be allowed
    fn check_and_record(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();

        // Clean up old requests outside the window
        self.cleanup_old_requests(now, config.window_duration);

        // Reset window if needed
        if now.duration_since(self.window_start) > config.window_duration {
            self.window_start = now;
            self.requests.clear();
            self.burst_used = 0;
        }

        let current_requests = self.requests.len() as u32;

        // Check if within normal limits
        if current_requests < config.max_requests {
            self.requests.push(now);
            return true;
        }

        // Check if burst allowance is available
        if self.burst_used < config.burst_allowance {
            self.requests.push(now);
            self.burst_used += 1;
            return true;
        }

        false
    }

    /// Remove requests older than the time window
    fn cleanup_old_requests(&mut self, now: Instant, window_duration: Duration) {
        let cutoff = now - window_duration;
        self.requests.retain(|&timestamp| timestamp > cutoff);
    }

    /// Get current request count in window
    fn current_count(&self) -> u32 {
        self.requests.len() as u32
    }

    /// Get time until window resets
    fn time_until_reset(&self, window_duration: Duration) -> Duration {
        let elapsed = Instant::now().duration_since(self.window_start);
        if elapsed >= window_duration {
            Duration::from_secs(0)
        } else {
            window_duration - elapsed
        }
    }
}

/// In-memory rate limiter storage
#[derive(Debug)]
pub struct RateLimiter {
    /// IP-based rate limiting entries
    ip_entries: Arc<Mutex<HashMap<String, RateLimitEntry>>>,
    /// User-based rate limiting entries
    user_entries: Arc<Mutex<HashMap<String, RateLimitEntry>>>,
    /// Global rate limiting entry
    global_entry: Arc<Mutex<RateLimitEntry>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            ip_entries: Arc::new(Mutex::new(HashMap::new())),
            user_entries: Arc::new(Mutex::new(HashMap::new())),
            global_entry: Arc::new(Mutex::new(RateLimitEntry::new())),
        }
    }

    /// Check if an IP-based request should be allowed
    pub fn check_ip_limit(&self, ip: &str, config: &RateLimitConfig) -> RateLimitResult {
        let mut entries = self.ip_entries.lock().unwrap();
        let entry = entries
            .entry(ip.to_string())
            .or_insert_with(RateLimitEntry::new);

        if entry.check_and_record(config) {
            RateLimitResult::Allowed {
                remaining: config.max_requests.saturating_sub(entry.current_count()),
                reset_time: entry.time_until_reset(config.window_duration),
            }
        } else {
            RateLimitResult::Exceeded {
                retry_after: entry.time_until_reset(config.window_duration),
            }
        }
    }

    /// Check if a user-based request should be allowed
    pub fn check_user_limit(&self, user_id: &str, config: &RateLimitConfig) -> RateLimitResult {
        let mut entries = self.user_entries.lock().unwrap();
        let entry = entries
            .entry(user_id.to_string())
            .or_insert_with(RateLimitEntry::new);

        if entry.check_and_record(config) {
            RateLimitResult::Allowed {
                remaining: config.max_requests.saturating_sub(entry.current_count()),
                reset_time: entry.time_until_reset(config.window_duration),
            }
        } else {
            RateLimitResult::Exceeded {
                retry_after: entry.time_until_reset(config.window_duration),
            }
        }
    }

    /// Check global rate limit
    pub fn check_global_limit(&self, config: &RateLimitConfig) -> RateLimitResult {
        let mut entry = self.global_entry.lock().unwrap();

        if entry.check_and_record(config) {
            RateLimitResult::Allowed {
                remaining: config.max_requests.saturating_sub(entry.current_count()),
                reset_time: entry.time_until_reset(config.window_duration),
            }
        } else {
            RateLimitResult::Exceeded {
                retry_after: entry.time_until_reset(config.window_duration),
            }
        }
    }

    /// Cleanup old entries to prevent memory leaks
    pub fn cleanup_old_entries(&self) {
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour

        // Cleanup IP entries
        {
            let mut entries = self.ip_entries.lock().unwrap();
            entries.retain(|_, entry| entry.window_start > cutoff || !entry.requests.is_empty());
        }

        // Cleanup user entries
        {
            let mut entries = self.user_entries.lock().unwrap();
            entries.retain(|_, entry| entry.window_start > cutoff || !entry.requests.is_empty());
        }
    }
}

/// Result of a rate limit check
#[derive(Debug)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed {
        /// Remaining requests in current window
        remaining: u32,
        /// Time until window resets
        reset_time: Duration,
    },
    /// Request exceeds rate limit
    Exceeded {
        /// Time to wait before retrying
        retry_after: Duration,
    },
}

/// Global rate limiter instance
static RATE_LIMITER: once_cell::sync::Lazy<RateLimiter> =
    once_cell::sync::Lazy::new(RateLimiter::new);

/// Rate limiting middleware factory
pub fn create_rate_limit_layer() -> tower::util::MapRequestLayer<fn(Request) -> Request> {
    tower::util::MapRequestLayer::new(|request: Request| {
        // Add rate limit info to request extensions if needed
        request
    })
}

/// IP-based rate limiting middleware
pub async fn ip_rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let ip = addr.ip().to_string();

    // Determine rate limit config based on request path
    let config = get_rate_limit_config(request.uri().path());

    match RATE_LIMITER.check_ip_limit(&ip, &config) {
        RateLimitResult::Allowed {
            remaining,
            reset_time,
        } => {
            debug!("IP {} allowed, {} requests remaining", ip, remaining);

            let mut response = next.run(request).await;

            // Add rate limit headers
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                config.max_requests.to_string().parse().unwrap(),
            );
            headers.insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            headers.insert(
                "X-RateLimit-Reset",
                reset_time.as_secs().to_string().parse().unwrap(),
            );

            Ok(response)
        }
        RateLimitResult::Exceeded { retry_after } => {
            warn!("IP {} rate limit exceeded", ip);

            let mut error = ApiError::rate_limit_error();
            error.message = format!(
                "Rate limit exceeded. Retry after {} seconds",
                retry_after.as_secs()
            );

            Err(error)
        }
    }
}

/// User-based rate limiting middleware (requires authentication)
pub async fn user_rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Get user context if available
    if let Some(user_context) = request.extensions().get::<UserContext>() {
        let user_id = user_context.user_id.to_string();

        // Determine rate limit config
        let config = get_user_rate_limit_config(request.uri().path(), &user_context.role);

        match RATE_LIMITER.check_user_limit(&user_id, &config) {
            RateLimitResult::Allowed {
                remaining,
                reset_time,
            } => {
                debug!("User {} allowed, {} requests remaining", user_id, remaining);

                let mut response = next.run(request).await;

                // Add rate limit headers
                let headers = response.headers_mut();
                headers.insert(
                    "X-RateLimit-User-Limit",
                    config.max_requests.to_string().parse().unwrap(),
                );
                headers.insert(
                    "X-RateLimit-User-Remaining",
                    remaining.to_string().parse().unwrap(),
                );
                headers.insert(
                    "X-RateLimit-User-Reset",
                    reset_time.as_secs().to_string().parse().unwrap(),
                );

                Ok(response)
            }
            RateLimitResult::Exceeded { retry_after } => {
                warn!("User {} rate limit exceeded", user_id);

                let mut error = ApiError::rate_limit_error();
                error.message = format!(
                    "User rate limit exceeded. Retry after {} seconds",
                    retry_after.as_secs()
                );

                Err(error)
            }
        }
    } else {
        // No user context, proceed without user-based rate limiting
        Ok(next.run(request).await)
    }
}

/// Get rate limit configuration based on request path
fn get_rate_limit_config(path: &str) -> RateLimitConfig {
    if path.starts_with("/api/v1/auth") {
        RateLimitConfig::auth_endpoints()
    } else if path.starts_with("/api/v1/admin") {
        RateLimitConfig::admin_endpoints()
    } else if path.starts_with("/api/v1/messages") || path.starts_with("/api/v1/rooms") {
        RateLimitConfig::message_endpoints()
    } else {
        RateLimitConfig::general_endpoints()
    }
}

/// Get user-based rate limit configuration
fn get_user_rate_limit_config(
    path: &str,
    role: &crate::server::api::models::auth::UserRole,
) -> RateLimitConfig {
    use crate::server::api::models::auth::UserRole;

    let base_config = get_rate_limit_config(path);

    // Admins get higher limits
    match role {
        UserRole::Admin => RateLimitConfig {
            max_requests: base_config.max_requests * 2,
            window_duration: base_config.window_duration,
            burst_allowance: base_config.burst_allowance * 2,
        },
        UserRole::Moderator => RateLimitConfig {
            max_requests: (base_config.max_requests * 3) / 2,
            window_duration: base_config.window_duration,
            burst_allowance: (base_config.burst_allowance * 3) / 2,
        },
        _ => base_config,
    }
}

/// Background task to cleanup old rate limit entries
pub async fn cleanup_rate_limiter() {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

    loop {
        interval.tick().await;
        RATE_LIMITER.cleanup_old_entries();
        debug!("Cleaned up old rate limiter entries");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limit_entry_basic() {
        let mut entry = RateLimitEntry::new();
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(60),
            burst_allowance: 1,
        };

        // Should allow first 3 requests
        assert!(entry.check_and_record(&config));
        assert!(entry.check_and_record(&config));
        assert!(entry.check_and_record(&config));

        // Should allow 1 burst request
        assert!(entry.check_and_record(&config));

        // Should deny further requests
        assert!(!entry.check_and_record(&config));
    }

    #[test]
    fn test_rate_limit_entry_cleanup() {
        let mut entry = RateLimitEntry::new();
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
            burst_allowance: 0,
        };

        // Use up the limit
        assert!(entry.check_and_record(&config));
        assert!(entry.check_and_record(&config));
        assert!(!entry.check_and_record(&config));

        // Wait for window to reset
        std::thread::sleep(Duration::from_millis(150));

        // Should allow requests again
        assert!(entry.check_and_record(&config));
        assert!(entry.check_and_record(&config));
    }

    #[test]
    fn test_rate_limiter_ip_limiting() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
            burst_allowance: 0,
        };

        // Test IP-based limiting
        match limiter.check_ip_limit("127.0.0.1", &config) {
            RateLimitResult::Allowed { remaining, .. } => {
                assert_eq!(remaining, 1); // 2 - 1 = 1
            }
            _ => panic!("Should be allowed"),
        }

        match limiter.check_ip_limit("127.0.0.1", &config) {
            RateLimitResult::Allowed { remaining, .. } => {
                assert_eq!(remaining, 0); // 2 - 2 = 0
            }
            _ => panic!("Should be allowed"),
        }

        // Third request should be denied
        match limiter.check_ip_limit("127.0.0.1", &config) {
            RateLimitResult::Exceeded { .. } => {
                // Expected
            }
            _ => panic!("Should be exceeded"),
        }

        // Different IP should be allowed
        match limiter.check_ip_limit("127.0.0.2", &config) {
            RateLimitResult::Allowed { .. } => {
                // Expected
            }
            _ => panic!("Should be allowed for different IP"),
        }
    }

    #[test]
    fn test_rate_limit_config_creation() {
        let auth_config = RateLimitConfig::auth_endpoints();
        assert_eq!(auth_config.max_requests, 5);
        assert_eq!(auth_config.burst_allowance, 2);

        let general_config = RateLimitConfig::general_endpoints();
        assert_eq!(general_config.max_requests, 1000);
        assert_eq!(general_config.burst_allowance, 50);

        let admin_config = RateLimitConfig::admin_endpoints();
        assert_eq!(admin_config.max_requests, 20);
        assert_eq!(admin_config.burst_allowance, 5);
    }

    #[test]
    fn test_get_rate_limit_config_by_path() {
        let auth_config = get_rate_limit_config("/api/v1/auth/login");
        assert_eq!(auth_config.max_requests, 5);

        let admin_config = get_rate_limit_config("/api/v1/admin/users");
        assert_eq!(admin_config.max_requests, 20);

        let message_config = get_rate_limit_config("/api/v1/messages/search");
        assert_eq!(message_config.max_requests, 200);

        let general_config = get_rate_limit_config("/api/v1/users/profile");
        assert_eq!(general_config.max_requests, 1000);
    }
}
