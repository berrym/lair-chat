//! API Handlers Module
//!
//! This module contains all HTTP request handlers for the REST API endpoints.
//! Handlers are organized by functionality and provide the business logic
//! for processing API requests and generating responses.
//!
//! # Architecture
//!
//! Handlers follow a consistent pattern:
//! 1. Extract and validate request data
//! 2. Authenticate and authorize the request
//! 3. Execute business logic using storage layer
//! 4. Format and return standardized responses
//!
//! # Error Handling
//!
//! All handlers use the `ApiResult<T>` type for consistent error handling.
//! Errors are automatically converted to appropriate HTTP status codes
//! and JSON error responses.

use axum::{extract::State, http::StatusCode, response::Json, Extension};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::server::api::{
    middleware::UserContext,
    models::{
        auth::*,
        common::{ApiError, ApiResult, EmptyResponse, SuccessResponse},
    },
    ApiState,
};

pub mod admin;
pub mod auth;
pub mod messages;
pub mod rooms;
pub mod sessions;
pub mod users;

// Re-export commonly used handlers
pub use admin::*;
pub use auth::*;
pub use messages::*;
pub use rooms::*;
pub use sessions::*;
pub use users::*;

/// Health check handler
pub async fn health_check(State(state): State<ApiState>) -> ApiResult<Json<Value>> {
    debug!("Health check requested");

    // Test database connectivity
    let db_status = match state.storage.health_check().await {
        Ok(_) => {
            debug!("Database connection healthy");
            "healthy"
        }
        Err(e) => {
            warn!("Database health check failed: {}", e);
            "degraded"
        }
    };

    // Get basic system information
    let response = serde_json::json!({
        "status": "ok",
        "service": "lair-chat-api",
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime": "unknown", // TODO: Track server uptime
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())
    });

    info!("Health check completed successfully");
    Ok(Json(response))
}

/// Extract user context from request (helper function)
pub fn get_current_user(Extension(user_context): Extension<UserContext>) -> UserContext {
    user_context
}

/// Require user context from request (helper function)
pub fn require_current_user(
    user_context: Option<Extension<UserContext>>,
) -> Result<UserContext, ApiError> {
    match user_context {
        Some(Extension(ctx)) => Ok(ctx),
        None => Err(ApiError::auth_error("Authentication required")),
    }
}

/// Common response helpers
pub mod responses {
    use super::*;
    use axum::response::Json;
    use uuid::Uuid;

    use crate::server::api::models::common::{EmptyResponse, IdResponse};

    /// Create a success response with data
    pub fn success<T>(data: T) -> Json<SuccessResponse<T>> {
        Json(SuccessResponse::new(data))
    }

    /// Create a success response with data and message
    pub fn success_with_message<T>(data: T, message: String) -> Json<SuccessResponse<T>> {
        Json(SuccessResponse::new(data).with_message(message))
    }

    /// Create an empty success response
    pub fn empty_success(message: String) -> Json<EmptyResponse> {
        Json(EmptyResponse::new(message))
    }

    /// Create an ID response
    pub fn id_response(id: Uuid) -> Json<IdResponse> {
        Json(IdResponse::new(id))
    }

    /// Create an ID response with message
    pub fn id_response_with_message(id: Uuid, message: String) -> Json<IdResponse> {
        Json(IdResponse::new(id).with_message(message))
    }

    /// Create a JSON response from any serializable data
    pub fn json<T: serde::Serialize>(data: T) -> Json<T> {
        Json(data)
    }
}

/// Common validation helpers
pub mod validation {
    use super::*;
    use validator::Validate;

    /// Validate request data and return validation errors
    pub fn validate_request<T: Validate>(data: &T) -> Result<(), ApiError> {
        data.validate().map_err(|errors| {
            let mut field_errors = std::collections::HashMap::new();

            for (field, field_errors_vec) in errors.field_errors() {
                let error_messages: Vec<String> = field_errors_vec
                    .iter()
                    .map(|error| {
                        error
                            .message
                            .as_ref()
                            .map(|msg| msg.to_string())
                            .unwrap_or_else(|| format!("Invalid {}", field))
                    })
                    .collect();

                field_errors.insert(field.to_string(), error_messages);
            }

            let details = crate::server::api::models::common::ErrorDetails::new()
                .field_errors
                .into_iter()
                .fold(
                    crate::server::api::models::common::ErrorDetails::new(),
                    |mut acc, (field, errors)| {
                        for error in errors {
                            acc = acc.with_field_error(field.clone(), error);
                        }
                        acc
                    },
                );

            ApiError::validation_error("Request validation failed").with_details(details)
        })
    }

    /// Validate and extract JSON request body
    pub async fn extract_and_validate_json<T: serde::de::DeserializeOwned + Validate>(
        payload: axum::extract::Json<T>,
    ) -> Result<T, ApiError> {
        let data = payload.0;
        validate_request(&data)?;
        Ok(data)
    }
}

/// Common pagination helpers
pub mod pagination {
    use super::*;
    use crate::server::api::models::{PaginatedResponse, PaginationMetadata, PaginationParams};

    /// Create a paginated response
    pub fn paginated_response<T>(
        data: Vec<T>,
        params: &PaginationParams,
        total_items: u64,
    ) -> PaginatedResponse<T> {
        let metadata = PaginationMetadata::new(params.page, params.page_size, total_items);

        PaginatedResponse {
            data,
            pagination: metadata,
        }
    }

    /// Calculate database offset from pagination parameters
    pub fn calculate_offset(params: &PaginationParams) -> u64 {
        params.page as u64 * params.page_size as u64
    }

    /// Calculate database limit from pagination parameters
    pub fn calculate_limit(params: &PaginationParams) -> u64 {
        params.page_size as u64
    }

    /// Validate pagination parameters
    pub fn validate_pagination(params: &PaginationParams) -> Result<(), ApiError> {
        if params.page_size == 0 || params.page_size > 100 {
            return Err(ApiError::validation_error(
                "Page size must be between 1 and 100",
            ));
        }

        Ok(())
    }
}

/// Common authentication helpers
pub mod auth_helpers {
    use super::*;
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use uuid::Uuid;

    use crate::server::api::models::auth::{JwtClaims, TokenType, UserRole};

    /// Generate JWT access token
    pub fn generate_access_token(
        user_id: Uuid,
        username: String,
        role: UserRole,
        session_id: Uuid,
        jwt_secret: &str,
    ) -> Result<String, ApiError> {
        let now = Utc::now();
        let exp = now + Duration::hours(1); // 1 hour expiration

        let claims = JwtClaims {
            sub: user_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "lair-chat".to_string(),
            aud: "lair-chat-api".to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
            role,
            session_id: session_id.to_string(),
            custom: std::collections::HashMap::new(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            error!("Failed to generate access token: {}", e);
            ApiError::internal_error("Token generation failed")
        })
    }

    /// Generate JWT refresh token
    pub fn generate_refresh_token(
        user_id: Uuid,
        session_id: Uuid,
        jwt_secret: &str,
    ) -> Result<String, ApiError> {
        let now = Utc::now();
        let exp = now + Duration::days(30); // 30 days expiration

        let claims = JwtClaims {
            sub: user_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "lair-chat".to_string(),
            aud: "lair-chat-api".to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Refresh,
            role: UserRole::User, // Default role for refresh tokens
            session_id: session_id.to_string(),
            custom: std::collections::HashMap::new(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            error!("Failed to generate refresh token: {}", e);
            ApiError::internal_error("Token generation failed")
        })
    }

    /// Hash password using Argon2
    pub fn hash_password(password: &str) -> Result<String, ApiError> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| {
                error!("Failed to hash password: {}", e);
                ApiError::internal_error("Password hashing failed")
            })
    }

    /// Verify password against hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
        use argon2::{password_hash::PasswordVerifier, Argon2, PasswordHash};

        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            error!("Failed to parse password hash: {}", e);
            ApiError::internal_error("Password verification failed")
        })?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::models::auth::UserRole;
    use uuid::Uuid;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = auth_helpers::hash_password(password).unwrap();

        assert_ne!(hash, password);
        assert!(hash.len() > 50); // Argon2 hashes are long

        // Verify the password
        assert!(auth_helpers::verify_password(password, &hash).unwrap());
        assert!(!auth_helpers::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_jwt_token_generation() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let jwt_secret = "test_secret_key";

        let access_token = auth_helpers::generate_access_token(
            user_id,
            "testuser".to_string(),
            UserRole::User,
            session_id,
            jwt_secret,
        )
        .unwrap();

        let refresh_token =
            auth_helpers::generate_refresh_token(user_id, session_id, jwt_secret).unwrap();

        assert!(!access_token.is_empty());
        assert!(!refresh_token.is_empty());
        assert_ne!(access_token, refresh_token);

        // Both should be valid JWT format (3 parts separated by dots)
        assert_eq!(access_token.matches('.').count(), 2);
        assert_eq!(refresh_token.matches('.').count(), 2);
    }

    #[tokio::test]
    async fn test_pagination_helpers() {
        let params = crate::server::api::models::PaginationParams {
            page: 1,
            page_size: 20,
            sort_by: None,
            sort_order: crate::server::api::models::SortOrder::Ascending,
        };

        let offset = pagination::calculate_offset(&params);
        let limit = pagination::calculate_limit(&params);

        assert_eq!(offset, 20); // page 1 * page_size 20
        assert_eq!(limit, 20);

        // Test validation
        assert!(pagination::validate_pagination(&params).is_ok());

        let invalid_params = crate::server::api::models::PaginationParams {
            page_size: 0,
            ..params
        };
        assert!(pagination::validate_pagination(&invalid_params).is_err());

        let invalid_params = crate::server::api::models::PaginationParams {
            page_size: 101,
            ..params
        };
        assert!(pagination::validate_pagination(&invalid_params).is_err());
    }

    #[test]
    fn test_paginated_response_creation() {
        let data = vec!["item1", "item2", "item3"];
        let params = crate::server::api::models::PaginationParams::default();
        let total_items = 100;

        let response = pagination::paginated_response(data.clone(), &params, total_items);

        assert_eq!(response.data, data);
        assert_eq!(response.pagination.total_items, total_items);
        assert_eq!(response.pagination.page, params.page);
        assert_eq!(response.pagination.page_size, params.page_size);
        assert_eq!(response.pagination.total_pages, 5); // 100 / 20 = 5
        assert!(response.pagination.has_next);
        assert!(!response.pagination.has_prev);
    }
}
