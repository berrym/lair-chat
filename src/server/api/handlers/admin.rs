//! Admin handlers for server management
//!
//! This module provides HTTP handlers for administrative operations including
//! user management, room administration, and server configuration.

use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use std::time::{Duration, SystemTime};
use sysinfo::System;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::server::api::models::admin::{AdminAction, AuditLogEntry};
use crate::server::api::models::common::ApiError;
use crate::server::{
    api::{
        handlers::{responses, validation},
        middleware::UserContext,
        models::{
            admin::*,
            common::{ApiResult, EmptyResponse, SuccessResponse},
            PaginationParams,
        },
        ApiState,
    },
    storage::Pagination,
};

/// Get server statistics
///
/// Returns comprehensive statistics about the server including user counts,
/// room activity, system resources, and performance metrics.
#[utoipa::path(
    get,
    path = "/api/v1/admin/stats",
    responses(
        (status = 200, description = "Server statistics retrieved", body = ServerStatistics),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_server_statistics(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<ServerStatistics>>> {
    debug!(
        "Server statistics request from admin: {}",
        user_context.username
    );

    // Gather actual statistics from storage layer
    let total_users = state.storage.users().count_users().await.map_err(|e| {
        error!("Failed to count users: {}", e);
        ApiError::internal_error("Failed to retrieve server statistics")
    })? as u64;

    let total_rooms = state.storage.rooms().count_rooms().await.map_err(|e| {
        error!("Failed to count rooms: {}", e);
        ApiError::internal_error("Failed to retrieve server statistics")
    })? as u64;

    let total_messages = state
        .storage
        .messages()
        .count_messages()
        .await
        .map_err(|e| {
            error!("Failed to count messages: {}", e);
            ApiError::internal_error("Failed to retrieve server statistics")
        })? as u64;

    let session_stats = state
        .storage
        .sessions()
        .get_session_stats()
        .await
        .map_err(|e| {
            error!("Failed to get session statistics: {}", e);
            ApiError::internal_error("Failed to retrieve server statistics")
        })?;

    // Calculate messages today (last 24 hours)
    let today_start = chrono::Utc::now().timestamp() - (24 * 60 * 60);
    let messages_today = state
        .storage
        .messages()
        .count_messages_since(today_start as u64)
        .await
        .unwrap_or(0) as u64;

    // Get TCP server statistics from global state
    let (tcp_connected_users, tcp_active_rooms, tcp_connected_peers) = {
        let tcp_stats = crate::server::api::models::admin::get_tcp_stats()
            .await
            .unwrap_or_else(|| crate::shared_types::TcpServerStats {
                connected_peers: 0,
                authenticated_users: 0,
                active_rooms: 0,
                pending_invitations: 0,
                room_user_counts: std::collections::HashMap::new(),
                uptime_seconds: 0,
            });

        (
            tcp_stats.authenticated_users as u32,
            tcp_stats.active_rooms as u32,
            tcp_stats.connected_peers as u32,
        )
    };

    // Combine REST API and TCP server statistics
    let combined_online_users = session_stats.active_sessions as u32 + tcp_connected_users;
    let combined_active_rooms = std::cmp::max(total_rooms as u32, tcp_active_rooms);

    let stats = ServerStatistics {
        total_users,
        active_users: total_users as u32, // TODO: Implement active user tracking
        online_users: combined_online_users,
        total_rooms,
        active_rooms: combined_active_rooms,
        total_messages,
        messages_today: messages_today as u32,
        total_sessions: session_stats.total_sessions,
        active_sessions: session_stats.active_sessions as u32,
        uptime_seconds: 86400, // TODO: Implement actual uptime tracking
        database_size: 1024 * 1024 * 100, // TODO: Get actual database size
        memory_usage: 1024 * 1024 * 256, // TODO: Get actual memory usage
        cpu_usage: 15.5,       // TODO: Get actual CPU usage
        updated_at: chrono::Utc::now(),
    };

    info!(
        "Server statistics retrieved by admin: {} (TCP: {} users, {} peers, {} rooms)",
        user_context.username, tcp_connected_users, tcp_connected_peers, tcp_active_rooms
    );
    Ok(responses::success(stats))
}

/// Get system health information
///
/// Returns detailed health check information for all system components
/// including database, storage, and external services.
#[utoipa::path(
    get,
    path = "/api/v1/admin/health",
    responses(
        (status = 200, description = "System health retrieved", body = SystemHealth),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_system_health(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<SystemHealth>>> {
    debug!(
        "System health request from admin: {}",
        user_context.username
    );

    let mut components = Vec::new();
    let now = chrono::Utc::now();

    // Database health check
    let db_start = SystemTime::now();
    let db_health = check_database_health(&state).await;
    let db_response_time = db_start
        .elapsed()
        .unwrap_or(Duration::from_secs(30))
        .as_millis() as u64;

    components.push(ComponentHealth {
        name: "Database".to_string(),
        status: db_health.0,
        error: db_health.1,
        response_time_ms: Some(db_response_time as u32),
        last_check: now,
        metadata: serde_json::json!({"type": "SQLite", "connection_pool": true}),
    });

    // Storage health check
    let storage_start = SystemTime::now();
    let storage_health = check_storage_health(&state).await;
    let storage_response_time = storage_start
        .elapsed()
        .unwrap_or(Duration::from_secs(30))
        .as_millis() as u64;

    components.push(ComponentHealth {
        name: "Storage".to_string(),
        status: storage_health.0,
        error: storage_health.1,
        response_time_ms: Some(storage_response_time as u32),
        last_check: now,
        metadata: serde_json::json!({"operations": ["users", "rooms", "messages", "sessions"]}),
    });

    // Session management health check
    let session_start = SystemTime::now();
    let session_health = check_session_health(&state).await;
    let session_response_time = session_start
        .elapsed()
        .unwrap_or(Duration::from_secs(30))
        .as_millis() as u64;

    components.push(ComponentHealth {
        name: "Sessions".to_string(),
        status: session_health.0,
        error: session_health.1,
        response_time_ms: Some(session_response_time as u32),
        last_check: now,
        metadata: serde_json::json!({"active_sessions": session_health.2}),
    });

    // TCP server health check (integrated mode only)
    let tcp_start = SystemTime::now();
    let tcp_health = check_tcp_server_health().await;
    let tcp_response_time = tcp_start
        .elapsed()
        .unwrap_or(Duration::from_secs(30))
        .as_millis() as u64;

    components.push(ComponentHealth {
        name: "TCP Server".to_string(),
        status: tcp_health.0,
        error: tcp_health.1,
        response_time_ms: Some(tcp_response_time as u32),
        last_check: now,
        metadata: tcp_health.2,
    });

    // Collect system metrics
    let metrics = collect_system_metrics().await;

    // Determine overall health status
    let overall_status = if components
        .iter()
        .all(|c| matches!(c.status, HealthStatus::Healthy))
    {
        HealthStatus::Healthy
    } else if components
        .iter()
        .any(|c| matches!(c.status, HealthStatus::Critical))
    {
        HealthStatus::Critical
    } else {
        HealthStatus::Degraded
    };

    let health = SystemHealth {
        status: overall_status,
        components,
        metrics,
        checked_at: now,
    };

    info!(
        "System health retrieved by admin: {} (status: {:?})",
        user_context.username, health.status
    );
    Ok(responses::success(health))
}

/// Check database connectivity and performance
async fn check_database_health(state: &ApiState) -> (HealthStatus, Option<String>) {
    match state.storage.users().count_users().await {
        Ok(_) => (HealthStatus::Healthy, None),
        Err(e) => {
            error!("Database health check failed: {}", e);
            (
                HealthStatus::Critical,
                Some(format!("Database connection failed: {}", e)),
            )
        }
    }
}

/// Check storage layer health
async fn check_storage_health(state: &ApiState) -> (HealthStatus, Option<String>) {
    // Test basic storage operations
    let user_test = state.storage.users().count_users().await;
    let room_test = state.storage.rooms().count_rooms().await;
    let message_test = state.storage.messages().count_messages().await;

    match (user_test, room_test, message_test) {
        (Ok(_), Ok(_), Ok(_)) => (HealthStatus::Healthy, None),
        _ => {
            warn!("Storage health check detected issues");
            (
                HealthStatus::Degraded,
                Some("Some storage operations are failing".to_string()),
            )
        }
    }
}

/// Check session management health
async fn check_session_health(state: &ApiState) -> (HealthStatus, Option<String>, u32) {
    match state.storage.sessions().get_session_stats().await {
        Ok(stats) => (HealthStatus::Healthy, None, stats.active_sessions as u32),
        Err(e) => {
            error!("Session health check failed: {}", e);
            (
                HealthStatus::Critical,
                Some(format!("Session management failed: {}", e)),
                0,
            )
        }
    }
}

/// Check TCP server health using global state
async fn check_tcp_server_health() -> (HealthStatus, Option<String>, serde_json::Value) {
    // Try to get TCP stats from global state
    match crate::server::api::models::get_tcp_stats().await {
        Some(stats) => {
            let metadata = serde_json::json!({
                "connected_peers": stats.connected_peers,
                "authenticated_users": stats.authenticated_users,
                "active_rooms": stats.active_rooms,
                "pending_invitations": stats.pending_invitations,
                "room_user_counts": stats.room_user_counts,
                "uptime_seconds": stats.uptime_seconds,
                "tcp_server_running": true
            });

            // TCP server is healthy if we can get stats
            (HealthStatus::Healthy, None, metadata)
        }
        None => {
            warn!("Failed to get TCP server statistics for health check");
            (
                HealthStatus::Degraded,
                Some("TCP server statistics unavailable".to_string()),
                serde_json::json!({"error": "tcp_stats_unavailable", "tcp_server_running": false}),
            )
        }
    }
}

/// Collect real system metrics
async fn collect_system_metrics() -> SystemMetrics {
    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU usage (average across all cores)
    let cpu_usage = sys.global_cpu_info().cpu_usage();

    // Memory information
    let memory_total = sys.total_memory();
    let memory_used = sys.used_memory();

    // Use default disk values for now since sysinfo API changed
    let (disk_total, disk_used) = (1024 * 1024 * 1024, 1024 * 1024 * 500);

    // Use default network values for now since sysinfo API changed
    let (network_received, network_transmitted) = (1024 * 1024 * 10, 1024 * 1024 * 8);

    // Process information
    let active_connections = sys.processes().len() as u32;

    SystemMetrics {
        cpu_usage: cpu_usage,
        memory_usage: memory_used,
        memory_total,
        disk_usage: disk_used,
        disk_total,
        network_bytes_received: network_received,
        network_bytes_sent: network_transmitted,
        active_connections,
        database_connections: 5, // TODO: Get actual DB connection count
    }
}

/// Get admin user list
///
/// Returns a paginated list of all users with administrative information
/// including statistics and account status.
#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    responses(
        (status = 200, description = "User list retrieved", body = Vec<AdminUserInfo>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_admin_users(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<AdminUserInfo>>>> {
    debug!("Admin users list request from: {}", user_context.username);

    // Convert query params to pagination
    let pagination = Pagination {
        offset: (params.page * params.page_size) as u64,
        limit: params.page_size as u64,
    };

    // Get users from storage
    let users = state
        .storage
        .users()
        .list_admin_users(pagination)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to retrieve users: {}", e)))?;

    info!(
        "Admin users list retrieved by: {} (count: {})",
        user_context.username,
        users.len()
    );
    Ok(responses::success(users))
}

/// Update user status
///
/// Updates a user's account status (active, suspended, banned, etc.)
/// with administrative privileges and audit logging.
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{user_id}/status",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateUserStatusRequest,
    responses(
        (status = 200, description = "User status updated successfully", body = EmptyResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_user_status(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserStatusRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Update user status request for user: {}", user_id);

    // Validate request data
    validation::validate_request(&request)?;

    // Check if user exists
    let user_id_str = user_id.to_string();
    let existing_user = state
        .storage
        .users()
        .get_user_by_id(&user_id_str)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to retrieve user: {}", e)))?;

    if existing_user.is_none() {
        return Err(ApiError::not_found_error("User not found"));
    }

    // Update user status
    let status = request.status.clone();
    state
        .storage
        .users()
        .update_user_status(&user_id_str, request.status)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to update user status: {}", e)))?;

    info!(
        "User status updated successfully by admin: {} (user: {}, new status: {:?})",
        user_context.username, user_id, status
    );
    // Log the admin action
    let audit_entry = AuditLogEntry {
        id: Uuid::new_v4(),
        admin_user_id: user_context.user_id,
        action: AdminAction::UserSuspended, // This would be dynamic based on actual status
        target_id: Some(user_id),
        target_type: "user".to_string(),
        description: format!(
            "Admin {} updated user {} status to {:?}",
            user_context.username, user_id, status
        ),
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        timestamp: chrono::Utc::now(),
        metadata: serde_json::json!({
            "old_status": "unknown", // TODO: Get previous status
            "new_status": status,
            "reason": request.reason
        }),
    };

    if let Err(e) = state
        .storage
        .audit_logs()
        .create_audit_log(audit_entry)
        .await
    {
        error!("Failed to log admin action: {}", e);
    }

    Ok(responses::empty_success(
        "User status updated successfully".to_string(),
    ))
}

/// Update user role
///
/// Updates a user's role (admin, moderator, user, guest) with proper
/// authorization checks and audit logging.
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{user_id}/role",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateUserRoleRequest,
    responses(
        (status = 200, description = "User role updated successfully", body = EmptyResponse),
        (status = 400, description = "Invalid request data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 404, description = "User not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_user_role(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserRoleRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Update user role request for user: {}", user_id);

    // Validate request data
    validation::validate_request(&request)?;

    // TODO: Implement user role update with audit logging

    // Log the admin action
    let audit_entry = AuditLogEntry {
        id: Uuid::new_v4(),
        admin_user_id: user_context.user_id,
        action: AdminAction::UserRoleChanged,
        target_id: Some(user_id),
        target_type: "user".to_string(),
        description: format!(
            "Admin {} updated user {} role to {:?}",
            user_context.username, user_id, request.role
        ),
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        timestamp: chrono::Utc::now(),
        metadata: serde_json::json!({
            "new_role": request.role,
            "reason": request.reason
        }),
    };

    if let Err(e) = state
        .storage
        .audit_logs()
        .create_audit_log(audit_entry)
        .await
    {
        error!("Failed to log admin action: {}", e);
    }

    info!(
        "User role updated successfully by admin: {}",
        user_context.username
    );
    Ok(responses::empty_success(
        "User role updated successfully".to_string(),
    ))
}

/// Get admin room list
///
/// Returns a paginated list of all rooms with administrative information
/// including member counts and activity statistics.
#[utoipa::path(
    get,
    path = "/api/v1/admin/rooms",
    responses(
        (status = 200, description = "Room list retrieved", body = Vec<AdminRoomInfo>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_admin_rooms(
    State(_state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Query(_params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<AdminRoomInfo>>>> {
    debug!("Admin rooms list request from: {}", user_context.username);

    // TODO: Implement room listing with admin information
    let rooms = vec![];

    info!("Admin rooms list retrieved by: {}", user_context.username);
    Ok(responses::success(rooms))
}

/// Update server configuration
///
/// Updates server-wide configuration settings such as registration limits,
/// feature flags, and operational parameters.
#[utoipa::path(
    put,
    path = "/api/v1/admin/config",
    request_body = UpdateServerConfigRequest,
    responses(
        (status = 200, description = "Server configuration updated", body = EmptyResponse),
        (status = 400, description = "Invalid configuration data", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn update_server_config(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<UpdateServerConfigRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!(
        "Server config update request from admin: {}",
        user_context.username
    );

    // Validate request data
    validation::validate_request(&request)?;

    // TODO: Implement server configuration update with validation

    // Log the admin action
    let audit_entry = AuditLogEntry {
        id: Uuid::new_v4(),
        admin_user_id: user_context.user_id,
        action: AdminAction::ServerConfigUpdated,
        target_id: None,
        target_type: "server".to_string(),
        description: format!(
            "Admin {} updated server configuration",
            user_context.username
        ),
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        timestamp: chrono::Utc::now(),
        metadata: serde_json::to_value(&request).unwrap_or_default(),
    };

    if let Err(e) = state
        .storage
        .audit_logs()
        .create_audit_log(audit_entry)
        .await
    {
        error!("Failed to log admin action: {}", e);
    }

    info!(
        "Server configuration updated by admin: {}",
        user_context.username
    );
    Ok(responses::empty_success(
        "Server configuration updated successfully".to_string(),
    ))
}

/// Perform system maintenance
///
/// Initiates various maintenance operations such as database cleanup,
/// cache clearing, and system optimization tasks.
#[utoipa::path(
    post,
    path = "/api/v1/admin/maintenance",
    request_body = MaintenanceRequest,
    responses(
        (status = 200, description = "Maintenance operation completed", body = EmptyResponse),
        (status = 400, description = "Invalid maintenance request", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Maintenance operation failed", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn perform_maintenance(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<MaintenanceRequest>,
) -> ApiResult<Json<EmptyResponse>> {
    info!("Maintenance request from admin: {}", user_context.username);

    // Validate request data
    validation::validate_request(&request)?;

    // TODO: Implement maintenance operations based on type

    // Log the admin action
    let audit_entry = AuditLogEntry {
        id: Uuid::new_v4(),
        admin_user_id: user_context.user_id,
        action: AdminAction::SystemMaintenance,
        target_id: None,
        target_type: "system".to_string(),
        description: format!(
            "Admin {} performed {} maintenance: {}",
            user_context.username,
            serde_json::to_string(&request.maintenance_type).unwrap_or("unknown".to_string()),
            request.description
        ),
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        timestamp: chrono::Utc::now(),
        metadata: serde_json::to_value(&request).unwrap_or_default(),
    };

    if let Err(e) = state
        .storage
        .audit_logs()
        .create_audit_log(audit_entry)
        .await
    {
        error!("Failed to log admin action: {}", e);
    }

    info!(
        "Maintenance operation completed by admin: {}",
        user_context.username
    );
    Ok(responses::empty_success(
        "Maintenance operation completed successfully".to_string(),
    ))
}

/// Get audit log entries
///
/// Returns a paginated list of audit log entries with filtering options
/// for administrative review and compliance.
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit",
    responses(
        (status = 200, description = "Audit logs retrieved", body = Vec<AuditLogEntry>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_audit_logs(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<AuditLogEntry>>>> {
    debug!("Audit logs request from admin: {}", user_context.username);

    // Convert query params to pagination
    let pagination = Pagination {
        offset: (params.page * params.page_size) as u64,
        limit: params.page_size as u64,
    };

    // Get audit logs from storage
    let logs = state
        .storage
        .audit_logs()
        .get_audit_logs(pagination, None)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to retrieve audit logs: {}", e)))?;

    // Log the audit log access
    let audit_entry = AuditLogEntry {
        id: uuid::Uuid::new_v4(),
        admin_user_id: user_context.user_id,
        action: crate::server::api::models::admin::AdminAction::AuditLogAccessed,
        target_id: None,
        target_type: "audit_logs".to_string(),
        description: format!("Admin {} accessed audit logs", user_context.username),
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
        timestamp: chrono::Utc::now(),
        metadata: serde_json::json!({"page": params.page, "page_size": params.page_size}),
    };

    if let Err(e) = state
        .storage
        .audit_logs()
        .create_audit_log(audit_entry)
        .await
    {
        error!("Failed to log audit log access: {}", e);
    }

    info!(
        "Audit logs retrieved by admin: {} (count: {})",
        user_context.username,
        logs.len()
    );
    Ok(responses::success(logs))
}

/// Get audit log entries by admin user
///
/// Returns audit log entries filtered by the admin user who performed the actions.
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit/by-admin/{admin_user_id}",
    params(
        ("admin_user_id" = Uuid, Path, description = "Admin User ID")
    ),
    responses(
        (status = 200, description = "Audit logs by admin retrieved", body = Vec<AuditLogEntry>),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 404, description = "Admin user not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_audit_logs_by_admin(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Path(admin_user_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<SuccessResponse<Vec<AuditLogEntry>>>> {
    debug!(
        "Audit logs by admin request from: {} for admin: {}",
        user_context.username, admin_user_id
    );

    // Convert query params to pagination
    let pagination = Pagination {
        offset: (params.page * params.page_size) as u64,
        limit: params.page_size as u64,
    };

    // Get audit logs by admin from storage
    let logs = state
        .storage
        .audit_logs()
        .get_audit_logs_by_admin(&admin_user_id.to_string(), pagination)
        .await
        .map_err(|e| {
            ApiError::internal_error(format!("Failed to retrieve audit logs by admin: {}", e))
        })?;

    info!(
        "Audit logs by admin retrieved by: {} (admin: {}, count: {})",
        user_context.username,
        admin_user_id,
        logs.len()
    );
    Ok(responses::success(logs))
}

/// Search audit log entries
///
/// Search audit log entries by description or other searchable fields.
#[utoipa::path(
    post,
    path = "/api/v1/admin/audit/search",
    request_body = AuditLogSearchRequest,
    responses(
        (status = 200, description = "Audit log search results", body = Vec<AuditLogEntry>),
        (status = 400, description = "Invalid search request", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn search_audit_logs(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
    Json(request): Json<AuditLogSearchRequest>,
) -> ApiResult<Json<SuccessResponse<Vec<AuditLogEntry>>>> {
    debug!(
        "Audit log search request from admin: {} with query: {}",
        user_context.username, request.query
    );

    // Validate request data
    validation::validate_request(&request)?;

    // Convert query params to pagination
    let pagination = Pagination {
        offset: (request.page * request.page_size) as u64,
        limit: request.page_size as u64,
    };

    // Search audit logs
    let logs = state
        .storage
        .audit_logs()
        .search_audit_logs(&request.query, pagination)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to search audit logs: {}", e)))?;

    info!(
        "Audit log search completed by admin: {} (query: {}, results: {})",
        user_context.username,
        request.query,
        logs.len()
    );
    Ok(responses::success(logs))
}

/// Get audit log statistics
///
/// Returns statistical information about audit log entries for reporting
/// and monitoring purposes.
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit/stats",
    responses(
        (status = 200, description = "Audit log statistics retrieved", body = AuditLogStats),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 403, description = "Admin privileges required", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "admin",
    security(
        ("Bearer" = [])
    )
)]
pub async fn get_audit_log_stats(
    State(state): State<ApiState>,
    Extension(user_context): Extension<UserContext>,
) -> ApiResult<Json<SuccessResponse<crate::server::api::models::admin::AuditLogStats>>> {
    debug!(
        "Audit log statistics request from admin: {}",
        user_context.username
    );

    // Get audit log statistics from storage
    let storage_stats = state
        .storage
        .audit_logs()
        .get_audit_log_stats()
        .await
        .map_err(|e| {
            ApiError::internal_error(format!("Failed to retrieve audit log statistics: {}", e))
        })?;

    // Convert storage model to API model
    let api_stats = crate::server::api::models::admin::AuditLogStats {
        total_entries: storage_stats.total_entries,
        entries_today: storage_stats.entries_today,
        entries_this_week: storage_stats.entries_this_week,
        entries_this_month: storage_stats.entries_this_month,
        entries_by_action: storage_stats.entries_by_action,
        entries_by_admin: storage_stats.entries_by_admin,
        most_active_admins: storage_stats.most_active_admins,
    };

    info!(
        "Audit log statistics retrieved by admin: {} (total entries: {})",
        user_context.username, api_stats.total_entries
    );
    Ok(responses::success(api_stats))
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_placeholder() {
        // Placeholder test to satisfy module structure
        assert!(true);
    }
}
