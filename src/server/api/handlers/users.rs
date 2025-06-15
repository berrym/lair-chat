//! User management handlers
//!
//! This module provides HTTP handlers for user-related operations including
//! profile management, settings, user search, and account operations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::server::api::{
    handlers::{responses, validation},
    middleware::UserContext,
    models::{
        common::{ApiError, ApiResult, EmptyResponse, SuccessResponse},
        users::*,
        PaginationParams,
    },
    ApiState,
};

/// Get current user profile
///
/// Returns the authenticated user's profile information including
/// username, email, display name, and account settings.
#[utoipa::path(
    get,
    path = "/api/v1/users/profile",
    responses(
        (status = 200, description = "User profile retrieved successfully", body = UserProfile),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_profile(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<UserProfile>>> {
    debug!("Profile request for user: {}", user_context.username);

    // Get user from database
    let user = state
        .storage
        .users()
        .get_user(&user_context.user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user profile: {}", e);
            ApiError::internal_error("Failed to retrieve profile")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    let profile = UserProfile {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.status),
        avatar_url: user.avatar_url,
        timezone: user.timezone,
        last_login: user.last_login,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    info!("Profile retrieved for user: {}", user_context.username);
    Ok(responses::success(profile))
}

/// Update user profile
///
/// Updates the authenticated user's profile information such as
/// display name, avatar, and timezone preferences.
#[utoipa::path(
    put,
    path = "/api/v1/users/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = UserProfile),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_profile(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<UpdateProfileRequest>,
) -> ApiResult<Json<SuccessResponse<UserProfile>>> {
    info!("Profile update request for user: {}", user_context.username);

    // Validate request data
    validation::validate_request(&request)?;

    // Get current user
    let mut user = state
        .storage
        .users()
        .get_user(&user_context.user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user for profile update: {}", e);
            ApiError::internal_error("Profile update failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    // Update fields if provided
    if let Some(display_name) = request.display_name {
        user.display_name = display_name;
    }
    if let Some(avatar_url) = request.avatar_url {
        user.avatar_url = Some(avatar_url);
    }
    if let Some(timezone) = request.timezone {
        user.timezone = timezone;
    }

    user.updated_at = chrono::Utc::now();

    // Save updated user
    state
        .storage
        .users()
        .update_user(&user)
        .await
        .map_err(|e| {
            error!("Failed to update user profile: {}", e);
            ApiError::internal_error("Profile update failed")
        })?;

    let profile = UserProfile {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.status),
        avatar_url: user.avatar_url,
        timezone: user.timezone,
        last_login: user.last_login,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    info!(
        "Profile updated successfully for user: {}",
        user_context.username
    );
    Ok(responses::success_with_message(
        profile,
        "Profile updated successfully".to_string(),
    ))
}

/// Get user settings
///
/// Returns the authenticated user's application settings and preferences.
#[utoipa::path(
    get,
    path = "/api/v1/users/settings",
    responses(
        (status = 200, description = "User settings retrieved successfully", body = UserSettings),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_settings(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<UserSettings>>> {
    debug!("Settings request for user: {}", user_context.username);

    // Get user settings (placeholder - implement actual settings storage)
    let settings = UserSettings {
        user_id: user_context.user_id,
        email_notifications: true,
        push_notifications: true,
        desktop_notifications: false,
        sound_notifications: true,
        theme: UserTheme::System,
        language: "en".to_string(),
        timezone: "UTC".to_string(),
        privacy: PrivacySettings::default(),
        updated_at: chrono::Utc::now(),
    };

    info!("Settings retrieved for user: {}", user_context.username);
    Ok(responses::success(settings))
}

/// Update user settings
///
/// Updates the authenticated user's application settings and preferences.
#[utoipa::path(
    put,
    path = "/api/v1/users/settings",
    request_body = UpdateSettingsRequest,
    responses(
        (status = 200, description = "Settings updated successfully", body = UserSettings),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_settings(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<UpdateSettingsRequest>,
) -> ApiResult<Json<SuccessResponse<UserSettings>>> {
    info!(
        "Settings update request for user: {}",
        user_context.username
    );

    // Validate request data
    validation::validate_request(&request)?;

    // Placeholder implementation - would need actual settings storage
    let settings = UserSettings {
        user_id: user_context.user_id,
        email_notifications: request.email_notifications.unwrap_or(true),
        push_notifications: request.push_notifications.unwrap_or(true),
        desktop_notifications: request.desktop_notifications.unwrap_or(false),
        sound_notifications: request.sound_notifications.unwrap_or(true),
        theme: request.theme.unwrap_or(UserTheme::System),
        language: request.language.unwrap_or_else(|| "en".to_string()),
        timezone: request.timezone.unwrap_or_else(|| "UTC".to_string()),
        privacy: request.privacy.unwrap_or_default(),
        updated_at: chrono::Utc::now(),
    };

    info!(
        "Settings updated successfully for user: {}",
        user_context.username
    );
    Ok(responses::success_with_message(
        settings,
        "Settings updated successfully".to_string(),
    ))
}

/// Search for users
///
/// Searches for users by username, display name, or email address.
/// Results are limited to prevent abuse and protect user privacy.
#[utoipa::path(
    post,
    path = "/api/v1/users/search",
    request_body = UserSearchRequest,
    responses(
        (status = 200, description = "User search completed", body = Vec<UserSearchResult>),
        (status = 400, description = "Invalid search request", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn search_users(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<UserSearchRequest>,
) -> ApiResult<Json<SuccessResponse<Vec<UserSearchResult>>>> {
    debug!("User search request: {}", request.query);

    // Validate request data
    validation::validate_request(&request)?;

    // Placeholder implementation - would need actual user search
    let results = vec![UserSearchResult {
        id: Uuid::new_v4(),
        username: "example_user".to_string(),
        display_name: "Example User".to_string(),
        avatar_url: None,
        is_online: false,
        last_seen: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
    }];

    info!("User search completed with {} results", results.len());
    Ok(responses::success(results))
}

// Helper functions for converting between storage and API types
use crate::server::api::models::auth::{UserRole, UserStatus};
use crate::server::storage::models::{
    UserRole as StorageUserRole, UserStatus as StorageUserStatus,
};

fn convert_user_role(role: &StorageUserRole) -> UserRole {
    match role {
        StorageUserRole::Admin => UserRole::Admin,
        StorageUserRole::Moderator => UserRole::Moderator,
        StorageUserRole::User => UserRole::User,
        StorageUserRole::Guest => UserRole::Guest,
    }
}

fn convert_user_status(status: &StorageUserStatus) -> UserStatus {
    match status {
        StorageUserStatus::Active => UserStatus::Active,
        StorageUserStatus::Suspended => UserStatus::Suspended,
        StorageUserStatus::Banned => UserStatus::Banned,
        StorageUserStatus::PendingVerification => UserStatus::PendingVerification,
        StorageUserStatus::Deactivated => UserStatus::Deactivated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_conversion() {
        assert!(matches!(
            convert_user_role(&StorageUserRole::Admin),
            UserRole::Admin
        ));
        assert!(matches!(
            convert_user_role(&StorageUserRole::User),
            UserRole::User
        ));
    }

    #[test]
    fn test_user_status_conversion() {
        assert!(matches!(
            convert_user_status(&StorageUserStatus::Active),
            UserStatus::Active
        ));
        assert!(matches!(
            convert_user_status(&StorageUserStatus::Suspended),
            UserStatus::Suspended
        ));
    }
}
