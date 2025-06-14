//! Stress testing module for Lair-Chat
//!
//! This module contains stress tests designed to verify system behavior under heavy load
//! and extreme conditions. These tests are more intensive than regular integration tests
//! and may take longer to run.

mod connection_stress_tests;

use std::time::Duration;

/// Default timeout for stress tests
pub const STRESS_TEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Default number of concurrent operations for stress tests
pub const DEFAULT_CONCURRENT_OPS: usize = 100;

/// Maximum number of retries for operations under stress
pub const MAX_RETRIES: usize = 3;

/// Stress test configuration parameters
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Maximum number of concurrent connections to attempt
    pub max_connections: usize,
    /// Size of connection batches for concurrent testing
    pub batch_size: usize,
    /// Duration of sustained load tests
    pub sustained_duration: Duration,
    /// Size of messages used in stress tests
    pub message_size: usize,
    /// Maximum allowed error rate (0.0 - 1.0)
    pub max_error_rate: f64,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            batch_size: 50,
            sustained_duration: Duration::from_secs(30),
            message_size: 1024,   // 1KB
            max_error_rate: 0.05, // 5% error rate allowed
        }
    }
}

/// Helper function to determine if a test result meets performance criteria
pub fn meets_performance_criteria(
    success_count: usize,
    total_attempts: usize,
    required_success_rate: f64,
) -> bool {
    let success_rate = success_count as f64 / total_attempts as f64;
    success_rate >= required_success_rate
}

/// Helper function to calculate error rate
pub fn calculate_error_rate(error_count: usize, total_operations: usize) -> f64 {
    error_count as f64 / total_operations as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_criteria_calculation() {
        assert!(meets_performance_criteria(95, 100, 0.95));
        assert!(!meets_performance_criteria(94, 100, 0.95));
        assert!(meets_performance_criteria(100, 100, 0.95));
    }

    #[test]
    fn test_error_rate_calculation() {
        assert_eq!(calculate_error_rate(5, 100), 0.05);
        assert_eq!(calculate_error_rate(0, 100), 0.0);
        assert_eq!(calculate_error_rate(100, 100), 1.0);
    }
}
