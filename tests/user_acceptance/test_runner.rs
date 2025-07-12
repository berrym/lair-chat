//! User Acceptance Testing Test Runner
//!
//! This module provides the main orchestration and execution engine for user acceptance
//! testing, coordinating test execution across multiple phases and categories.

use super::*;
use crate::tests::integration::TEST_TIMEOUT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// UAT Test Runner - Main orchestration engine
pub struct UatTestRunner {
    /// Framework instance
    framework: Arc<UatFramework>,
    /// Scenarios manager
    scenarios: Arc<Mutex<UatScenarios>>,
    /// Active test sessions
    active_sessions: Arc<RwLock<HashMap<String, UatTestSession>>>,
    /// Test execution queue
    execution_queue: Arc<Mutex<Vec<TestExecutionRequest>>>,
    /// Results collector
    results_collector: Arc<Mutex<Vec<UatTestResult>>>,
    /// Metrics aggregator
    metrics_aggregator: Arc<Mutex<UatMetrics>>,
    /// Runner configuration
    config: UatRunnerConfig,
    /// Runner status
    status: Arc<RwLock<RunnerStatus>>,
}

/// UAT Test Runner Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatRunnerConfig {
    /// Maximum concurrent test sessions
    pub max_concurrent_sessions: usize,
    /// Test execution timeout
    pub execution_timeout: Duration,
    /// Result collection batch size
    pub batch_size: usize,
    /// Auto-retry failed tests
    pub auto_retry: bool,
    /// Maximum retry attempts
    pub max_retries: usize,
    /// Detailed logging enabled
    pub verbose_logging: bool,
    /// Performance monitoring enabled
    pub performance_monitoring: bool,
    /// Real-time reporting enabled
    pub real_time_reporting: bool,
}

/// Test execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionRequest {
    /// Request identifier
    pub request_id: String,
    /// Session configuration
    pub session_config: UatSession,
    /// Priority level
    pub priority: ExecutionPriority,
    /// Requested execution time
    pub scheduled_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Dependencies on other tests
    pub dependencies: Vec<String>,
    /// Custom parameters
    pub parameters: HashMap<String, String>,
}

/// Execution priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExecutionPriority {
    /// Low priority execution
    Low = 1,
    /// Normal priority execution
    Normal = 2,
    /// High priority execution
    High = 3,
    /// Critical priority execution
    Critical = 4,
}

/// Runner execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerStatus {
    /// Runner is initializing
    Initializing,
    /// Runner is idle and ready
    Idle,
    /// Runner is executing tests
    Executing,
    /// Runner is paused
    Paused,
    /// Runner encountered an error
    Error(String),
    /// Runner is shutting down
    Shutdown,
}

/// Test execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Plan name
    pub name: String,
    /// Plan description
    pub description: String,
    /// Execution phases
    pub phases: Vec<ExecutionPhase>,
    /// Total estimated duration
    pub estimated_duration: Duration,
    /// Success criteria for the plan
    pub success_criteria: Vec<String>,
}

/// Individual execution phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    /// Phase identifier
    pub phase_id: String,
    /// Phase name
    pub name: String,
    /// Phase description
    pub description: String,
    /// Test categories in this phase
    pub categories: Vec<TestCategory>,
    /// User personas for this phase
    pub personas: Vec<UserPersona>,
    /// Phase dependencies
    pub dependencies: Vec<String>,
    /// Phase duration estimate
    pub estimated_duration: Duration,
    /// Phase success criteria
    pub success_criteria: Vec<String>,
}

/// Execution summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// Summary identifier
    pub summary_id: String,
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Execution end time
    pub end_time: chrono::DateTime<chrono::Utc>,
    /// Total duration
    pub total_duration: Duration,
    /// Number of sessions executed
    pub sessions_executed: usize,
    /// Total test cases executed
    pub total_test_cases: usize,
    /// Overall success rate
    pub success_rate: f64,
    /// Performance metrics
    pub performance_metrics: UatMetrics,
    /// Phase summaries
    pub phase_summaries: Vec<PhaseSummary>,
    /// Overall assessment
    pub overall_assessment: OverallAssessment,
}

/// Phase execution summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSummary {
    /// Phase that was executed
    pub phase: ExecutionPhase,
    /// Phase execution result
    pub result: UatResult,
    /// Phase duration
    pub duration: Duration,
    /// Test results in this phase
    pub test_results: Vec<UatTestResult>,
    /// Phase-specific metrics
    pub metrics: UatMetrics,
    /// Issues encountered
    pub issues: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Overall execution assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallAssessment {
    /// Production readiness score (0-100)
    pub readiness_score: f64,
    /// User satisfaction score (0-10)
    pub satisfaction_score: f64,
    /// System quality score (0-100)
    pub quality_score: f64,
    /// Critical issues found
    pub critical_issues: Vec<String>,
    /// Blocking issues
    pub blocking_issues: Vec<String>,
    /// Recommendations for production
    pub production_recommendations: Vec<String>,
    /// Overall recommendation
    pub overall_recommendation: ProductionRecommendation,
}

/// Production deployment recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProductionRecommendation {
    /// Ready for production deployment
    ReadyForProduction,
    /// Ready with minor fixes
    ReadyWithMinorFixes(Vec<String>),
    /// Requires significant improvements
    RequiresImprovements(Vec<String>),
    /// Not ready for production
    NotReady(Vec<String>),
}

impl Default for UatRunnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 3,
            execution_timeout: Duration::from_secs(3600), // 1 hour
            batch_size: 50,
            auto_retry: true,
            max_retries: 2,
            verbose_logging: true,
            performance_monitoring: true,
            real_time_reporting: true,
        }
    }
}

impl UatTestRunner {
    /// Create new UAT test runner
    pub fn new(framework: UatFramework, scenarios: UatScenarios, config: UatRunnerConfig) -> Self {
        Self {
            framework: Arc::new(framework),
            scenarios: Arc::new(Mutex::new(scenarios)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            execution_queue: Arc::new(Mutex::new(Vec::new())),
            results_collector: Arc::new(Mutex::new(Vec::new())),
            metrics_aggregator: Arc::new(Mutex::new(UatMetrics::new())),
            config,
            status: Arc::new(RwLock::new(RunnerStatus::Initializing)),
        }
    }

    /// Initialize the test runner
    pub async fn initialize(&self) -> Result<(), UatError> {
        info!("Initializing UAT Test Runner");

        // Initialize framework
        self.framework.initialize().await?;

        // Set status to idle
        {
            let mut status = self.status.write().await;
            *status = RunnerStatus::Idle;
        }

        info!("UAT Test Runner initialized successfully");
        Ok(())
    }

    /// Create comprehensive execution plan for Task 8.4
    pub async fn create_task_8_4_execution_plan(&self) -> Result<ExecutionPlan, UatError> {
        let plan = ExecutionPlan {
            plan_id: "TASK_8_4_UAT_PLAN".to_string(),
            name: "Phase 8 Task 8.4 User Acceptance Testing".to_string(),
            description:
                "Comprehensive 5-day user acceptance testing plan as specified in Task 8.4"
                    .to_string(),
            phases: vec![
                // Phase 1: Functional User Acceptance Testing (Days 1-2)
                ExecutionPhase {
                    phase_id: "PHASE_1_FUNCTIONAL".to_string(),
                    name: "Functional User Acceptance Testing".to_string(),
                    description: "Core functionality validation from user perspective".to_string(),
                    categories: vec![TestCategory::Functional],
                    personas: vec![
                        UserPersona::Novice,
                        UserPersona::Regular,
                        UserPersona::Administrator,
                    ],
                    dependencies: Vec::new(),
                    estimated_duration: Duration::from_secs(2 * 24 * 3600), // 2 days
                    success_criteria: vec![
                        "100% of core features functional".to_string(),
                        "Authentication system works reliably".to_string(),
                        "Real-time messaging performs correctly".to_string(),
                        "Room management functions properly".to_string(),
                        "Admin functions are accessible and functional".to_string(),
                    ],
                },
                // Phase 2: Usability and User Experience Testing (Days 3-4)
                ExecutionPhase {
                    phase_id: "PHASE_2_USABILITY".to_string(),
                    name: "Usability and User Experience Testing".to_string(),
                    description: "User interface and experience validation".to_string(),
                    categories: vec![TestCategory::Usability],
                    personas: vec![
                        UserPersona::Novice,
                        UserPersona::Regular,
                        UserPersona::Expert,
                    ],
                    dependencies: vec!["PHASE_1_FUNCTIONAL".to_string()],
                    estimated_duration: Duration::from_secs(2 * 24 * 3600), // 2 days
                    success_criteria: vec![
                        "User satisfaction rating above 8/10".to_string(),
                        "Task completion rate above 90%".to_string(),
                        "Interface navigation is intuitive".to_string(),
                        "Error messages are clear and helpful".to_string(),
                        "Onboarding experience is effective".to_string(),
                    ],
                },
                // Phase 3: Compatibility and Final Validation (Day 5)
                ExecutionPhase {
                    phase_id: "PHASE_3_COMPATIBILITY".to_string(),
                    name: "Compatibility and Final Validation".to_string(),
                    description: "Cross-platform compatibility and final system validation"
                        .to_string(),
                    categories: vec![
                        TestCategory::Compatibility,
                        TestCategory::Integration,
                        TestCategory::Performance,
                    ],
                    personas: vec![UserPersona::Regular, UserPersona::Expert],
                    dependencies: vec![
                        "PHASE_1_FUNCTIONAL".to_string(),
                        "PHASE_2_USABILITY".to_string(),
                    ],
                    estimated_duration: Duration::from_secs(24 * 3600), // 1 day
                    success_criteria: vec![
                        "Consistent functionality across all platforms".to_string(),
                        "Performance within acceptable limits".to_string(),
                        "Integration points work reliably".to_string(),
                        "System ready for production deployment".to_string(),
                    ],
                },
            ],
            estimated_duration: Duration::from_secs(5 * 24 * 3600), // 5 days
            success_criteria: vec![
                "All phases complete successfully".to_string(),
                "Overall user satisfaction above 8/10".to_string(),
                "System passes production readiness criteria".to_string(),
                "No critical or blocking issues identified".to_string(),
            ],
        };

        Ok(plan)
    }

    /// Execute the complete Task 8.4 testing plan
    pub async fn execute_task_8_4_plan(&self) -> Result<ExecutionSummary, UatError> {
        info!("Starting Task 8.4 User Acceptance Testing execution");

        let plan = self.create_task_8_4_execution_plan().await?;
        let start_time = chrono::Utc::now();

        // Update runner status
        {
            let mut status = self.status.write().await;
            *status = RunnerStatus::Executing;
        }

        let mut phase_summaries = Vec::new();
        let mut overall_metrics = UatMetrics::new();

        // Execute each phase
        for phase in &plan.phases {
            info!("Executing phase: {}", phase.name);

            let phase_result = self.execute_phase(phase).await?;

            // Aggregate metrics
            overall_metrics.total_tests += phase_result.metrics.total_tests;
            overall_metrics.passed_tests += phase_result.metrics.passed_tests;
            overall_metrics.failed_tests += phase_result.metrics.failed_tests;
            overall_metrics.warning_tests += phase_result.metrics.warning_tests;
            overall_metrics.skipped_tests += phase_result.metrics.skipped_tests;
            overall_metrics.error_tests += phase_result.metrics.error_tests;
            overall_metrics.total_duration += phase_result.metrics.total_duration;

            phase_summaries.push(phase_result);

            info!("Completed phase: {}", phase.name);
        }

        let end_time = chrono::Utc::now();
        let total_duration = (end_time - start_time).to_std().unwrap_or_default();

        // Calculate overall metrics
        if overall_metrics.total_tests > 0 {
            overall_metrics.average_duration =
                overall_metrics.total_duration / overall_metrics.total_tests as u32;
        }

        // Generate overall assessment
        let overall_assessment = self
            .generate_overall_assessment(&phase_summaries, &overall_metrics)
            .await;

        let summary = ExecutionSummary {
            summary_id: format!("TASK_8_4_SUMMARY_{}", start_time.timestamp()),
            start_time,
            end_time,
            total_duration,
            sessions_executed: phase_summaries.len(),
            total_test_cases: overall_metrics.total_tests,
            success_rate: overall_metrics.pass_rate(),
            performance_metrics: overall_metrics,
            phase_summaries,
            overall_assessment,
        };

        // Update runner status
        {
            let mut status = self.status.write().await;
            *status = RunnerStatus::Idle;
        }

        info!("Task 8.4 User Acceptance Testing execution completed");
        Ok(summary)
    }

    /// Execute a single phase
    async fn execute_phase(&self, phase: &ExecutionPhase) -> Result<PhaseSummary, UatError> {
        let phase_start = Instant::now();
        let mut test_results = Vec::new();
        let mut phase_metrics = UatMetrics::new();
        let mut issues = Vec::new();

        // Create session for this phase
        let session_config = UatSession {
            session_id: format!("{}_{}", phase.phase_id, chrono::Utc::now().timestamp()),
            name: phase.name.clone(),
            categories: phase.categories.clone(),
            personas: phase.personas.clone(),
            devices: vec![DeviceType::Desktop, DeviceType::Mobile],
            browsers: vec![
                BrowserType::Chrome,
                BrowserType::Firefox,
                BrowserType::Terminal,
            ],
            operating_systems: vec![
                OperatingSystem::Linux,
                OperatingSystem::Windows,
                OperatingSystem::MacOS,
            ],
            max_duration: phase.estimated_duration,
            user_count: DEFAULT_TEST_USER_COUNT,
            include_manual: true,
            detailed_reporting: true,
        };

        // Execute the session
        match self.framework.create_session(session_config).await {
            Ok(session_id) => {
                match self.framework.execute_session(&session_id).await {
                    Ok(metrics) => {
                        phase_metrics = metrics;

                        // Collect results
                        match self.framework.get_session_results(&session_id).await {
                            Ok(results) => test_results = results,
                            Err(e) => issues.push(format!("Failed to collect results: {}", e)),
                        }
                    }
                    Err(e) => {
                        issues.push(format!("Session execution failed: {}", e));
                    }
                }
            }
            Err(e) => {
                issues.push(format!("Failed to create session: {}", e));
            }
        }

        let phase_duration = phase_start.elapsed();

        // Determine phase result
        let phase_result = if issues.is_empty() && phase_metrics.pass_rate() >= 0.9 {
            UatResult::Pass
        } else if phase_metrics.pass_rate() >= 0.7 {
            UatResult::Warning
        } else {
            UatResult::Fail
        };

        // Generate recommendations
        let recommendations = self
            .generate_phase_recommendations(phase, &phase_metrics, &issues)
            .await;

        Ok(PhaseSummary {
            phase: phase.clone(),
            result: phase_result,
            duration: phase_duration,
            test_results,
            metrics: phase_metrics,
            issues,
            recommendations,
        })
    }

    /// Generate phase-specific recommendations
    async fn generate_phase_recommendations(
        &self,
        phase: &ExecutionPhase,
        metrics: &UatMetrics,
        issues: &[String],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        match phase.phase_id.as_str() {
            "PHASE_1_FUNCTIONAL" => {
                if metrics.failure_rate() > 0.1 {
                    recommendations.push(
                        "Review and fix failing functional tests before proceeding".to_string(),
                    );
                }
                if metrics.average_duration > Duration::from_secs(30) {
                    recommendations.push(
                        "Investigate performance issues affecting user workflows".to_string(),
                    );
                }
                if !issues.is_empty() {
                    recommendations
                        .push("Address all functional issues before usability testing".to_string());
                }
            }
            "PHASE_2_USABILITY" => {
                if metrics.average_satisfaction() < 7.0 {
                    recommendations
                        .push("Conduct additional user interface improvements".to_string());
                }
                if metrics.average_completion_rate() < 0.9 {
                    recommendations.push(
                        "Simplify user workflows and improve task completion rates".to_string(),
                    );
                }
                recommendations
                    .push("Gather detailed user feedback for interface enhancements".to_string());
            }
            "PHASE_3_COMPATIBILITY" => {
                if metrics.failure_rate() > 0.05 {
                    recommendations
                        .push("Address compatibility issues across platforms".to_string());
                }
                recommendations.push(
                    "Validate performance consistency across all target platforms".to_string(),
                );
                recommendations.push("Confirm production deployment readiness".to_string());
            }
            _ => {
                recommendations
                    .push("Review phase results and address any identified issues".to_string());
            }
        }

        recommendations
    }

    /// Generate overall assessment
    async fn generate_overall_assessment(
        &self,
        phase_summaries: &[PhaseSummary],
        overall_metrics: &UatMetrics,
    ) -> OverallAssessment {
        let mut critical_issues = Vec::new();
        let mut blocking_issues = Vec::new();
        let mut production_recommendations = Vec::new();

        // Analyze phase results
        let mut functional_passed = false;
        let mut usability_passed = false;
        let mut compatibility_passed = false;

        for summary in phase_summaries {
            match summary.phase.phase_id.as_str() {
                "PHASE_1_FUNCTIONAL" => {
                    functional_passed =
                        matches!(summary.result, UatResult::Pass | UatResult::Warning);
                    if !functional_passed {
                        blocking_issues.push("Core functionality testing failed".to_string());
                    }
                }
                "PHASE_2_USABILITY" => {
                    usability_passed =
                        matches!(summary.result, UatResult::Pass | UatResult::Warning);
                    if !usability_passed {
                        critical_issues
                            .push("Usability testing identified significant issues".to_string());
                    }
                }
                "PHASE_3_COMPATIBILITY" => {
                    compatibility_passed =
                        matches!(summary.result, UatResult::Pass | UatResult::Warning);
                    if !compatibility_passed {
                        critical_issues
                            .push("Compatibility issues found across platforms".to_string());
                    }
                }
                _ => {}
            }

            // Collect issues
            for issue in &summary.issues {
                if issue.contains("critical") || issue.contains("blocking") {
                    blocking_issues.push(issue.clone());
                } else {
                    critical_issues.push(issue.clone());
                }
            }
        }

        // Calculate scores
        let readiness_score = if functional_passed && usability_passed && compatibility_passed {
            85.0 + (overall_metrics.pass_rate() * 15.0)
        } else if functional_passed && usability_passed {
            60.0 + (overall_metrics.pass_rate() * 25.0)
        } else if functional_passed {
            30.0 + (overall_metrics.pass_rate() * 30.0)
        } else {
            overall_metrics.pass_rate() * 30.0
        };

        let satisfaction_score = overall_metrics.average_satisfaction().max(6.0);
        let quality_score = overall_metrics.pass_rate() * 100.0;

        // Generate production recommendations
        if blocking_issues.is_empty() && critical_issues.is_empty() && readiness_score >= 90.0 {
            production_recommendations
                .push("System is ready for production deployment".to_string());
            production_recommendations
                .push("Monitor system performance closely during initial deployment".to_string());
        } else if blocking_issues.is_empty() && readiness_score >= 80.0 {
            production_recommendations
                .push("Address minor issues before production deployment".to_string());
            production_recommendations
                .push("Prepare rollback plan for production deployment".to_string());
        } else {
            production_recommendations
                .push("Significant improvements required before production".to_string());
            production_recommendations
                .push("Re-run user acceptance testing after fixes".to_string());
        }

        // Determine overall recommendation
        let overall_recommendation = if blocking_issues.is_empty() && readiness_score >= 90.0 {
            ProductionRecommendation::ReadyForProduction
        } else if blocking_issues.is_empty() && readiness_score >= 80.0 {
            ProductionRecommendation::ReadyWithMinorFixes(critical_issues.clone())
        } else if blocking_issues.is_empty() {
            ProductionRecommendation::RequiresImprovements(critical_issues.clone())
        } else {
            ProductionRecommendation::NotReady(blocking_issues.clone())
        };

        OverallAssessment {
            readiness_score,
            satisfaction_score,
            quality_score,
            critical_issues,
            blocking_issues,
            production_recommendations,
            overall_recommendation,
        }
    }

    /// Get runner status
    pub async fn get_status(&self) -> RunnerStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> UatMetrics {
        let metrics = self.metrics_aggregator.lock().await;
        metrics.clone()
    }

    /// Add execution request to queue
    pub async fn queue_execution(&self, request: TestExecutionRequest) -> Result<(), UatError> {
        let mut queue = self.execution_queue.lock().await;
        queue.push(request);

        // Sort by priority
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(())
    }

    /// Process execution queue
    pub async fn process_queue(&self) -> Result<Vec<String>, UatError> {
        let mut executed_sessions = Vec::new();

        loop {
            let request = {
                let mut queue = self.execution_queue.lock().await;
                queue.pop()
            };

            match request {
                Some(req) => {
                    info!("Processing execution request: {}", req.request_id);

                    match self.framework.create_session(req.session_config).await {
                        Ok(session_id) => match self.framework.execute_session(&session_id).await {
                            Ok(_) => {
                                executed_sessions.push(session_id);
                            }
                            Err(e) => {
                                error!("Failed to execute session: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to create session: {}", e);
                        }
                    }
                }
                None => break,
            }
        }

        Ok(executed_sessions)
    }

    /// Cleanup completed sessions
    pub async fn cleanup(&self) -> Result<usize, UatError> {
        self.framework.cleanup_sessions().await
    }

    /// Shutdown runner
    pub async fn shutdown(&self) -> Result<(), UatError> {
        info!("Shutting down UAT Test Runner");

        // Update status
        {
            let mut status = self.status.write().await;
            *status = RunnerStatus::Shutdown;
        }

        // Shutdown framework
        self.framework.shutdown().await?;

        info!("UAT Test Runner shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::user_acceptance::framework::UatFrameworkConfig;

    #[tokio::test]
    async fn test_runner_initialization() {
        let config = UatFrameworkConfig::default();
        let env = UatEnvironment {
            name: "test".to_string(),
            server_endpoint: "http://localhost:8080".to_string(),
            database_url: "sqlite::memory:".to_string(),
            test_data_dir: "/tmp/uat_test_data".to_string(),
            log_dir: "/tmp/uat_logs".to_string(),
            report_dir: "/tmp/uat_reports".to_string(),
            settings: std::collections::HashMap::new(),
        };

        let framework = UatFramework::new(config, env);
        let scenarios = UatScenarios::with_default_scenarios();
        let runner_config = UatRunnerConfig::default();

        let runner = UatTestRunner::new(framework, scenarios, runner_config);

        // Test runner creation
        let status = runner.get_status().await;
        assert!(matches!(status, RunnerStatus::Initializing));
    }

    #[tokio::test]
    async fn test_execution_plan_creation() {
        let config = UatFrameworkConfig::default();
        let env = UatEnvironment {
            name: "test".to_string(),
            server_endpoint: "http://localhost:8080".to_string(),
            database_url: "sqlite::memory:".to_string(),
            test_data_dir: "/tmp/uat_test_data".to_string(),
            log_dir: "/tmp/uat_logs".to_string(),
            report_dir: "/tmp/uat_reports".to_string(),
            settings: std::collections::HashMap::new(),
        };

        let framework = UatFramework::new(config, env);
        let scenarios = UatScenarios::with_default_scenarios();
        let runner_config = UatRunnerConfig::default();

        let runner = UatTestRunner::new(framework, scenarios, runner_config);

        let plan = runner.create_task_8_4_execution_plan().await.unwrap();

        assert_eq!(plan.plan_id, "TASK_8_4_UAT_PLAN");
        assert_eq!(plan.phases.len(), 3);
        assert_eq!(plan.phases[0].phase_id, "PHASE_1_FUNCTIONAL");
        assert_eq!(plan.phases[1].phase_id, "PHASE_2_USABILITY");
        assert_eq!(plan.phases[2].phase_id, "PHASE_3_COMPATIBILITY");
    }

    #[test]
    fn test_execution_priority_ordering() {
        let mut requests = vec![
            TestExecutionRequest {
                request_id: "low".to_string(),
                session_config: UatSession {
                    session_id: "test".to_string(),
                    name: "test".to_string(),
                    categories: vec![],
                    personas: vec![],
                    devices: vec![],
                    browsers: vec![],
                    operating_systems: vec![],
                    max_duration: Duration::from_secs(60),
                    user_count: 1,
                    include_manual: false,
                    detailed_reporting: false,
                },
                priority: ExecutionPriority::Low,
                scheduled_time: None,
                dependencies: vec![],
                parameters: std::collections::HashMap::new(),
            },
            TestExecutionRequest {
                request_id: "critical".to_string(),
                session_config: UatSession {
                    session_id: "test".to_string(),
                    name: "test".to_string(),
                    categories: vec![],
                    personas: vec![],
                    devices: vec![],
                    browsers: vec![],
                    operating_systems: vec![],
                    max_duration: Duration::from_secs(60),
                    user_count: 1,
                    include_manual: false,
                    detailed_reporting: false,
                },
                priority: ExecutionPriority::Critical,
                scheduled_time: None,
                dependencies: vec![],
                parameters: std::collections::HashMap::new(),
            },
        ];

        requests.sort_by(|a, b| b.priority.cmp(&a.priority));

        assert_eq!(requests[0].request_id, "critical");
        assert_eq!(requests[1].request_id, "low");
    }
}
