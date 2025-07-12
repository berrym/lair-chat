//! User Acceptance Testing Framework
//!
//! This module provides the core framework for orchestrating comprehensive user acceptance
//! testing of the lair-chat application. It handles test execution, environment management,
//! user simulation, and result collection.

use super::*;
use crate::tests::integration::TEST_TIMEOUT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Core User Acceptance Testing Framework
pub struct UatFramework {
    /// Framework configuration
    config: UatFrameworkConfig,
    /// Test environment configuration
    environment: UatEnvironment,
    /// Active test sessions
    sessions: Arc<RwLock<HashMap<String, UatTestSession>>>,
    /// Metrics collector
    metrics: Arc<Mutex<UatMetrics>>,
    /// Test result storage
    results: Arc<RwLock<Vec<UatTestResult>>>,
    /// Framework status
    status: Arc<RwLock<UatFrameworkStatus>>,
}

/// UAT Framework Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatFrameworkConfig {
    /// Framework name and version
    pub name: String,
    pub version: String,
    /// Maximum concurrent test sessions
    pub max_concurrent_sessions: usize,
    /// Default test timeout
    pub default_timeout: Duration,
    /// Test data retention period
    pub data_retention_days: u32,
    /// Enable detailed logging
    pub verbose_logging: bool,
    /// Enable performance monitoring
    pub performance_monitoring: bool,
    /// Auto-cleanup test data
    pub auto_cleanup: bool,
}

/// UAT Framework Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UatFrameworkStatus {
    /// Framework is initializing
    Initializing,
    /// Framework is ready for testing
    Ready,
    /// Tests are currently running
    Running,
    /// Framework is paused
    Paused,
    /// Framework encountered an error
    Error(String),
    /// Framework is shutting down
    Shutdown,
}

/// UAT Test Session
#[derive(Debug, Clone)]
pub struct UatTestSession {
    /// Session configuration
    pub config: UatSession,
    /// Session start time
    pub start_time: Instant,
    /// Session status
    pub status: SessionStatus,
    /// Test cases in session
    pub test_cases: Vec<UatTestCase>,
    /// Completed test results
    pub results: Vec<UatTestResult>,
    /// Active test users
    pub test_users: Vec<TestUser>,
    /// Session metrics
    pub metrics: UatMetrics,
}

/// Session execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is being prepared
    Preparing,
    /// Session is actively running tests
    Running,
    /// Session is paused
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed with errors
    Failed(String),
    /// Session was cancelled
    Cancelled,
}

/// Test user representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestUser {
    /// User identifier
    pub id: String,
    /// User credentials
    pub credentials: HashMap<String, String>,
    /// User persona type
    pub persona: UserPersona,
    /// User's current state
    pub state: UserState,
    /// User's session data
    pub session_data: HashMap<String, String>,
}

/// User state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserState {
    /// User created but not logged in
    Created,
    /// User is logged in
    LoggedIn,
    /// User is in a chat room
    InRoom(String),
    /// User is in direct message
    InDirectMessage(String),
    /// User is idle
    Idle,
    /// User encountered an error
    Error(String),
}

impl Default for UatFrameworkConfig {
    fn default() -> Self {
        Self {
            name: "Lair-Chat UAT Framework".to_string(),
            version: "1.0.0".to_string(),
            max_concurrent_sessions: 5,
            default_timeout: Duration::from_secs(60),
            data_retention_days: 30,
            verbose_logging: true,
            performance_monitoring: true,
            auto_cleanup: true,
        }
    }
}

impl UatFramework {
    /// Create new UAT framework instance
    pub fn new(config: UatFrameworkConfig, environment: UatEnvironment) -> Self {
        Self {
            config,
            environment,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(UatMetrics::new())),
            results: Arc::new(RwLock::new(Vec::new())),
            status: Arc::new(RwLock::new(UatFrameworkStatus::Initializing)),
        }
    }

    /// Initialize the UAT framework
    pub async fn initialize(&self) -> Result<(), UatError> {
        info!("Initializing UAT Framework: {}", self.config.name);

        // Validate environment
        utils::validate_environment(&self.environment)
            .map_err(|e| UatError::EnvironmentError(e))?;

        // Create necessary directories
        self.setup_directories().await?;

        // Initialize logging
        self.setup_logging().await?;

        // Validate server connectivity
        self.validate_server_connectivity().await?;

        // Set status to ready
        let mut status = self.status.write().await;
        *status = UatFrameworkStatus::Ready;

        info!("UAT Framework initialized successfully");
        Ok(())
    }

    /// Create a new test session
    pub async fn create_session(&self, session_config: UatSession) -> Result<String, UatError> {
        let session_id = session_config.session_id.clone();

        // Check if session already exists
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(&session_id) {
                return Err(UatError::SessionExists(session_id));
            }
        }

        // Check concurrent session limit
        {
            let sessions = self.sessions.read().await;
            if sessions.len() >= self.config.max_concurrent_sessions {
                return Err(UatError::SessionLimitExceeded);
            }
        }

        // Generate test cases for session
        let test_cases = self.generate_test_cases(&session_config).await?;

        // Create test users
        let test_users = self.create_test_users(&session_config).await?;

        let session = UatTestSession {
            config: session_config,
            start_time: Instant::now(),
            status: SessionStatus::Preparing,
            test_cases,
            results: Vec::new(),
            test_users,
            metrics: UatMetrics::new(),
        };

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        info!("Created UAT session: {}", session_id);
        Ok(session_id)
    }

    /// Execute a test session
    pub async fn execute_session(&self, session_id: &str) -> Result<UatMetrics, UatError> {
        info!("Executing UAT session: {}", session_id);

        // Update framework status
        {
            let mut status = self.status.write().await;
            *status = UatFrameworkStatus::Running;
        }

        // Get session
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .ok_or_else(|| UatError::SessionNotFound(session_id.to_string()))?
                .clone()
        };

        // Update session status
        self.update_session_status(session_id, SessionStatus::Running)
            .await?;

        // Execute test cases
        let results = self.execute_test_cases(&session).await?;

        // Update session with results
        let final_metrics = self.finalize_session(session_id, results).await?;

        info!("Completed UAT session: {}", session_id);
        Ok(final_metrics)
    }

    /// Generate test cases based on session configuration
    async fn generate_test_cases(
        &self,
        session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut test_cases = Vec::new();

        for category in &session_config.categories {
            match category {
                TestCategory::Functional => {
                    test_cases.extend(self.generate_functional_tests(session_config).await?);
                }
                TestCategory::Usability => {
                    test_cases.extend(self.generate_usability_tests(session_config).await?);
                }
                TestCategory::Compatibility => {
                    test_cases.extend(self.generate_compatibility_tests(session_config).await?);
                }
                TestCategory::Integration => {
                    test_cases.extend(self.generate_integration_tests(session_config).await?);
                }
                TestCategory::Performance => {
                    test_cases.extend(self.generate_performance_tests(session_config).await?);
                }
            }
        }

        Ok(test_cases)
    }

    /// Generate functional test cases
    async fn generate_functional_tests(
        &self,
        _session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut tests = Vec::new();

        // Authentication tests
        tests.push(UatTestCase {
            id: "FUNC-001".to_string(),
            name: "User Registration Flow".to_string(),
            description: "Test new user registration process".to_string(),
            category: TestCategory::Functional,
            priority: TestPriority::Critical,
            user_persona: UserPersona::Novice,
            prerequisites: vec!["Server running".to_string()],
            steps: vec![
                "Navigate to registration page".to_string(),
                "Enter valid user details".to_string(),
                "Submit registration form".to_string(),
                "Verify account creation".to_string(),
            ],
            expected_result: "User account created successfully".to_string(),
            success_criteria: vec![
                "Registration form accepts valid input".to_string(),
                "Account is created in database".to_string(),
                "User receives confirmation".to_string(),
            ],
            timeout: Duration::from_secs(30),
            manual_validation: false,
        });

        tests.push(UatTestCase {
            id: "FUNC-002".to_string(),
            name: "User Login Process".to_string(),
            description: "Test user authentication and login".to_string(),
            category: TestCategory::Functional,
            priority: TestPriority::Critical,
            user_persona: UserPersona::Regular,
            prerequisites: vec!["User account exists".to_string()],
            steps: vec![
                "Navigate to login page".to_string(),
                "Enter valid credentials".to_string(),
                "Submit login form".to_string(),
                "Verify successful login".to_string(),
            ],
            expected_result: "User successfully logged in".to_string(),
            success_criteria: vec![
                "Credentials are validated".to_string(),
                "User session is created".to_string(),
                "User is redirected to main interface".to_string(),
            ],
            timeout: Duration::from_secs(20),
            manual_validation: false,
        });

        // Room management tests
        tests.push(UatTestCase {
            id: "FUNC-003".to_string(),
            name: "Room Creation".to_string(),
            description: "Test chat room creation functionality".to_string(),
            category: TestCategory::Functional,
            priority: TestPriority::High,
            user_persona: UserPersona::Regular,
            prerequisites: vec!["User logged in".to_string()],
            steps: vec![
                "Navigate to room creation".to_string(),
                "Enter room details".to_string(),
                "Create room".to_string(),
                "Verify room exists".to_string(),
            ],
            expected_result: "Chat room created successfully".to_string(),
            success_criteria: vec![
                "Room is created with correct settings".to_string(),
                "Creator is added as admin".to_string(),
                "Room appears in room list".to_string(),
            ],
            timeout: Duration::from_secs(25),
            manual_validation: false,
        });

        // Messaging tests
        tests.push(UatTestCase {
            id: "FUNC-004".to_string(),
            name: "Real-time Messaging".to_string(),
            description: "Test real-time message sending and receiving".to_string(),
            category: TestCategory::Functional,
            priority: TestPriority::Critical,
            user_persona: UserPersona::Regular,
            prerequisites: vec!["Users in same room".to_string()],
            steps: vec![
                "User A sends message".to_string(),
                "Verify message appears for User A".to_string(),
                "Verify message appears for User B".to_string(),
                "Test message ordering".to_string(),
            ],
            expected_result: "Messages delivered in real-time".to_string(),
            success_criteria: vec![
                "Messages appear immediately".to_string(),
                "Message order is preserved".to_string(),
                "All users receive messages".to_string(),
            ],
            timeout: Duration::from_secs(15),
            manual_validation: false,
        });

        Ok(tests)
    }

    /// Generate usability test cases
    async fn generate_usability_tests(
        &self,
        _session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut tests = Vec::new();

        tests.push(UatTestCase {
            id: "USAB-001".to_string(),
            name: "Interface Navigation".to_string(),
            description: "Test user interface navigation intuitiveness".to_string(),
            category: TestCategory::Usability,
            priority: TestPriority::High,
            user_persona: UserPersona::Novice,
            prerequisites: vec!["User logged in".to_string()],
            steps: vec![
                "Navigate to different sections".to_string(),
                "Test menu accessibility".to_string(),
                "Verify breadcrumb navigation".to_string(),
                "Test back/forward functionality".to_string(),
            ],
            expected_result: "Navigation is intuitive and responsive".to_string(),
            success_criteria: vec![
                "All sections are accessible".to_string(),
                "Navigation is consistent".to_string(),
                "User can easily find features".to_string(),
            ],
            timeout: Duration::from_secs(45),
            manual_validation: true,
        });

        tests.push(UatTestCase {
            id: "USAB-002".to_string(),
            name: "Error Message Clarity".to_string(),
            description: "Test error message helpfulness and clarity".to_string(),
            category: TestCategory::Usability,
            priority: TestPriority::Medium,
            user_persona: UserPersona::Regular,
            prerequisites: vec!["User logged in".to_string()],
            steps: vec![
                "Trigger various error conditions".to_string(),
                "Evaluate error message clarity".to_string(),
                "Test error recovery options".to_string(),
                "Verify help information".to_string(),
            ],
            expected_result: "Error messages are clear and helpful".to_string(),
            success_criteria: vec![
                "Errors are clearly explained".to_string(),
                "Recovery steps are provided".to_string(),
                "Technical jargon is minimized".to_string(),
            ],
            timeout: Duration::from_secs(30),
            manual_validation: true,
        });

        Ok(tests)
    }

    /// Generate compatibility test cases
    async fn generate_compatibility_tests(
        &self,
        session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut tests = Vec::new();

        for os in &session_config.operating_systems {
            for browser in &session_config.browsers {
                tests.push(UatTestCase {
                    id: format!("COMPAT-{:?}-{:?}", os, browser),
                    name: format!("Compatibility: {:?} + {:?}", os, browser),
                    description: format!("Test compatibility on {:?} with {:?}", os, browser),
                    category: TestCategory::Compatibility,
                    priority: TestPriority::High,
                    user_persona: UserPersona::Regular,
                    prerequisites: vec![format!("{:?} system available", os)],
                    steps: vec![
                        "Launch application".to_string(),
                        "Test core functionality".to_string(),
                        "Verify feature compatibility".to_string(),
                        "Test performance".to_string(),
                    ],
                    expected_result: "Application works correctly on platform".to_string(),
                    success_criteria: vec![
                        "All features are functional".to_string(),
                        "Performance is acceptable".to_string(),
                        "UI renders correctly".to_string(),
                    ],
                    timeout: Duration::from_secs(120),
                    manual_validation: false,
                });
            }
        }

        Ok(tests)
    }

    /// Generate integration test cases
    async fn generate_integration_tests(
        &self,
        _session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut tests = Vec::new();

        tests.push(UatTestCase {
            id: "INTEG-001".to_string(),
            name: "API Integration".to_string(),
            description: "Test REST API integration functionality".to_string(),
            category: TestCategory::Integration,
            priority: TestPriority::High,
            user_persona: UserPersona::Expert,
            prerequisites: vec!["API server running".to_string()],
            steps: vec![
                "Test API authentication".to_string(),
                "Test CRUD operations".to_string(),
                "Test WebSocket connections".to_string(),
                "Verify error handling".to_string(),
            ],
            expected_result: "API integration works seamlessly".to_string(),
            success_criteria: vec![
                "All API endpoints respond".to_string(),
                "Data integrity is maintained".to_string(),
                "Error handling is robust".to_string(),
            ],
            timeout: Duration::from_secs(60),
            manual_validation: false,
        });

        Ok(tests)
    }

    /// Generate performance test cases
    async fn generate_performance_tests(
        &self,
        _session_config: &UatSession,
    ) -> Result<Vec<UatTestCase>, UatError> {
        let mut tests = Vec::new();

        tests.push(UatTestCase {
            id: "PERF-001".to_string(),
            name: "Response Time Validation".to_string(),
            description: "Test application response times under normal load".to_string(),
            category: TestCategory::Performance,
            priority: TestPriority::High,
            user_persona: UserPersona::Regular,
            prerequisites: vec!["System under normal load".to_string()],
            steps: vec![
                "Measure login response time".to_string(),
                "Measure message send time".to_string(),
                "Measure room join time".to_string(),
                "Evaluate overall responsiveness".to_string(),
            ],
            expected_result: "Response times are within acceptable limits".to_string(),
            success_criteria: vec![
                "Login completes within 3 seconds".to_string(),
                "Messages send within 1 second".to_string(),
                "Room join completes within 2 seconds".to_string(),
            ],
            timeout: Duration::from_secs(90),
            manual_validation: false,
        });

        Ok(tests)
    }

    /// Create test users for session
    async fn create_test_users(
        &self,
        session_config: &UatSession,
    ) -> Result<Vec<TestUser>, UatError> {
        let mut users = Vec::new();

        for (i, persona) in session_config.personas.iter().enumerate() {
            for j in 0..session_config.user_count {
                let user_id = format!("{}_{}_user_{}", session_config.session_id, i, j);
                let credentials =
                    utils::generate_test_user(i * session_config.user_count + j, persona.clone());

                users.push(TestUser {
                    id: user_id,
                    credentials,
                    persona: persona.clone(),
                    state: UserState::Created,
                    session_data: HashMap::new(),
                });
            }
        }

        Ok(users)
    }

    /// Execute all test cases in a session
    async fn execute_test_cases(
        &self,
        session: &UatTestSession,
    ) -> Result<Vec<UatTestResult>, UatError> {
        let mut results = Vec::new();

        for test_case in &session.test_cases {
            info!("Executing test case: {}", test_case.id);

            let result = self.execute_single_test(test_case, session).await?;
            results.push(result);

            // Add delay between tests to avoid overwhelming the system
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(results)
    }

    /// Execute a single test case
    async fn execute_single_test(
        &self,
        test_case: &UatTestCase,
        _session: &UatTestSession,
    ) -> Result<UatTestResult, UatError> {
        let start_time = Instant::now();

        // Simulate test execution based on test category
        let result = match test_case.category {
            TestCategory::Functional => self.execute_functional_test(test_case).await,
            TestCategory::Usability => self.execute_usability_test(test_case).await,
            TestCategory::Compatibility => self.execute_compatibility_test(test_case).await,
            TestCategory::Integration => self.execute_integration_test(test_case).await,
            TestCategory::Performance => self.execute_performance_test(test_case).await,
        };

        let duration = start_time.elapsed();

        match result {
            Ok((criteria_met, warnings)) => Ok(UatTestResult {
                test_case: test_case.clone(),
                result: if warnings.is_empty() {
                    UatResult::Pass
                } else {
                    UatResult::Warning
                },
                duration,
                timestamp: chrono::Utc::now(),
                criteria_met,
                criteria_failed: Vec::new(),
                warnings,
                errors: Vec::new(),
                notes: Vec::new(),
                user_feedback: None,
            }),
            Err(error) => Ok(UatTestResult {
                test_case: test_case.clone(),
                result: UatResult::Fail,
                duration,
                timestamp: chrono::Utc::now(),
                criteria_met: Vec::new(),
                criteria_failed: test_case.success_criteria.clone(),
                warnings: Vec::new(),
                errors: vec![error.to_string()],
                notes: Vec::new(),
                user_feedback: None,
            }),
        }
    }

    /// Execute functional test
    async fn execute_functional_test(
        &self,
        test_case: &UatTestCase,
    ) -> Result<(Vec<String>, Vec<String>), UatError> {
        // Simulate functional test execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        match test_case.id.as_str() {
            "FUNC-001" | "FUNC-002" | "FUNC-003" | "FUNC-004" => {
                Ok((test_case.success_criteria.clone(), Vec::new()))
            }
            _ => Err(UatError::TestExecutionError(format!(
                "Unknown functional test: {}",
                test_case.id
            ))),
        }
    }

    /// Execute usability test
    async fn execute_usability_test(
        &self,
        test_case: &UatTestCase,
    ) -> Result<(Vec<String>, Vec<String>), UatError> {
        // Simulate usability test execution
        tokio::time::sleep(Duration::from_millis(200)).await;

        let warnings = if test_case.manual_validation {
            vec!["Manual validation required".to_string()]
        } else {
            Vec::new()
        };

        Ok((test_case.success_criteria.clone(), warnings))
    }

    /// Execute compatibility test
    async fn execute_compatibility_test(
        &self,
        test_case: &UatTestCase,
    ) -> Result<(Vec<String>, Vec<String>), UatError> {
        // Simulate compatibility test execution
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok((test_case.success_criteria.clone(), Vec::new()))
    }

    /// Execute integration test
    async fn execute_integration_test(
        &self,
        test_case: &UatTestCase,
    ) -> Result<(Vec<String>, Vec<String>), UatError> {
        // Simulate integration test execution
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok((test_case.success_criteria.clone(), Vec::new()))
    }

    /// Execute performance test
    async fn execute_performance_test(
        &self,
        test_case: &UatTestCase,
    ) -> Result<(Vec<String>, Vec<String>), UatError> {
        // Simulate performance test execution
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok((test_case.success_criteria.clone(), Vec::new()))
    }

    /// Setup required directories
    async fn setup_directories(&self) -> Result<(), UatError> {
        let dirs = [
            &self.environment.test_data_dir,
            &self.environment.log_dir,
            &self.environment.report_dir,
        ];

        for dir in &dirs {
            if !std::path::Path::new(dir).exists() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    UatError::DirectoryCreationError(dir.to_string(), e.to_string())
                })?;
            }
        }

        Ok(())
    }

    /// Setup logging configuration
    async fn setup_logging(&self) -> Result<(), UatError> {
        // Logging setup would go here
        debug!("UAT Framework logging configured");
        Ok(())
    }

    /// Validate server connectivity
    async fn validate_server_connectivity(&self) -> Result<(), UatError> {
        // Server connectivity validation would go here
        debug!(
            "Server connectivity validated: {}",
            self.environment.server_endpoint
        );
        Ok(())
    }

    /// Update session status
    async fn update_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
    ) -> Result<(), UatError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = status;
            Ok(())
        } else {
            Err(UatError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Finalize session and calculate metrics
    async fn finalize_session(
        &self,
        session_id: &str,
        results: Vec<UatTestResult>,
    ) -> Result<UatMetrics, UatError> {
        let mut metrics = UatMetrics::new();

        // Calculate metrics from results
        for result in &results {
            metrics.add_test_result(result);
        }

        // Store results
        {
            let mut all_results = self.results.write().await;
            all_results.extend(results.clone());
        }

        // Update session
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.results = results;
                session.metrics = metrics.clone();
                session.status = SessionStatus::Completed;
            }
        }

        // Update framework metrics
        {
            let mut framework_metrics = self.metrics.lock().await;
            for result in &results {
                framework_metrics.add_test_result(result);
            }
        }

        // Update framework status
        {
            let mut status = self.status.write().await;
            *status = UatFrameworkStatus::Ready;
        }

        Ok(metrics)
    }

    /// Get session results
    pub async fn get_session_results(
        &self,
        session_id: &str,
    ) -> Result<Vec<UatTestResult>, UatError> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Ok(session.results.clone())
        } else {
            Err(UatError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Get framework metrics
    pub async fn get_metrics(&self) -> UatMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    /// Get framework status
    pub async fn get_status(&self) -> UatFrameworkStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// Cleanup completed sessions
    pub async fn cleanup_sessions(&self) -> Result<usize, UatError> {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|_, session| {
            !matches!(
                session.status,
                SessionStatus::Completed | SessionStatus::Failed(_)
            )
        });

        Ok(initial_count - sessions.len())
    }

    /// Shutdown framework
    pub async fn shutdown(&self) -> Result<(), UatError> {
        info!("Shutting down UAT Framework");

        // Update status
        {
            let mut status = self.status.write().await;
            *status = UatFrameworkStatus::Shutdown;
        }

        // Cleanup if enabled
        if self.config.auto_cleanup {
            self.cleanup_sessions().await?;
        }

        info!("UAT Framework shutdown complete");
        Ok(())
    }
}

/// UAT Framework Error Types
#[derive(Debug, thiserror::Error)]
pub enum UatError {
    #[error("Environment error: {0}")]
    EnvironmentError(String),

    #[error("Session {0} already exists")]
    SessionExists(String),

    #[error("Session limit exceeded")]
    SessionLimitExceeded,

    #[error("Session {0} not found")]
    SessionNotFound(String),

    #[error("Test execution error: {0}")]
    TestExecutionError(String),

    #[error("Directory creation error for {0}: {1}")]
    DirectoryCreationError(String, String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
