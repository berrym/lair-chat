//! Admin API models
//!
//! This module contains all data structures related to administrative operations,
//! including server management, user administration, and system monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::api::models::auth::{UserRole, UserStatus};

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerStatistics {
    /// Total registered users
    pub total_users: u64,

    /// Active users (last 24 hours)
    pub active_users: u32,

    /// Online users (currently)
    pub online_users: u32,

    /// Total rooms created
    pub total_rooms: u64,

    /// Active rooms (with recent messages)
    pub active_rooms: u32,

    /// Total messages sent
    pub total_messages: u64,

    /// Messages sent today
    pub messages_today: u32,

    /// Total sessions (all time)
    pub total_sessions: u64,

    /// Active sessions (currently)
    pub active_sessions: u32,

    /// Server uptime in seconds
    pub uptime_seconds: u64,

    /// Database size in bytes
    pub database_size: u64,

    /// Memory usage in bytes
    pub memory_usage: u64,

    /// CPU usage percentage
    pub cpu_usage: f32,

    /// Statistics last updated
    pub updated_at: DateTime<Utc>,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,

    /// Component health checks
    pub components: Vec<ComponentHealth>,

    /// System metrics
    pub metrics: SystemMetrics,

    /// Health check timestamp
    pub checked_at: DateTime<Utc>,
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
    /// Component name
    pub name: String,

    /// Component status
    pub status: HealthStatus,

    /// Optional error message
    pub error: Option<String>,

    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,

    /// Last check timestamp
    pub last_check: DateTime<Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f32,

    /// Memory usage in bytes
    pub memory_usage: u64,

    /// Total memory in bytes
    pub memory_total: u64,

    /// Disk usage in bytes
    pub disk_usage: u64,

    /// Total disk space in bytes
    pub disk_total: u64,

    /// Network bytes received
    pub network_bytes_received: u64,

    /// Network bytes sent
    pub network_bytes_sent: u64,

    /// Active connections
    pub active_connections: u32,

    /// Database connections
    pub database_connections: u32,
}

/// Admin user management
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminUserInfo {
    /// User ID
    pub id: Uuid,

    /// Username
    pub username: String,

    /// Email address
    pub email: String,

    /// Display name
    pub display_name: String,

    /// User role
    pub role: UserRole,

    /// Account status
    pub status: UserStatus,

    /// Registration timestamp
    pub created_at: DateTime<Utc>,

    /// Last login timestamp
    pub last_login: Option<DateTime<Utc>>,

    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,

    /// Total messages sent
    pub messages_sent: u64,

    /// Total sessions created
    pub sessions_created: u64,

    /// Current active sessions
    pub active_sessions: u32,
}

/// Update user status request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserStatusRequest {
    /// New user status
    pub status: UserStatus,

    /// Reason for status change
    #[validate(length(min = 1, max = 500))]
    pub reason: String,

    /// Whether to notify the user
    #[serde(default)]
    pub notify_user: bool,
}

/// Update user role request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRoleRequest {
    /// New user role
    pub role: UserRole,

    /// Reason for role change
    #[validate(length(min = 1, max = 500))]
    pub reason: String,
}

/// Admin room information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminRoomInfo {
    /// Room ID
    pub id: Uuid,

    /// Room name
    pub name: String,

    /// Room type
    pub room_type: crate::server::api::models::rooms::RoomType,

    /// Privacy level
    pub privacy: crate::server::api::models::rooms::PrivacyLevel,

    /// Room owner ID
    pub owner_id: Uuid,

    /// Member count
    pub member_count: u32,

    /// Total messages
    pub message_count: u64,

    /// Room creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,

    /// Whether room is active
    pub is_active: bool,
}

/// Server configuration update request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateServerConfigRequest {
    /// Server name
    #[validate(length(min = 1, max = 100))]
    pub server_name: Option<String>,

    /// Server description
    #[validate(length(max = 500))]
    pub server_description: Option<String>,

    /// Maximum users allowed
    #[validate(range(min = 1, max = 1000000))]
    pub max_users: Option<u32>,

    /// Maximum rooms allowed
    #[validate(range(min = 1, max = 100000))]
    pub max_rooms: Option<u32>,

    /// Whether registration is enabled
    pub registration_enabled: Option<bool>,

    /// Whether guest access is allowed
    pub guest_access_enabled: Option<bool>,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogEntry {
    /// Log entry ID
    pub id: Uuid,

    /// Admin user who performed the action
    pub admin_user_id: Uuid,

    /// Action performed
    pub action: AdminAction,

    /// Target of the action (user ID, room ID, etc.)
    pub target_id: Option<Uuid>,

    /// Target type (user, room, system, etc.)
    pub target_type: String,

    /// Action description
    pub description: String,

    /// IP address of admin
    pub ip_address: Option<String>,

    /// User agent of admin
    pub user_agent: Option<String>,

    /// Action timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Admin actions for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    /// User account created
    UserCreated,
    /// User account updated
    UserUpdated,
    /// User account suspended
    UserSuspended,
    /// User account banned
    UserBanned,
    /// User account deleted
    UserDeleted,
    /// User role changed
    UserRoleChanged,
    /// Room created
    RoomCreated,
    /// Room updated
    RoomUpdated,
    /// Room deleted
    RoomDeleted,
    /// Server configuration changed
    ServerConfigUpdated,
    /// System maintenance performed
    SystemMaintenance,
    /// Database backup created
    DatabaseBackup,
    /// Security alert raised
    SecurityAlert,
    /// Audit log accessed
    AuditLogAccessed,
}

/// System maintenance request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MaintenanceRequest {
    /// Maintenance type
    pub maintenance_type: MaintenanceType,

    /// Maintenance description
    #[validate(length(min = 1, max = 500))]
    pub description: String,

    /// Whether to put server in maintenance mode
    #[serde(default)]
    pub enable_maintenance_mode: bool,

    /// Estimated duration in minutes
    pub estimated_duration_minutes: Option<u32>,
}

/// Types of maintenance operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceType {
    /// Database cleanup
    DatabaseCleanup,
    /// Database backup
    DatabaseBackup,
    /// Log rotation
    LogRotation,
    /// Cache cleanup
    CacheCleanup,
    /// Security update
    SecurityUpdate,
    /// Performance optimization
    PerformanceOptimization,
    /// General maintenance
    General,
}

/// Server announcement
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ServerAnnouncement {
    /// Announcement ID
    pub id: Uuid,

    /// Announcement title
    #[validate(length(min = 1, max = 200))]
    pub title: String,

    /// Announcement content
    #[validate(length(min = 1, max = 2000))]
    pub content: String,

    /// Announcement type
    pub announcement_type: AnnouncementType,

    /// Whether announcement is active
    pub is_active: bool,

    /// Target audience
    pub target_audience: TargetAudience,

    /// Admin who created the announcement
    pub created_by: Uuid,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
}

/// Types of server announcements
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementType {
    /// General information
    Info,
    /// Warning message
    Warning,
    /// Maintenance notification
    Maintenance,
    /// Security alert
    Security,
    /// Feature announcement
    Feature,
    /// Emergency notification
    Emergency,
}

/// Target audience for announcements
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TargetAudience {
    /// All users
    AllUsers,
    /// Active users only
    ActiveUsers,
    /// Admins only
    AdminsOnly,
    /// Moderators and above
    ModeratorsAndAbove,
    /// Specific user group
    UserGroup(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_update_user_status_request_validation() {
        let valid_request = UpdateUserStatusRequest {
            status: UserStatus::Suspended,
            reason: "Violation of terms".to_string(),
            notify_user: true,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UpdateUserStatusRequest {
            status: UserStatus::Suspended,
            reason: "".to_string(), // Empty reason
            notify_user: false,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_server_config_update_validation() {
        let valid_request = UpdateServerConfigRequest {
            server_name: Some("Test Server".to_string()),
            server_description: Some("A test server".to_string()),
            max_users: Some(1000),
            max_rooms: Some(100),
            registration_enabled: Some(true),
            guest_access_enabled: Some(false),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UpdateServerConfigRequest {
            server_name: Some("".to_string()), // Empty name
            server_description: None,
            max_users: Some(0), // Invalid minimum
            max_rooms: None,
            registration_enabled: None,
            guest_access_enabled: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_maintenance_request_validation() {
        let valid_request = MaintenanceRequest {
            maintenance_type: MaintenanceType::DatabaseCleanup,
            description: "Cleaning up old data".to_string(),
            enable_maintenance_mode: true,
            estimated_duration_minutes: Some(30),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = MaintenanceRequest {
            maintenance_type: MaintenanceType::General,
            description: "".to_string(), // Empty description
            enable_maintenance_mode: false,
            estimated_duration_minutes: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_health_status_variants() {
        assert!(matches!(
            serde_json::from_str::<HealthStatus>("\"healthy\"").unwrap(),
            HealthStatus::Healthy
        ));
        assert!(matches!(
            serde_json::from_str::<HealthStatus>("\"degraded\"").unwrap(),
            HealthStatus::Degraded
        ));
        assert!(matches!(
            serde_json::from_str::<HealthStatus>("\"unhealthy\"").unwrap(),
            HealthStatus::Unhealthy
        ));
    }
}
