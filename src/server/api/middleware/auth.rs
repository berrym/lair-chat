//! Authentication middleware module
//!
//! This module provides authentication-related middleware components,
//! including JWT validation, user context extraction, and role-based
//! authorization checks.

pub use crate::server::api::middleware::{
    admin_auth_middleware, get_user_context, jwt_auth_middleware, moderator_auth_middleware,
    optional_jwt_auth_middleware, require_user_context, UserContext,
};

// Re-export for convenience
pub use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
