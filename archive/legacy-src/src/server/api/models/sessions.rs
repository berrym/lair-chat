//! Session management API models
//!
//! This module contains all data structures related to session management,
//! including session information, device tracking, and session-related operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    /// Session ID
    pub id: Uuid,

    /// User ID who owns this session
    pub user_id: Uuid,

    /// Device name/identifier
    pub device_name: Option<String>,

    /// Device type (mobile, desktop, tablet, etc.)
    pub device_type: Option<String>,

    /// IP address
    pub ip_address: Option<String>,

    /// User agent string
    pub user_agent: Option<String>,

    /// Whether session is currently active
    pub is_active: bool,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Session expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,

    /// Session metadata
    pub metadata: serde_json::Value,
}

/// Active session summary (for current user)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActiveSession {
    /// Session ID
    pub id: Uuid,

    /// Device name
    pub device_name: Option<String>,

    /// Device type
    pub device_type: Option<String>,

    /// IP address (masked for privacy)
    pub ip_address_masked: Option<String>,

    /// Location (city/country if available)
    pub location: Option<String>,

    /// Whether this is the current session
    pub is_current: bool,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Terminate session request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TerminateSessionRequest {
    /// Session ID to terminate
    pub session_id: Uuid,
}

/// Terminate all sessions request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TerminateAllSessionsRequest {
    /// Whether to keep the current session active
    #[serde(default = "default_keep_current")]
    pub keep_current: bool,
}

/// Update session metadata request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSessionRequest {
    /// Device name
    #[validate(length(max = 100))]
    pub device_name: Option<String>,

    /// Device type
    #[validate(length(max = 50))]
    pub device_type: Option<String>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionStatistics {
    /// User ID
    pub user_id: Uuid,

    /// Total active sessions
    pub active_sessions: u32,

    /// Total sessions created
    pub total_sessions: u64,

    /// Most recent login
    pub last_login: Option<DateTime<Utc>>,

    /// Most common device type
    pub most_common_device: Option<String>,

    /// Total login locations (unique IPs)
    pub unique_locations: u32,

    /// Statistics last updated
    pub updated_at: DateTime<Utc>,
}

/// Device information for session tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceInfo {
    /// Device name/identifier
    pub name: String,

    /// Device type (mobile, desktop, tablet, etc.)
    pub device_type: String,

    /// Operating system
    pub os: Option<String>,

    /// Browser/client application
    pub client: Option<String>,

    /// Screen resolution
    pub screen_resolution: Option<String>,

    /// Device capabilities
    pub capabilities: Vec<String>,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            name: "Unknown Device".to_string(),
            device_type: "unknown".to_string(),
            os: None,
            client: None,
            screen_resolution: None,
            capabilities: Vec::new(),
        }
    }
}

/// Session activity log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionActivity {
    /// Session ID
    pub session_id: Uuid,

    /// Activity type
    pub activity_type: ActivityType,

    /// Activity description
    pub description: String,

    /// IP address at time of activity
    pub ip_address: Option<String>,

    /// User agent at time of activity
    pub user_agent: Option<String>,

    /// Activity timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Types of session activities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    /// Session created (login)
    SessionCreated,
    /// Session terminated (logout)
    SessionTerminated,
    /// Session expired
    SessionExpired,
    /// Password changed
    PasswordChanged,
    /// Profile updated
    ProfileUpdated,
    /// Settings changed
    SettingsChanged,
    /// Suspicious activity detected
    SuspiciousActivity,
    /// Security alert
    SecurityAlert,
}

fn default_keep_current() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_device_info_default() {
        let device = DeviceInfo::default();
        assert_eq!(device.name, "Unknown Device");
        assert_eq!(device.device_type, "unknown");
        assert!(device.capabilities.is_empty());
    }

    #[test]
    fn test_terminate_all_sessions_default() {
        let request = TerminateAllSessionsRequest {
            keep_current: default_keep_current(),
        };
        assert!(request.keep_current);
    }

    #[test]
    fn test_update_session_request_validation() {
        let valid_request = UpdateSessionRequest {
            device_name: Some("My Phone".to_string()),
            device_type: Some("mobile".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UpdateSessionRequest {
            device_name: Some("a".repeat(150)), // Too long
            device_type: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_session_activity_types() {
        let activity = SessionActivity {
            session_id: Uuid::new_v4(),
            activity_type: ActivityType::SessionCreated,
            description: "User logged in".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        };

        assert!(matches!(
            activity.activity_type,
            ActivityType::SessionCreated
        ));
    }
}
