//! Request ID middleware for API request tracing
//!
//! This module provides middleware for generating and managing unique request IDs
//! that can be used for distributed tracing, logging correlation, and debugging.
//! Each request gets a unique ID that is propagated through the entire request
//! processing pipeline.

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use tracing::{info_span, Instrument};
use uuid::Uuid;

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Request ID extracted from headers or generated
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new random request ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from existing string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the request ID as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the request ID as a string
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Request ID middleware that generates or extracts request IDs
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Try to extract existing request ID from headers
    let request_id = extract_or_generate_request_id(request.headers());

    // Add request ID to request extensions for handlers to access
    request.extensions_mut().insert(request_id.clone());

    // Create a tracing span with the request ID for structured logging
    let span = info_span!(
        "http_request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    // Process the request within the tracing span
    let response = next.run(request).instrument(span).await;

    // Add request ID to response headers
    add_request_id_to_response(response, request_id)
}

/// Extract request ID from headers or generate a new one
fn extract_or_generate_request_id(headers: &HeaderMap) -> RequestId {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|s| !s.is_empty() && is_valid_request_id(s))
        .map(|s| RequestId::from_string(s.to_string()))
        .unwrap_or_else(RequestId::generate)
}

/// Add request ID to response headers
fn add_request_id_to_response(mut response: Response, request_id: RequestId) -> Response {
    if let Ok(header_value) = request_id.to_string().parse() {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }
    response
}

/// Validate that a request ID has a reasonable format
fn is_valid_request_id(id: &str) -> bool {
    // Check length (reasonable bounds)
    if id.len() < 8 || id.len() > 128 {
        return false;
    }

    // Check characters (alphanumeric, hyphens, underscores)
    id.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Extract request ID from request extensions
pub fn get_request_id(request: &Request) -> Option<&RequestId> {
    request.extensions().get::<RequestId>()
}

/// Extract request ID from request extensions (required)
pub fn require_request_id(request: &Request) -> RequestId {
    get_request_id(request)
        .cloned()
        .unwrap_or_else(RequestId::generate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();

        assert_ne!(id1.to_string(), id2.to_string());
        assert!(id1.as_str().len() > 0);
        assert!(is_valid_request_id(id1.as_str()));
    }

    #[test]
    fn test_request_id_from_string() {
        let id_str = "test-request-123";
        let request_id = RequestId::from_string(id_str.to_string());

        assert_eq!(request_id.as_str(), id_str);
        assert_eq!(request_id.to_string(), id_str);
    }

    #[test]
    fn test_valid_request_id() {
        assert!(is_valid_request_id("valid-request-id-123"));
        assert!(is_valid_request_id("12345678"));
        assert!(is_valid_request_id("test_request_456"));
        assert!(is_valid_request_id("a1b2c3d4-e5f6-7890"));

        // Invalid cases
        assert!(!is_valid_request_id(""));
        assert!(!is_valid_request_id("short"));
        assert!(!is_valid_request_id("contains spaces"));
        assert!(!is_valid_request_id("contains@special"));
        assert!(!is_valid_request_id(&"x".repeat(200))); // Too long
    }

    #[test]
    fn test_extract_request_id_from_headers() {
        let mut headers = HeaderMap::new();

        // Test with no header - should generate new ID
        let request_id = extract_or_generate_request_id(&headers);
        assert!(is_valid_request_id(request_id.as_str()));

        // Test with valid header
        let test_id = "test-request-123";
        headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static(test_id));
        let request_id = extract_or_generate_request_id(&headers);
        assert_eq!(request_id.as_str(), test_id);

        // Test with invalid header - should generate new ID
        headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("invalid id"));
        let request_id = extract_or_generate_request_id(&headers);
        assert_ne!(request_id.as_str(), "invalid id");
        assert!(is_valid_request_id(request_id.as_str()));
    }

    #[test]
    fn test_request_id_display() {
        let request_id = RequestId::from_string("test-123".to_string());
        assert_eq!(format!("{}", request_id), "test-123");
    }
}
