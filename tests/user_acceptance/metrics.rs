//! User Acceptance Testing Metrics Module
//!
//! This module provides comprehensive metrics collection, analysis, and reporting
//! capabilities for user acceptance testing of the lair-chat application.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// UAT Metrics Collector - Central metrics aggregation and analysis
pub struct UatMetricsCollector {
    /// Collector configuration
    config: MetricsConfig,
    /// Real-time metrics
    real_time_metrics: RealTimeMetrics,
    /// Aggregated metrics by category
    category_metrics: HashMap<TestCategory, CategoryMetrics>,
    /// Time-series metrics data
    time_series: Vec<MetricsSnapshot>,
    /// Performance benchmarks
    benchmarks: HashMap<String, PerformanceBenchmark>,
    /// User satisfaction data
    satisfaction_data: Vec<UserSatisfactionEntry>,
    /// Metrics collection start time
    collection_start: Instant,
}

/// Metrics collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Collection interval for real-time metrics
    pub collection_interval: Duration,
    /// Enable detailed performance tracking
    pub detailed_performance: bool,
    /// Enable user behavior tracking
    pub behavior_tracking: bool,
    /// Retention period for metrics data
    pub retention_period: Duration,
    /// Export format preferences
    pub export_formats: Vec<ExportFormat>,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Real-time metrics tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealTimeMetrics {
    /// Current test execution rate (tests per minute)
    pub test_execution_rate: f64,
    /// Current success rate
    pub current_success_rate: f64,
    /// Current failure rate
    pub current_failure_rate: f64,
    /// Average response time (last 100 operations)
    pub avg_response_time: Duration,
    /// Active concurrent tests
    pub active_tests: usize,
    /// System resource utilization
    pub resource_utilization: ResourceUtilization,
    /// Quality score trend
    pub quality_trend: QualityTrend,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_usage: u64,
    /// Network bandwidth utilization
    pub network_usage: NetworkUsage,
    /// Disk I/O metrics
    pub disk_io: DiskIoMetrics,
}

/// Network usage metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes sent per second
    pub bytes_sent_per_sec: u64,
    /// Bytes received per second
    pub bytes_received_per_sec: u64,
    /// Connection count
    pub active_connections: usize,
    /// Average latency
    pub avg_latency: Duration,
}

/// Disk I/O metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskIoMetrics {
    /// Read operations per second
    pub reads_per_sec: f64,
    /// Write operations per second
    pub writes_per_sec: f64,
    /// Read bytes per second
    pub read_bytes_per_sec: u64,
    /// Write bytes per second
    pub write_bytes_per_sec: u64,
}

/// Quality trend tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityTrend {
    /// Quality is improving
    Improving,
    /// Quality is stable
    Stable,
    /// Quality is declining
    Declining,
    /// Insufficient data for trend analysis
    Unknown,
}

/// Category-specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryMetrics {
    /// Total tests in category
    pub total_tests: usize,
    /// Passed tests
    pub passed_tests: usize,
    /// Failed tests
    pub failed_tests: usize,
    /// Tests with warnings
    pub warning_tests: usize,
    /// Skipped tests
    pub skipped_tests: usize,
    /// Average execution time
    pub avg_execution_time: Duration,
    /// Success rate
    pub success_rate: f64,
    /// Category-specific KPIs
    pub category_kpis: CategoryKpis,
}

/// Category-specific KPIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CategoryKpis {
    /// Functional testing KPIs
    Functional(FunctionalKpis),
    /// Usability testing KPIs
    Usability(UsabilityKpis),
    /// Compatibility testing KPIs
    Compatibility(CompatibilityKpis),
    /// Integration testing KPIs
    Integration(IntegrationKpis),
    /// Performance testing KPIs
    Performance(PerformanceKpis),
}

/// Functional testing KPIs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionalKpis {
    /// Feature coverage percentage
    pub feature_coverage: f64,
    /// Business requirement coverage
    pub requirement_coverage: f64,
    /// Critical path success rate
    pub critical_path_success: f64,
    /// Error recovery rate
    pub error_recovery_rate: f64,
    /// Data integrity score
    pub data_integrity_score: f64,
}

/// Usability testing KPIs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsabilityKpis {
    /// Task completion rate
    pub task_completion_rate: f64,
    /// User satisfaction score (1-10)
    pub user_satisfaction: f64,
    /// Task efficiency score
    pub task_efficiency: f64,
    /// Error rate per task
    pub error_rate: f64,
    /// Learnability score
    pub learnability_score: f64,
    /// Accessibility compliance score
    pub accessibility_score: f64,
}

/// Compatibility testing KPIs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityKpis {
    /// Cross-platform compatibility rate
    pub cross_platform_rate: f64,
    /// Browser compatibility rate
    pub browser_compatibility: f64,
    /// Device compatibility rate
    pub device_compatibility: f64,
    /// Feature parity score
    pub feature_parity: f64,
    /// Performance consistency score
    pub performance_consistency: f64,
    /// UI consistency score
    pub ui_consistency: f64,
}

/// Integration testing KPIs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationKpis {
    /// API integration success rate
    pub api_integration_rate: f64,
    /// Service reliability score
    pub service_reliability: f64,
    /// Data flow integrity
    pub data_flow_integrity: f64,
    /// End-to-end scenario success
    pub e2e_success_rate: f64,
    /// Third-party integration score
    pub third_party_integration: f64,
}

/// Performance testing KPIs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceKpis {
    /// Response time percentiles
    pub response_time_percentiles: ResponseTimePercentiles,
    /// Throughput metrics
    pub throughput: f64,
    /// Resource efficiency score
    pub resource_efficiency: f64,
    /// Scalability score
    pub scalability_score: f64,
    /// Reliability score under load
    pub load_reliability: f64,
}

/// Response time percentiles
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseTimePercentiles {
    /// 50th percentile (median)
    pub p50: Duration,
    /// 90th percentile
    pub p90: Duration,
    /// 95th percentile
    pub p95: Duration,
    /// 99th percentile
    pub p99: Duration,
    /// 99.9th percentile
    pub p999: Duration,
}

/// Metrics snapshot for time-series analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Snapshot timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Overall metrics at this point
    pub overall_metrics: UatMetrics,
    /// Category metrics
    pub category_metrics: HashMap<TestCategory, CategoryMetrics>,
    /// System metrics
    pub system_metrics: ResourceUtilization,
    /// Quality indicators
    pub quality_indicators: QualityIndicators,
}

/// Quality indicators
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityIndicators {
    /// Overall quality score (0-100)
    pub overall_quality: f64,
    /// Reliability indicator
    pub reliability: f64,
    /// Performance indicator
    pub performance: f64,
    /// Usability indicator
    pub usability: f64,
    /// Maintainability indicator
    pub maintainability: f64,
    /// Security indicator
    pub security: f64,
}

/// Performance benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBenchmark {
    /// Benchmark name
    pub name: String,
    /// Benchmark category
    pub category: String,
    /// Target value
    pub target_value: f64,
    /// Current value
    pub current_value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Benchmark status
    pub status: BenchmarkStatus,
    /// Historical data
    pub historical_values: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
}

/// Benchmark status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkStatus {
    /// Meeting or exceeding target
    Meeting,
    /// Close to target (within 10%)
    NearTarget,
    /// Below target but acceptable
    BelowTarget,
    /// Significantly below target
    Critical,
    /// No data available
    NoData,
}

/// User satisfaction entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSatisfactionEntry {
    /// User identifier
    pub user_id: String,
    /// User persona
    pub persona: UserPersona,
    /// Satisfaction score (1-10)
    pub satisfaction_score: f64,
    /// Ease of use rating (1-10)
    pub ease_of_use: f64,
    /// Feature completeness rating (1-10)
    pub feature_completeness: f64,
    /// Performance satisfaction (1-10)
    pub performance_satisfaction: f64,
    /// Overall recommendation score (1-10)
    pub recommendation_score: f64,
    /// Feedback timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Detailed feedback
    pub feedback: String,
    /// Task context
    pub task_context: String,
}

/// Export formats for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
    /// HTML report
    Html,
    /// PDF report
    Pdf,
    /// Excel spreadsheet
    Excel,
}

/// Alert thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Minimum success rate before alert
    pub min_success_rate: f64,
    /// Maximum failure rate before alert
    pub max_failure_rate: f64,
    /// Maximum average response time before alert
    pub max_response_time: Duration,
    /// Minimum user satisfaction before alert
    pub min_user_satisfaction: f64,
    /// Maximum error rate before alert
    pub max_error_rate: f64,
    /// Quality score threshold
    pub min_quality_score: f64,
}

/// Metrics analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAnalysis {
    /// Analysis timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Overall assessment
    pub overall_assessment: OverallAssessment,
    /// Trend analysis
    pub trend_analysis: TrendAnalysis,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Recommendations
    pub recommendations: Vec<MetricsRecommendation>,
    /// Benchmark comparison
    pub benchmark_comparison: BenchmarkComparison,
}

/// Overall assessment from metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallAssessment {
    /// Production readiness score (0-100)
    pub readiness_score: f64,
    /// Quality gate status
    pub quality_gate_status: QualityGateStatus,
    /// Critical issues count
    pub critical_issues: usize,
    /// Blocker issues count
    pub blocker_issues: usize,
    /// Areas of concern
    pub areas_of_concern: Vec<String>,
    /// Strengths identified
    pub strengths: Vec<String>,
}

/// Quality gate status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityGateStatus {
    /// All quality gates passed
    Passed,
    /// Some quality gates passed with warnings
    Warning,
    /// Quality gates failed
    Failed,
    /// Insufficient data for assessment
    Inconclusive,
}

/// Trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Success rate trend
    pub success_rate_trend: TrendDirection,
    /// Performance trend
    pub performance_trend: TrendDirection,
    /// User satisfaction trend
    pub satisfaction_trend: TrendDirection,
    /// Quality trend
    pub quality_trend: TrendDirection,
    /// Velocity trend (tests per hour)
    pub velocity_trend: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Strongly improving
    StronglyImproving,
    /// Improving
    Improving,
    /// Stable
    Stable,
    /// Declining
    Declining,
    /// Strongly declining
    StronglyDeclining,
    /// Volatile
    Volatile,
    /// Insufficient data
    InsufficientData,
}

/// Risk assessment from metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub overall_risk: RiskLevel,
    /// Identified risks
    pub identified_risks: Vec<IdentifiedRisk>,
    /// Risk mitigation suggestions
    pub mitigation_suggestions: Vec<String>,
    /// Production deployment risk
    pub deployment_risk: RiskLevel,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

/// Identified risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedRisk {
    /// Risk category
    pub category: String,
    /// Risk description
    pub description: String,
    /// Risk level
    pub level: RiskLevel,
    /// Impact assessment
    pub impact: String,
    /// Probability
    pub probability: f64,
    /// Suggested mitigation
    pub mitigation: String,
}

/// Metrics-based recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsRecommendation {
    /// Recommendation category
    pub category: String,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Recommendation description
    pub description: String,
    /// Expected impact
    pub expected_impact: String,
    /// Implementation effort
    pub implementation_effort: EffortLevel,
    /// Supporting metrics
    pub supporting_metrics: Vec<String>,
}

/// Recommendation priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Critical - must implement
    Critical,
    /// High priority
    High,
    /// Medium priority
    Medium,
    /// Low priority
    Low,
    /// Nice to have
    Optional,
}

/// Implementation effort levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    /// Minimal effort required
    Minimal,
    /// Low effort required
    Low,
    /// Medium effort required
    Medium,
    /// High effort required
    High,
    /// Significant effort required
    Significant,
}

/// Benchmark comparison results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    /// Benchmarks meeting targets
    pub meeting_targets: usize,
    /// Benchmarks near targets
    pub near_targets: usize,
    /// Benchmarks below targets
    pub below_targets: usize,
    /// Critical benchmarks
    pub critical_benchmarks: usize,
    /// Best performing benchmarks
    pub best_performers: Vec<String>,
    /// Worst performing benchmarks
    pub worst_performers: Vec<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(10),
            detailed_performance: true,
            behavior_tracking: true,
            retention_period: Duration::from_secs(30 * 24 * 3600), // 30 days
            export_formats: vec![ExportFormat::Json, ExportFormat::Html],
            alert_thresholds: AlertThresholds {
                min_success_rate: 0.85,
                max_failure_rate: 0.15,
                max_response_time: Duration::from_secs(5),
                min_user_satisfaction: 7.0,
                max_error_rate: 0.05,
                min_quality_score: 80.0,
            },
        }
    }
}

impl Default for CategoryKpis {
    fn default() -> Self {
        CategoryKpis::Functional(FunctionalKpis::default())
    }
}

impl Default for QualityTrend {
    fn default() -> Self {
        QualityTrend::Unknown
    }
}

impl UatMetricsCollector {
    /// Create new metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            real_time_metrics: RealTimeMetrics::default(),
            category_metrics: HashMap::new(),
            time_series: Vec::new(),
            benchmarks: HashMap::new(),
            satisfaction_data: Vec::new(),
            collection_start: Instant::now(),
        }
    }

    /// Record test result and update metrics
    pub fn record_test_result(&mut self, result: &UatTestResult) {
        // Update overall metrics
        self.update_real_time_metrics(result);

        // Update category metrics
        self.update_category_metrics(result);

        // Update benchmarks
        self.update_benchmarks(result);

        // Create snapshot if interval elapsed
        self.maybe_create_snapshot();
    }

    /// Update real-time metrics
    fn update_real_time_metrics(&mut self, result: &UatTestResult) {
        // Update success/failure rates
        match result.result {
            UatResult::Pass => {
                self.real_time_metrics.current_success_rate =
                    (self.real_time_metrics.current_success_rate * 0.9) + 0.1;
                self.real_time_metrics.current_failure_rate =
                    self.real_time_metrics.current_failure_rate * 0.9;
            }
            UatResult::Fail => {
                self.real_time_metrics.current_failure_rate =
                    (self.real_time_metrics.current_failure_rate * 0.9) + 0.1;
                self.real_time_metrics.current_success_rate =
                    self.real_time_metrics.current_success_rate * 0.9;
            }
            _ => {}
        }

        // Update response time
        self.real_time_metrics.avg_response_time =
            (self.real_time_metrics.avg_response_time * 9 + result.duration) / 10;

        // Update execution rate
        let elapsed_minutes = self.collection_start.elapsed().as_secs_f64() / 60.0;
        if elapsed_minutes > 0.0 {
            self.real_time_metrics.test_execution_rate = 1.0 / elapsed_minutes;
        }
    }

    /// Update category-specific metrics
    fn update_category_metrics(&mut self, result: &UatTestResult) {
        let category_metrics = self
            .category_metrics
            .entry(result.test_case.category.clone())
            .or_insert_with(CategoryMetrics::default);

        category_metrics.total_tests += 1;

        match result.result {
            UatResult::Pass => category_metrics.passed_tests += 1,
            UatResult::Fail => category_metrics.failed_tests += 1,
            UatResult::Warning => category_metrics.warning_tests += 1,
            UatResult::Skipped => category_metrics.skipped_tests += 1,
            UatResult::Error => category_metrics.failed_tests += 1,
        }

        // Update average execution time
        let total_duration = category_metrics.avg_execution_time
            * (category_metrics.total_tests - 1) as u32
            + result.duration;
        category_metrics.avg_execution_time = total_duration / category_metrics.total_tests as u32;

        // Update success rate
        category_metrics.success_rate =
            category_metrics.passed_tests as f64 / category_metrics.total_tests as f64;
    }

    /// Update performance benchmarks
    fn update_benchmarks(&mut self, result: &UatTestResult) {
        // Update response time benchmark
        let response_time_ms = result.duration.as_millis() as f64;
        self.update_benchmark(
            "response_time",
            "Performance",
            response_time_ms,
            "ms",
            1000.0,
        );

        // Update success rate benchmark
        let success_rate = if matches!(result.result, UatResult::Pass) {
            100.0
        } else {
            0.0
        };
        self.update_benchmark("success_rate", "Quality", success_rate, "%", 95.0);
    }

    /// Update individual benchmark
    fn update_benchmark(
        &mut self,
        name: &str,
        category: &str,
        value: f64,
        unit: &str,
        target: f64,
    ) {
        let benchmark =
            self.benchmarks
                .entry(name.to_string())
                .or_insert_with(|| PerformanceBenchmark {
                    name: name.to_string(),
                    category: category.to_string(),
                    target_value: target,
                    current_value: value,
                    unit: unit.to_string(),
                    status: BenchmarkStatus::NoData,
                    historical_values: Vec::new(),
                });

        benchmark.current_value = value;
        benchmark
            .historical_values
            .push((chrono::Utc::now(), value));

        // Update status
        let ratio = value / target;
        benchmark.status = if ratio >= 1.0 {
            BenchmarkStatus::Meeting
        } else if ratio >= 0.9 {
            BenchmarkStatus::NearTarget
        } else if ratio >= 0.7 {
            BenchmarkStatus::BelowTarget
        } else {
            BenchmarkStatus::Critical
        };

        // Keep only recent historical data
        let cutoff = chrono::Utc::now()
            - chrono::Duration::from_std(self.config.retention_period).unwrap_or_default();
        benchmark
            .historical_values
            .retain(|(timestamp, _)| *timestamp > cutoff);
    }

    /// Create metrics snapshot
    fn maybe_create_snapshot(&mut self) {
        if self.time_series.is_empty()
            || self.time_series.last().unwrap().timestamp
                + chrono::Duration::from_std(self.config.collection_interval).unwrap_or_default()
                < chrono::Utc::now()
        {
            let snapshot = MetricsSnapshot {
                timestamp: chrono::Utc::now(),
                overall_metrics: self.calculate_overall_metrics(),
                category_metrics: self.category_metrics.clone(),
                system_metrics: self.collect_system_metrics(),
                quality_indicators: self.calculate_quality_indicators(),
            };

            self.time_series.push(snapshot);

            // Cleanup old snapshots
            let cutoff = chrono::Utc::now()
                - chrono::Duration::from_std(self.config.retention_period).unwrap_or_default();
            self.time_series
                .retain(|snapshot| snapshot.timestamp > cutoff);
        }
    }

    /// Calculate overall metrics
    fn calculate_overall_metrics(&self) -> UatMetrics {
        let mut overall = UatMetrics::new();

        for metrics in self.category_metrics.values() {
            overall.total_tests += metrics.total_tests;
            overall.passed_tests += metrics.passed_tests;
            overall.failed_tests += metrics.failed_tests;
            overall.warning_tests += metrics.warning_tests;
            overall.skipped_tests += metrics.skipped_tests;
        }

        if overall.total_tests > 0 {
            overall.total_duration = self
                .category_metrics
                .values()
                .map(|m| m.avg_execution_time * m.total_tests as u32)
                .sum();
            overall.average_duration = overall.total_duration / overall.total_tests as u32;
        }

        overall
    }

    /// Collect system metrics
    fn collect_system_metrics(&self) -> ResourceUtilization {
        // In a real implementation, this would collect actual system metrics
        ResourceUtilization {
            cpu_usage: 25.0,
            memory_usage: 512,
            network_usage: NetworkUsage {
                bytes_sent_per_sec: 1024,
                bytes_received_per_sec: 2048,
                active_connections: 10,
                avg_latency: Duration::from_millis(50),
            },
            disk_io: DiskIoMetrics {
                reads_per_sec: 10.0,
                writes_per_sec: 5.0,
                read_bytes_per_sec: 1024,
                write_bytes_per_sec: 512,
            },
        }
    }

    /// Calculate quality indicators
    fn calculate_quality_indicators(&self) -> QualityIndicators {
        let overall_metrics = self.calculate_overall_metrics();

        QualityIndicators {
            overall_quality: overall_metrics.pass_rate() * 100.0,
            reliability: overall_metrics.pass_rate() * 100.0,
            performance: self.calculate_performance_indicator(),
            usability: self.calculate_usability_indicator(),
            maintainability: 85.0, // Placeholder
            security: 90.0,        // Placeholder
        }
    }

    /// Calculate performance indicator
    fn calculate_performance_indicator(&self) -> f64 {
        if let Some(benchmark) = self.benchmarks.get("response_time") {
            match benchmark.status {
                BenchmarkStatus::Meeting => 95.0,
                BenchmarkStatus::NearTarget => 85.0,
                BenchmarkStatus::BelowTarget => 70.0,
                BenchmarkStatus::Critical => 50.0,
                BenchmarkStatus::NoData => 0.0,
            }
        } else {
            0.0
        }
    }

    /// Calculate usability indicator
    fn calculate_usability_indicator(&self) -> f64 {
        if self.satisfaction_data.is_empty() {
            return 0.0;
        }

        let avg_satisfaction = self
            .satisfaction_data
            .iter()
            .map(|entry| entry.satisfaction_score)
            .sum::<f64>()
            / self.satisfaction_data.len() as f64;

        (avg_satisfaction / 10.0) * 100.0
    }

    /// Record user satisfaction
    pub fn record_user_satisfaction(&mut self, satisfaction: UserSatisfactionEntry) {
        self.satisfaction_data.push(satisfaction);

        // Update usability KPIs if applicable
        if let Some(CategoryMetrics {
            category_kpis: CategoryKpis::Usability(ref mut kpis),
            ..
        }) = self.category_metrics.get_mut(&TestCategory::Usability)
        {
            let avg_satisfaction = self
                .satisfaction_data
                .iter()
                .map(|entry| entry.satisfaction_score)
                .sum::<f64>()
                / self.satisfaction_data.len() as f64;

            kpis.user_satisfaction = avg_satisfaction;
        }
    }

    /// Analyze metrics and generate insights
    pub fn analyze_metrics(&self) -> MetricsAnalysis {
        MetricsAnalysis {
            timestamp: chrono::Utc::now(),
            overall_assessment: self.generate_overall_assessment(),
            trend_analysis: self.analyze_trends(),
            risk_assessment: self.assess_risks(),
            recommendations: self.generate_recommendations(),
            benchmark_comparison: self.compare_benchmarks(),
        }
    }

    /// Generate overall assessment
    fn generate_overall_assessment(&self) -> OverallAssessment {
        let overall_metrics = self.calculate_overall_metrics();
        let quality_indicators = self.calculate_quality_indicators();

        let readiness_score = (quality_indicators.overall_quality * 0.3
            + quality_indicators.reliability * 0.25
            + quality_indicators.performance * 0.2
            + quality_indicators.usability * 0.15
            + quality_indicators.security * 0.1);

        let quality_gate_status = if readiness_score >= 90.0 {
            QualityGateStatus::Passed
        } else if readiness_score >= 75.0 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Failed
        };

        OverallAssessment {
            readiness_score,
            quality_gate_status,
            critical_issues: overall_metrics.failed_tests,
            blocker_issues: overall_metrics.error_tests,
            areas_of_concern: self.identify_areas_of_concern(),
            strengths: self.identify_strengths(),
        }
    }

    /// Identify areas of concern
    fn identify_areas_of_concern(&self) -> Vec<String> {
        let mut concerns = Vec::new();

        for (category, metrics) in &self.category_metrics {
            if metrics.success_rate < 0.8 {
                concerns.push(format!(
                    "{:?} testing has low success rate: {:.1}%",
                    category,
                    metrics.success_rate * 100.0
                ));
            }
        }

        for (name, benchmark) in &self.benchmarks {
            if matches!(benchmark.status, BenchmarkStatus::Critical) {
                concerns.push(format!(
                    "Benchmark {} is critical: {:.2} {} (target: {:.2})",
                    name, benchmark.current_value, benchmark.unit, benchmark.target_value
                ));
            }
        }

        concerns
    }

    /// Identify strengths
    fn identify_strengths(&self) -> Vec<String> {
        let mut strengths = Vec::new();

        for (category, metrics) in &self.category_metrics {
            if metrics.success_rate >= 0.95 {
                strengths.push(format!(
                    "{:?} testing shows excellent results: {:.1}%",
                    category,
                    metrics.success_rate * 100.0
                ));
            }
        }

        for (name, benchmark) in &self.benchmarks {
            if matches!(benchmark.status, BenchmarkStatus::Meeting) {
                strengths.push(format!(
                    "Benchmark {} exceeds target: {:.2} {} (target: {:.2})",
                    name, benchmark.current_value, benchmark.unit, benchmark.target_value
                ));
            }
        }

        strengths
    }

    /// Analyze trends based on time-series data
    fn analyze_trends(&self) -> TrendAnalysis {
        TrendAnalysis {
            success_rate_trend: self.calculate_success_rate_trend(),
            performance_trend: self.calculate_performance_trend(),
            satisfaction_trend: self.calculate_satisfaction_trend(),
            quality_trend: self.calculate_quality_trend(),
            velocity_trend: self.calculate_velocity_trend(),
        }
    }

    fn calculate_success_rate_trend(&self) -> TrendDirection {
        if self.time_series.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let recent_snapshots: Vec<_> = self.time_series.iter().rev().take(5).collect();

        if recent_snapshots.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let mut success_rates: Vec<f64> = recent_snapshots
            .iter()
            .map(|snapshot| {
                let total = snapshot.overall_metrics.total_tests as f64;
                if total > 0.0 {
                    snapshot.overall_metrics.passed_tests as f64 / total
                } else {
                    0.0
                }
            })
            .collect();

        success_rates.reverse(); // Oldest to newest

        let trend_slope = self.calculate_linear_trend(&success_rates);

        if trend_slope > 0.05 {
            TrendDirection::StronglyImproving
        } else if trend_slope > 0.01 {
            TrendDirection::Improving
        } else if trend_slope > -0.01 {
            TrendDirection::Stable
        } else if trend_slope > -0.05 {
            TrendDirection::Declining
        } else {
            TrendDirection::StronglyDeclining
        }
    }

    fn calculate_performance_trend(&self) -> TrendDirection {
        if self.time_series.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let recent_snapshots: Vec<_> = self.time_series.iter().rev().take(5).collect();

        if recent_snapshots.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let mut performance_scores: Vec<f64> = recent_snapshots
            .iter()
            .map(|snapshot| snapshot.quality_indicators.performance)
            .collect();

        performance_scores.reverse(); // Oldest to newest

        let trend_slope = self.calculate_linear_trend(&performance_scores);

        if trend_slope > 5.0 {
            TrendDirection::StronglyImproving
        } else if trend_slope > 1.0 {
            TrendDirection::Improving
        } else if trend_slope > -1.0 {
            TrendDirection::Stable
        } else if trend_slope > -5.0 {
            TrendDirection::Declining
        } else {
            TrendDirection::StronglyDeclining
        }
    }

    fn calculate_satisfaction_trend(&self) -> TrendDirection {
        if self.satisfaction_data.len() < 5 {
            return TrendDirection::InsufficientData;
        }

        let recent_satisfaction: Vec<_> = self
            .satisfaction_data
            .iter()
            .rev()
            .take(10)
            .map(|entry| entry.satisfaction_score)
            .collect();

        if recent_satisfaction.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let mut satisfaction_scores = recent_satisfaction;
        satisfaction_scores.reverse(); // Oldest to newest

        let trend_slope = self.calculate_linear_trend(&satisfaction_scores);

        if trend_slope > 0.5 {
            TrendDirection::StronglyImproving
        } else if trend_slope > 0.1 {
            TrendDirection::Improving
        } else if trend_slope > -0.1 {
            TrendDirection::Stable
        } else if trend_slope > -0.5 {
            TrendDirection::Declining
        } else {
            TrendDirection::StronglyDeclining
        }
    }

    fn calculate_quality_trend(&self) -> TrendDirection {
        if self.time_series.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let recent_snapshots: Vec<_> = self.time_series.iter().rev().take(5).collect();

        if recent_snapshots.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let mut quality_scores: Vec<f64> = recent_snapshots
            .iter()
            .map(|snapshot| snapshot.quality_indicators.overall_quality)
            .collect();

        quality_scores.reverse(); // Oldest to newest

        let trend_slope = self.calculate_linear_trend(&quality_scores);

        if trend_slope > 5.0 {
            TrendDirection::StronglyImproving
        } else if trend_slope > 1.0 {
            TrendDirection::Improving
        } else if trend_slope > -1.0 {
            TrendDirection::Stable
        } else if trend_slope > -5.0 {
            TrendDirection::Declining
        } else {
            TrendDirection::StronglyDeclining
        }
    }

    fn calculate_velocity_trend(&self) -> TrendDirection {
        if self.time_series.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let recent_snapshots: Vec<_> = self.time_series.iter().rev().take(5).collect();

        if recent_snapshots.len() < 2 {
            return TrendDirection::InsufficientData;
        }

        let mut velocities: Vec<f64> = Vec::new();

        for i in 1..recent_snapshots.len() {
            let prev = &recent_snapshots[i];
            let curr = &recent_snapshots[i - 1];

            let time_diff = (curr.timestamp - prev.timestamp).num_seconds() as f64;
            if time_diff > 0.0 {
                let test_diff =
                    (curr.overall_metrics.total_tests - prev.overall_metrics.total_tests) as f64;
                let velocity = test_diff / time_diff * 3600.0; // Tests per hour
                velocities.push(velocity);
            }
        }

        if velocities.is_empty() {
            return TrendDirection::InsufficientData;
        }

        let trend_slope = self.calculate_linear_trend(&velocities);

        if trend_slope > 5.0 {
            TrendDirection::StronglyImproving
        } else if trend_slope > 1.0 {
            TrendDirection::Improving
        } else if trend_slope > -1.0 {
            TrendDirection::Stable
        } else if trend_slope > -5.0 {
            TrendDirection::Declining
        } else {
            TrendDirection::StronglyDeclining
        }
    }

    fn calculate_linear_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Assess risks based on current metrics
    fn assess_risks(&self) -> RiskAssessment {
        let mut identified_risks = Vec::new();
        let overall_metrics = self.calculate_overall_metrics();
        let quality_indicators = self.calculate_quality_indicators();

        // Check for critical failure rates
        if overall_metrics.failed_tests as f64 / overall_metrics.total_tests as f64 > 0.2 {
            identified_risks.push(IdentifiedRisk {
                category: "Quality".to_string(),
                description: "High failure rate detected in testing".to_string(),
                level: RiskLevel::High,
                impact: "May cause production issues and user dissatisfaction".to_string(),
                probability: 0.8,
                mitigation:
                    "Review failed tests, fix critical bugs, add more comprehensive testing"
                        .to_string(),
            });
        }

        // Check for performance issues
        if quality_indicators.performance < 70.0 {
            identified_risks.push(IdentifiedRisk {
                category: "Performance".to_string(),
                description: "Performance metrics below acceptable threshold".to_string(),
                level: RiskLevel::Medium,
                impact: "Slow response times may affect user experience".to_string(),
                probability: 0.6,
                mitigation: "Optimize critical paths, review resource usage, conduct load testing"
                    .to_string(),
            });
        }

        // Check for usability concerns
        if quality_indicators.usability < 80.0 {
            identified_risks.push(IdentifiedRisk {
                category: "Usability".to_string(),
                description: "Usability scores indicate potential user experience issues"
                    .to_string(),
                level: RiskLevel::Medium,
                impact: "Poor user experience may lead to user churn".to_string(),
                probability: 0.5,
                mitigation: "Conduct user testing, review UI/UX design, simplify complex workflows"
                    .to_string(),
            });
        }

        // Check for reliability issues
        if quality_indicators.reliability < 90.0 {
            identified_risks.push(IdentifiedRisk {
                category: "Reliability".to_string(),
                description: "System reliability below production standards".to_string(),
                level: RiskLevel::High,
                impact: "System instability may cause downtime and data loss".to_string(),
                probability: 0.7,
                mitigation: "Improve error handling, add monitoring, implement failover mechanisms"
                    .to_string(),
            });
        }

        // Check benchmark failures
        for (name, benchmark) in &self.benchmarks {
            if matches!(benchmark.status, BenchmarkStatus::Critical) {
                identified_risks.push(IdentifiedRisk {
                    category: "Performance".to_string(),
                    description: format!("Critical benchmark failure: {}", name),
                    level: RiskLevel::Critical,
                    impact: "May prevent successful deployment".to_string(),
                    probability: 0.9,
                    mitigation: format!("Address {} performance issues immediately", name),
                });
            }
        }

        let overall_risk = if identified_risks
            .iter()
            .any(|r| matches!(r.level, RiskLevel::Critical))
        {
            RiskLevel::Critical
        } else if identified_risks
            .iter()
            .any(|r| matches!(r.level, RiskLevel::High))
        {
            RiskLevel::High
        } else if identified_risks
            .iter()
            .any(|r| matches!(r.level, RiskLevel::Medium))
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let deployment_risk = match overall_risk {
            RiskLevel::Critical => "Do not deploy - critical issues must be resolved",
            RiskLevel::High => "High risk deployment - consider delaying",
            RiskLevel::Medium => "Medium risk - deploy with caution and monitoring",
            RiskLevel::Low => "Low risk - safe to deploy",
        };

        RiskAssessment {
            overall_risk,
            identified_risks,
            mitigation_suggestions: vec![
                "Implement comprehensive monitoring".to_string(),
                "Set up automated alerting".to_string(),
                "Prepare rollback procedures".to_string(),
                "Conduct thorough testing".to_string(),
            ],
            deployment_risk: deployment_risk.to_string(),
        }
    }

    /// Generate actionable recommendations
    fn generate_recommendations(&self) -> Vec<MetricsRecommendation> {
        let mut recommendations = Vec::new();
        let overall_metrics = self.calculate_overall_metrics();
        let quality_indicators = self.calculate_quality_indicators();

        // Test coverage recommendations
        if overall_metrics.total_tests < 100 {
            recommendations.push(MetricsRecommendation {
                category: "Test Coverage".to_string(),
                priority: RecommendationPriority::High,
                description: "Increase test coverage to ensure comprehensive validation"
                    .to_string(),
                expected_impact: "Improved defect detection and system reliability".to_string(),
                implementation_effort: EffortLevel::Medium,
                supporting_metrics: vec![
                    "total_tests".to_string(),
                    "coverage_percentage".to_string(),
                ],
            });
        }

        // Performance optimization recommendations
        if quality_indicators.performance < 80.0 {
            recommendations.push(MetricsRecommendation {
                category: "Performance".to_string(),
                priority: RecommendationPriority::High,
                description: "Optimize system performance to meet user expectations".to_string(),
                expected_impact: "Faster response times and better user experience".to_string(),
                implementation_effort: EffortLevel::High,
                supporting_metrics: vec!["response_time".to_string(), "throughput".to_string()],
            });
        }

        // Usability improvements
        if quality_indicators.usability < 85.0 {
            recommendations.push(MetricsRecommendation {
                category: "Usability".to_string(),
                priority: RecommendationPriority::Medium,
                description: "Improve user interface and experience design".to_string(),
                expected_impact: "Higher user satisfaction and reduced support requests"
                    .to_string(),
                implementation_effort: EffortLevel::Medium,
                supporting_metrics: vec![
                    "user_satisfaction".to_string(),
                    "task_completion_rate".to_string(),
                ],
            });
        }

        // Automation recommendations
        recommendations.push(MetricsRecommendation {
            category: "Automation".to_string(),
            priority: RecommendationPriority::Medium,
            description: "Implement automated testing and monitoring".to_string(),
            expected_impact: "Reduced manual effort and faster feedback cycles".to_string(),
            implementation_effort: EffortLevel::High,
            supporting_metrics: vec!["automation_rate".to_string(), "feedback_time".to_string()],
        });

        // Documentation recommendations
        recommendations.push(MetricsRecommendation {
            category: "Documentation".to_string(),
            priority: RecommendationPriority::Low,
            description: "Improve test documentation and reporting".to_string(),
            expected_impact: "Better team communication and knowledge sharing".to_string(),
            implementation_effort: EffortLevel::Low,
            supporting_metrics: vec!["documentation_coverage".to_string()],
        });

        recommendations
    }

    /// Compare current performance against benchmarks
    fn compare_benchmarks(&self) -> BenchmarkComparison {
        let mut meeting_targets = 0;
        let mut near_targets = 0;
        let mut below_targets = 0;
        let mut critical_benchmarks = Vec::new();
        let mut best_performers = Vec::new();
        let mut worst_performers = Vec::new();

        for (name, benchmark) in &self.benchmarks {
            match benchmark.status {
                BenchmarkStatus::Meeting => {
                    meeting_targets += 1;
                    best_performers.push(name.clone());
                }
                BenchmarkStatus::NearTarget => {
                    near_targets += 1;
                }
                BenchmarkStatus::BelowTarget => {
                    below_targets += 1;
                    worst_performers.push(name.clone());
                }
                BenchmarkStatus::Critical => {
                    below_targets += 1;
                    critical_benchmarks.push(name.clone());
                    worst_performers.push(name.clone());
                }
                BenchmarkStatus::NoData => {
                    // Don't count in statistics
                }
            }
        }

        BenchmarkComparison {
            meeting_targets,
            near_targets,
            below_targets,
            critical_benchmarks,
            best_performers,
            worst_performers,
        }
    }

    /// Export metrics in specified format
    pub fn export_metrics(&self, format: ExportFormat) -> Result<String, crate::UatError> {
        let metrics_analysis = self.analyze_metrics();

        match format {
            ExportFormat::Json => serde_json::to_string_pretty(&metrics_analysis)
                .map_err(|e| crate::UatError::MetricsError(format!("JSON export failed: {}", e))),
            ExportFormat::Csv => self.export_to_csv(&metrics_analysis),
            ExportFormat::Xml => self.export_to_xml(&metrics_analysis),
            ExportFormat::Html => self.export_to_html(&metrics_analysis),
            ExportFormat::Pdf => Err(crate::UatError::MetricsError(
                "PDF export not yet implemented".to_string(),
            )),
            ExportFormat::Excel => Err(crate::UatError::MetricsError(
                "Excel export not yet implemented".to_string(),
            )),
        }
    }

    fn export_to_csv(&self, analysis: &MetricsAnalysis) -> Result<String, crate::UatError> {
        let mut csv = String::new();

        // Header
        csv.push_str("Metric,Value,Unit,Status\n");

        // Overall assessment
        csv.push_str(&format!(
            "Readiness Score,{:.2},percent,{:?}\n",
            analysis.overall_assessment.readiness_score,
            analysis.overall_assessment.quality_gate_status
        ));

        // Benchmarks
        for (name, benchmark) in &self.benchmarks {
            csv.push_str(&format!(
                "{},{:.2},{},{:?}\n",
                name, benchmark.current_value, benchmark.unit, benchmark.status
            ));
        }

        Ok(csv)
    }

    fn export_to_xml(&self, analysis: &MetricsAnalysis) -> Result<String, crate::UatError> {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<metrics_analysis>\n");
        xml.push_str(&format!(
            "  <timestamp>{}</timestamp>\n",
            analysis.timestamp
        ));
        xml.push_str(&format!(
            "  <readiness_score>{:.2}</readiness_score>\n",
            analysis.overall_assessment.readiness_score
        ));
        xml.push_str(&format!(
            "  <quality_gate_status>{:?}</quality_gate_status>\n",
            analysis.overall_assessment.quality_gate_status
        ));
        xml.push_str("</metrics_analysis>\n");
        Ok(xml)
    }

    fn export_to_html(&self, analysis: &MetricsAnalysis) -> Result<String, crate::UatError> {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html><head><title>UAT Metrics Report</title></head><body>\n");
        html.push_str(&format!("<h1>UAT Metrics Analysis</h1>\n"));
        html.push_str(&format!("<p>Generated: {}</p>\n", analysis.timestamp));
        html.push_str(&format!("<h2>Overall Assessment</h2>\n"));
        html.push_str(&format!(
            "<p>Readiness Score: {:.2}%</p>\n",
            analysis.overall_assessment.readiness_score
        ));
        html.push_str(&format!(
            "<p>Quality Gate: {:?}</p>\n",
            analysis.overall_assessment.quality_gate_status
        ));
        html.push_str("</body></html>\n");
        Ok(html)
    }
}

impl Default for UatMetricsCollector {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}
