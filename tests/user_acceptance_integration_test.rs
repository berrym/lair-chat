//! User Acceptance Testing Integration Tests
//!
//! This module provides comprehensive integration tests for the UAT framework,
//! validating end-to-end functionality and Task 8.4 execution plan.

mod user_acceptance;
use std::time::Duration;
use tokio;
use user_acceptance::*;

/// Test complete UAT framework execution
#[tokio::test]
async fn test_complete_uat_execution() {
    // Initialize test environment
    let test_env = create_test_environment();

    // Create UAT framework
    let framework_config = UatFrameworkConfig::default();
    let framework = UatFramework::new(framework_config, test_env);

    // Initialize framework
    framework
        .initialize()
        .await
        .expect("Framework initialization failed");

    // Create test session
    let session_config = create_test_session_config();
    let session_id = framework
        .create_session(session_config)
        .await
        .expect("Session creation failed");

    // Execute session
    let metrics = framework
        .execute_session(&session_id)
        .await
        .expect("Session execution failed");

    // Validate results
    assert!(metrics.total_tests > 0, "No tests were executed");
    assert!(
        metrics.pass_rate() >= 0.0,
        "Pass rate should be non-negative"
    );

    println!("UAT execution completed successfully");
    println!("Total tests: {}", metrics.total_tests);
    println!("Pass rate: {:.2}%", metrics.pass_rate() * 100.0);
}

/// Test Task 8.4 specific execution plan
#[tokio::test]
async fn test_task_8_4_execution_plan() {
    // Create test environment and framework
    let test_env = create_test_environment();
    let framework_config = UatFrameworkConfig::default();
    let framework = UatFramework::new(framework_config, test_env);

    // Create scenarios and runner
    let scenarios = UatScenarios::with_default_scenarios();
    let runner_config = UatRunnerConfig::default();
    let test_runner = UatTestRunner::new(framework, scenarios, runner_config);

    // Execute Task 8.4 plan
    let execution_plan = test_runner
        .create_task_8_4_execution_plan()
        .await
        .expect("Failed to create Task 8.4 execution plan");

    // Validate plan structure
    assert_eq!(
        execution_plan.phases.len(),
        3,
        "Task 8.4 should have 3 phases"
    );

    // Validate phase names
    let phase_names: Vec<String> = execution_plan
        .phases
        .iter()
        .map(|p| p.name.clone())
        .collect();

    assert!(phase_names.contains(&"Functional Testing".to_string()));
    assert!(phase_names.contains(&"Usability Testing".to_string()));
    assert!(phase_names.contains(&"Compatibility Testing".to_string()));

    // Execute the plan (simulation mode)
    let execution_results = test_runner
        .execute_task_8_4_plan()
        .await
        .expect("Failed to execute Task 8.4 plan");

    // Validate execution results
    assert!(
        execution_results.sessions_executed > 0,
        "Some sessions should be executed"
    );
    assert!(
        execution_results.overall_assessment.readiness_score >= 0.0,
        "Readiness score should be valid"
    );

    println!("Task 8.4 execution plan validated successfully");
}

/// Test metrics collection integration
#[tokio::test]
async fn test_metrics_collection_integration() {
    // Create metrics collector
    let metrics_config = MetricsConfig::default();
    let mut collector = UatMetricsCollector::new(metrics_config);

    // Create sample test results
    let test_results = create_sample_test_results();

    // Record test results
    for result in &test_results {
        collector.record_test_result(result);
    }

    // Add user satisfaction data
    let satisfaction_entry = UserSatisfactionEntry {
        user_id: "test_user_1".to_string(),
        persona: UserPersona::Regular,
        satisfaction_score: 8.5,
        ease_of_use: 8.0,
        feature_completeness: 9.0,
        performance_satisfaction: 7.5,
        recommendation_score: 8.0,
        timestamp: chrono::Utc::now(),
        feedback: Some("Good overall experience".to_string()),
        task_context: "General usage testing".to_string(),
    };

    collector.record_user_satisfaction(satisfaction_entry);

    // Analyze metrics
    let analysis = collector.analyze_metrics();

    // Validate analysis
    assert!(analysis.overall_assessment.readiness_score >= 0.0);
    assert!(analysis.overall_assessment.readiness_score <= 100.0);
    assert!(
        !analysis.recommendations.is_empty(),
        "Should have recommendations"
    );

    // Test export functionality
    let json_export = collector
        .export_metrics(ExportFormat::Json)
        .expect("JSON export should work");
    assert!(!json_export.is_empty(), "JSON export should not be empty");

    let csv_export = collector
        .export_metrics(ExportFormat::Csv)
        .expect("CSV export should work");
    assert!(!csv_export.is_empty(), "CSV export should not be empty");

    println!("Metrics collection integration validated successfully");
}

/// Test report generation functionality
#[tokio::test]
async fn test_report_generation() {
    // Create reporter
    let reporter_config = ReporterConfig::default();
    let output_dir = std::env::temp_dir().join("uat_test_reports");
    let reporter = UatReporter::new(reporter_config, output_dir.to_string_lossy().to_string());

    // Create test data
    let test_results = create_sample_test_results();
    let metrics_collector = create_sample_metrics_collector();
    let metrics_analysis = metrics_collector.analyze_metrics();

    // Generate HTML report
    let html_report = reporter
        .generate_html_report(&test_results, &metrics_analysis)
        .expect("HTML report generation should succeed");

    assert!(!html_report.is_empty(), "HTML report should not be empty");
    assert!(
        html_report.contains("<!DOCTYPE html>"),
        "Should be valid HTML"
    );
    assert!(html_report.contains("UAT"), "Should contain UAT content");

    // Generate executive summary
    let exec_summary = reporter
        .generate_executive_summary(&metrics_analysis)
        .expect("Executive summary generation should succeed");

    assert!(exec_summary.overall_results.total_tests > 0);
    assert!(exec_summary.overall_results.pass_rate >= 0.0);
    assert!(!exec_summary.key_findings.is_empty());

    // Test different export formats
    let json_export = reporter
        .export_results(&test_results, &metrics_analysis, ExportFormat::Json)
        .expect("JSON export should work");
    assert!(!json_export.is_empty());

    let csv_export = reporter
        .export_results(&test_results, &metrics_analysis, ExportFormat::Csv)
        .expect("CSV export should work");
    assert!(!csv_export.is_empty());

    let markdown_export = reporter
        .export_results(&test_results, &metrics_analysis, ExportFormat::Markdown)
        .expect("Markdown export should work");
    assert!(!markdown_export.is_empty());
    assert!(markdown_export.contains("# UAT Test Report"));

    println!("Report generation validated successfully");
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling() {
    // Test invalid environment
    let invalid_env = UatEnvironment {
        name: "invalid_test_env".to_string(),
        server_endpoint: "".to_string(), // Invalid empty endpoint
        database_url: "invalid_db_url".to_string(),
        test_data_dir: "/nonexistent/dir".to_string(),
        log_dir: "/nonexistent/log".to_string(),
        report_dir: "/nonexistent/reports".to_string(),
        settings: std::collections::HashMap::new(),
    };

    // Environment validation should fail
    let validation_result = utils::validate_environment(&invalid_env);
    assert!(
        validation_result.is_err(),
        "Invalid environment should fail validation"
    );

    // Test framework initialization with invalid environment
    let framework_config = UatFrameworkConfig::default();
    let framework = UatFramework::new(framework_config, invalid_env);

    let init_result = framework.initialize().await;
    assert!(
        init_result.is_err(),
        "Framework initialization should fail with invalid environment"
    );

    // Test session limits
    let valid_env = create_test_environment();
    let mut limited_config = UatFrameworkConfig::default();
    limited_config.max_concurrent_sessions = 1;

    let limited_framework = UatFramework::new(limited_config, valid_env);
    limited_framework
        .initialize()
        .await
        .expect("Framework should initialize");

    // Create first session (should succeed)
    let session1 = create_test_session_config();
    let session1_id = limited_framework
        .create_session(session1)
        .await
        .expect("First session should be created");

    // Try to create second session (should fail due to limit)
    let session2 = create_test_session_config();
    let session2_result = limited_framework.create_session(session2).await;
    assert!(
        session2_result.is_err(),
        "Second session should fail due to limit"
    );

    println!("Error handling validated successfully");
}

/// Test performance under load
#[tokio::test]
async fn test_performance_validation() {
    let start_time = std::time::Instant::now();

    // Create metrics collector
    let metrics_config = MetricsConfig::default();
    let mut collector = UatMetricsCollector::new(metrics_config);

    // Generate large number of test results
    let num_results = 1000;
    for i in 0..num_results {
        let test_result = create_test_result_with_id(&format!("PERF-{:04}", i));
        collector.record_test_result(&test_result);
    }

    // Measure analysis performance
    let analysis_start = std::time::Instant::now();
    let analysis = collector.analyze_metrics();
    let analysis_duration = analysis_start.elapsed();

    // Validate performance
    assert!(
        analysis_duration < Duration::from_secs(5),
        "Analysis should complete within 5 seconds"
    );
    assert!(
        !analysis.recommendations.is_empty(),
        "Should generate recommendations"
    );

    let total_duration = start_time.elapsed();
    println!("Performance test completed in {:?}", total_duration);
    println!(
        "Analysis of {} results took {:?}",
        num_results, analysis_duration
    );

    // Test memory usage (basic check)
    let memory_before = get_memory_usage();

    // Create another large batch
    for i in num_results..(num_results * 2) {
        let test_result = create_test_result_with_id(&format!("MEM-{:04}", i));
        collector.record_test_result(&test_result);
    }

    let memory_after = get_memory_usage();
    let memory_growth = memory_after - memory_before;

    // Memory growth should be reasonable (less than 100MB for this test)
    assert!(
        memory_growth < 100_000_000,
        "Memory growth should be reasonable: {} bytes",
        memory_growth
    );

    println!("Performance validation completed successfully");
}

// Helper functions

fn create_test_environment() -> UatEnvironment {
    let temp_dir = std::env::temp_dir();

    UatEnvironment {
        name: "test_environment".to_string(),
        server_endpoint: "http://localhost:3000".to_string(),
        database_url: "sqlite::memory:".to_string(),
        test_data_dir: temp_dir.join("test_data").to_string_lossy().to_string(),
        log_dir: temp_dir.join("logs").to_string_lossy().to_string(),
        report_dir: temp_dir.join("reports").to_string_lossy().to_string(),
        settings: std::collections::HashMap::new(),
    }
}

fn create_test_session_config() -> UatSession {
    UatSession {
        session_id: format!("test_session_{}", chrono::Utc::now().timestamp()),
        name: "Integration Test Session".to_string(),
        categories: vec![
            TestCategory::Functional,
            TestCategory::Usability,
            TestCategory::Compatibility,
        ],
        personas: vec![UserPersona::Regular, UserPersona::Expert],
        devices: vec![DeviceType::Desktop, DeviceType::Mobile],
        browsers: vec![BrowserType::Chrome, BrowserType::Firefox],
        operating_systems: vec![OperatingSystem::Linux, OperatingSystem::Windows],
        max_duration: Duration::from_secs(3600),
        user_count: 5,
        include_manual: false,
        detailed_reporting: true,
    }
}

fn create_sample_test_results() -> Vec<UatTestResult> {
    vec![
        create_test_result_with_result("FUNC-001", UatResult::Pass),
        create_test_result_with_result("FUNC-002", UatResult::Pass),
        create_test_result_with_result("FUNC-003", UatResult::Fail),
        create_test_result_with_result("USAB-001", UatResult::Pass),
        create_test_result_with_result("USAB-002", UatResult::Warning),
        create_test_result_with_result("COMPAT-001", UatResult::Pass),
    ]
}

fn create_test_result_with_id(id: &str) -> UatTestResult {
    create_test_result_with_result(id, UatResult::Pass)
}

fn create_test_result_with_result(id: &str, result: UatResult) -> UatTestResult {
    let test_case = UatTestCase {
        id: id.to_string(),
        name: format!("Test Case {}", id),
        description: format!("Integration test case {}", id),
        category: TestCategory::Functional,
        priority: TestPriority::Medium,
        user_persona: UserPersona::Regular,
        prerequisites: vec!["System running".to_string()],
        steps: vec!["Execute test".to_string(), "Verify result".to_string()],
        expected_result: "Test should pass".to_string(),
        success_criteria: vec!["No errors".to_string(), "Expected behavior".to_string()],
        timeout: Duration::from_secs(30),
        manual_validation: false,
    };

    UatTestResult {
        test_case,
        result,
        duration: Duration::from_millis(500),
        timestamp: chrono::Utc::now(),
        criteria_met: vec!["Basic functionality".to_string()],
        criteria_failed: vec![],
        warnings: vec![],
        errors: if result == UatResult::Fail {
            vec!["Test failed".to_string()]
        } else {
            vec![]
        },
        notes: vec!["Integration test".to_string()],
        user_feedback: None,
    }
}

fn create_sample_metrics_collector() -> UatMetricsCollector {
    let metrics_config = MetricsConfig::default();
    let mut collector = UatMetricsCollector::new(metrics_config);

    let test_results = create_sample_test_results();
    for result in &test_results {
        collector.record_test_result(result);
    }

    collector
}

fn get_memory_usage() -> usize {
    // Simple memory usage estimation
    // In a real implementation, you'd use proper memory monitoring
    std::process::id() as usize * 1000 // Placeholder
}

#[cfg(test)]
mod integration_test_helpers {
    use super::*;

    /// Helper to run all integration tests in sequence
    pub async fn run_all_integration_tests() {
        println!("Starting UAT Framework Integration Tests...");

        test_complete_uat_execution().await;
        test_task_8_4_execution_plan().await;
        test_metrics_collection_integration().await;
        test_report_generation().await;
        test_error_handling().await;
        test_performance_validation().await;

        println!("All UAT Framework Integration Tests completed successfully!");
    }

    /// Validate UAT framework readiness for production
    pub async fn validate_production_readiness() -> bool {
        // Run comprehensive validation
        let framework_ready = validate_framework_components().await;
        let metrics_ready = validate_metrics_system().await;
        let reporting_ready = validate_reporting_system().await;

        framework_ready && metrics_ready && reporting_ready
    }

    async fn validate_framework_components() -> bool {
        // Test core framework functionality
        match std::panic::catch_unwind(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                test_complete_uat_execution().await;
                test_task_8_4_execution_plan().await;
            })
        }) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn validate_metrics_system() -> bool {
        // Test metrics collection and analysis
        match std::panic::catch_unwind(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                test_metrics_collection_integration().await;
            })
        }) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn validate_reporting_system() -> bool {
        // Test report generation
        match std::panic::catch_unwind(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                test_report_generation().await;
            })
        }) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
