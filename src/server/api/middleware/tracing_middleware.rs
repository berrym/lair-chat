//! Tracing middleware for API request logging and observability
//!
//! This module provides comprehensive request tracing and logging middleware
//! for the API server. It captures request/response details, performance
//! metrics, and error information for observability and debugging.

use axum::{
    extract::Request,
    http::{Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{debug, error, info, warn, Span};

use crate::server::api::middleware::{get_request_id, RequestId, UserContext};

/// Request tracing information
#[derive(Debug, Clone)]
pub struct RequestTrace {
    /// Request ID for correlation
    pub request_id: String,
    /// HTTP method
    pub method: Method,
    /// Request URI
    pub uri: Uri,
    /// Request start time
    pub start_time: Instant,
    /// User ID if authenticated
    pub user_id: Option<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
}

impl RequestTrace {
    pub fn new(request: &Request) -> Self {
        let request_id = get_request_id(request)
            .map(|id| id.to_string())
            .unwrap_or_else(|| RequestId::generate().to_string());

        let user_id = request
            .extensions()
            .get::<UserContext>()
            .map(|ctx| ctx.user_id.to_string());

        let user_agent = request
            .headers()
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_string());

        Self {
            request_id,
            method: request.method().clone(),
            uri: request.uri().clone(),
            start_time: Instant::now(),
            user_id,
            client_ip: None, // Will be set by IP extraction middleware
            user_agent,
        }
    }

    /// Calculate request duration
    pub fn duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Check if request is slow (over threshold)
    pub fn is_slow(&self, threshold_ms: u64) -> bool {
        self.duration().as_millis() > threshold_ms as u128
    }
}

/// Request tracing middleware with comprehensive logging
pub async fn tracing_middleware(request: Request, next: Next) -> Response {
    let trace = RequestTrace::new(&request);

    // Create structured logging fields
    let span = tracing::info_span!(
        "api_request",
        request_id = %trace.request_id,
        method = %trace.method,
        uri = %trace.uri,
        user_id = trace.user_id.as_deref().unwrap_or("anonymous"),
        user_agent = trace.user_agent.as_deref().unwrap_or("unknown"),
    );

    let _enter = span.enter();

    // Log request start
    info!(
        "Request started: {} {}",
        trace.method,
        trace
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("")
    );

    // Add trace to request extensions
    let mut request = request;
    request.extensions_mut().insert(trace.clone());

    // Process request
    let response = next.run(request).await;

    // Calculate duration
    let duration = trace.duration();
    let duration_ms = duration.as_millis();

    // Get response status
    let status = response.status();

    // Log request completion with appropriate level
    match status.as_u16() {
        200..=299 => {
            if trace.is_slow(1000) {
                // Log slow successful requests as warnings
                warn!(
                    status = %status,
                    duration_ms = duration_ms,
                    "Request completed (SLOW): {} {} - {} in {}ms",
                    trace.method,
                    trace.uri.path(),
                    status,
                    duration_ms
                );
            } else {
                info!(
                    status = %status,
                    duration_ms = duration_ms,
                    "Request completed: {} {} - {} in {}ms",
                    trace.method,
                    trace.uri.path(),
                    status,
                    duration_ms
                );
            }
        }
        300..=399 => {
            debug!(
                status = %status,
                duration_ms = duration_ms,
                "Request redirected: {} {} - {} in {}ms",
                trace.method,
                trace.uri.path(),
                status,
                duration_ms
            );
        }
        400..=499 => {
            warn!(
                status = %status,
                duration_ms = duration_ms,
                "Request failed (client error): {} {} - {} in {}ms",
                trace.method,
                trace.uri.path(),
                status,
                duration_ms
            );
        }
        500..=599 => {
            error!(
                status = %status,
                duration_ms = duration_ms,
                "Request failed (server error): {} {} - {} in {}ms",
                trace.method,
                trace.uri.path(),
                status,
                duration_ms
            );
        }
        _ => {
            warn!(
                status = %status,
                duration_ms = duration_ms,
                "Request completed (unknown status): {} {} - {} in {}ms",
                trace.method,
                trace.uri.path(),
                status,
                duration_ms
            );
        }
    }

    // Add performance metrics to response headers (for development)
    let mut response = response;
    if let Ok(duration_header) = format!("{}ms", duration_ms).parse() {
        response
            .headers_mut()
            .insert("X-Response-Time", duration_header);
    }

    response
}

/// Security-focused request logging middleware
pub async fn security_tracing_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path();

    // Check for suspicious patterns
    let is_suspicious = check_suspicious_request(&request);

    if is_suspicious {
        warn!(
            method = %method,
            uri = %uri,
            "Suspicious request detected: {} {}",
            method,
            path
        );
    }

    // Log authentication attempts
    if path.starts_with("/api/v1/auth") {
        info!(
            method = %method,
            uri = %uri,
            endpoint = "auth",
            "Authentication request: {} {}",
            method,
            path
        );
    }

    // Log admin endpoint access
    if path.starts_with("/api/v1/admin") {
        let user_id = request
            .extensions()
            .get::<UserContext>()
            .map(|ctx| ctx.user_id.to_string())
            .unwrap_or_else(|| "unauthenticated".to_string());

        warn!(
            method = %method,
            uri = %uri,
            user_id = %user_id,
            endpoint = "admin",
            "Admin endpoint access: {} {} by user {}",
            method,
            path,
            user_id
        );
    }

    next.run(request).await
}

/// Check for suspicious request patterns
fn check_suspicious_request(request: &Request) -> bool {
    let uri = request.uri();
    let path = uri.path();

    // Check for common attack patterns
    let suspicious_patterns = [
        "../",         // Path traversal
        "..\\",        // Windows path traversal
        "<script",     // XSS attempts
        "javascript:", // JavaScript injection
        "sql",         // SQL injection keywords
        "union",       // SQL injection
        "select",      // SQL injection
        "drop",        // SQL injection
        "delete",      // SQL injection (in path, not method)
        "%27",         // URL encoded single quote
        "%22",         // URL encoded double quote
        "%3c",         // URL encoded less than
        "%3e",         // URL encoded greater than
    ];

    let path_lower = path.to_lowercase();
    for pattern in &suspicious_patterns {
        if path_lower.contains(pattern) {
            return true;
        }
    }

    // Check query parameters for suspicious content
    if let Some(query) = uri.query() {
        let query_lower = query.to_lowercase();
        for pattern in &suspicious_patterns {
            if query_lower.contains(pattern) {
                return true;
            }
        }
    }

    // Check for excessively long paths (potential buffer overflow)
    if path.len() > 2048 {
        return true;
    }

    // Check for excessive query parameters
    if let Some(query) = uri.query() {
        if query.len() > 4096 {
            return true;
        }
    }

    false
}

/// Performance monitoring middleware
pub async fn performance_monitoring_middleware(request: Request, next: Next) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    let duration = start_time.elapsed();
    let status = response.status();

    // Log performance metrics
    if duration.as_millis() > 1000 {
        warn!(
            method = %method,
            path = %path,
            status = %status,
            duration_ms = duration.as_millis(),
            "Slow request detected"
        );
    } else if duration.as_millis() > 500 {
        info!(
            method = %method,
            path = %path,
            status = %status,
            duration_ms = duration.as_millis(),
            "Moderately slow request"
        );
    }

    // Collect metrics for monitoring systems
    // In a real implementation, this would send metrics to a monitoring system
    debug!(
        method = %method,
        path = %path,
        status = status.as_u16(),
        duration_ms = duration.as_millis(),
        "Request metrics"
    );

    response
}

/// Error logging middleware
pub async fn error_logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = get_request_id(&request)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let response = next.run(request).await;

    // Log errors with detailed context
    if response.status().is_server_error() {
        error!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %response.status(),
            "Server error occurred"
        );
    } else if response.status().is_client_error() {
        // Log client errors at info level (they're expected)
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %response.status(),
            "Client error occurred"
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Uri};

    #[test]
    fn test_suspicious_request_detection() {
        // Create a test request with suspicious path
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/api/../admin")
            .body(())
            .unwrap();

        assert!(check_suspicious_request(&request));

        // Test normal request
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/api/v1/users")
            .body(())
            .unwrap();

        assert!(!check_suspicious_request(&request));

        // Test XSS attempt
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/api/search?q=<script>alert('xss')</script>")
            .body(())
            .unwrap();

        assert!(check_suspicious_request(&request));
    }

    #[test]
    fn test_request_trace_creation() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("http://example.com/api/v1/messages")
            .header("user-agent", "test-client/1.0")
            .body(())
            .unwrap();

        let trace = RequestTrace::new(&request);

        assert_eq!(trace.method, Method::POST);
        assert_eq!(trace.uri.path(), "/api/v1/messages");
        assert_eq!(trace.user_agent, Some("test-client/1.0".to_string()));
        assert!(trace.user_id.is_none()); // No user context in test
    }

    #[test]
    fn test_trace_duration_calculation() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/test")
            .body(())
            .unwrap();

        let trace = RequestTrace::new(&request);

        // Duration should be very small immediately after creation
        assert!(trace.duration().as_millis() < 10);

        // Simulate some processing time
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Duration should now be at least 10ms
        assert!(trace.duration().as_millis() >= 10);
    }

    #[test]
    fn test_slow_request_detection() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com/test")
            .body(())
            .unwrap();

        let mut trace = RequestTrace::new(&request);

        // Simulate a request that started 2 seconds ago
        trace.start_time = Instant::now() - std::time::Duration::from_secs(2);

        assert!(trace.is_slow(1000)); // 1000ms threshold
        assert!(!trace.is_slow(3000)); // 3000ms threshold
    }
}
