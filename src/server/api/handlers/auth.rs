//! Authentication handlers for user registration, login, and token management
//!
//! This module provides HTTP handlers for all authentication-related operations
//! including user registration, login, logout, token refresh, and password
//! management. All handlers follow REST API conventions and return standardized
//! JSON responses.

use axum::{extract::State, http::StatusCode, response::Json, Extension};
use chrono::{Duration, Utc};
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    api::{
        handlers::{auth_helpers, responses, validation},
        middleware::UserContext,
        models::{
            auth::*,
            common::{ApiError, ApiResult, EmptyResponse, SuccessResponse},
        },
        ApiState,
    },
    storage::{
        models::{Session, User, UserRole as StorageUserRole, UserStatus as StorageUserStatus},
        StorageError,
    },
};

/// Register a new user account
///
/// Creates a new user account with the provided credentials and returns
/// authentication tokens upon successful registration.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = AuthResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 409, description = "Username or email already exists", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<ApiState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    info!(
        "User registration attempt for username: {}",
        request.username
    );

    // Validate request data
    validation::validate_request(&request)?;

    // Hash password
    let password_hash = auth_helpers::hash_password(&request.password)?;

    // Check if username or email already exists
    let existing_by_username = state
        .storage
        .users()
        .get_user_by_username(&request.username)
        .await;

    let existing_by_email = state
        .storage
        .users()
        .get_user_by_email(&request.email)
        .await;

    match (existing_by_username, existing_by_email) {
        (Ok(Some(_)), _) => {
            warn!(
                "Registration failed: username '{}' already exists",
                request.username
            );
            return Err(ApiError::conflict_error("Username already exists"));
        }
        (_, Ok(Some(_))) => {
            warn!(
                "Registration failed: email '{}' already exists",
                request.email
            );
            return Err(ApiError::conflict_error("Email already exists"));
        }
        _ => {} // Continue with registration
    }

    // Create new user
    let new_user = User {
        id: Uuid::new_v4(),
        username: request.username.clone(),
        email: request.email.clone(),
        password_hash,
        display_name: request
            .display_name
            .unwrap_or_else(|| request.username.clone()),
        role: StorageUserRole::User,
        status: StorageUserStatus::Active,
        avatar_url: None,
        timezone: request.timezone.unwrap_or_else(|| "UTC".to_string()),
        last_login: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        metadata: serde_json::json!({}),
    };

    // Store user in database
    let user_id = state
        .storage
        .users()
        .create_user(&new_user)
        .await
        .map_err(|e| {
            error!("Failed to create user: {}", e);
            match e {
                StorageError::AlreadyExists(_) => ApiError::conflict_error("User already exists"),
                _ => ApiError::internal_error("Failed to create user account"),
            }
        })?;

    info!("User created successfully with ID: {}", user_id);

    // Create session
    let session = Session {
        id: Uuid::new_v4(),
        user_id,
        device_name: None,
        device_type: None,
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        is_active: true,
        last_activity: Utc::now(),
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
        metadata: serde_json::json!({}),
    };

    let session_id = state
        .storage
        .sessions()
        .create_session(&session)
        .await
        .map_err(|e| {
            error!("Failed to create session: {}", e);
            ApiError::internal_error("Failed to create session")
        })?;

    // Generate tokens
    let access_token = auth_helpers::generate_access_token(
        user_id,
        new_user.username.clone(),
        convert_user_role(&new_user.role),
        session_id,
        &state.jwt_secret,
    )?;

    let refresh_token =
        auth_helpers::generate_refresh_token(user_id, session_id, &state.jwt_secret)?;

    // Create response
    let auth_response = AuthResponse::new(
        access_token,
        refresh_token,
        3600, // 1 hour
        AuthUserInfo {
            id: user_id,
            username: new_user.username,
            email: new_user.email,
            display_name: new_user.display_name,
            role: convert_user_role(&new_user.role),
            status: convert_user_status(&new_user.status),
            created_at: new_user.created_at,
        },
        SessionInfo {
            id: session_id,
            created_at: session.created_at,
            expires_at: session.expires_at,
            device: None,
        },
    );

    info!(
        "User registration completed successfully for: {}",
        request.username
    );
    Ok((StatusCode::CREATED, Json(auth_response)))
}

/// Login with username/email and password
///
/// Authenticates a user with their credentials and returns JWT tokens
/// for subsequent API requests.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Invalid credentials", body = ApiError),
        (status = 429, description = "Too many login attempts", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<ApiState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    info!("Login attempt for identifier: {}", request.identifier);

    // Validate request data
    validation::validate_request(&request)?;

    // Find user by username or email
    let user = if request.identifier.contains('@') {
        state
            .storage
            .users()
            .get_user_by_email(&request.identifier)
            .await
    } else {
        state
            .storage
            .users()
            .get_user_by_username(&request.identifier)
            .await
    }
    .map_err(|e| {
        error!("Database error during login: {}", e);
        ApiError::internal_error("Authentication failed")
    })?
    .ok_or_else(|| {
        warn!(
            "Login failed: user not found for identifier '{}'",
            request.identifier
        );
        ApiError::auth_error("Invalid username or password")
    })?;

    // Check user status
    match user.status {
        StorageUserStatus::Active => {}
        StorageUserStatus::Suspended => {
            warn!("Login blocked: user {} is suspended", user.username);
            return Err(ApiError::forbidden_error("Account is suspended"));
        }
        StorageUserStatus::Banned => {
            warn!("Login blocked: user {} is banned", user.username);
            return Err(ApiError::forbidden_error("Account is banned"));
        }
        StorageUserStatus::PendingVerification => {
            warn!(
                "Login blocked: user {} needs email verification",
                user.username
            );
            return Err(ApiError::forbidden_error("Email verification required"));
        }
        StorageUserStatus::Deactivated => {
            warn!(
                "Login blocked: user {} account is deactivated",
                user.username
            );
            return Err(ApiError::forbidden_error("Account is deactivated"));
        }
    }

    // Verify password
    if !auth_helpers::verify_password(&request.password, &user.password_hash)? {
        warn!(
            "Login failed: invalid password for user '{}'",
            user.username
        );
        return Err(ApiError::auth_error("Invalid username or password"));
    }

    // Update last login time
    let mut updated_user = user.clone();
    updated_user.last_login = Some(Utc::now());
    updated_user.updated_at = Utc::now();

    state
        .storage
        .users()
        .update_user(&updated_user)
        .await
        .map_err(|e| {
            error!("Failed to update user last login: {}", e);
            // Don't fail the login for this
        })
        .ok();

    // Create session
    let session_expires = if request.remember_me {
        Utc::now() + Duration::days(30) // 30 days for "remember me"
    } else {
        Utc::now() + Duration::days(7) // 7 days for normal login
    };

    let session = Session {
        id: Uuid::new_v4(),
        user_id: user.id,
        device_name: request.device_info.as_ref().map(|d| d.name.clone()),
        device_type: request.device_info.as_ref().map(|d| d.device_type.clone()),
        ip_address: None, // TODO: Extract from request headers
        user_agent: None, // TODO: Extract from request headers
        is_active: true,
        last_activity: Utc::now(),
        expires_at: session_expires,
        created_at: Utc::now(),
        metadata: serde_json::json!({
            "remember_me": request.remember_me,
            "device_info": request.device_info
        }),
    };

    let session_id = state
        .storage
        .sessions()
        .create_session(&session)
        .await
        .map_err(|e| {
            error!("Failed to create session: {}", e);
            ApiError::internal_error("Failed to create session")
        })?;

    // Generate tokens
    let access_token = auth_helpers::generate_access_token(
        user.id,
        user.username.clone(),
        convert_user_role(&user.role),
        session_id,
        &state.jwt_secret,
    )?;

    let refresh_token =
        auth_helpers::generate_refresh_token(user.id, session_id, &state.jwt_secret)?;

    // Create response
    let auth_response = AuthResponse::new(
        access_token,
        refresh_token,
        3600, // 1 hour
        AuthUserInfo {
            id: user.id,
            username: user.username.clone(),
            email: user.email,
            display_name: user.display_name,
            role: convert_user_role(&user.role),
            status: convert_user_status(&user.status),
            created_at: user.created_at,
        },
        SessionInfo {
            id: session_id,
            created_at: session.created_at,
            expires_at: session.expires_at,
            device: request.device_info,
        },
    );

    info!("Login successful for user: {}", user.username);
    Ok(Json(auth_response))
}

/// Refresh access token using refresh token
///
/// Issues a new access token using a valid refresh token. Optionally
/// can rotate the refresh token for enhanced security.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Invalid or expired refresh token", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "auth"
)]
pub async fn refresh(
    State(state): State<ApiState>,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<Json<RefreshResponse>> {
    debug!("Token refresh attempt");

    // Validate request data
    validation::validate_request(&request)?;

    // Decode and validate refresh token
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<JwtClaims>(
        &request.refresh_token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        warn!("Invalid refresh token: {}", e);
        ApiError::auth_error("Invalid refresh token")
    })?;

    let claims = token_data.claims;

    // Verify this is a refresh token
    if !matches!(claims.token_type, TokenType::Refresh) {
        warn!("Token refresh attempt with non-refresh token");
        return Err(ApiError::auth_error("Invalid token type"));
    }

    // Parse user and session IDs
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        error!("Invalid user ID in refresh token");
        ApiError::auth_error("Invalid token")
    })?;

    let session_id = Uuid::parse_str(&claims.session_id).map_err(|_| {
        error!("Invalid session ID in refresh token");
        ApiError::auth_error("Invalid token")
    })?;

    // Verify session exists and is active
    let session = state
        .storage
        .sessions()
        .get_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session for token refresh: {}", e);
            ApiError::internal_error("Session validation failed")
        })?
        .ok_or_else(|| {
            warn!("Token refresh failed: session not found");
            ApiError::auth_error("Invalid session")
        })?;

    if !session.is_active {
        warn!("Token refresh failed: session is inactive");
        return Err(ApiError::auth_error("Session is no longer active"));
    }

    if session.expires_at < Utc::now() {
        warn!("Token refresh failed: session has expired");
        return Err(ApiError::auth_error("Session has expired"));
    }

    // Get user information
    let user = state
        .storage
        .users()
        .get_user(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user for token refresh: {}", e);
            ApiError::internal_error("User validation failed")
        })?
        .ok_or_else(|| {
            warn!("Token refresh failed: user not found");
            ApiError::auth_error("User not found")
        })?;

    // Check user is still active
    if !matches!(user.status, StorageUserStatus::Active) {
        warn!("Token refresh failed: user account is not active");
        return Err(ApiError::forbidden_error("Account is not active"));
    }

    // Update session activity
    let mut updated_session = session;
    updated_session.last_activity = Utc::now();

    state
        .storage
        .sessions()
        .update_session(&updated_session)
        .await
        .map_err(|e| {
            error!("Failed to update session activity: {}", e);
            // Don't fail the refresh for this
        })
        .ok();

    // Generate new access token
    let access_token = auth_helpers::generate_access_token(
        user.id,
        user.username,
        convert_user_role(&user.role),
        session_id,
        &state.jwt_secret,
    )?;

    // Optionally rotate refresh token (recommended for security)
    let new_refresh_token = if state.config.security.rotate_refresh_tokens {
        Some(auth_helpers::generate_refresh_token(
            user.id,
            session_id,
            &state.jwt_secret,
        )?)
    } else {
        None
    };

    let response = RefreshResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: new_refresh_token,
    };

    debug!("Token refresh successful for user: {}", user_id);
    Ok(Json(response))
}

/// Logout and invalidate current session
///
/// Invalidates the current session and optionally all sessions for the user.
/// This effectively logs the user out from the current device or all devices.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "auth",
    security(
        ("Bearer" = [])
    )
)]
pub async fn logout(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<LogoutRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Logout request from user: {}", user_context.username);

    if request.all_devices {
        // Deactivate all sessions for the user
        match state
            .storage
            .sessions()
            .deactivate_user_sessions(&user_context.user_id)
            .await
        {
            Ok(count) => {
                info!(
                    "Deactivated {} sessions for user: {}",
                    count, user_context.username
                );
            }
            Err(e) => {
                error!("Failed to deactivate all sessions: {}", e);
                return Err(ApiError::internal_error("Logout failed"));
            }
        }
    } else {
        // Deactivate only current session
        match state
            .storage
            .sessions()
            .deactivate_session(&user_context.session_id)
            .await
        {
            Ok(()) => {
                info!("Deactivated session for user: {}", user_context.username);
            }
            Err(e) => {
                error!("Failed to deactivate session: {}", e);
                return Err(ApiError::internal_error("Logout failed"));
            }
        }
    }

    let message = if request.all_devices {
        "Logged out from all devices successfully"
    } else {
        "Logged out successfully"
    };

    Ok(Json(EmptyResponse::new(message)))
}

/// Change user password
///
/// Changes the user's password after verifying the current password.
/// All sessions except the current one are invalidated for security.
#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = EmptyResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Current password is incorrect", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "auth",
    security(
        ("Bearer" = [])
    )
)]
pub async fn change_password(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<ChangePasswordRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!(
        "Password change request from user: {}",
        user_context.username
    );

    // Validate request data
    validation::validate_request(&request)?;

    // Get current user
    let user = state
        .storage
        .users()
        .get_user(&user_context.user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user for password change: {}", e);
            ApiError::internal_error("Password change failed")
        })?
        .ok_or_else(|| ApiError::auth_error("User not found"))?;

    // Verify current password
    if !auth_helpers::verify_password(&request.current_password, &user.password_hash)? {
        warn!(
            "Password change failed: incorrect current password for user '{}'",
            user.username
        );
        return Err(ApiError::auth_error("Current password is incorrect"));
    }

    // Hash new password
    let new_password_hash = auth_helpers::hash_password(&request.new_password)?;

    // Update user password
    let mut updated_user = user;
    updated_user.password_hash = new_password_hash;
    updated_user.updated_at = Utc::now();

    state
        .storage
        .users()
        .update_user(&updated_user)
        .await
        .map_err(|e| {
            error!("Failed to update user password: {}", e);
            ApiError::internal_error("Password change failed")
        })?;

    // Deactivate all other sessions for security (keep current session active)
    match state
        .storage
        .sessions()
        .deactivate_user_sessions_except(&user_context.user_id, &user_context.session_id)
        .await
    {
        Ok(count) => {
            info!("Deactivated {} other sessions after password change", count);
        }
        Err(e) => {
            warn!(
                "Failed to deactivate other sessions after password change: {}",
                e
            );
            // Don't fail the password change for this
        }
    }

    info!(
        "Password changed successfully for user: {}",
        user_context.username
    );
    Ok(Json(EmptyResponse::new("Password changed successfully")))
}

// Helper functions

/// Convert storage user role to API user role
fn convert_user_role(role: &StorageUserRole) -> UserRole {
    match role {
        StorageUserRole::Admin => UserRole::Admin,
        StorageUserRole::Moderator => UserRole::Moderator,
        StorageUserRole::User => UserRole::User,
        StorageUserRole::Guest => UserRole::Guest,
    }
}

/// Convert storage user status to API user status
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
    use crate::server::storage::models::{
        UserRole as StorageUserRole, UserStatus as StorageUserStatus,
    };

    #[test]
    fn test_user_role_conversion() {
        assert!(matches!(
            convert_user_role(&StorageUserRole::Admin),
            UserRole::Admin
        ));
        assert!(matches!(
            convert_user_role(&StorageUserRole::Moderator),
            UserRole::Moderator
        ));
        assert!(matches!(
            convert_user_role(&StorageUserRole::User),
            UserRole::User
        ));
        assert!(matches!(
            convert_user_role(&StorageUserRole::Guest),
            UserRole::Guest
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
        assert!(matches!(
            convert_user_status(&StorageUserStatus::Banned),
            UserStatus::Banned
        ));
        assert!(matches!(
            convert_user_status(&StorageUserStatus::PendingVerification),
            UserStatus::PendingVerification
        ));
        assert!(matches!(
            convert_user_status(&StorageUserStatus::Deactivated),
            UserStatus::Deactivated
        ));
    }
}
