//! Common API models and error handling
//!
//! This module provides shared types used throughout the API, including
//! standardized error responses, common validation types, and utility
//! structures for consistent API behavior.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Standard API error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Additional error details
    #[serde(default)]
    pub details: Option<ErrorDetails>,

    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,

    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            timestamp: Utc::now(),
            request_id: None,
        }
    }

    /// Add error details
    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = Some(details);
        self
    }

    /// Add request ID for tracing
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Create validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    /// Create authentication error
    pub fn auth_error(message: impl Into<String>) -> Self {
        Self::new("AUTH_ERROR", message)
    }

    /// Create authorization error
    pub fn forbidden_error(message: impl Into<String>) -> Self {
        Self::new("FORBIDDEN", message)
    }

    /// Create not found error
    pub fn not_found_error(resource: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", format!("{} not found", resource.into()))
    }

    /// Create internal server error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }

    /// Create conflict error
    pub fn conflict_error(message: impl Into<String>) -> Self {
        Self::new("CONFLICT", message)
    }

    /// Create rate limit error
    pub fn rate_limit_error() -> Self {
        Self::new("RATE_LIMIT_EXCEEDED", "Too many requests")
    }

    /// Create bad request error
    pub fn bad_request_error(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }
}

/// Additional error details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetails {
    /// Field-specific validation errors
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub field_errors: HashMap<String, Vec<String>>,

    /// Additional context information
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, serde_json::Value>,

    /// Suggested actions for the client
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

impl ErrorDetails {
    pub fn new() -> Self {
        Self {
            field_errors: HashMap::new(),
            context: HashMap::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_field_error(mut self, field: String, error: String) -> Self {
        self.field_errors.entry(field).or_default().push(error);
        self
    }

    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.insert(key, value);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestions.push(suggestion);
        self
    }
}

impl Default for ErrorDetails {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement IntoResponse for ApiError to use with Axum
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = match self.code.as_str() {
            "VALIDATION_ERROR" | "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            "AUTH_ERROR" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "CONFLICT" => StatusCode::CONFLICT,
            "RATE_LIMIT_EXCEEDED" => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, Json(self)).into_response()
    }
}

/// Convert from storage errors to API errors
impl From<crate::server::storage::StorageError> for ApiError {
    fn from(err: crate::server::storage::StorageError) -> Self {
        use crate::server::storage::StorageError;

        match err {
            StorageError::NotFound(resource) => ApiError::not_found_error(resource),
            StorageError::AlreadyExists(resource) => {
                ApiError::conflict_error(format!("{} already exists", resource))
            }
            StorageError::InvalidInput(msg) => ApiError::validation_error(msg),
            StorageError::PermissionDenied(msg) => ApiError::forbidden_error(msg),
            StorageError::Database(e) => {
                tracing::error!("Database error: {}", e);
                ApiError::internal_error("Database operation failed")
            }
            StorageError::Serialization(e) => {
                tracing::error!("Serialization error: {}", e);
                ApiError::internal_error("Data serialization failed")
            }
            StorageError::Configuration(e) => {
                tracing::error!("Configuration error: {}", e);
                ApiError::internal_error("Configuration error")
            }
        }
    }
}

/// Success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse<T> {
    /// Response data
    pub data: T,

    /// Success message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            message: None,
            timestamp: Utc::now(),
            request_id: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

/// Empty response for operations that don't return data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmptyResponse {
    /// Operation success message
    pub message: String,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

impl EmptyResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            timestamp: Utc::now(),
        }
    }
}

/// ID response for operations that return a single ID
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdResponse {
    /// The created/updated resource ID
    pub id: Uuid,

    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

impl IdResponse {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            message: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// Utility type for API result handling
pub type ApiResult<T> = Result<T, ApiError>;

/// Helper trait for converting results to API responses
pub trait IntoApiResult<T> {
    fn into_api_result(self) -> ApiResult<T>;
}

impl<T, E> IntoApiResult<T> for Result<T, E>
where
    E: Into<ApiError>,
{
    fn into_api_result(self) -> ApiResult<T> {
        self.map_err(Into::into)
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status
    pub status: HealthStatus,

    /// Service name
    pub service: String,

    /// Service version
    pub version: String,

    /// Component health checks
    pub components: HashMap<String, ComponentHealth>,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some components degraded but service operational
    Degraded,
    /// Service not operational
    Unhealthy,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// Component status
    pub status: HealthStatus,

    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Last check timestamp
    pub last_check: DateTime<Utc>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ComponentHealth {
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            error: None,
            last_check: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn degraded(error: String) -> Self {
        Self {
            status: HealthStatus::Degraded,
            error: Some(error),
            last_check: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn unhealthy(error: String) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            error: Some(error),
            last_check: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("TEST_ERROR", "Test message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_api_error_with_details() {
        let details = ErrorDetails::new()
            .with_field_error("email".to_string(), "Invalid format".to_string())
            .with_suggestion("Use a valid email address".to_string());

        let error = ApiError::validation_error("Invalid input").with_details(details);

        assert_eq!(error.code, "VALIDATION_ERROR");
        assert!(error.details.is_some());

        let details = error.details.unwrap();
        assert!(details.field_errors.contains_key("email"));
        assert_eq!(details.suggestions.len(), 1);
    }

    #[test]
    fn test_success_response() {
        let response =
            SuccessResponse::new("test data").with_message("Operation completed".to_string());

        assert_eq!(response.data, "test data");
        assert_eq!(response.message, Some("Operation completed".to_string()));
    }

    #[test]
    fn test_health_response() {
        let mut components = HashMap::new();
        components.insert("database".to_string(), ComponentHealth::healthy());
        components.insert(
            "cache".to_string(),
            ComponentHealth::degraded("Connection slow".to_string()),
        );

        let health = HealthResponse {
            status: HealthStatus::Degraded,
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            components,
            timestamp: Utc::now(),
        };

        assert!(matches!(health.status, HealthStatus::Degraded));
        assert_eq!(health.components.len(), 2);
    }
}
