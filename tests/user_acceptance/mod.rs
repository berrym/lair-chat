//! User Acceptance Testing Framework Module
//!
//! This module provides comprehensive user acceptance testing infrastructure for the lair-chat
//! application, implementing a structured approach to validate user requirements, usability,
//! and system readiness for production deployment.

pub mod compatibility_tests;
pub mod framework;
pub mod functional_tests;
pub mod metrics;
pub mod reporting;
pub mod scenarios;
pub mod test_runner;
pub mod usability_tests;

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// UAT Framework Error Types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UatError {
    /// Environment validation error
    EnvironmentError(String),
    /// Session already exists
    SessionExists(String),
    /// Session not found
    SessionNotFound(String),
    /// Session limit exceeded
    SessionLimitExceeded,
    /// Test execution error
    TestExecutionError(String),
    /// Metrics collection error
    MetricsError(String),
    /// Reporting error
    ReportingError(String),
    /// Configuration error
    ConfigurationError(String),
    /// Network or connectivity error
    NetworkError(String),
    /// Database error
    DatabaseError(String),
    /// File system error
    FileSystemError(String),
    /// Serialization/deserialization error
    SerializationError(String),
    /// Generic error with message
    Other(String),
}

impl std::fmt::Display for UatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UatError::EnvironmentError(msg) => write!(f, "Environment error: {}", msg),
            UatError::SessionExists(id) => write!(f, "Session already exists: {}", id),
            UatError::SessionNotFound(id) => write!(f, "Session not found: {}", id),
            UatError::SessionLimitExceeded => write!(f, "Session limit exceeded"),
            UatError::TestExecutionError(msg) => write!(f, "Test execution error: {}", msg),
            UatError::MetricsError(msg) => write!(f, "Metrics error: {}", msg),
            UatError::ReportingError(msg) => write!(f, "Reporting error: {}", msg),
            UatError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            UatError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            UatError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            UatError::FileSystemError(msg) => write!(f, "File system error: {}", msg),
            UatError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            UatError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for UatError {}

impl From<std::io::Error> for UatError {
    fn from(error: std::io::Error) -> Self {
        UatError::FileSystemError(error.to_string())
    }
}

impl From<serde_json::Error> for UatError {
    fn from(error: serde_json::Error) -> Self {
        UatError::SerializationError(error.to_string())
    }
}

/// Test execution timeout for user acceptance tests
pub const UAT_TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default test user configuration
pub const DEFAULT_TEST_USER_COUNT: usize = 10;

/// UAT Test Result Classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UatResult {
    /// Test passed successfully
    Pass,
    /// Test failed with errors
    Fail,
    /// Test passed with warnings or minor issues
    Warning,
    /// Test was skipped due to dependencies
    Skipped,
    /// Test encountered unexpected errors
    Error,
}

/// UAT Test Priority Levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPriority {
    /// Critical functionality that must work
    Critical,
    /// High priority features
    High,
    /// Medium priority features
    Medium,
    /// Low priority or nice-to-have features
    Low,
}

/// UAT Test Categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestCategory {
    /// Functional testing category
    Functional,
    /// Usability testing category
    Usability,
    /// Compatibility testing category
    Compatibility,
    /// Integration testing category
    Integration,
    /// Performance testing category
    Performance,
}

/// User persona types for testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserPersona {
    /// New user with no experience
    Novice,
    /// Regular user with basic experience
    Regular,
    /// Power user with advanced knowledge
    Expert,
    /// Administrator with full privileges
    Administrator,
    /// Mobile user primarily using mobile interface
    Mobile,
}

/// Device types for compatibility testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// Desktop computer
    Desktop,
    /// Laptop computer
    Laptop,
    /// Tablet device
    Tablet,
    /// Mobile phone
    Mobile,
}

/// Browser types for compatibility testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserType {
    /// Chrome browser
    Chrome,
    /// Firefox browser
    Firefox,
    /// Safari browser
    Safari,
    /// Edge browser
    Edge,
    /// Terminal/CLI interface
    Terminal,
}

/// Operating system types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    /// Windows operating system
    Windows,
    /// macOS operating system
    MacOS,
    /// Linux operating system
    Linux,
    /// iOS mobile operating system
    IOS,
    /// Android mobile operating system
    Android,
}

/// UAT Test Case Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatTestCase {
    /// Unique test case identifier
    pub id: String,
    /// Human-readable test name
    pub name: String,
    /// Detailed test description
    pub description: String,
    /// Test category
    pub category: TestCategory,
    /// Test priority level
    pub priority: TestPriority,
    /// Target user persona
    pub user_persona: UserPersona,
    /// Prerequisites for test execution
    pub prerequisites: Vec<String>,
    /// Test execution steps
    pub steps: Vec<String>,
    /// Expected test outcome
    pub expected_result: String,
    /// Success criteria for the test
    pub success_criteria: Vec<String>,
    /// Test timeout duration
    pub timeout: Duration,
    /// Whether test requires manual validation
    pub manual_validation: bool,
}

/// UAT Test Execution Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatTestResult {
    /// Test case that was executed
    pub test_case: UatTestCase,
    /// Test execution result
    pub result: UatResult,
    /// Test execution duration
    pub duration: Duration,
    /// Test execution timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Success criteria that were met
    pub criteria_met: Vec<String>,
    /// Success criteria that failed
    pub criteria_failed: Vec<String>,
    /// Any warnings or issues encountered
    pub warnings: Vec<String>,
    /// Error messages if test failed
    pub errors: Vec<String>,
    /// Additional notes or observations
    pub notes: Vec<String>,
    /// User feedback if applicable
    pub user_feedback: Option<String>,
}

/// UAT Test Session Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatSession {
    /// Session identifier
    pub session_id: String,
    /// Session name/description
    pub name: String,
    /// Test categories to include
    pub categories: Vec<TestCategory>,
    /// User personas to test with
    pub personas: Vec<UserPersona>,
    /// Target devices for testing
    pub devices: Vec<DeviceType>,
    /// Target browsers for testing
    pub browsers: Vec<BrowserType>,
    /// Target operating systems
    pub operating_systems: Vec<OperatingSystem>,
    /// Maximum test execution time
    pub max_duration: Duration,
    /// Number of test users
    pub user_count: usize,
    /// Whether to include manual tests
    pub include_manual: bool,
    /// Whether to generate detailed reports
    pub detailed_reporting: bool,
}

/// UAT Test Environment Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatEnvironment {
    /// Environment name
    pub name: String,
    /// Server endpoint for testing
    pub server_endpoint: String,
    /// Database configuration
    pub database_url: String,
    /// Test data directory
    pub test_data_dir: String,
    /// Log output directory
    pub log_dir: String,
    /// Report output directory
    pub report_dir: String,
    /// Environment-specific settings
    pub settings: std::collections::HashMap<String, String>,
}

/// UAT Metrics Collection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UatMetrics {
    /// Total number of tests executed
    pub total_tests: usize,
    /// Number of passed tests
    pub passed_tests: usize,
    /// Number of failed tests
    pub failed_tests: usize,
    /// Number of tests with warnings
    pub warning_tests: usize,
    /// Number of skipped tests
    pub skipped_tests: usize,
    /// Number of error tests
    pub error_tests: usize,
    /// Total test execution time
    pub total_duration: Duration,
    /// Average test execution time
    pub average_duration: Duration,
    /// User satisfaction scores (0-10)
    pub satisfaction_scores: Vec<f64>,
    /// Task completion rates (0-1)
    pub completion_rates: Vec<f64>,
    /// Error rates by category
    pub error_rates: std::collections::HashMap<TestCategory, f64>,
}

impl UatMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.passed_tests as f64 / self.total_tests as f64
        }
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.failed_tests as f64 / self.total_tests as f64
        }
    }

    /// Calculate average user satisfaction
    pub fn average_satisfaction(&self) -> f64 {
        if self.satisfaction_scores.is_empty() {
            0.0
        } else {
            self.satisfaction_scores.iter().sum::<f64>() / self.satisfaction_scores.len() as f64
        }
    }

    /// Calculate average task completion rate
    pub fn average_completion_rate(&self) -> f64 {
        if self.completion_rates.is_empty() {
            0.0
        } else {
            self.completion_rates.iter().sum::<f64>() / self.completion_rates.len() as f64
        }
    }

    /// Add test result to metrics
    pub fn add_test_result(&mut self, result: &UatTestResult) {
        self.total_tests += 1;
        self.total_duration += result.duration;
        self.average_duration = self.total_duration / self.total_tests as u32;

        match result.result {
            UatResult::Pass => self.passed_tests += 1,
            UatResult::Fail => self.failed_tests += 1,
            UatResult::Warning => self.warning_tests += 1,
            UatResult::Skipped => self.skipped_tests += 1,
            UatResult::Error => self.error_tests += 1,
        }
    }
}

/// Common UAT testing utilities
pub mod utils {
    use super::*;
    use std::collections::HashMap;

    /// Generate test user credentials
    pub fn generate_test_user(id: usize, persona: UserPersona) -> HashMap<String, String> {
        let mut user = HashMap::new();
        user.insert("id".to_string(), format!("test_user_{}", id));
        user.insert("username".to_string(), format!("uat_user_{}", id));
        user.insert("email".to_string(), format!("uat_user_{}@example.com", id));
        user.insert("password".to_string(), "UatTestPassword123!".to_string());
        user.insert("persona".to_string(), format!("{:?}", persona));
        user
    }

    /// Create test room configuration
    pub fn create_test_room(id: usize, room_type: &str) -> HashMap<String, String> {
        let mut room = HashMap::new();
        room.insert("id".to_string(), format!("test_room_{}", id));
        room.insert("name".to_string(), format!("UAT Test Room {}", id));
        room.insert(
            "description".to_string(),
            format!("User acceptance testing room for {}", room_type),
        );
        room.insert("type".to_string(), room_type.to_string());
        room
    }

    /// Generate test message content
    pub fn generate_test_message(sender: &str, content_type: &str) -> String {
        match content_type {
            "simple" => format!("Test message from {}", sender),
            "emoji" => format!("Test with emojis 🚀 from {} 🎉", sender),
            "long" => format!("This is a longer test message from {} that contains multiple sentences and should test how the application handles longer content. It includes various punctuation marks, numbers like 123, and should wrap properly in the user interface.", sender),
            "special" => format!("Special chars test from {}: @#$%^&*()[]{{}}|\\:;\"'<>,.?/~`", sender),
            _ => format!("Default test message from {}", sender),
        }
    }

    /// Validate test environment
    pub fn validate_environment(env: &UatEnvironment) -> Result<(), String> {
        // Check if server is reachable
        if env.server_endpoint.is_empty() {
            return Err("Server endpoint not configured".to_string());
        }

        // Check if directories exist
        if !std::path::Path::new(&env.test_data_dir).exists() {
            return Err(format!(
                "Test data directory does not exist: {}",
                env.test_data_dir
            ));
        }

        if !std::path::Path::new(&env.log_dir).exists() {
            return Err(format!("Log directory does not exist: {}", env.log_dir));
        }

        if !std::path::Path::new(&env.report_dir).exists() {
            return Err(format!(
                "Report directory does not exist: {}",
                env.report_dir
            ));
        }

        Ok(())
    }
}

// Re-export commonly used types for convenience
pub use framework::UatFramework;
pub use metrics::UatMetricsCollector;
pub use reporting::UatReporter;
pub use scenarios::UatScenarios;
pub use test_runner::UatTestRunner;
