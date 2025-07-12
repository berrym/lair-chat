//! Security Testing Framework for Phase 8 Task 8.3
//!
//! This module provides the core security testing framework for comprehensive
//! penetration testing, including attack simulation, metrics collection,
//! and security validation tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Security test execution result
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityTestResult {
    /// Attack was successfully blocked by security measures
    Blocked,
    /// Attack was detected but not blocked
    Detected,
    /// Attack was neither detected nor blocked (CRITICAL)
    Bypassed,
    /// Test failed due to technical issues
    Failed(String),
}

impl SecurityTestResult {
    pub fn is_secure(&self) -> bool {
        matches!(
            self,
            SecurityTestResult::Blocked | SecurityTestResult::Detected
        )
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, SecurityTestResult::Bypassed)
    }
}

/// Security test metrics for comprehensive assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub total_tests: u32,
    pub blocked_attacks: u32,
    pub detected_attacks: u32,
    pub bypassed_attacks: u32,
    pub failed_tests: u32,
    pub average_detection_time: Duration,
    pub false_positive_rate: f64,
    pub attack_success_rate: f64,
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            blocked_attacks: 0,
            detected_attacks: 0,
            bypassed_attacks: 0,
            failed_tests: 0,
            average_detection_time: Duration::new(0, 0),
            false_positive_rate: 0.0,
            attack_success_rate: 0.0,
        }
    }

    pub fn record_result(&mut self, result: &SecurityTestResult, detection_time: Duration) {
        self.total_tests += 1;

        match result {
            SecurityTestResult::Blocked => self.blocked_attacks += 1,
            SecurityTestResult::Detected => self.detected_attacks += 1,
            SecurityTestResult::Bypassed => self.bypassed_attacks += 1,
            SecurityTestResult::Failed(_) => self.failed_tests += 1,
        }

        // Update average detection time
        let total_time =
            self.average_detection_time.as_millis() as u64 * (self.total_tests - 1) as u64;
        let new_average =
            (total_time + detection_time.as_millis() as u64) / self.total_tests as u64;
        self.average_detection_time = Duration::from_millis(new_average);

        // Calculate attack success rate
        self.attack_success_rate = (self.bypassed_attacks as f64 / self.total_tests as f64) * 100.0;
    }

    pub fn security_score(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }

        let secure_tests = self.blocked_attacks + self.detected_attacks;
        (secure_tests as f64 / self.total_tests as f64) * 100.0
    }
}

/// Attack pattern definitions for comprehensive testing
#[derive(Debug, Clone)]
pub struct AttackPattern {
    pub name: String,
    pub payload: String,
    pub category: AttackCategory,
    pub severity: AttackSeverity,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackCategory {
    SqlInjection,
    XssAttack,
    CommandInjection,
    PathTraversal,
    BufferOverflow,
    AuthenticationBypass,
    SessionHijacking,
    PrivilegeEscalation,
    DdosAttack,
    NetworkScanning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Security test configuration for different test phases
#[derive(Debug, Clone)]
pub struct SecurityTestConfig {
    pub max_concurrent_tests: usize,
    pub test_timeout: Duration,
    pub retry_attempts: u32,
    pub rate_limit_delay: Duration,
    pub baseline_integration: bool,
}

impl Default for SecurityTestConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 10,
            test_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            rate_limit_delay: Duration::from_millis(100),
            baseline_integration: true,
        }
    }
}

/// Core security testing framework
pub struct SecurityTestFramework {
    config: SecurityTestConfig,
    metrics: SecurityMetrics,
    attack_patterns: HashMap<AttackCategory, Vec<AttackPattern>>,
    baseline_metrics: Option<PerformanceBaseline>,
}

/// Performance baseline from Task 8.2 integration
#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    pub normal_response_time: Duration,
    pub normal_cpu_usage: f64,
    pub normal_memory_usage: f64,
    pub normal_network_usage: f64,
    pub error_rate_threshold: f64,
}

impl SecurityTestFramework {
    pub fn new(config: SecurityTestConfig) -> Self {
        let mut framework = Self {
            config,
            metrics: SecurityMetrics::new(),
            attack_patterns: HashMap::new(),
            baseline_metrics: None,
        };

        framework.initialize_attack_patterns();
        framework
    }

    /// Initialize comprehensive attack patterns for testing
    fn initialize_attack_patterns(&mut self) {
        // SQL Injection patterns
        let sql_patterns = vec![
            AttackPattern {
                name: "Classic SQL Injection".to_string(),
                payload: "'; DROP TABLE users; --".to_string(),
                category: AttackCategory::SqlInjection,
                severity: AttackSeverity::Critical,
                description: "Basic SQL injection with table drop".to_string(),
            },
            AttackPattern {
                name: "Union-based SQL Injection".to_string(),
                payload: "' UNION SELECT password FROM users --".to_string(),
                category: AttackCategory::SqlInjection,
                severity: AttackSeverity::High,
                description: "Union-based data extraction attempt".to_string(),
            },
            AttackPattern {
                name: "Boolean-based Blind SQL Injection".to_string(),
                payload: "' OR 1=1 --".to_string(),
                category: AttackCategory::SqlInjection,
                severity: AttackSeverity::High,
                description: "Boolean-based blind SQL injection".to_string(),
            },
        ];
        self.attack_patterns
            .insert(AttackCategory::SqlInjection, sql_patterns);

        // XSS patterns
        let xss_patterns = vec![
            AttackPattern {
                name: "Script Tag XSS".to_string(),
                payload: "<script>alert('xss')</script>".to_string(),
                category: AttackCategory::XssAttack,
                severity: AttackSeverity::High,
                description: "Basic script tag XSS injection".to_string(),
            },
            AttackPattern {
                name: "Image XSS".to_string(),
                payload: "<img src=x onerror=alert('xss')>".to_string(),
                category: AttackCategory::XssAttack,
                severity: AttackSeverity::High,
                description: "Image-based XSS with onerror handler".to_string(),
            },
            AttackPattern {
                name: "SVG XSS".to_string(),
                payload: "<svg/onload=alert('xss')>".to_string(),
                category: AttackCategory::XssAttack,
                severity: AttackSeverity::High,
                description: "SVG-based XSS injection".to_string(),
            },
        ];
        self.attack_patterns
            .insert(AttackCategory::XssAttack, xss_patterns);

        // Command Injection patterns
        let cmd_patterns = vec![
            AttackPattern {
                name: "Basic Command Injection".to_string(),
                payload: "; cat /etc/passwd".to_string(),
                category: AttackCategory::CommandInjection,
                severity: AttackSeverity::Critical,
                description: "Basic command injection attempt".to_string(),
            },
            AttackPattern {
                name: "Pipe Command Injection".to_string(),
                payload: "| whoami".to_string(),
                category: AttackCategory::CommandInjection,
                severity: AttackSeverity::High,
                description: "Pipe-based command injection".to_string(),
            },
        ];
        self.attack_patterns
            .insert(AttackCategory::CommandInjection, cmd_patterns);

        // Path Traversal patterns
        let path_patterns = vec![
            AttackPattern {
                name: "Directory Traversal".to_string(),
                payload: "../../../../etc/passwd".to_string(),
                category: AttackCategory::PathTraversal,
                severity: AttackSeverity::High,
                description: "Directory traversal to access system files".to_string(),
            },
            AttackPattern {
                name: "Windows Path Traversal".to_string(),
                payload: "..\\..\\..\\windows\\system32\\config\\sam".to_string(),
                category: AttackCategory::PathTraversal,
                severity: AttackSeverity::High,
                description: "Windows-style path traversal".to_string(),
            },
        ];
        self.attack_patterns
            .insert(AttackCategory::PathTraversal, path_patterns);
    }

    /// Execute a security test with comprehensive monitoring
    pub async fn execute_test<F, Fut>(&mut self, test_name: &str, test_fn: F) -> SecurityTestResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = SecurityTestResult>,
    {
        let start_time = Instant::now();

        // Execute the test with timeout
        let result = tokio::time::timeout(self.config.test_timeout, test_fn()).await;

        let detection_time = start_time.elapsed();

        let test_result = match result {
            Ok(result) => result,
            Err(_) => SecurityTestResult::Failed("Test timeout".to_string()),
        };

        // Record metrics
        self.metrics.record_result(&test_result, detection_time);

        println!(
            "Test '{}' completed in {:?}: {:?}",
            test_name, detection_time, test_result
        );

        test_result
    }

    /// Get attack patterns for a specific category
    pub fn get_attack_patterns(&self, category: &AttackCategory) -> Option<&Vec<AttackPattern>> {
        self.attack_patterns.get(category)
    }

    /// Get current security metrics
    pub fn get_metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }

    /// Set performance baseline for integration testing
    pub fn set_baseline(&mut self, baseline: PerformanceBaseline) {
        self.baseline_metrics = Some(baseline);
    }

    /// Generate comprehensive security report
    pub fn generate_report(&self) -> SecurityReport {
        SecurityReport {
            metrics: self.metrics.clone(),
            test_summary: self.generate_test_summary(),
            recommendations: self.generate_recommendations(),
            compliance_status: self.assess_compliance(),
        }
    }

    fn generate_test_summary(&self) -> String {
        format!(
            "Security Test Summary:\n\
            Total Tests: {}\n\
            Security Score: {:.2}%\n\
            Blocked Attacks: {}\n\
            Detected Attacks: {}\n\
            Bypassed Attacks: {}\n\
            Failed Tests: {}\n\
            Average Detection Time: {:?}",
            self.metrics.total_tests,
            self.metrics.security_score(),
            self.metrics.blocked_attacks,
            self.metrics.detected_attacks,
            self.metrics.bypassed_attacks,
            self.metrics.failed_tests,
            self.metrics.average_detection_time
        )
    }

    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.metrics.bypassed_attacks > 0 {
            recommendations
                .push("CRITICAL: Address bypassed attack vectors immediately".to_string());
        }

        if self.metrics.security_score() < 95.0 {
            recommendations
                .push("Improve security controls to achieve 95%+ security score".to_string());
        }

        if self.metrics.average_detection_time > Duration::from_secs(1) {
            recommendations.push("Optimize security detection time to under 1 second".to_string());
        }

        recommendations
    }

    fn assess_compliance(&self) -> ComplianceStatus {
        let security_score = self.metrics.security_score();
        let has_bypassed = self.metrics.bypassed_attacks > 0;
        let detection_time_ok = self.metrics.average_detection_time <= Duration::from_secs(1);

        if security_score >= 95.0 && !has_bypassed && detection_time_ok {
            ComplianceStatus::Compliant
        } else if security_score >= 90.0 && self.metrics.bypassed_attacks <= 1 {
            ComplianceStatus::PartiallyCompliant
        } else {
            ComplianceStatus::NonCompliant
        }
    }
}

/// Security assessment report
#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub metrics: SecurityMetrics,
    pub test_summary: String,
    pub recommendations: Vec<String>,
    pub compliance_status: ComplianceStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceStatus {
    Compliant,
    PartiallyCompliant,
    NonCompliant,
}

/// Utility functions for security testing
pub mod utils {
    use super::*;

    /// Check if input contains potentially malicious patterns
    pub fn contains_malicious_pattern(input: &str) -> bool {
        let malicious_patterns = [
            "DROP TABLE",
            "<script",
            "javascript:",
            "../",
            "SELECT * FROM",
            "'; ",
            "' OR 1=1",
            "UNION SELECT",
            "<img src=x onerror=",
            "eval(",
            "document.cookie",
            "; cat /",
            "| whoami",
            "../../../../",
            "..\\..\\",
            "cmd.exe",
            "/bin/sh",
        ];

        let input_lower = input.to_lowercase();
        malicious_patterns
            .iter()
            .any(|&pattern| input_lower.contains(&pattern.to_lowercase()))
    }

    /// Simulate network delay for realistic testing
    pub async fn simulate_network_delay() {
        sleep(Duration::from_millis(10)).await;
    }

    /// Generate random attack variation
    pub fn generate_attack_variation(base_payload: &str, variation_type: &str) -> String {
        match variation_type {
            "case_variation" => base_payload.to_uppercase(),
            "encoding" => base_payload.replace("'", "%27").replace(" ", "%20"),
            "obfuscation" => base_payload
                .replace("SELECT", "SeLeCt")
                .replace("DROP", "DrOp"),
            _ => base_payload.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_metrics_initialization() {
        let metrics = SecurityMetrics::new();
        assert_eq!(metrics.total_tests, 0);
        assert_eq!(metrics.security_score(), 0.0);
    }

    #[test]
    fn test_security_test_result_classification() {
        assert!(SecurityTestResult::Blocked.is_secure());
        assert!(SecurityTestResult::Detected.is_secure());
        assert!(!SecurityTestResult::Bypassed.is_secure());
        assert!(SecurityTestResult::Bypassed.is_critical());
    }

    #[test]
    fn test_attack_pattern_initialization() {
        let framework = SecurityTestFramework::new(SecurityTestConfig::default());

        assert!(framework
            .get_attack_patterns(&AttackCategory::SqlInjection)
            .is_some());
        assert!(framework
            .get_attack_patterns(&AttackCategory::XssAttack)
            .is_some());
        assert!(framework
            .get_attack_patterns(&AttackCategory::CommandInjection)
            .is_some());
    }

    #[test]
    fn test_malicious_pattern_detection() {
        assert!(utils::contains_malicious_pattern("'; DROP TABLE users; --"));
        assert!(utils::contains_malicious_pattern(
            "<script>alert('xss')</script>"
        ));
        assert!(utils::contains_malicious_pattern("../../../../etc/passwd"));
        assert!(!utils::contains_malicious_pattern("hello world"));
    }

    #[tokio::test]
    async fn test_security_framework_execution() {
        let mut framework = SecurityTestFramework::new(SecurityTestConfig::default());

        let result = framework
            .execute_test("test", || async { SecurityTestResult::Blocked })
            .await;

        assert_eq!(result, SecurityTestResult::Blocked);
        assert_eq!(framework.get_metrics().total_tests, 1);
        assert_eq!(framework.get_metrics().blocked_attacks, 1);
    }
}
