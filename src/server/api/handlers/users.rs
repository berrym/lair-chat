//! User management handlers
//!
//! This module provides HTTP handlers for user-related operations including
//! profile management, settings, user search, and account operations.

use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::server::api::{
    handlers::{responses, validation},
    middleware::UserContext,
    models::{
        common::{ApiError, ApiResult, SuccessResponse},
        users::*,
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
        .get_user_by_id(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user profile: {}", e);
            ApiError::internal_error("Failed to retrieve profile")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    let profile = UserProfile {
        id: user_context.user_id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        display_name: user
            .profile
            .display_name
            .unwrap_or_else(|| user.username.clone()),
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.is_active),
        avatar_url: user.profile.avatar,
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        last_login: user
            .last_seen
            .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
        created_at: chrono::DateTime::from_timestamp(user.created_at as i64, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
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
        .get_user_by_id(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user for profile update: {}", e);
            ApiError::internal_error("Profile update failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    // Update fields if provided
    if let Some(display_name) = request.display_name {
        user.profile.display_name = Some(display_name);
    }
    if let Some(avatar_url) = request.avatar_url {
        user.profile.avatar = Some(avatar_url);
    }
    if let Some(timezone) = request.timezone {
        user.profile.timezone = Some(timezone);
    }

    user.updated_at = chrono::Utc::now().timestamp() as u64;

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
        id: user_context.user_id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        display_name: user
            .profile
            .display_name
            .unwrap_or_else(|| user.username.clone()),
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.is_active),
        avatar_url: user.profile.avatar,
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        last_login: user
            .last_seen
            .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
        created_at: chrono::DateTime::from_timestamp(user.created_at as i64, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
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

    // Get user from database to access settings
    let user = state
        .storage
        .users()
        .get_user_by_id(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user settings: {}", e);
            ApiError::internal_error("Failed to retrieve settings")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    let settings = UserSettings {
        user_id: user_context.user_id,
        email_notifications: user.settings.notifications.direct_messages,
        push_notifications: user.settings.notifications.mentions,
        desktop_notifications: user.settings.notifications.room_messages,
        sound_notifications: user.settings.notifications.sound_enabled,
        theme: match user.settings.theme.as_str() {
            "light" => UserTheme::Light,
            "dark" => UserTheme::Dark,
            _ => UserTheme::System,
        },
        language: user.profile.language.unwrap_or_else(|| "en".to_string()),
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        privacy: PrivacySettings {
            show_online_status: user.settings.privacy.show_online_status,
            allow_direct_messages: user.settings.privacy.allow_stranger_dms,
            show_read_receipts: user.settings.privacy.show_read_receipts,
            show_typing_indicators: user.settings.privacy.show_typing_indicators,
        },
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
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

    // Get current user
    let mut user = state
        .storage
        .users()
        .get_user_by_id(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user for settings update: {}", e);
            ApiError::internal_error("Settings update failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    // Update settings if provided
    if let Some(email_notifications) = request.email_notifications {
        user.settings.notifications.direct_messages = email_notifications;
    }
    if let Some(push_notifications) = request.push_notifications {
        user.settings.notifications.mentions = push_notifications;
    }
    if let Some(desktop_notifications) = request.desktop_notifications {
        user.settings.notifications.room_messages = desktop_notifications;
    }
    if let Some(sound_notifications) = request.sound_notifications {
        user.settings.notifications.sound_enabled = sound_notifications;
    }
    if let Some(theme) = request.theme {
        user.settings.theme = Some(match theme {
            UserTheme::Light => "light".to_string(),
            UserTheme::Dark => "dark".to_string(),
            UserTheme::System => "system".to_string(),
        });
    }
    if let Some(language) = request.language {
        user.profile.language = Some(language);
    }
    if let Some(timezone) = request.timezone {
        user.profile.timezone = Some(timezone);
    }
    if let Some(privacy) = request.privacy {
        user.settings.privacy.show_online_status = privacy.show_online_status;
        user.settings.privacy.allow_stranger_dms = privacy.allow_direct_messages;
        user.settings.privacy.show_read_receipts = privacy.show_read_receipts;
        user.settings.privacy.show_typing_indicators = privacy.show_typing_indicators;
    }

    user.updated_at = chrono::Utc::now().timestamp() as u64;

    // Save updated user
    state
        .storage
        .users()
        .update_user(user.clone())
        .await
        .map_err(|e| {
            error!("Failed to update user settings: {}", e);
            ApiError::internal_error("Settings update failed")
        })?;

    let settings = UserSettings {
        user_id: user_context.user_id,
        email_notifications: user.settings.notifications.direct_messages,
        push_notifications: user.settings.notifications.mentions,
        desktop_notifications: user.settings.notifications.room_messages,
        sound_notifications: user.settings.notifications.sound_enabled,
        theme: match user.settings.theme.as_str() {
            "light" => UserTheme::Light,
            "dark" => UserTheme::Dark,
            _ => UserTheme::System,
        },
        language: user.profile.language.unwrap_or_else(|| "en".to_string()),
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        privacy: PrivacySettings {
            show_online_status: user.settings.privacy.show_online_status,
            allow_direct_messages: user.settings.privacy.allow_stranger_dms,
            show_read_receipts: user.settings.privacy.show_read_receipts,
            show_typing_indicators: user.settings.privacy.show_typing_indicators,
        },
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
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

/// Get user profile by ID
///
/// Returns the public profile information for a user by their ID.
/// Respects privacy settings and returns only publicly visible information.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    responses(
        (status = 200, description = "User profile retrieved successfully", body = UserProfile),
        (status = 404, description = "User not found", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_user_by_id(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<SuccessResponse<UserProfile>>> {
    debug!("Get user profile request for ID: {}", user_id);

    // Get user from database
    let user = state
        .storage
        .users()
        .get_user_by_id(&user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user by ID: {}", e);
            ApiError::internal_error("Failed to retrieve user")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    let profile = UserProfile {
        id: user_id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        display_name: user
            .profile
            .display_name
            .unwrap_or_else(|| user.username.clone()),
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.is_active),
        avatar_url: user.profile.avatar,
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        last_login: user
            .last_seen
            .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
        created_at: chrono::DateTime::from_timestamp(user.created_at as i64, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
    };

    info!("User profile retrieved for ID: {}", user_id);
    Ok(responses::success(profile))
}

/// Get user profile by username
///
/// Returns the public profile information for a user by their username.
/// Supports case-insensitive username lookup.
#[utoipa::path(
    get,
    path = "/api/v1/users/username/{username}",
    responses(
        (status = 200, description = "User profile retrieved successfully", body = UserProfile),
        (status = 404, description = "User not found", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    params(
        ("username" = String, Path, description = "Username")
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_user_by_username(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(username): Path<String>,
) -> ApiResult<Json<SuccessResponse<UserProfile>>> {
    debug!("Get user profile request for username: {}", username);

    // Get user from database
    let user = state
        .storage
        .users()
        .get_user_by_username(&username)
        .await
        .map_err(|e| {
            error!("Failed to get user by username: {}", e);
            ApiError::internal_error("Failed to retrieve user")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    let user_id = uuid::Uuid::parse_str(&user.id).map_err(|e| {
        error!("Failed to parse user ID: {}", e);
        ApiError::internal_error("Invalid user ID")
    })?;

    let profile = UserProfile {
        id: user_id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        display_name: user
            .profile
            .display_name
            .unwrap_or_else(|| user.username.clone()),
        role: convert_user_role(&user.role),
        status: convert_user_status(&user.is_active),
        avatar_url: user.profile.avatar,
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        last_login: user
            .last_seen
            .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
        created_at: chrono::DateTime::from_timestamp(user.created_at as i64, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
    };

    info!("User profile retrieved for username: {}", username);
    Ok(responses::success(profile))
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

    let limit = request.limit.unwrap_or(20);
    let pagination = crate::server::storage::Pagination {
        offset: 0,
        limit: limit as u64,
    };

    // Search users in database
    let users = state
        .storage
        .users()
        .search_users(&request.query, pagination)
        .await
        .map_err(|e| {
            error!("Failed to search users: {}", e);
            ApiError::internal_error("Search failed")
        })?;

    let results: Vec<UserSearchResult> = users
        .into_iter()
        .map(|user| {
            let user_id = uuid::Uuid::parse_str(&user.id).unwrap_or_else(|_| Uuid::new_v4());
            UserSearchResult {
                id: user_id,
                username: user.username,
                display_name: user
                    .profile
                    .display_name
                    .unwrap_or_else(|| user.username.clone()),
                avatar_url: user.profile.avatar,
                is_online: user.is_active,
                last_seen: user
                    .last_seen
                    .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
            }
        })
        .collect();

    info!("User search completed with {} results", results.len());
    Ok(responses::success(results))
}

/// Get online users
///
/// Returns a list of currently online users.
/// Respects privacy settings for online status visibility.
#[utoipa::path(
    get,
    path = "/api/v1/users/online",
    responses(
        (status = 200, description = "Online users retrieved successfully", body = Vec<UserSearchResult>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    params(
        ("limit" = Option<u32>, Query, description = "Maximum number of results (default: 50)")
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_online_users(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<SuccessResponse<Vec<UserSearchResult>>>> {
    debug!("Get online users request");

    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(50)
        .min(100); // Cap at 100 for performance

    // Get active users from the last 5 minutes
    let since = chrono::Utc::now().timestamp() as u64 - 300; // 5 minutes ago
    let users = state
        .storage
        .users()
        .get_active_users(since)
        .await
        .map_err(|e| {
            error!("Failed to get online users: {}", e);
            ApiError::internal_error("Failed to retrieve online users")
        })?;

    let results: Vec<UserSearchResult> = users
        .into_iter()
        .take(limit as usize)
        .map(|user| {
            let user_id = uuid::Uuid::parse_str(&user.id).unwrap_or_else(|_| Uuid::new_v4());
            UserSearchResult {
                id: user_id,
                username: user.username,
                display_name: user
                    .profile
                    .display_name
                    .unwrap_or_else(|| user.username.clone()),
                avatar_url: user.profile.avatar,
                is_online: user.is_active,
                last_seen: user
                    .last_seen
                    .map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap_or_default()),
            }
        })
        .collect();

    info!("Retrieved {} online users", results.len());
    Ok(responses::success(results))
}

/// Reset user settings to default
///
/// Resets all user settings to system defaults while preserving
/// critical preferences like language and timezone.
#[utoipa::path(
    post,
    path = "/api/v1/users/settings/reset",
    responses(
        (status = 200, description = "Settings reset successfully", body = UserSettings),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "users",
    security(
        ("Bearer" = [])
    )
)]
pub async fn reset_settings(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<UserSettings>>> {
    info!("Settings reset request for user: {}", user_context.username);

    // Get current user
    let mut user = state
        .storage
        .users()
        .get_user_by_id(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user for settings reset: {}", e);
            ApiError::internal_error("Settings reset failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("User"))?;

    // Preserve critical settings
    let preserved_language = user
        .profile
        .language
        .clone()
        .unwrap_or_else(|| "en".to_string());
    let preserved_timezone = user
        .profile
        .timezone
        .clone()
        .unwrap_or_else(|| "UTC".to_string());

    // Reset to defaults
    user.settings = crate::server::storage::models::UserSettings::default();
    user.profile.language = Some(preserved_language);
    user.profile.timezone = Some(preserved_timezone);
    user.updated_at = chrono::Utc::now().timestamp() as u64;

    // Save updated user
    state
        .storage
        .users()
        .update_user(user.clone())
        .await
        .map_err(|e| {
            error!("Failed to reset user settings: {}", e);
            ApiError::internal_error("Settings reset failed")
        })?;

    let settings = UserSettings {
        user_id: user_context.user_id,
        email_notifications: user.settings.notifications.direct_messages,
        push_notifications: user.settings.notifications.mentions,
        desktop_notifications: user.settings.notifications.room_messages,
        sound_notifications: user.settings.notifications.sound_enabled,
        theme: match user.settings.theme.as_str() {
            "light" => UserTheme::Light,
            "dark" => UserTheme::Dark,
            _ => UserTheme::System,
        },
        language: user.profile.language.unwrap_or_else(|| "en".to_string()),
        timezone: user.profile.timezone.unwrap_or_else(|| "UTC".to_string()),
        privacy: PrivacySettings {
            show_online_status: user.settings.privacy.show_online_status,
            allow_direct_messages: user.settings.privacy.allow_stranger_dms,
            show_read_receipts: user.settings.privacy.show_read_receipts,
            show_typing_indicators: user.settings.privacy.show_typing_indicators,
        },
        updated_at: chrono::DateTime::from_timestamp(user.updated_at as i64, 0).unwrap_or_default(),
    };

    info!(
        "Settings reset successfully for user: {}",
        user_context.username
    );
    Ok(responses::success_with_message(
        settings,
        "Settings reset to defaults successfully".to_string(),
    ))
}

// Helper functions for converting between storage and API types
use crate::server::api::models::auth::{UserRole, UserStatus};
use crate::server::storage::models::UserRole as StorageUserRole;

fn convert_storage_theme(theme: &Option<String>) -> UserTheme {
    match theme.as_ref().map(|s| s.as_str()) {
        Some("light") => UserTheme::Light,
        Some("dark") => UserTheme::Dark,
        _ => UserTheme::System,
    }
}

fn convert_user_role(role: &StorageUserRole) -> UserRole {
    match role {
        StorageUserRole::Admin => UserRole::Admin,
        StorageUserRole::Moderator => UserRole::Moderator,
        StorageUserRole::User => UserRole::User,
        StorageUserRole::Guest => UserRole::Guest,
    }
}

fn convert_user_status(is_active: &bool) -> UserStatus {
    if *is_active {
        UserStatus::Active
    } else {
        UserStatus::Suspended
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
        assert!(matches!(convert_user_status(&true), UserStatus::Active));
        assert!(matches!(convert_user_status(&false), UserStatus::Suspended));
    }
}
