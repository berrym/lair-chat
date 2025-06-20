//! API Models Module
//!
//! This module contains all data structures used for API requests and responses.
//! Models are organized by functionality and include comprehensive validation
//! and serialization support.
//!
//! # Organization
//!
//! - `auth` - Authentication-related request/response models
//! - `users` - User management models
//! - `rooms` - Room and membership models
//! - `messages` - Message-related models
//! - `sessions` - Session management models
//! - `admin` - Administrative operation models
//! - `common` - Shared types and error handling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod admin;
pub mod auth;
pub mod common;
pub mod messages;
pub mod rooms;
pub mod sessions;
pub mod users;

// Re-export commonly used types
pub use admin::*;
pub use auth::*;
pub use common::*;
pub use messages::*;
pub use rooms::*;
pub use sessions::*;
pub use users::*;

/// Standard pagination parameters for list endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: u32,

    /// Number of items per page (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,

    /// Sort field
    #[serde(default)]
    pub sort_by: Option<String>,

    /// Sort direction (asc/desc)
    #[serde(default = "default_sort_order")]
    pub sort_order: SortOrder,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
            sort_by: None,
            sort_order: default_sort_order(),
        }
    }
}

/// Sort order enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Response data items
    pub data: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMetadata {
    /// Current page number (0-based)
    pub page: u32,

    /// Items per page
    pub page_size: u32,

    /// Total number of items
    pub total_items: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there are more pages
    pub has_next: bool,

    /// Whether there are previous pages
    pub has_prev: bool,
}

impl PaginationMetadata {
    pub fn new(page: u32, page_size: u32, total_items: u64) -> Self {
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;

        Self {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page + 1 < total_pages,
            has_prev: page > 0,
        }
    }
}

/// Search parameters for full-text search endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchParams {
    /// Search query string
    pub query: String,

    /// Search filters
    #[serde(default)]
    pub filters: SearchFilters,

    /// Pagination parameters
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Search filters for refining search results
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SearchFilters {
    /// Filter by date range
    pub date_range: Option<DateRange>,

    /// Filter by user ID
    pub user_id: Option<Uuid>,

    /// Filter by room ID
    pub room_id: Option<Uuid>,

    /// Additional key-value filters
    #[serde(default)]
    pub additional: std::collections::HashMap<String, String>,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateRange {
    /// Start date (inclusive)
    pub start: DateTime<Utc>,

    /// End date (exclusive)
    pub end: DateTime<Utc>,
}

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,

    /// Response metadata
    #[serde(default)]
    pub meta: ResponseMetadata,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data,
            meta: ResponseMetadata::default(),
        }
    }

    pub fn with_meta(data: T, meta: ResponseMetadata) -> Self {
        Self { data, meta }
    }
}

/// Response metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ResponseMetadata {
    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Request ID for tracing
    pub request_id: Option<String>,

    /// API version
    pub version: String,
}

impl ResponseMetadata {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            request_id: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

/// Helper functions
fn default_page_size() -> u32 {
    20
}

fn default_sort_order() -> SortOrder {
    SortOrder::Ascending
}

/// TCP server statistics access functions
/// These functions provide access to TCP server statistics for integrated monitoring

/// Global TCP server statistics reference
pub static TCP_SERVER_STATS: once_cell::sync::Lazy<
    Arc<Mutex<Option<crate::shared_types::TcpServerStats>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Update TCP server statistics
pub async fn update_tcp_stats(stats: crate::shared_types::TcpServerStats) {
    if let Ok(mut global_stats) = TCP_SERVER_STATS.try_lock() {
        *global_stats = Some(stats);
    }
}

/// Get current TCP server statistics
pub async fn get_tcp_stats() -> Option<crate::shared_types::TcpServerStats> {
    TCP_SERVER_STATS.try_lock().ok()?.clone()
}

/// Validation helpers
pub mod validation {
    use validator::ValidationError;

    /// Validate page size is within acceptable limits
    pub fn validate_page_size(page_size: u32) -> Result<(), ValidationError> {
        if page_size == 0 || page_size > 100 {
            return Err(ValidationError::new("page_size must be between 1 and 100"));
        }
        Ok(())
    }

    /// Validate search query is not empty and not too long
    pub fn validate_search_query(query: &str) -> Result<(), ValidationError> {
        if query.trim().is_empty() {
            return Err(ValidationError::new("search query cannot be empty"));
        }
        if query.len() > 500 {
            return Err(ValidationError::new(
                "search query too long (max 500 characters)",
            ));
        }
        Ok(())
    }

    /// Validate date range is logical
    pub fn validate_date_range(range: &super::DateRange) -> Result<(), ValidationError> {
        if range.start >= range.end {
            return Err(ValidationError::new("start date must be before end date"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_metadata() {
        let meta = PaginationMetadata::new(0, 20, 100);
        assert_eq!(meta.total_pages, 5);
        assert!(meta.has_next);
        assert!(!meta.has_prev);

        let meta = PaginationMetadata::new(2, 20, 100);
        assert!(meta.has_next);
        assert!(meta.has_prev);

        let meta = PaginationMetadata::new(4, 20, 100);
        assert!(!meta.has_next);
        assert!(meta.has_prev);
    }

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 0);
        assert_eq!(params.page_size, 20);
        assert!(params.sort_by.is_none());
        assert!(matches!(params.sort_order, SortOrder::Ascending));
    }

    #[test]
    fn test_response_metadata() {
        let meta = ResponseMetadata::new();
        assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
        assert!(meta.request_id.is_none());

        let meta = meta.with_request_id("test-123".to_string());
        assert_eq!(meta.request_id, Some("test-123".to_string()));
    }

    #[test]
    fn test_validation_page_size() {
        use validation::validate_page_size;

        assert!(validate_page_size(0).is_err());
        assert!(validate_page_size(1).is_ok());
        assert!(validate_page_size(50).is_ok());
        assert!(validate_page_size(100).is_ok());
        assert!(validate_page_size(101).is_err());
    }

    #[test]
    fn test_validation_search_query() {
        use validation::validate_search_query;

        assert!(validate_search_query("").is_err());
        assert!(validate_search_query("   ").is_err());
        assert!(validate_search_query("hello").is_ok());
        assert!(validate_search_query(&"a".repeat(500)).is_ok());
        assert!(validate_search_query(&"a".repeat(501)).is_err());
    }

    #[test]
    fn test_validation_date_range() {
        use chrono::{Duration, Utc};
        use validation::validate_date_range;

        let now = Utc::now();
        let later = now + Duration::hours(1);

        let valid_range = DateRange {
            start: now,
            end: later,
        };
        assert!(validate_date_range(&valid_range).is_ok());

        let invalid_range = DateRange {
            start: later,
            end: now,
        };
        assert!(validate_date_range(&invalid_range).is_err());
    }
}
