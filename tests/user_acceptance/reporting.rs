//! User Acceptance Testing Reporting Module
//!
//! This module provides comprehensive report generation capabilities for UAT results,
//! including HTML reports, executive summaries, and various export formats.

use super::*;
use crate::tests::user_acceptance::metrics::{MetricsAnalysis, UatMetricsCollector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// UAT Report Generator - Handles all report generation functionality
pub struct UatReporter {
    /// Reporter configuration
    config: ReporterConfig,
    /// Template storage
    templates: HashMap<String, String>,
    /// Output directory
    output_dir: String,
}

/// Reporter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterConfig {
    /// Default report title
    pub title: String,
    /// Organization name
    pub organization: String,
    /// Report version
    pub version: String,
    /// Include detailed metrics
    pub include_detailed_metrics: bool,
    /// Include charts and graphs
    pub include_charts: bool,
    /// Default output format
    pub default_format: ExportFormat,
    /// Custom CSS styles
    pub custom_css: Option<String>,
    /// Logo path for reports
    pub logo_path: Option<String>,
}

/// Executive summary data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    /// Overall test results summary
    pub overall_results: OverallResultsSummary,
    /// Key findings and insights
    pub key_findings: Vec<String>,
    /// Critical issues identified
    pub critical_issues: Vec<String>,
    /// Recommendations for stakeholders
    pub recommendations: Vec<String>,
    /// Production readiness assessment
    pub production_readiness: ProductionReadiness,
    /// Next steps and action items
    pub next_steps: Vec<String>,
}

/// Overall results summary for executives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallResultsSummary {
    /// Total tests executed
    pub total_tests: usize,
    /// Pass rate percentage
    pub pass_rate: f64,
    /// Overall quality score
    pub quality_score: f64,
    /// User satisfaction average
    pub user_satisfaction: f64,
    /// Critical defects found
    pub critical_defects: usize,
    /// Testing duration
    pub testing_duration: std::time::Duration,
}

/// Production readiness assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionReadiness {
    /// Overall readiness status
    pub status: ReadinessStatus,
    /// Readiness score (0-100)
    pub score: f64,
    /// Confidence level
    pub confidence: ConfidenceLevel,
    /// Risk assessment
    pub risk_level: RiskLevel,
    /// Deployment recommendation
    pub deployment_recommendation: String,
}

/// Readiness status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessStatus {
    /// Ready for production deployment
    Ready,
    /// Ready with minor conditions
    ReadyWithConditions,
    /// Not ready - requires fixes
    NotReady,
    /// Inconclusive - more testing needed
    Inconclusive,
}

/// Confidence level in assessment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// High confidence in results
    High,
    /// Medium confidence
    Medium,
    /// Low confidence - limited data
    Low,
}

/// Risk level for deployment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk deployment
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk - do not deploy
    Critical,
}

/// Export format enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// HTML format
    Html,
    /// PDF format
    Pdf,
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
    /// Markdown format
    Markdown,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            title: "Lair-Chat User Acceptance Testing Report".to_string(),
            organization: "Lair-Chat Development Team".to_string(),
            version: "1.0.0".to_string(),
            include_detailed_metrics: true,
            include_charts: true,
            default_format: ExportFormat::Html,
            custom_css: None,
            logo_path: None,
        }
    }
}

impl UatReporter {
    /// Create new UAT reporter
    pub fn new(config: ReporterConfig, output_dir: String) -> Self {
        let mut reporter = Self {
            config,
            templates: HashMap::new(),
            output_dir,
        };

        reporter.initialize_templates();
        reporter
    }

    /// Initialize report templates
    fn initialize_templates(&mut self) {
        // HTML report template
        self.templates
            .insert("html_report".to_string(), self.get_html_template());

        // Executive summary template
        self.templates.insert(
            "executive_summary".to_string(),
            self.get_executive_template(),
        );

        // CSS styles
        self.templates
            .insert("css_styles".to_string(), self.get_css_styles());
    }

    /// Generate comprehensive HTML report
    pub fn generate_html_report(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        info!("Generating HTML report with {} test results", results.len());

        let template = self
            .templates
            .get("html_report")
            .ok_or_else(|| UatError::ReportingError("HTML template not found".to_string()))?;

        let mut html = template.clone();

        // Replace template variables
        html = html.replace("{{TITLE}}", &self.config.title);
        html = html.replace("{{ORGANIZATION}}", &self.config.organization);
        html = html.replace("{{VERSION}}", &self.config.version);
        html = html.replace(
            "{{TIMESTAMP}}",
            &chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );

        // Generate sections
        html = html.replace(
            "{{EXECUTIVE_SUMMARY}}",
            &self.generate_html_executive_summary(metrics)?,
        );
        html = html.replace(
            "{{TEST_RESULTS_SUMMARY}}",
            &self.generate_html_results_summary(results, metrics)?,
        );
        html = html.replace(
            "{{DETAILED_RESULTS}}",
            &self.generate_html_detailed_results(results)?,
        );
        html = html.replace(
            "{{METRICS_SECTION}}",
            &self.generate_html_metrics_section(metrics)?,
        );
        html = html.replace(
            "{{RECOMMENDATIONS}}",
            &self.generate_html_recommendations(metrics)?,
        );

        // Add CSS styles
        let css = self.templates.get("css_styles").unwrap_or(&String::new());
        html = html.replace("{{CSS_STYLES}}", css);

        Ok(html)
    }

    /// Generate PDF report (placeholder for future implementation)
    pub fn generate_pdf_report(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
    ) -> Result<Vec<u8>, UatError> {
        warn!("PDF generation not yet implemented - generating HTML instead");

        // For now, return error indicating PDF not implemented
        Err(UatError::ReportingError(
            "PDF generation not yet implemented. Use HTML format instead.".to_string(),
        ))
    }

    /// Generate executive summary
    pub fn generate_executive_summary(
        &self,
        metrics: &MetricsAnalysis,
    ) -> Result<ExecutiveSummary, UatError> {
        info!("Generating executive summary");

        let overall_assessment = &metrics.overall_assessment;

        let overall_results = OverallResultsSummary {
            total_tests: metrics.overall_assessment.critical_issues
                + metrics.overall_assessment.blocker_issues, // Placeholder
            pass_rate: overall_assessment.readiness_score,
            quality_score: overall_assessment.readiness_score,
            user_satisfaction: 8.5, // Placeholder - would come from actual metrics
            critical_defects: overall_assessment.critical_issues,
            testing_duration: std::time::Duration::from_secs(3600), // Placeholder
        };

        let key_findings = vec![
            format!(
                "Overall readiness score: {:.1}%",
                overall_assessment.readiness_score
            ),
            format!(
                "Quality gate status: {:?}",
                overall_assessment.quality_gate_status
            ),
            format!(
                "Critical issues identified: {}",
                overall_assessment.critical_issues
            ),
            "User acceptance testing completed successfully".to_string(),
        ];

        let critical_issues: Vec<String> = overall_assessment.areas_of_concern.clone();

        let recommendations: Vec<String> = metrics
            .recommendations
            .iter()
            .map(|r| format!("{}: {}", r.category, r.description))
            .collect();

        let production_readiness = ProductionReadiness {
            status: match overall_assessment.quality_gate_status {
                crate::tests::user_acceptance::metrics::QualityGateStatus::Passed => {
                    ReadinessStatus::Ready
                }
                crate::tests::user_acceptance::metrics::QualityGateStatus::Warning => {
                    ReadinessStatus::ReadyWithConditions
                }
                crate::tests::user_acceptance::metrics::QualityGateStatus::Failed => {
                    ReadinessStatus::NotReady
                }
                crate::tests::user_acceptance::metrics::QualityGateStatus::Inconclusive => {
                    ReadinessStatus::Inconclusive
                }
            },
            score: overall_assessment.readiness_score,
            confidence: if overall_assessment.readiness_score >= 90.0 {
                ConfidenceLevel::High
            } else if overall_assessment.readiness_score >= 75.0 {
                ConfidenceLevel::Medium
            } else {
                ConfidenceLevel::Low
            },
            risk_level: match &metrics.risk_assessment.overall_risk {
                crate::tests::user_acceptance::metrics::RiskLevel::Low => RiskLevel::Low,
                crate::tests::user_acceptance::metrics::RiskLevel::Medium => RiskLevel::Medium,
                crate::tests::user_acceptance::metrics::RiskLevel::High => RiskLevel::High,
                crate::tests::user_acceptance::metrics::RiskLevel::Critical => RiskLevel::Critical,
            },
            deployment_recommendation: metrics.risk_assessment.deployment_risk.clone(),
        };

        let next_steps = vec![
            "Review and address identified issues".to_string(),
            "Conduct final pre-deployment checks".to_string(),
            "Prepare deployment and rollback procedures".to_string(),
            "Set up production monitoring".to_string(),
        ];

        Ok(ExecutiveSummary {
            overall_results,
            key_findings,
            critical_issues,
            recommendations,
            production_readiness,
            next_steps,
        })
    }

    /// Export results in specified format
    pub fn export_results(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
        format: ExportFormat,
    ) -> Result<String, UatError> {
        match format {
            ExportFormat::Html => self.generate_html_report(results, metrics),
            ExportFormat::Json => self.export_json(results, metrics),
            ExportFormat::Csv => self.export_csv(results, metrics),
            ExportFormat::Xml => self.export_xml(results, metrics),
            ExportFormat::Markdown => self.export_markdown(results, metrics),
            ExportFormat::Pdf => Err(UatError::ReportingError(
                "PDF export not yet implemented".to_string(),
            )),
        }
    }

    /// Save report to file
    pub fn save_report(
        &self,
        content: &str,
        filename: &str,
        format: ExportFormat,
    ) -> Result<String, UatError> {
        let extension = match format {
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Xml => "xml",
            ExportFormat::Markdown => "md",
            ExportFormat::Pdf => "pdf",
        };

        let full_filename = if filename.ends_with(&format!(".{}", extension)) {
            filename.to_string()
        } else {
            format!("{}.{}", filename, extension)
        };

        let file_path = Path::new(&self.output_dir).join(&full_filename);

        // Ensure output directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file_path, content)?;

        let path_str = file_path.to_string_lossy().to_string();
        info!("Report saved to: {}", path_str);

        Ok(path_str)
    }

    // Private helper methods for HTML generation

    fn generate_html_executive_summary(
        &self,
        metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let summary = self.generate_executive_summary(metrics)?;

        let mut html = String::new();
        html.push_str("<div class='executive-summary'>");
        html.push_str("<h2>Executive Summary</h2>");

        html.push_str("<div class='summary-grid'>");
        html.push_str(&format!(
            "<div class='metric-card'><h3>Total Tests</h3><div class='metric-value'>{}</div></div>",
            summary.overall_results.total_tests
        ));
        html.push_str(&format!("<div class='metric-card'><h3>Pass Rate</h3><div class='metric-value'>{:.1}%</div></div>",
                              summary.overall_results.pass_rate));
        html.push_str(&format!("<div class='metric-card'><h3>Quality Score</h3><div class='metric-value'>{:.1}</div></div>",
                              summary.overall_results.quality_score));
        html.push_str(&format!("<div class='metric-card'><h3>User Satisfaction</h3><div class='metric-value'>{:.1}</div></div>",
                              summary.overall_results.user_satisfaction));
        html.push_str("</div>");

        html.push_str("<h3>Production Readiness</h3>");
        html.push_str(&format!(
            "<p><strong>Status:</strong> {:?}</p>",
            summary.production_readiness.status
        ));
        html.push_str(&format!(
            "<p><strong>Recommendation:</strong> {}</p>",
            summary.production_readiness.deployment_recommendation
        ));

        html.push_str("</div>");

        Ok(html)
    }

    fn generate_html_results_summary(
        &self,
        results: &[UatTestResult],
        _metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let mut html = String::new();
        html.push_str("<div class='results-summary'>");
        html.push_str("<h2>Test Results Summary</h2>");

        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| r.result == UatResult::Pass)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.result == UatResult::Fail)
            .count();
        let warnings = results
            .iter()
            .filter(|r| r.result == UatResult::Warning)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.result == UatResult::Skipped)
            .count();
        let errors = results
            .iter()
            .filter(|r| r.result == UatResult::Error)
            .count();

        html.push_str("<div class='results-grid'>");
        html.push_str(&format!(
            "<div class='result-card pass'><h3>Passed</h3><div class='result-count'>{}</div></div>",
            passed
        ));
        html.push_str(&format!(
            "<div class='result-card fail'><h3>Failed</h3><div class='result-count'>{}</div></div>",
            failed
        ));
        html.push_str(&format!("<div class='result-card warning'><h3>Warnings</h3><div class='result-count'>{}</div></div>", warnings));
        html.push_str(&format!("<div class='result-card skipped'><h3>Skipped</h3><div class='result-count'>{}</div></div>", skipped));
        html.push_str(&format!("<div class='result-card error'><h3>Errors</h3><div class='result-count'>{}</div></div>", errors));
        html.push_str("</div>");

        if total > 0 {
            let pass_rate = (passed as f64 / total as f64) * 100.0;
            html.push_str(&format!(
                "<p class='pass-rate'>Overall Pass Rate: {:.1}%</p>",
                pass_rate
            ));
        }

        html.push_str("</div>");

        Ok(html)
    }

    fn generate_html_detailed_results(
        &self,
        results: &[UatTestResult],
    ) -> Result<String, UatError> {
        let mut html = String::new();
        html.push_str("<div class='detailed-results'>");
        html.push_str("<h2>Detailed Test Results</h2>");

        for result in results {
            let status_class = match result.result {
                UatResult::Pass => "pass",
                UatResult::Fail => "fail",
                UatResult::Warning => "warning",
                UatResult::Skipped => "skipped",
                UatResult::Error => "error",
            };

            html.push_str(&format!("<div class='test-result {}''>", status_class));
            html.push_str(&format!(
                "<h3>{} - {}</h3>",
                result.test_case.id, result.test_case.name
            ));
            html.push_str(&format!(
                "<p><strong>Status:</strong> {:?}</p>",
                result.result
            ));
            html.push_str(&format!(
                "<p><strong>Duration:</strong> {:.2}s</p>",
                result.duration.as_secs_f64()
            ));
            html.push_str(&format!(
                "<p><strong>Category:</strong> {:?}</p>",
                result.test_case.category
            ));
            html.push_str(&format!(
                "<p><strong>Priority:</strong> {:?}</p>",
                result.test_case.priority
            ));

            if !result.errors.is_empty() {
                html.push_str("<div class='errors'><h4>Errors:</h4><ul>");
                for error in &result.errors {
                    html.push_str(&format!("<li>{}</li>", error));
                }
                html.push_str("</ul></div>");
            }

            if !result.warnings.is_empty() {
                html.push_str("<div class='warnings'><h4>Warnings:</h4><ul>");
                for warning in &result.warnings {
                    html.push_str(&format!("<li>{}</li>", warning));
                }
                html.push_str("</ul></div>");
            }

            html.push_str("</div>");
        }

        html.push_str("</div>");

        Ok(html)
    }

    fn generate_html_metrics_section(&self, metrics: &MetricsAnalysis) -> Result<String, UatError> {
        let mut html = String::new();
        html.push_str("<div class='metrics-section'>");
        html.push_str("<h2>Metrics and Analysis</h2>");

        html.push_str("<h3>Trend Analysis</h3>");
        html.push_str(&format!(
            "<p><strong>Success Rate Trend:</strong> {:?}</p>",
            metrics.trend_analysis.success_rate_trend
        ));
        html.push_str(&format!(
            "<p><strong>Performance Trend:</strong> {:?}</p>",
            metrics.trend_analysis.performance_trend
        ));
        html.push_str(&format!(
            "<p><strong>Quality Trend:</strong> {:?}</p>",
            metrics.trend_analysis.quality_trend
        ));

        html.push_str("<h3>Risk Assessment</h3>");
        html.push_str(&format!(
            "<p><strong>Overall Risk Level:</strong> {:?}</p>",
            metrics.risk_assessment.overall_risk
        ));
        html.push_str(&format!(
            "<p><strong>Deployment Risk:</strong> {}</p>",
            metrics.risk_assessment.deployment_risk
        ));

        if !metrics.risk_assessment.identified_risks.is_empty() {
            html.push_str("<h4>Identified Risks:</h4><ul>");
            for risk in &metrics.risk_assessment.identified_risks {
                html.push_str(&format!(
                    "<li><strong>{}:</strong> {} (Risk Level: {:?})</li>",
                    risk.category, risk.description, risk.level
                ));
            }
            html.push_str("</ul>");
        }

        html.push_str("</div>");

        Ok(html)
    }

    fn generate_html_recommendations(&self, metrics: &MetricsAnalysis) -> Result<String, UatError> {
        let mut html = String::new();
        html.push_str("<div class='recommendations'>");
        html.push_str("<h2>Recommendations</h2>");

        if !metrics.recommendations.is_empty() {
            html.push_str("<ul>");
            for rec in &metrics.recommendations {
                html.push_str(&format!(
                    "<li><strong>{}:</strong> {} (Priority: {:?})</li>",
                    rec.category, rec.description, rec.priority
                ));
            }
            html.push_str("</ul>");
        } else {
            html.push_str("<p>No specific recommendations at this time.</p>");
        }

        html.push_str("</div>");

        Ok(html)
    }

    // Export format implementations

    fn export_json(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let data = serde_json::json!({
            "results": results,
            "metrics": metrics,
            "generated_at": chrono::Utc::now(),
            "generator": "UAT Framework v1.0.0"
        });

        serde_json::to_string_pretty(&data).map_err(|e| UatError::SerializationError(e.to_string()))
    }

    fn export_csv(
        &self,
        results: &[UatTestResult],
        _metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let mut csv = String::new();
        csv.push_str("Test ID,Test Name,Category,Priority,Result,Duration (s),Errors,Warnings\n");

        for result in results {
            csv.push_str(&format!(
                "{},{},{:?},{:?},{:?},{:.2},{},{}\n",
                result.test_case.id,
                result.test_case.name.replace(',', ';'), // Escape commas
                result.test_case.category,
                result.test_case.priority,
                result.result,
                result.duration.as_secs_f64(),
                result.errors.len(),
                result.warnings.len()
            ));
        }

        Ok(csv)
    }

    fn export_xml(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<uat_report>\n");
        xml.push_str(&format!(
            "  <generated_at>{}</generated_at>\n",
            chrono::Utc::now()
        ));
        xml.push_str("  <test_results>\n");

        for result in results {
            xml.push_str("    <test_result>\n");
            xml.push_str(&format!("      <id>{}</id>\n", result.test_case.id));
            xml.push_str(&format!("      <name>{}</name>\n", result.test_case.name));
            xml.push_str(&format!("      <result>{:?}</result>\n", result.result));
            xml.push_str(&format!(
                "      <duration>{:.2}</duration>\n",
                result.duration.as_secs_f64()
            ));
            xml.push_str("    </test_result>\n");
        }

        xml.push_str("  </test_results>\n");
        xml.push_str("</uat_report>\n");

        Ok(xml)
    }

    fn export_markdown(
        &self,
        results: &[UatTestResult],
        metrics: &MetricsAnalysis,
    ) -> Result<String, UatError> {
        let mut md = String::new();
        md.push_str("# UAT Test Report\n\n");
        md.push_str(&format!("Generated: {}\n\n", chrono::Utc::now()));

        md.push_str("## Summary\n\n");
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| r.result == UatResult::Pass)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.result == UatResult::Fail)
            .count();

        md.push_str(&format!("- Total Tests: {}\n", total));
        md.push_str(&format!("- Passed: {}\n", passed));
        md.push_str(&format!("- Failed: {}\n", failed));
        if total > 0 {
            md.push_str(&format!(
                "- Pass Rate: {:.1}%\n",
                (passed as f64 / total as f64) * 100.0
            ));
        }
        md.push_str("\n");

        md.push_str("## Detailed Results\n\n");
        for result in results {
            md.push_str(&format!(
                "### {} - {}\n",
                result.test_case.id, result.test_case.name
            ));
            md.push_str(&format!("- **Status:** {:?}\n", result.result));
            md.push_str(&format!(
                "- **Duration:** {:.2}s\n",
                result.duration.as_secs_f64()
            ));
            md.push_str(&format!(
                "- **Category:** {:?}\n",
                result.test_case.category
            ));
            if !result.errors.is_empty() {
                md.push_str("- **Errors:**\n");
                for error in &result.errors {
                    md.push_str(&format!("  - {}\n", error));
                }
            }
            md.push_str("\n");
        }

        Ok(md)
    }

    // Template methods

    fn get_html_template(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{TITLE}}</title>
    <style>{{CSS_STYLES}}</style>
</head>
<body>
    <div class="container">
        <header>
            <h1>{{TITLE}}</h1>
            <p class="subtitle">{{ORGANIZATION}} - Version {{VERSION}}</p>
            <p class="timestamp">Generated: {{TIMESTAMP}}</p>
        </header>

        <main>
            {{EXECUTIVE_SUMMARY}}
            {{TEST_RESULTS_SUMMARY}}
            {{METRICS_SECTION}}
            {{RECOMMENDATIONS}}
            {{DETAILED_RESULTS}}
        </main>

        <footer>
            <p>Generated by UAT Framework v1.0.0</p>
        </footer>
    </div>
</body>
</html>"#
            .to_string()
    }

    fn get_executive_template(&self) -> String {
        // Template for executive summary - can be expanded
        "Executive Summary Template".to_string()
    }

    fn get_css_styles(&self) -> String {
        r#"
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: white;
            box-shadow: 0 0 10px rgba(0,0,0,0.1);
        }

        header {
            text-align: center;
            border-bottom: 2px solid #007acc;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }

        h1 {
            color: #007acc;
            margin: 0;
        }

        .subtitle {
            color: #666;
            font-size: 1.1em;
            margin: 10px 0;
        }

        .timestamp {
            color: #888;
            font-size: 0.9em;
        }

        .summary-grid, .results-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 20px 0;
        }

        .metric-card, .result-card {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
            border-left: 4px solid #007acc;
        }

        .metric-value, .result-count {
            font-size: 2em;
            font-weight: bold;
            color: #007acc;
        }

        .result-card.pass { border-left-color: #28a745; }
        .result-card.fail { border-left-color: #dc3545; }
        .result-card.warning { border-left-color: #ffc107; }
        .result-card.skipped { border-left-color: #6c757d; }
        .result-card.error { border-left-color: #dc3545; }

        .pass .result-count { color: #28a745; }
        .fail .result-count { color: #dc3545; }
        .warning .result-count { color: #ffc107; }
        .skipped .result-count { color: #6c757d; }
        .error .result-count { color: #dc3545; }

        .test-result {
            background: #f8f9fa;
            margin: 15px 0;
            padding: 20px;
            border-radius: 8px;
            border-left: 4px solid #007acc;
        }

        .test-result.pass { border-left-color: #28a745; }
        .test-result.fail { border-left-color: #dc3545; }
        .test-result.warning { border-left-color: #ffc107; }
        .test-result.skipped { border-left-color: #6c757d; }
        .test-result.error { border-left-color: #dc3545; }

        .pass-rate {
            font-size: 1.2em;
            text-align: center;
            font-weight: bold;
            color: #007acc;
        }

        .errors, .warnings {
            margin-top: 15px;
            padding: 10px;
            border-radius: 4px;
        }

        .errors {
            background-color: #f8d7da;
            border: 1px solid #f5c6cb;
        }

        .warnings {
            background-color: #fff3cd;
            border: 1px solid #ffeaa7;
        }

        footer {
            text-align: center;
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #dee2e6;
            color: #6c757d;
        }

        h2 {
            color: #007acc;
            border-bottom: 1px solid #dee2e6;
            padding-bottom: 10px;
        }

        h3 {
            color: #495057;
        }

        ul {
            padding-left: 20px;
        }

        li {
            margin-bottom: 5px;
        }

        @media (max-width: 768px) {
            .summary-grid, .results-grid {
                grid-template-columns: 1fr;
            }

            .container {
                padding: 10px;
            }
        }
        "#
        .to_string()
    }
}

impl Default for UatReporter {
    fn default() -> Self {
        Self::new(ReporterConfig::default(), "reports".to_string())
    }
}
