//! Session management handlers
//!
//! This module provides HTTP handlers for session-related operations including
//! session listing, termination, and device management.

use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::server::api::{
    handlers::{responses, validation},
    middleware::UserContext,
    models::{
        common::{ApiError, ApiResult, EmptyResponse, SuccessResponse},
        sessions::*,
        PaginationParams,
    },
    ApiState,
};

/// Get current user's active sessions
///
/// Returns a list of all active sessions for the authenticated user
/// across all devices and platforms.
#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    responses(
        (status = 200, description = "Sessions retrieved successfully", body = Vec<ActiveSession>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_sessions(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Query(_params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<ActiveSession>>>> {
    debug!("Get sessions request for user: {}", user_context.username);

    // Get active sessions for user
    let user_sessions = state
        .storage
        .sessions()
        .get_active_user_sessions(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user sessions: {}", e);
            ApiError::internal_error("Failed to retrieve sessions")
        })?;

    // Convert to API response format
    let mut sessions = Vec::new();
    for session in user_sessions {
        let is_current = session.id == user_context.session_id.to_string();

        // Mask IP address for privacy
        let ip_address_masked = session.ip_address.as_ref().map(|ip| {
            let parts: Vec<&str> = ip.split('.').collect();
            if parts.len() == 4 {
                format!("{}.{}.{}.***", parts[0], parts[1], parts[2])
            } else {
                "***".to_string()
            }
        });

        sessions.push(ActiveSession {
            id: Uuid::parse_str(&session.id).unwrap_or_else(|_| Uuid::new_v4()),
            device_name: session.metadata.device_info.clone(),
            device_type: session.metadata.client_type.clone(),
            ip_address_masked,
            location: session.metadata.location.clone(),
            is_current,
            last_activity: chrono::DateTime::from_timestamp(session.last_activity as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
            created_at: chrono::DateTime::from_timestamp(session.created_at as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
        });
    }

    info!(
        "Retrieved {} sessions for user: {}",
        sessions.len(),
        user_context.username
    );
    Ok(responses::success(sessions))
}

/// Get current session information
///
/// Returns detailed information about the current authenticated session.
#[utoipa::path(
    get,
    path = "/api/v1/sessions/current",
    responses(
        (status = 200, description = "Current session retrieved", body = SessionInfo),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_current_session(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<SessionInfo>>> {
    debug!(
        "Get current session request for user: {}",
        user_context.username
    );

    // Get session from storage
    let session = state
        .storage
        .sessions()
        .get_session(&user_context.session_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get current session: {}", e);
            ApiError::internal_error("Failed to retrieve session information")
        })?
        .ok_or_else(|| ApiError::not_found_error("Session"))?;

    let session_info = session_to_session_info(&session)?;

    info!(
        "Current session retrieved for user: {}",
        user_context.username
    );
    Ok(responses::success(session_info))
}

/// Update current session metadata
///
/// Updates metadata for the current session such as device name and type.
#[utoipa::path(
    put,
    path = "/api/v1/sessions/current",
    request_body = UpdateSessionRequest,
    responses(
        (status = 200, description = "Session updated successfully", body = SessionInfo),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_current_session(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<UpdateSessionRequest>,
) -> ApiResult<Json<SuccessResponse<SessionInfo>>> {
    info!("Update session request for user: {}", user_context.username);

    // Validate request data
    validation::validate_request(&request)?;

    // Get current session
    let mut session = state
        .storage
        .sessions()
        .get_session(&user_context.session_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get session for update: {}", e);
            ApiError::internal_error("Session update failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("Session"))?;

    // Update session metadata
    let mut updated = false;
    if let Some(device_name) = request.device_name {
        session.metadata.device_info = Some(device_name);
        updated = true;
    }
    if let Some(device_type) = request.device_type {
        session.metadata.client_type = Some(device_type);
        updated = true;
    }

    // Save updated session only if changes were made
    if updated {
        state
            .storage
            .sessions()
            .update_session(&session)
            .await
            .map_err(|e| {
                error!("Failed to update session: {}", e);
                ApiError::internal_error("Session update failed")
            })?;
    }

    let session_info = session_to_session_info(&session)?;

    info!(
        "Session updated successfully for user: {}",
        user_context.username
    );
    Ok(responses::success_with_message(
        session_info,
        "Session updated successfully".to_string(),
    ))
}

/// Terminate a specific session
///
/// Invalidates and terminates the specified session. Users can only
/// terminate their own sessions.
#[utoipa::path(
    delete,
    path = "/api/v1/sessions/{session_id}",
    params(
        ("session_id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Session terminated successfully", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Permission denied", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn terminate_session(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Terminate session request for session: {}", session_id);

    // Verify session belongs to the user
    let session = state
        .storage
        .sessions()
        .get_session(&session_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get session for termination: {}", e);
            ApiError::internal_error("Session termination failed")
        })?
        .ok_or_else(|| ApiError::not_found_error("Session"))?;

    if session.user_id != user_context.user_id.to_string() {
        warn!(
            "User {} attempted to terminate session belonging to user {}",
            user_context.user_id, session.user_id
        );
        return Err(ApiError::forbidden_error(
            "Cannot terminate another user's session",
        ));
    }

    // Deactivate the session
    state
        .storage
        .sessions()
        .deactivate_session(&session_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to deactivate session: {}", e);
            ApiError::internal_error("Session termination failed")
        })?;

    info!("Session {} terminated successfully", session_id);
    Ok(responses::empty_success(
        "Session terminated successfully".to_string(),
    ))
}

/// Terminate all sessions except current
///
/// Invalidates all sessions for the user except the current one.
/// Useful for security purposes when credentials are compromised.
#[utoipa::path(
    post,
    path = "/api/v1/sessions/terminate-all",
    request_body = TerminateAllSessionsRequest,
    responses(
        (status = 200, description = "Sessions terminated successfully", body = EmptyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn terminate_all_sessions(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<TerminateAllSessionsRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!(
        "Terminate all sessions request for user: {}",
        user_context.username
    );

    let count = if request.keep_current {
        // Terminate all sessions except the current one
        state
            .storage
            .sessions()
            .deactivate_user_sessions_except(
                &user_context.user_id.to_string(),
                &user_context.session_id.to_string(),
            )
            .await
            .map_err(|e| {
                error!("Failed to terminate other sessions: {}", e);
                ApiError::internal_error("Session termination failed")
            })?
    } else {
        // Terminate all sessions including current
        state
            .storage
            .sessions()
            .deactivate_user_sessions(&user_context.user_id.to_string())
            .await
            .map_err(|e| {
                error!("Failed to terminate all sessions: {}", e);
                ApiError::internal_error("Session termination failed")
            })?
    };

    let message = if request.keep_current {
        format!("Terminated {} other sessions successfully", count)
    } else {
        format!("Terminated {} sessions successfully", count)
    };

    info!("{} for user: {}", message, user_context.username);
    Ok(responses::empty_success(message))
}

/// Get session statistics
///
/// Returns statistics about the user's session history and current activity.
#[utoipa::path(
    get,
    path = "/api/v1/sessions/stats",
    responses(
        (status = 200, description = "Session statistics retrieved", body = SessionStatistics),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "sessions",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_session_statistics(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<SessionStatistics>>> {
    debug!(
        "Session statistics request for user: {}",
        user_context.username
    );

    // Get session statistics from storage
    let global_stats = state
        .storage
        .sessions()
        .get_session_stats()
        .await
        .map_err(|e| {
            error!("Failed to get session statistics: {}", e);
            ApiError::internal_error("Failed to retrieve session statistics")
        })?;

    // Get user-specific session count
    let user_active_sessions = state
        .storage
        .sessions()
        .get_active_user_sessions(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to get user active sessions: {}", e);
            ApiError::internal_error("Failed to retrieve session statistics")
        })?;

    let user_total_sessions = state
        .storage
        .sessions()
        .count_user_sessions(&user_context.user_id.to_string())
        .await
        .map_err(|e| {
            error!("Failed to count user sessions: {}", e);
            ApiError::internal_error("Failed to retrieve session statistics")
        })?;

    // Find most recent session for last login
    let last_login = if !user_active_sessions.is_empty() {
        let mut sessions_clone = user_active_sessions.clone();
        sessions_clone.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Some(
            chrono::DateTime::from_timestamp(sessions_clone[0].created_at as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
        )
    } else {
        None
    };

    // Find most common device type
    let most_common_device = global_stats
        .sessions_by_client
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(device, _)| device);

    let stats = SessionStatistics {
        user_id: user_context.user_id,
        active_sessions: user_active_sessions.len() as u32,
        total_sessions: user_total_sessions,
        last_login,
        most_common_device,
        unique_locations: 1, // TODO: Calculate unique locations from session data
        updated_at: chrono::Utc::now(),
    };

    info!(
        "Session statistics retrieved for user: {}",
        user_context.username
    );
    Ok(responses::success(stats))
}

/// Helper function to convert storage Session to API SessionInfo
fn session_to_session_info(
    session: &crate::server::storage::models::Session,
) -> Result<SessionInfo, ApiError> {
    let id = Uuid::parse_str(&session.id)
        .map_err(|_| ApiError::internal_error("Invalid session ID format"))?;
    let user_id = Uuid::parse_str(&session.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let last_activity = chrono::DateTime::from_timestamp(session.last_activity as i64, 0)
        .unwrap_or_else(chrono::Utc::now);
    let expires_at = chrono::DateTime::from_timestamp(session.expires_at as i64, 0)
        .unwrap_or_else(chrono::Utc::now);
    let created_at = chrono::DateTime::from_timestamp(session.created_at as i64, 0)
        .unwrap_or_else(chrono::Utc::now);

    let metadata = serde_json::to_value(&session.metadata)
        .map_err(|_| ApiError::internal_error("Failed to serialize session metadata"))?;

    Ok(SessionInfo {
        id,
        user_id,
        device_name: session.metadata.device_info.clone(),
        device_type: session.metadata.client_type.clone(),
        ip_address: session.ip_address.clone(),
        user_agent: session.user_agent.clone(),
        is_active: session.is_active,
        last_activity,
        expires_at,
        created_at,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_session_to_session_info_conversion() {
        use crate::server::storage::models::{Session, SessionMetadata};
        use std::collections::HashMap;

        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            token: "test_token".to_string(),
            created_at: 1640995200, // 2022-01-01 00:00:00 UTC
            expires_at: 1641081600, // 2022-01-02 00:00:00 UTC
            last_activity: 1641000000,
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test User Agent".to_string()),
            is_active: true,
            metadata: SessionMetadata {
                client_type: Some("desktop".to_string()),
                client_version: Some("1.0.0".to_string()),
                device_info: Some("Test Device".to_string()),
                location: Some("Test Location".to_string()),
                custom: HashMap::new(),
            },
        };

        let result = session_to_session_info(&session);
        assert!(result.is_ok());

        let session_info = result.unwrap();
        assert_eq!(session_info.device_name, Some("Test Device".to_string()));
        assert_eq!(session_info.device_type, Some("desktop".to_string()));
        assert_eq!(session_info.ip_address, Some("127.0.0.1".to_string()));
        assert!(session_info.is_active);
    }
}
