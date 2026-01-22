//! Structured logging framework for the TCP server
//!
//! This module provides comprehensive logging with audit trail capabilities,
//! structured logging, and log aggregation for security and operational monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, Level};

use crate::server::error::TcpError;
use crate::server::storage::current_timestamp;

/// Structured logger for the TCP server
pub struct StructuredLogger {
    /// Audit trail storage
    audit_trail: Arc<RwLock<AuditTrail>>,
    /// Log configuration
    config: LogConfig,
    /// Log statistics
    stats: Arc<RwLock<LogStats>>,
}

/// Audit trail for security and operational events
#[derive(Debug, Clone, Default)]
pub struct AuditTrail {
    /// Security events
    pub security_events: Vec<SecurityEvent>,
    /// Authentication events
    pub auth_events: Vec<AuthEvent>,
    /// System events
    pub system_events: Vec<SystemEvent>,
    /// User actions
    pub user_actions: Vec<UserAction>,
    /// Maximum entries to keep
    pub max_entries: usize,
}

/// Security event for audit logging
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: SecurityEventType,
    /// User ID (if applicable)
    pub user_id: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// Event description
    pub description: String,
    /// Timestamp
    pub timestamp: i64,
    /// Severity level
    pub severity: SecuritySeverity,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Authentication event for audit logging
#[derive(Debug, Clone)]
pub struct AuthEvent {
    /// Event type
    pub event_type: AuthEventType,
    /// User ID
    pub user_id: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Success/failure
    pub success: bool,
    /// Timestamp
    pub timestamp: i64,
    /// Additional details
    pub details: HashMap<String, String>,
}

/// System event for operational monitoring
#[derive(Debug, Clone)]
pub struct SystemEvent {
    /// Event type
    pub event_type: SystemEventType,
    /// Component that generated the event
    pub component: String,
    /// Event description
    pub description: String,
    /// Timestamp
    pub timestamp: i64,
    /// Log level
    pub level: Level,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// User action for audit trail
#[derive(Debug, Clone)]
pub struct UserAction {
    /// Action type
    pub action_type: UserActionType,
    /// User ID
    pub user_id: String,
    /// Target (room, user, etc.)
    pub target: Option<String>,
    /// Action description
    pub description: String,
    /// Timestamp
    pub timestamp: i64,
    /// Result of the action
    pub result: ActionResult,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Security event types
#[derive(Debug, Clone)]
pub enum SecurityEventType {
    IntrusionAttempt,
    SuspiciousActivity,
    AccessDenied,
    SecurityViolation,
    RateLimitExceeded,
    InvalidInput,
    SecurityBreach,
    AuthenticationFailure,
    AuthorizationFailure,
}

/// Authentication event types
#[derive(Debug, Clone)]
pub enum AuthEventType {
    Login,
    Logout,
    Register,
    PasswordChange,
    AccountLock,
    AccountUnlock,
    SessionExpired,
    TokenRefresh,
}

/// System event types
#[derive(Debug, Clone)]
pub enum SystemEventType {
    ServerStart,
    ServerStop,
    DatabaseConnection,
    DatabaseDisconnection,
    ErrorOccurred,
    ConfigurationChange,
    PerformanceAlert,
    SystemAlert,
}

/// User action types
#[derive(Debug, Clone)]
pub enum UserActionType {
    SendMessage,
    EditMessage,
    DeleteMessage,
    CreateRoom,
    JoinRoom,
    LeaveRoom,
    InviteUser,
    AcceptInvitation,
    DeclineInvitation,
    UpdateProfile,
    ChangeSettings,
}

/// Security severity levels
#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Action result
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Failure(String),
    Partial(String),
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Enable audit logging
    pub enable_audit: bool,
    /// Enable security logging
    pub enable_security: bool,
    /// Enable performance logging
    pub enable_performance: bool,
    /// Log level
    pub log_level: Level,
    /// Maximum log entries to keep in memory
    pub max_entries: usize,
    /// Log file path
    pub log_file: Option<String>,
}

/// Logging statistics
#[derive(Debug, Clone, Default)]
pub struct LogStats {
    /// Total log entries
    pub total_entries: u64,
    /// Entries by level
    pub entries_by_level: HashMap<String, u64>,
    /// Security events count
    pub security_events: u64,
    /// Authentication events count
    pub auth_events: u64,
    /// System events count
    pub system_events: u64,
    /// User actions count
    pub user_actions: u64,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(config: LogConfig) -> Self {
        Self {
            audit_trail: Arc::new(RwLock::new(AuditTrail {
                max_entries: config.max_entries,
                ..Default::default()
            })),
            config,
            stats: Arc::new(RwLock::new(LogStats::default())),
        }
    }

    /// Log a security event
    pub async fn log_security_event(&self, event: SecurityEvent) {
        if !self.config.enable_security {
            return;
        }

        let level = match event.severity {
            SecuritySeverity::Low => Level::INFO,
            SecuritySeverity::Medium => Level::WARN,
            SecuritySeverity::High => Level::ERROR,
            SecuritySeverity::Critical => Level::ERROR,
        };

        match level {
            Level::ERROR => {
                error!(
                    event_type = format!("{:?}", event.event_type),
                    user_id = event.user_id,
                    ip_address = event.ip_address,
                    severity = format!("{:?}", event.severity),
                    context = format!("{:?}", event.context),
                    "Security Event: {}",
                    event.description
                );
            }
            Level::WARN => {
                warn!(
                    event_type = format!("{:?}", event.event_type),
                    user_id = event.user_id,
                    ip_address = event.ip_address,
                    severity = format!("{:?}", event.severity),
                    context = format!("{:?}", event.context),
                    "Security Event: {}",
                    event.description
                );
            }
            _ => {
                info!(
                    event_type = format!("{:?}", event.event_type),
                    user_id = event.user_id,
                    ip_address = event.ip_address,
                    severity = format!("{:?}", event.severity),
                    context = format!("{:?}", event.context),
                    "Security Event: {}",
                    event.description
                );
            }
        }

        // Store in audit trail
        let mut audit_trail = self.audit_trail.write().await;
        audit_trail.security_events.push(event);
        self.trim_audit_trail(&mut audit_trail).await;

        // Update statistics
        self.update_stats("security").await;
    }

    /// Log an authentication event
    pub async fn log_auth_event(&self, event: AuthEvent) {
        if !self.config.enable_audit {
            return;
        }

        let level = if event.success {
            Level::INFO
        } else {
            Level::WARN
        };

        match level {
            Level::WARN => {
                warn!(
                    event_type = format!("{:?}", event.event_type),
                    user_id = event.user_id,
                    ip_address = event.ip_address,
                    success = event.success,
                    details = format!("{:?}", event.details),
                    "Authentication Event: {:?}",
                    event.event_type
                );
            }
            _ => {
                info!(
                    event_type = format!("{:?}", event.event_type),
                    user_id = event.user_id,
                    ip_address = event.ip_address,
                    success = event.success,
                    details = format!("{:?}", event.details),
                    "Authentication Event: {:?}",
                    event.event_type
                );
            }
        }

        // Store in audit trail
        let mut audit_trail = self.audit_trail.write().await;
        audit_trail.auth_events.push(event);
        self.trim_audit_trail(&mut audit_trail).await;

        // Update statistics
        self.update_stats("auth").await;
    }

    /// Log a system event
    pub async fn log_system_event(&self, event: SystemEvent) {
        match event.level {
            Level::ERROR => {
                error!(
                    event_type = format!("{:?}", event.event_type),
                    component = event.component,
                    context = format!("{:?}", event.context),
                    "System Event: {}",
                    event.description
                );
            }
            Level::WARN => {
                warn!(
                    event_type = format!("{:?}", event.event_type),
                    component = event.component,
                    context = format!("{:?}", event.context),
                    "System Event: {}",
                    event.description
                );
            }
            Level::INFO => {
                info!(
                    event_type = format!("{:?}", event.event_type),
                    component = event.component,
                    context = format!("{:?}", event.context),
                    "System Event: {}",
                    event.description
                );
            }
            _ => {
                debug!(
                    event_type = format!("{:?}", event.event_type),
                    component = event.component,
                    context = format!("{:?}", event.context),
                    "System Event: {}",
                    event.description
                );
            }
        }

        // Store in audit trail
        let mut audit_trail = self.audit_trail.write().await;
        audit_trail.system_events.push(event);
        self.trim_audit_trail(&mut audit_trail).await;

        // Update statistics
        self.update_stats("system").await;
    }

    /// Log a user action
    pub async fn log_user_action(&self, action: UserAction) {
        if !self.config.enable_audit {
            return;
        }

        let level = match action.result {
            ActionResult::Success => Level::INFO,
            ActionResult::Failure(_) => Level::WARN,
            ActionResult::Partial(_) => Level::INFO,
        };

        match level {
            Level::WARN => {
                warn!(
                    action_type = format!("{:?}", action.action_type),
                    user_id = action.user_id,
                    target = action.target,
                    result = format!("{:?}", action.result),
                    context = format!("{:?}", action.context),
                    "User Action: {}",
                    action.description
                );
            }
            _ => {
                info!(
                    action_type = format!("{:?}", action.action_type),
                    user_id = action.user_id,
                    target = action.target,
                    result = format!("{:?}", action.result),
                    context = format!("{:?}", action.context),
                    "User Action: {}",
                    action.description
                );
            }
        }

        // Store in audit trail
        let mut audit_trail = self.audit_trail.write().await;
        audit_trail.user_actions.push(action);
        self.trim_audit_trail(&mut audit_trail).await;

        // Update statistics
        self.update_stats("user_action").await;
    }

    /// Log an error with context
    pub async fn log_error(&self, error: &TcpError, context: HashMap<String, String>) {
        let system_event = SystemEvent {
            event_type: SystemEventType::ErrorOccurred,
            component: "tcp_server".to_string(),
            description: error.user_message(),
            timestamp: current_timestamp(),
            level: error.log_level(),
            context,
        };

        self.log_system_event(system_event).await;
    }

    /// Get audit trail
    pub async fn get_audit_trail(&self) -> AuditTrail {
        self.audit_trail.read().await.clone()
    }

    /// Get security events
    pub async fn get_security_events(&self) -> Vec<SecurityEvent> {
        let audit_trail = self.audit_trail.read().await;
        audit_trail.security_events.clone()
    }

    /// Get authentication events
    pub async fn get_auth_events(&self) -> Vec<AuthEvent> {
        let audit_trail = self.audit_trail.read().await;
        audit_trail.auth_events.clone()
    }

    /// Get user actions
    pub async fn get_user_actions(&self) -> Vec<UserAction> {
        let audit_trail = self.audit_trail.read().await;
        audit_trail.user_actions.clone()
    }

    /// Get logging statistics
    pub async fn get_stats(&self) -> LogStats {
        self.stats.read().await.clone()
    }

    /// Clear audit trail
    pub async fn clear_audit_trail(&self) {
        let mut audit_trail = self.audit_trail.write().await;
        audit_trail.security_events.clear();
        audit_trail.auth_events.clear();
        audit_trail.system_events.clear();
        audit_trail.user_actions.clear();
    }

    /// Trim audit trail to maximum size
    async fn trim_audit_trail(&self, audit_trail: &mut AuditTrail) {
        let max_entries = audit_trail.max_entries;

        if audit_trail.security_events.len() > max_entries {
            audit_trail
                .security_events
                .drain(0..audit_trail.security_events.len() - max_entries);
        }

        if audit_trail.auth_events.len() > max_entries {
            audit_trail
                .auth_events
                .drain(0..audit_trail.auth_events.len() - max_entries);
        }

        if audit_trail.system_events.len() > max_entries {
            audit_trail
                .system_events
                .drain(0..audit_trail.system_events.len() - max_entries);
        }

        if audit_trail.user_actions.len() > max_entries {
            audit_trail
                .user_actions
                .drain(0..audit_trail.user_actions.len() - max_entries);
        }
    }

    /// Update logging statistics
    async fn update_stats(&self, category: &str) {
        let mut stats = self.stats.write().await;
        stats.total_entries += 1;

        match category {
            "security" => stats.security_events += 1,
            "auth" => stats.auth_events += 1,
            "system" => stats.system_events += 1,
            "user_action" => stats.user_actions += 1,
            _ => {}
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enable_audit: true,
            enable_security: true,
            enable_performance: true,
            log_level: Level::INFO,
            max_entries: 10000,
            log_file: None,
        }
    }
}

/// Global structured logger instance
static STRUCTURED_LOGGER: std::sync::OnceLock<StructuredLogger> = std::sync::OnceLock::new();

/// Get the global structured logger
pub fn get_structured_logger() -> &'static StructuredLogger {
    STRUCTURED_LOGGER.get_or_init(|| StructuredLogger::new(LogConfig::default()))
}

/// Initialize the global structured logger
pub fn init_structured_logger(config: LogConfig) -> &'static StructuredLogger {
    STRUCTURED_LOGGER.get_or_init(|| StructuredLogger::new(config))
}

/// Convenience macro for logging security events
#[macro_export]
macro_rules! log_security {
    ($event_type:expr, $description:expr) => {
        $crate::server::logging::get_structured_logger()
            .log_security_event(SecurityEvent {
                event_type: $event_type,
                user_id: None,
                ip_address: None,
                description: $description.to_string(),
                timestamp: $crate::server::storage::current_timestamp(),
                severity: SecuritySeverity::Medium,
                context: std::collections::HashMap::new(),
            })
            .await;
    };
    ($event_type:expr, $description:expr, $user_id:expr) => {
        $crate::server::logging::get_structured_logger()
            .log_security_event(SecurityEvent {
                event_type: $event_type,
                user_id: Some($user_id.to_string()),
                ip_address: None,
                description: $description.to_string(),
                timestamp: $crate::server::storage::current_timestamp(),
                severity: SecuritySeverity::Medium,
                context: std::collections::HashMap::new(),
            })
            .await;
    };
}

/// Convenience macro for logging authentication events
#[macro_export]
macro_rules! log_auth {
    ($event_type:expr, $user_id:expr, $success:expr) => {
        $crate::server::logging::get_structured_logger()
            .log_auth_event(AuthEvent {
                event_type: $event_type,
                user_id: $user_id.to_string(),
                ip_address: None,
                success: $success,
                timestamp: $crate::server::storage::current_timestamp(),
                details: std::collections::HashMap::new(),
            })
            .await;
    };
}

/// Convenience macro for logging user actions
#[macro_export]
macro_rules! log_user_action {
    ($action_type:expr, $user_id:expr, $description:expr, $result:expr) => {
        $crate::server::logging::get_structured_logger()
            .log_user_action(UserAction {
                action_type: $action_type,
                user_id: $user_id.to_string(),
                target: None,
                description: $description.to_string(),
                timestamp: $crate::server::storage::current_timestamp(),
                result: $result,
                context: std::collections::HashMap::new(),
            })
            .await;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_structured_logger_creation() {
        let config = LogConfig::default();
        let logger = StructuredLogger::new(config);
        let stats = logger.get_stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let config = LogConfig::default();
        let logger = StructuredLogger::new(config);

        let event = SecurityEvent {
            event_type: SecurityEventType::IntrusionAttempt,
            user_id: Some("user123".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            description: "Test security event".to_string(),
            timestamp: current_timestamp(),
            severity: SecuritySeverity::High,
            context: HashMap::new(),
        };

        logger.log_security_event(event).await;

        let events = logger.get_security_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some("user123".to_string()));
    }

    #[tokio::test]
    async fn test_auth_event_logging() {
        let config = LogConfig::default();
        let logger = StructuredLogger::new(config);

        let event = AuthEvent {
            event_type: AuthEventType::Login,
            user_id: "user123".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            success: true,
            timestamp: current_timestamp(),
            details: HashMap::new(),
        };

        logger.log_auth_event(event).await;

        let events = logger.get_auth_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, "user123");
        assert!(events[0].success);
    }

    #[tokio::test]
    async fn test_user_action_logging() {
        let config = LogConfig::default();
        let logger = StructuredLogger::new(config);

        let action = UserAction {
            action_type: UserActionType::SendMessage,
            user_id: "user123".to_string(),
            target: Some("room456".to_string()),
            description: "Sent a message".to_string(),
            timestamp: current_timestamp(),
            result: ActionResult::Success,
            context: HashMap::new(),
        };

        logger.log_user_action(action).await;

        let actions = logger.get_user_actions().await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].user_id, "user123");
        assert_eq!(actions[0].target, Some("room456".to_string()));
    }

    #[tokio::test]
    async fn test_audit_trail_trimming() {
        let config = LogConfig {
            max_entries: 2,
            ..Default::default()
        };
        let logger = StructuredLogger::new(config);

        // Add 3 security events
        for i in 0..3 {
            let event = SecurityEvent {
                event_type: SecurityEventType::IntrusionAttempt,
                user_id: Some(format!("user{}", i)),
                ip_address: None,
                description: format!("Test event {}", i),
                timestamp: current_timestamp(),
                severity: SecuritySeverity::Low,
                context: HashMap::new(),
            };
            logger.log_security_event(event).await;
        }

        let events = logger.get_security_events().await;
        assert_eq!(events.len(), 2); // Should be trimmed to max_entries
    }
}
