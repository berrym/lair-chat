//! Security framework for the TCP server
//!
//! This module provides comprehensive security validation, intrusion detection,
//! and threat prevention for the TCP server.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::server::error::{TcpError, ValidationError};
use crate::server::logging::{SecurityEvent, SecurityEventType, SecuritySeverity};
use crate::server::storage::current_timestamp;
use crate::server::validation::ValidatedInput;

/// Security validation middleware
pub struct SecurityMiddleware {
    /// Intrusion detection system
    intrusion_detector: Arc<RwLock<IntrusionDetector>>,
    /// Rate limiter for security checks
    rate_limiter: Arc<RwLock<SecurityRateLimiter>>,
    /// Audit logger
    audit_logger: Arc<RwLock<SecurityAuditLogger>>,
    /// Security configuration
    config: SecurityConfig,
}

/// Intrusion detection system
#[derive(Debug, Clone)]
pub struct IntrusionDetector {
    /// Suspicious activity tracking
    suspicious_activities: HashMap<String, SuspiciousActivity>,
    /// Blocked IP addresses
    blocked_ips: HashMap<String, BlockedIp>,
    /// Failed login attempts
    failed_logins: HashMap<String, FailedLoginAttempts>,
    /// Configuration
    config: IntrusionConfig,
}

/// Security rate limiter
#[derive(Debug, Clone)]
pub struct SecurityRateLimiter {
    /// Rate limits per IP
    ip_limits: HashMap<String, SecurityRateLimit>,
    /// Rate limits per user
    user_limits: HashMap<String, SecurityRateLimit>,
    /// Configuration
    config: SecurityRateLimitConfig,
}

/// Security audit logger
#[derive(Debug, Clone, Default)]
pub struct SecurityAuditLogger {
    /// Security events
    events: Vec<SecurityEvent>,
    /// Event statistics
    stats: SecurityEventStats,
}

/// Suspicious activity tracking
#[derive(Debug, Clone)]
pub struct SuspiciousActivity {
    /// IP address
    pub ip_address: String,
    /// User ID (if known)
    pub user_id: Option<String>,
    /// Activity type
    pub activity_type: SuspiciousActivityType,
    /// First occurrence
    pub first_seen: Instant,
    /// Last occurrence
    pub last_seen: Instant,
    /// Occurrence count
    pub count: u32,
    /// Severity level
    pub severity: ThreatSeverity,
}

/// Blocked IP information
#[derive(Debug, Clone)]
pub struct BlockedIp {
    /// IP address
    pub ip_address: String,
    /// Reason for blocking
    pub reason: String,
    /// Block timestamp
    pub blocked_at: Instant,
    /// Block duration
    pub duration: Duration,
    /// Whether the block is permanent
    pub permanent: bool,
}

/// Failed login attempts tracking
#[derive(Debug, Clone)]
pub struct FailedLoginAttempts {
    /// IP address
    pub ip_address: String,
    /// User ID (if known)
    pub user_id: Option<String>,
    /// Number of attempts
    pub attempts: u32,
    /// First attempt timestamp
    pub first_attempt: Instant,
    /// Last attempt timestamp
    pub last_attempt: Instant,
    /// Whether IP is locked
    pub locked: bool,
}

/// Security rate limit
#[derive(Debug, Clone)]
pub struct SecurityRateLimit {
    /// Number of requests in current window
    pub count: u32,
    /// Window start time
    pub window_start: Instant,
    /// Rate limit threshold
    pub limit: u32,
    /// Whether rate limit is exceeded
    pub exceeded: bool,
}

/// Security event statistics
#[derive(Debug, Clone, Default)]
pub struct SecurityEventStats {
    /// Total security events
    pub total_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events by severity
    pub events_by_severity: HashMap<String, u64>,
    /// Blocked IPs count
    pub blocked_ips: u32,
    /// Failed login attempts
    pub failed_logins: u32,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable intrusion detection
    pub enable_intrusion_detection: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Maximum failed login attempts
    pub max_failed_logins: u32,
    /// Login attempt window
    pub login_attempt_window: Duration,
    /// IP block duration
    pub ip_block_duration: Duration,
}

/// Intrusion detection configuration
#[derive(Debug, Clone)]
pub struct IntrusionConfig {
    /// Threshold for suspicious activity
    pub suspicious_activity_threshold: u32,
    /// Time window for activity tracking
    pub activity_window: Duration,
    /// Automatic IP blocking enabled
    pub auto_block_enabled: bool,
    /// IP block duration
    pub block_duration: Duration,
}

/// Security rate limiting configuration
#[derive(Debug, Clone)]
pub struct SecurityRateLimitConfig {
    /// Requests per minute per IP
    pub requests_per_minute_per_ip: u32,
    /// Requests per minute per user
    pub requests_per_minute_per_user: u32,
    /// Rate limit window
    pub window_duration: Duration,
}

/// Suspicious activity types
#[derive(Debug, Clone)]
pub enum SuspiciousActivityType {
    RepeatedFailedLogins,
    InvalidInputPatterns,
    RateLimitExceeded,
    SuspiciousCommands,
    MalformedRequests,
    InjectionAttempts,
    BruteForceAttack,
    AccountEnumeration,
}

/// Threat severity levels
#[derive(Debug, Clone)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security validation result
pub type SecurityResult<T> = Result<T, TcpError>;

impl SecurityMiddleware {
    /// Create a new security middleware
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            intrusion_detector: Arc::new(RwLock::new(IntrusionDetector::new(
                IntrusionConfig::default(),
            ))),
            rate_limiter: Arc::new(RwLock::new(SecurityRateLimiter::new(
                SecurityRateLimitConfig::default(),
            ))),
            audit_logger: Arc::new(RwLock::new(SecurityAuditLogger::default())),
            config,
        }
    }

    /// Validate request security
    pub async fn validate_request(
        &self,
        input: &ValidatedInput,
        ip_address: Option<&str>,
    ) -> SecurityResult<()> {
        // Check rate limits
        if self.config.enable_rate_limiting {
            self.check_rate_limits(input, ip_address).await?;
        }

        // Check for intrusion patterns
        if self.config.enable_intrusion_detection {
            self.check_intrusion_patterns(input, ip_address).await?;
        }

        // Log security event
        if self.config.enable_audit_logging {
            self.log_security_event(input, ip_address).await;
        }

        Ok(())
    }

    /// Check rate limits for the request
    async fn check_rate_limits(
        &self,
        input: &ValidatedInput,
        ip_address: Option<&str>,
    ) -> SecurityResult<()> {
        let mut rate_limiter = self.rate_limiter.write().await;

        // Check IP-based rate limit
        if let Some(ip) = ip_address {
            if rate_limiter.is_ip_rate_limited(ip) {
                return Err(TcpError::SecurityViolation(format!(
                    "Rate limit exceeded for IP: {}",
                    ip
                )));
            }
        }

        // Check user-based rate limit
        if let Some(user_id) = &input.user_id {
            if rate_limiter.is_user_rate_limited(user_id) {
                return Err(TcpError::SecurityViolation(format!(
                    "Rate limit exceeded for user: {}",
                    user_id
                )));
            }
        }

        Ok(())
    }

    /// Check for intrusion patterns
    async fn check_intrusion_patterns(
        &self,
        input: &ValidatedInput,
        ip_address: Option<&str>,
    ) -> SecurityResult<()> {
        let mut intrusion_detector = self.intrusion_detector.write().await;

        // Check if IP is blocked
        if let Some(ip) = ip_address {
            if intrusion_detector.is_ip_blocked(ip) {
                return Err(TcpError::IntrusionDetected(format!(
                    "IP address {} is blocked",
                    ip
                )));
            }
        }

        // Check for suspicious patterns in input
        if self.has_suspicious_patterns(&input.sanitized_input) {
            if let Some(ip) = ip_address {
                intrusion_detector
                    .record_suspicious_activity(
                        ip,
                        input.user_id.as_deref(),
                        SuspiciousActivityType::SuspiciousCommands,
                    )
                    .await;
            }
            return Err(TcpError::SuspiciousActivity(
                "Suspicious patterns detected in input".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if input contains suspicious patterns
    fn has_suspicious_patterns(&self, input: &str) -> bool {
        let suspicious_patterns = [
            "script",
            "eval",
            "exec",
            "system",
            "rm -rf",
            "DROP TABLE",
            "DELETE FROM",
            "INSERT INTO",
            "UPDATE SET",
            "../",
            "..\\",
            "cmd",
            "powershell",
            "bash",
            "sh",
            "cat /etc/passwd",
            "wget",
            "curl",
            "nc ",
            "netcat",
        ];

        let input_lower = input.to_lowercase();
        suspicious_patterns
            .iter()
            .any(|pattern| input_lower.contains(pattern))
    }

    /// Log security event
    async fn log_security_event(&self, input: &ValidatedInput, ip_address: Option<&str>) {
        let mut audit_logger = self.audit_logger.write().await;

        let event = SecurityEvent {
            event_type: SecurityEventType::AccessDenied,
            user_id: input.user_id.clone(),
            ip_address: ip_address.map(|ip| ip.to_string()),
            description: format!("Security validation for command: {}", input.command),
            timestamp: current_timestamp(),
            severity: SecuritySeverity::Low,
            context: HashMap::new(),
        };

        audit_logger.log_event(event).await;
    }

    /// Check if user should be blocked
    pub async fn should_block_user(&self, user_id: &str) -> bool {
        let intrusion_detector = self.intrusion_detector.read().await;
        intrusion_detector.should_block_user(user_id)
    }

    /// Block IP address
    pub async fn block_ip(&self, ip_address: &str, reason: &str, duration: Duration) {
        let mut intrusion_detector = self.intrusion_detector.write().await;
        intrusion_detector
            .block_ip(ip_address, reason, duration)
            .await;
    }

    /// Record failed login attempt
    pub async fn record_failed_login(&self, ip_address: &str, user_id: Option<&str>) {
        let mut intrusion_detector = self.intrusion_detector.write().await;
        intrusion_detector
            .record_failed_login(ip_address, user_id)
            .await;
    }

    /// Record suspicious activity and trigger automated response
    pub async fn record_suspicious_activity(
        &self,
        ip_address: std::net::IpAddr,
        user_id: Option<String>,
        activity_type: &str,
        description: &str,
    ) -> SecurityResult<()> {
        let mut intrusion_detector = self.intrusion_detector.write().await;
        intrusion_detector
            .record_suspicious_activity(&ip_address.to_string(), user_id.as_deref(), activity_type)
            .await?;

        // Log security event
        self.log_security_event(
            ip_address,
            user_id.clone(),
            activity_type,
            description.to_string(),
        )
        .await;

        // Check if we should trigger automated response
        let should_block = self
            .should_trigger_automated_block(&ip_address.to_string(), activity_type)
            .await;
        if should_block {
            // Automatic blocking for severe threats
            let block_duration = match activity_type {
                "suspicious_message_pattern" => std::time::Duration::from_secs(300), // 5 minutes
                "brute_force_attack" => std::time::Duration::from_secs(3600),        // 1 hour
                "injection_attempt" => std::time::Duration::from_secs(1800),         // 30 minutes
                "rate_limit_exceeded" => std::time::Duration::from_secs(600),        // 10 minutes
                _ => std::time::Duration::from_secs(300), // Default 5 minutes
            };

            intrusion_detector
                .block_ip(
                    &ip_address.to_string(),
                    &format!("Automated block: {}", activity_type),
                    block_duration,
                )
                .await;

            // Log the automated response
            self.log_security_event(
                ip_address,
                user_id,
                "automated_block",
                format!(
                    "Automatically blocked IP for {} minutes due to: {}",
                    block_duration.as_secs() / 60,
                    activity_type
                ),
            )
            .await;
        }

        Ok(())
    }

    /// Check if IP should be blocked
    pub async fn should_block_user(&self, ip_address: std::net::IpAddr) -> bool {
        let intrusion_detector = self.intrusion_detector.read().await;
        intrusion_detector.is_ip_blocked(&ip_address.to_string())
    }

    /// Log security event with IP address
    pub async fn log_security_event(
        &self,
        ip_address: std::net::IpAddr,
        user_id: Option<String>,
        event_type: &str,
        description: String,
    ) {
        let event = format!(
            "[{}] IP: {} | User: {} | Event: {} | Description: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            ip_address,
            user_id.unwrap_or_else(|| "unknown".to_string()),
            event_type,
            description
        );

        let mut audit_logger = self.audit_logger.write().await;
        audit_logger.log_event(event).await;
    }

    /// Check if automated blocking should be triggered
    async fn should_trigger_automated_block(&self, ip_address: &str, activity_type: &str) -> bool {
        let intrusion_detector = self.intrusion_detector.read().await;

        // Check existing suspicious activities for this IP
        if let Some(activities) = intrusion_detector.suspicious_activities.get(ip_address) {
            let recent_activities = activities
                .iter()
                .filter(|activity| {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    now - activity.last_seen < 300 // Last 5 minutes
                })
                .count();

            // Trigger blocking if:
            // - More than 3 suspicious activities in 5 minutes
            // - Any injection attempt
            // - Severe threat patterns
            match activity_type {
                "suspicious_message_pattern" | "injection_attempt" | "brute_force_attack" => true,
                _ => recent_activities > 3,
            }
        } else {
            // First time offense - block only for severe threats
            matches!(activity_type, "injection_attempt" | "brute_force_attack")
        }
    }

    /// Suspend user account
    pub async fn suspend_user(&self, user_id: &str, reason: &str, duration: std::time::Duration) {
        // Log suspension event
        self.log_security_event(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), // Placeholder IP
            Some(user_id.to_string()),
            "user_suspended",
            format!(
                "User suspended for {} seconds. Reason: {}",
                duration.as_secs(),
                reason
            ),
        )
        .await;

        // Note: User suspension would be implemented in the user management system
        // For now, we log the event for audit purposes
        tracing::warn!("User {} suspended for: {}", user_id, reason);
    }

    /// Get security statistics
    pub async fn get_security_stats(&self) -> SecurityEventStats {
        let audit_logger = self.audit_logger.read().await;
        audit_logger.stats.clone()
    }

    /// Generate security report
    pub async fn generate_security_report(&self) -> String {
        let audit_logger = self.audit_logger.read().await;
        let intrusion_detector = self.intrusion_detector.read().await;

        let total_events = audit_logger.events.len();
        let blocked_ips = intrusion_detector.blocked_ips.len();
        let suspicious_activities = intrusion_detector.suspicious_activities.len();

        format!(
            "=== SECURITY REPORT ===\n\
            Total Security Events: {}\n\
            Blocked IPs: {}\n\
            IPs with Suspicious Activity: {}\n\
            Report Generated: {}\n\
            ========================",
            total_events,
            blocked_ips,
            suspicious_activities,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Security health check
    pub async fn security_health_check(&self) -> SecurityHealthStatus {
        let intrusion_detector = self.intrusion_detector.read().await;
        let audit_logger = self.audit_logger.read().await;

        let active_blocks = intrusion_detector.blocked_ips.len();
        let recent_suspicious = intrusion_detector
            .suspicious_activities
            .values()
            .flatten()
            .filter(|activity| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now - activity.last_seen < 3600 // Last hour
            })
            .count();

        let total_events = audit_logger.events.len();

        SecurityHealthStatus {
            status: if active_blocks > 10 || recent_suspicious > 20 {
                "CRITICAL"
            } else if active_blocks > 5 || recent_suspicious > 10 {
                "WARNING"
            } else {
                "HEALTHY"
            }
            .to_string(),
            active_blocks,
            recent_suspicious_activities: recent_suspicious,
            total_security_events: total_events,
            last_check: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Get security configuration
    pub async fn get_security_config(&self) -> SecurityConfig {
        self.config.clone()
    }

    /// Update security configuration
    pub async fn update_security_config(&mut self, new_config: SecurityConfig) {
        self.config = new_config;
        // Reinitialize components with new config if needed
    }

    /// Force unblock IP (admin function)
    pub async fn force_unblock_ip(&self, ip_address: &str) -> SecurityResult<()> {
        let mut intrusion_detector = self.intrusion_detector.write().await;
        intrusion_detector.blocked_ips.remove(ip_address);

        self.log_security_event(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), // Admin action
            None,
            "admin_unblock",
            format!("Administrator force-unblocked IP: {}", ip_address),
        )
        .await;

        Ok(())
    }

    /// Get recent security events
    pub async fn get_recent_security_events(&self, limit: usize) -> Vec<String> {
        let audit_logger = self.audit_logger.read().await;
        audit_logger
            .events
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Security health status
#[derive(Debug, Clone)]
pub struct SecurityHealthStatus {
    pub status: String,
    pub active_blocks: usize,
    pub recent_suspicious_activities: usize,
    pub total_security_events: usize,
    pub last_check: u64,
}

impl IntrusionDetector {
    /// Create a new intrusion detector
    pub fn new(config: IntrusionConfig) -> Self {
        Self {
            suspicious_activities: HashMap::new(),
            blocked_ips: HashMap::new(),
            failed_logins: HashMap::new(),
            config,
        }
    }

    /// Check if IP is blocked
    pub fn is_ip_blocked(&self, ip_address: &str) -> bool {
        if let Some(blocked_ip) = self.blocked_ips.get(ip_address) {
            if blocked_ip.permanent {
                return true;
            }
            if blocked_ip.blocked_at.elapsed() < blocked_ip.duration {
                return true;
            }
        }
        false
    }

    /// Record suspicious activity
    pub async fn record_suspicious_activity(
        &mut self,
        ip_address: &str,
        user_id: Option<&str>,
        activity_type: SuspiciousActivityType,
    ) {
        let now = Instant::now();
        let key = format!("{}_{:?}", ip_address, activity_type);

        let activity =
            self.suspicious_activities
                .entry(key)
                .or_insert_with(|| SuspiciousActivity {
                    ip_address: ip_address.to_string(),
                    user_id: user_id.map(|u| u.to_string()),
                    activity_type: activity_type.clone(),
                    first_seen: now,
                    last_seen: now,
                    count: 0,
                    severity: ThreatSeverity::Low,
                });

        activity.count += 1;
        activity.last_seen = now;

        // Update severity based on count
        activity.severity = match activity.count {
            1..=5 => ThreatSeverity::Low,
            6..=15 => ThreatSeverity::Medium,
            16..=30 => ThreatSeverity::High,
            _ => ThreatSeverity::Critical,
        };

        // Auto-block if threshold exceeded
        if self.config.auto_block_enabled
            && activity.count >= self.config.suspicious_activity_threshold
        {
            self.block_ip(
                ip_address,
                &format!("Suspicious activity: {:?}", activity_type),
                self.config.block_duration,
            )
            .await;
        }
    }

    /// Record failed login attempt
    pub async fn record_failed_login(&mut self, ip_address: &str, user_id: Option<&str>) {
        let now = Instant::now();
        let key = ip_address.to_string();

        let failed_login = self
            .failed_logins
            .entry(key)
            .or_insert_with(|| FailedLoginAttempts {
                ip_address: ip_address.to_string(),
                user_id: user_id.map(|u| u.to_string()),
                attempts: 0,
                first_attempt: now,
                last_attempt: now,
                locked: false,
            });

        failed_login.attempts += 1;
        failed_login.last_attempt = now;

        // Lock IP if too many failed attempts
        if failed_login.attempts >= 5 {
            // Default threshold
            failed_login.locked = true;
            self.block_ip(
                ip_address,
                "Too many failed login attempts",
                Duration::from_minutes(15),
            )
            .await;
        }
    }

    /// Block IP address
    pub async fn block_ip(&mut self, ip_address: &str, reason: &str, duration: Duration) {
        let blocked_ip = BlockedIp {
            ip_address: ip_address.to_string(),
            reason: reason.to_string(),
            blocked_at: Instant::now(),
            duration,
            permanent: false,
        };

        self.blocked_ips.insert(ip_address.to_string(), blocked_ip);

        warn!(
            ip_address = ip_address,
            reason = reason,
            duration_secs = duration.as_secs(),
            "IP address blocked"
        );
    }

    /// Check if user should be blocked
    pub fn should_block_user(&self, user_id: &str) -> bool {
        // Check if user has too many suspicious activities
        self.suspicious_activities.values().any(|activity| {
            activity.user_id.as_deref() == Some(user_id)
                && matches!(activity.severity, ThreatSeverity::Critical)
        })
    }
}

impl SecurityRateLimiter {
    /// Create a new security rate limiter
    pub fn new(config: SecurityRateLimitConfig) -> Self {
        Self {
            ip_limits: HashMap::new(),
            user_limits: HashMap::new(),
            config,
        }
    }

    /// Check if IP is rate limited
    pub fn is_ip_rate_limited(&mut self, ip_address: &str) -> bool {
        let now = Instant::now();
        let limit = self
            .ip_limits
            .entry(ip_address.to_string())
            .or_insert_with(|| SecurityRateLimit {
                count: 0,
                window_start: now,
                limit: self.config.requests_per_minute_per_ip,
                exceeded: false,
            });

        self.update_rate_limit(limit, now)
    }

    /// Check if user is rate limited
    pub fn is_user_rate_limited(&mut self, user_id: &str) -> bool {
        let now = Instant::now();
        let limit = self
            .user_limits
            .entry(user_id.to_string())
            .or_insert_with(|| SecurityRateLimit {
                count: 0,
                window_start: now,
                limit: self.config.requests_per_minute_per_user,
                exceeded: false,
            });

        self.update_rate_limit(limit, now)
    }

    /// Update rate limit and check if exceeded
    fn update_rate_limit(&mut self, limit: &mut SecurityRateLimit, now: Instant) -> bool {
        // Reset window if needed
        if now.duration_since(limit.window_start) >= self.config.window_duration {
            limit.count = 0;
            limit.window_start = now;
            limit.exceeded = false;
        }

        // Check if limit exceeded
        if limit.count >= limit.limit {
            limit.exceeded = true;
            return true;
        }

        // Increment count
        limit.count += 1;
        false
    }
}

impl SecurityAuditLogger {
    /// Log security event
    pub async fn log_event(&mut self, event: SecurityEvent) {
        self.events.push(event.clone());

        // Update statistics
        self.stats.total_events += 1;

        let event_type = format!("{:?}", event.event_type);
        *self.stats.events_by_type.entry(event_type).or_insert(0) += 1;

        let severity = format!("{:?}", event.severity);
        *self.stats.events_by_severity.entry(severity).or_insert(0) += 1;
    }

    /// Get security events
    pub fn get_events(&self) -> &[SecurityEvent] {
        &self.events
    }

    /// Clear events
    pub fn clear_events(&mut self) {
        self.events.clear();
        self.stats = SecurityEventStats::default();
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_intrusion_detection: true,
            enable_rate_limiting: true,
            enable_audit_logging: true,
            max_failed_logins: 5,
            login_attempt_window: Duration::from_minutes(15),
            ip_block_duration: Duration::from_minutes(30),
        }
    }
}

impl Default for IntrusionConfig {
    fn default() -> Self {
        Self {
            suspicious_activity_threshold: 10,
            activity_window: Duration::from_minutes(5),
            auto_block_enabled: true,
            block_duration: Duration::from_minutes(30),
        }
    }
}

impl Default for SecurityRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute_per_ip: 100,
            requests_per_minute_per_user: 60,
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Global security middleware instance
static SECURITY_MIDDLEWARE: std::sync::OnceLock<SecurityMiddleware> = std::sync::OnceLock::new();

/// Get the global security middleware
pub fn get_security_middleware() -> &'static SecurityMiddleware {
    SECURITY_MIDDLEWARE.get_or_init(|| SecurityMiddleware::new(SecurityConfig::default()))
}

/// Initialize the global security middleware
pub fn init_security_middleware(config: SecurityConfig) -> &'static SecurityMiddleware {
    SECURITY_MIDDLEWARE.get_or_init(|| SecurityMiddleware::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_middleware_creation() {
        let config = SecurityConfig::default();
        let middleware = SecurityMiddleware::new(config);
        let stats = middleware.get_security_stats().await;
        assert_eq!(stats.total_events, 0);
    }

    #[tokio::test]
    async fn test_intrusion_detection() {
        let config = IntrusionConfig::default();
        let mut detector = IntrusionDetector::new(config);

        // Should not be blocked initially
        assert!(!detector.is_ip_blocked("192.168.1.1"));

        // Block IP
        detector
            .block_ip("192.168.1.1", "Test block", Duration::from_secs(60))
            .await;

        // Should be blocked now
        assert!(detector.is_ip_blocked("192.168.1.1"));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = SecurityRateLimitConfig {
            requests_per_minute_per_ip: 2,
            requests_per_minute_per_user: 2,
            window_duration: Duration::from_secs(60),
        };
        let mut limiter = SecurityRateLimiter::new(config);

        // First request should not be limited
        assert!(!limiter.is_ip_rate_limited("192.168.1.1"));

        // Second request should not be limited
        assert!(!limiter.is_ip_rate_limited("192.168.1.1"));

        // Third request should be limited
        assert!(limiter.is_ip_rate_limited("192.168.1.1"));
    }

    #[tokio::test]
    async fn test_suspicious_activity_recording() {
        let config = IntrusionConfig::default();
        let mut detector = IntrusionDetector::new(config);

        detector
            .record_suspicious_activity(
                "192.168.1.1",
                Some("user123"),
                SuspiciousActivityType::SuspiciousCommands,
            )
            .await;

        let key = format!(
            "192.168.1.1_{:?}",
            SuspiciousActivityType::SuspiciousCommands
        );
        assert!(detector.suspicious_activities.contains_key(&key));
        assert_eq!(detector.suspicious_activities[&key].count, 1);
    }

    #[tokio::test]
    async fn test_failed_login_recording() {
        let config = IntrusionConfig::default();
        let mut detector = IntrusionDetector::new(config);

        detector
            .record_failed_login("192.168.1.1", Some("user123"))
            .await;

        assert!(detector.failed_logins.contains_key("192.168.1.1"));
        assert_eq!(detector.failed_logins["192.168.1.1"].attempts, 1);
    }
}
