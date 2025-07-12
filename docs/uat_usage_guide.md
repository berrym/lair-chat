# UAT Framework Usage Guide

## Overview

The User Acceptance Testing (UAT) Framework provides comprehensive testing capabilities for the Lair-Chat application. This guide covers how to use the framework for Task 8.4 execution and general UAT scenarios.

## Quick Start

### 1. Basic Framework Usage

```rust
use lair_chat::tests::user_acceptance::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), UatError> {
    // Create framework configuration
    let config = UatFrameworkConfig::default();
    
    // Set up test environment
    let environment = UatEnvironment {
        name: "production_uat".to_string(),
        server_endpoint: "http://localhost:3000".to_string(),
        database_url: "sqlite:./test.db".to_string(),
        test_data_dir: "./test_data".to_string(),
        log_dir: "./logs".to_string(),
        report_dir: "./reports".to_string(),
        settings: HashMap::new(),
    };
    
    // Initialize framework
    let framework = UatFramework::new(config, environment);
    framework.initialize().await?;
    
    // Create test session
    let session = UatSession {
        session_id: "uat_session_001".to_string(),
        name: "Production Readiness UAT".to_string(),
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
        user_count: 10,
        include_manual: false,
        detailed_reporting: true,
    };
    
    // Execute tests
    let session_id = framework.create_session(session).await?;
    let results = framework.execute_session(&session_id).await?;
    
    println!("UAT completed: {} tests, {:.1}% pass rate", 
             results.total_tests, results.pass_rate() * 100.0);
    
    Ok(())
}
```

### 2. Task 8.4 Execution

The framework includes a specialized runner for Task 8.4 (5-day UAT plan):

```rust
use lair_chat::tests::user_acceptance::*;

#[tokio::main]
async fn main() -> Result<(), UatError> {
    // Set up environment
    let environment = create_production_test_environment();
    let framework_config = UatFrameworkConfig::default();
    let framework = UatFramework::new(framework_config, environment);
    
    // Create scenarios and runner
    let scenarios = UatScenarios::with_default_scenarios();
    let runner_config = UatRunnerConfig::default();
    let runner = UatTestRunner::new(framework, scenarios, runner_config);
    
    // Execute complete Task 8.4 plan
    let execution_summary = runner.execute_task_8_4_plan().await?;
    
    println!("Task 8.4 Execution Summary:");
    println!("- Sessions executed: {}", execution_summary.sessions_executed);
    println!("- Total test cases: {}", execution_summary.total_test_cases);
    println!("- Success rate: {:.2}%", execution_summary.success_rate * 100.0);
    println!("- Readiness score: {:.1}", execution_summary.overall_assessment.readiness_score);
    
    Ok(())
}
```

## Framework Components

### 1. Core Framework (`UatFramework`)

The main orchestration component that manages test sessions and execution.

**Key Methods:**
- `new(config, environment)` - Create framework instance
- `initialize()` - Initialize framework
- `create_session(session_config)` - Create test session
- `execute_session(session_id)` - Execute tests
- `get_status()` - Get framework status

### 2. Test Runner (`UatTestRunner`)

Specialized component for executing structured test plans.

**Key Methods:**
- `new(framework, scenarios, config)` - Create runner
- `create_task_8_4_execution_plan()` - Create Task 8.4 plan
- `execute_task_8_4_plan()` - Execute complete plan

### 3. Metrics Collector (`UatMetricsCollector`)

Collects and analyzes test metrics and performance data.

**Key Methods:**
- `new(config)` - Create collector
- `record_test_result(result)` - Record test result
- `record_user_satisfaction(entry)` - Record satisfaction data
- `analyze_metrics()` - Generate analysis
- `export_metrics(format)` - Export metrics

### 4. Report Generator (`UatReporter`)

Generates comprehensive reports in multiple formats.

**Key Methods:**
- `new(config, output_dir)` - Create reporter
- `generate_html_report(results, metrics)` - Generate HTML report
- `generate_executive_summary(metrics)` - Create executive summary
- `export_results(results, metrics, format)` - Export in various formats
- `save_report(content, filename, format)` - Save to file

### 5. Scenarios Manager (`UatScenarios`)

Manages test scenarios and execution workflows.

**Key Methods:**
- `with_default_scenarios()` - Load default scenarios
- `get_scenarios_by_category(category)` - Filter by category
- `execute_scenario(id, users)` - Execute specific scenario

## Configuration Options

### Framework Configuration

```rust
let config = UatFrameworkConfig {
    name: "Custom UAT Framework".to_string(),
    version: "1.0.0".to_string(),
    max_concurrent_sessions: 5,
    default_timeout: Duration::from_secs(60),
    data_retention_days: 30,
    verbose_logging: true,
    performance_monitoring: true,
    auto_cleanup: true,
};
```

### Test Session Configuration

```rust
let session = UatSession {
    session_id: "custom_session".to_string(),
    name: "Custom Test Session".to_string(),
    categories: vec![TestCategory::Functional],
    personas: vec![UserPersona::Regular],
    devices: vec![DeviceType::Desktop],
    browsers: vec![BrowserType::Chrome],
    operating_systems: vec![OperatingSystem::Linux],
    max_duration: Duration::from_secs(1800),
    user_count: 5,
    include_manual: true,
    detailed_reporting: true,
};
```

### Metrics Configuration

```rust
let metrics_config = MetricsConfig {
    collection_interval: Duration::from_secs(30),
    detailed_performance: true,
    behavior_tracking: true,
    retention_period: Duration::from_secs(86400 * 7), // 7 days
    export_formats: vec![ExportFormat::Json, ExportFormat::Html],
    alert_thresholds: AlertThresholds {
        min_success_rate: 0.95,
        max_failure_rate: 0.05,
        max_response_time: 1000.0,
        min_user_satisfaction: 8.0,
        max_error_rate: 0.02,
        min_quality_score: 85.0,
    },
};
```

### Reporter Configuration

```rust
let reporter_config = ReporterConfig {
    title: "Custom UAT Report".to_string(),
    organization: "Your Organization".to_string(),
    version: "1.0.0".to_string(),
    include_detailed_metrics: true,
    include_charts: true,
    default_format: ExportFormat::Html,
    custom_css: Some("body { font-family: Arial; }".to_string()),
    logo_path: Some("./logo.png".to_string()),
};
```

## Task 8.4 Five-Day Plan

The framework implements the complete Task 8.4 execution plan:

### Day 1-2: Functional Testing
- User authentication and registration
- Room management and messaging
- Direct messaging functionality
- Administrative features
- Core application workflows

### Day 3-4: Usability Testing
- Interface navigation and intuitiveness
- User experience validation
- Accessibility compliance
- Error handling and recovery
- Task completion efficiency

### Day 5: Compatibility Testing
- Cross-platform compatibility
- Browser compatibility
- Device compatibility
- Performance consistency
- Final integration validation

## Report Generation

### HTML Reports

```rust
let reporter = UatReporter::new(ReporterConfig::default(), "./reports".to_string());
let html_report = reporter.generate_html_report(&test_results, &metrics_analysis)?;
let report_path = reporter.save_report(&html_report, "uat_report", ExportFormat::Html)?;
```

### Executive Summary

```rust
let executive_summary = reporter.generate_executive_summary(&metrics_analysis)?;
println!("Production Readiness: {:?}", executive_summary.production_readiness.status);
println!("Deployment Recommendation: {}", executive_summary.production_readiness.deployment_recommendation);
```

### Multiple Formats

```rust
// Export as JSON
let json_report = reporter.export_results(&results, &metrics, ExportFormat::Json)?;

// Export as CSV
let csv_report = reporter.export_results(&results, &metrics, ExportFormat::Csv)?;

// Export as Markdown
let md_report = reporter.export_results(&results, &metrics, ExportFormat::Markdown)?;
```

## Metrics and Analysis

### Recording Test Results

```rust
let mut collector = UatMetricsCollector::new(MetricsConfig::default());

for test_result in test_results {
    collector.record_test_result(&test_result);
}

// Add user satisfaction data
let satisfaction = UserSatisfactionEntry {
    user_id: "user_001".to_string(),
    persona: UserPersona::Regular,
    satisfaction_score: 8.5,
    ease_of_use: 9.0,
    feature_completeness: 8.0,
    performance_satisfaction: 8.5,
    recommendation_score: 9.0,
    timestamp: chrono::Utc::now(),
    feedback: Some("Great experience overall".to_string()),
    task_context: "General usage testing".to_string(),
};

collector.record_user_satisfaction(satisfaction);
```

### Analysis and Insights

```rust
let analysis = collector.analyze_metrics();

println!("Overall Assessment:");
println!("- Readiness Score: {:.1}%", analysis.overall_assessment.readiness_score);
println!("- Quality Gate: {:?}", analysis.overall_assessment.quality_gate_status);
println!("- Risk Level: {:?}", analysis.risk_assessment.overall_risk);

println!("\nTrend Analysis:");
println!("- Success Rate Trend: {:?}", analysis.trend_analysis.success_rate_trend);
println!("- Performance Trend: {:?}", analysis.trend_analysis.performance_trend);

println!("\nRecommendations:");
for recommendation in &analysis.recommendations {
    println!("- {}: {} (Priority: {:?})", 
             recommendation.category, 
             recommendation.description, 
             recommendation.priority);
}
```

## Error Handling

The framework uses structured error types:

```rust
match framework.execute_session(&session_id).await {
    Ok(metrics) => {
        println!("Tests completed successfully");
    }
    Err(UatError::SessionNotFound(id)) => {
        eprintln!("Session not found: {}", id);
    }
    Err(UatError::TestExecutionError(msg)) => {
        eprintln!("Test execution failed: {}", msg);
    }
    Err(UatError::EnvironmentError(msg)) => {
        eprintln!("Environment issue: {}", msg);
    }
    Err(err) => {
        eprintln!("UAT error: {}", err);
    }
}
```

## Best Practices

### 1. Environment Setup
- Use isolated test environments
- Ensure proper database setup
- Configure appropriate timeouts
- Set up logging and monitoring

### 2. Test Planning
- Define clear test categories
- Select appropriate user personas
- Plan for different devices and browsers
- Set realistic execution timeouts

### 3. Metrics Collection
- Enable detailed performance monitoring
- Collect user satisfaction data
- Track quality indicators
- Monitor system resources

### 4. Reporting
- Generate comprehensive reports
- Include executive summaries
- Export in multiple formats
- Archive test results

### 5. Production Readiness
- Validate all quality gates
- Review risk assessments
- Address critical recommendations
- Confirm deployment readiness

## Integration Examples

### With CI/CD Pipeline

```bash
#!/bin/bash
# UAT execution script for CI/CD

echo "Starting UAT execution..."

# Set environment variables
export UAT_SERVER_ENDPOINT="https://staging.example.com"
export UAT_DATABASE_URL="postgresql://user:pass@db:5432/uat_db"
export UAT_REPORT_DIR="./ci_reports"

# Run UAT framework
cargo run --bin uat_runner -- \
    --config ./config/uat_config.toml \
    --session-config ./config/session_config.toml \
    --output-dir ./reports

# Check exit code
if [ $? -eq 0 ]; then
    echo "UAT completed successfully"
    exit 0
else
    echo "UAT failed"
    exit 1
fi
```

### With Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin uat_runner

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/uat_runner /usr/local/bin/
CMD ["uat_runner"]
```

## Troubleshooting

### Common Issues

1. **Session Creation Fails**
   - Check environment configuration
   - Verify database connectivity
   - Ensure proper permissions

2. **Test Execution Timeout**
   - Increase timeout values
   - Check server performance
   - Verify network connectivity

3. **Report Generation Fails**
   - Ensure output directory exists
   - Check disk space
   - Verify write permissions

4. **Metrics Collection Issues**
   - Check collection configuration
   - Verify metric thresholds
   - Review retention settings

### Debug Mode

Enable verbose logging for detailed troubleshooting:

```rust
let config = UatFrameworkConfig {
    verbose_logging: true,
    performance_monitoring: true,
    ..Default::default()
};
```

### Log Analysis

Check logs for detailed execution information:

```bash
tail -f ./logs/uat_framework.log
grep ERROR ./logs/uat_framework.log
grep WARNING ./logs/uat_framework.log
```

## Conclusion

The UAT Framework provides comprehensive testing capabilities for ensuring production readiness. Follow this guide to effectively implement user acceptance testing for the Lair-Chat application and achieve reliable deployment validation.

For additional support or feature requests, refer to the project documentation or contact the development team.