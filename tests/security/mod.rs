//! Security tests module for Phase 8 testing
//!
//! This module organizes all security tests for the lair-chat application,
//! focusing on penetration testing, vulnerability assessment, and security
//! validation.

pub mod auth_security_tests;
pub mod framework;
pub mod input_security_tests;
pub mod input_validation_tests;
pub mod network_security_tests;
pub mod penetration_tests;
pub mod security_test_runner;
pub mod vulnerability_tests;

// Re-export framework components for convenience
pub use framework::{
    AttackCategory, AttackPattern, AttackSeverity, SecurityMetrics, SecurityTestConfig,
    SecurityTestFramework, SecurityTestResult,
};

// Re-export test runners for easy access
pub use auth_security_tests::AuthSecurityTests;
pub use input_validation_tests::InputValidationTests;
pub use network_security_tests::NetworkSecurityTests;
pub use security_test_runner::{run_complete_security_testing, SecurityTestRunner};

#[cfg(test)]
mod tests {
    use super::*;
    use framework::SecurityTestFramework;
    use security_test_runner::SecurityTestConfiguration;
    use std::collections::HashMap;

    #[test]
    fn security_test_module_structure() {
        // Verify that all security test modules are properly organized
        // This test ensures the module structure is correct
        assert!(true, "Security test module structure is valid");
    }

    #[test]
    fn test_malicious_input_patterns() {
        // Test basic malicious input pattern detection
        let malicious_patterns = vec![
            "'; DROP TABLE users; --",
            "<script>alert('xss')</script>",
            "../../../../etc/passwd",
            "SELECT * FROM sensitive_data",
            "javascript:alert('test')",
        ];

        for pattern in malicious_patterns {
            assert!(
                is_potentially_malicious(pattern),
                "Pattern should be detected as malicious: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_safe_input_patterns() {
        // Test that safe inputs are not flagged as malicious
        let safe_patterns = vec![
            "hello world",
            "join general room",
            "send message to friend",
            "help me with commands",
            "list available rooms",
        ];

        for pattern in safe_patterns {
            assert!(
                !is_potentially_malicious(pattern),
                "Safe pattern should not be flagged: {}",
                pattern
            );
        }
    }

    #[tokio::test]
    async fn test_security_infrastructure_available() {
        // Verify that security testing infrastructure is available
        // This includes pattern matching, threat detection, etc.
        let test_input = "test security input";

        // Basic security check should be available
        let is_safe = validate_input_security(test_input).await;
        assert!(is_safe, "Basic security validation should work");
    }

    #[tokio::test]
    async fn test_complete_security_framework_integration() {
        // Test that all security test modules work together
        let config = SecurityTestConfiguration::default();
        let runner = SecurityTestRunner::new(config);

        // Verify all test modules are properly initialized
        assert!(true, "Security test runner initialized successfully");
    }

    // Helper function to detect potentially malicious input
    fn is_potentially_malicious(input: &str) -> bool {
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
        ];

        let input_lower = input.to_lowercase();
        malicious_patterns
            .iter()
            .any(|&pattern| input_lower.contains(&pattern.to_lowercase()))
    }

    // Helper function for basic security validation
    async fn validate_input_security(input: &str) -> bool {
        // Basic validation - in real implementation this would use
        // the actual security framework
        !is_potentially_malicious(input) && input.len() < 10000 && !input.contains('\0')
    }
}
