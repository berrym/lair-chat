//! User Acceptance Testing Scenarios
//!
//! This module defines comprehensive test scenarios for user acceptance testing,
//! covering various user workflows, edge cases, and real-world usage patterns.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// UAT Scenarios Management
pub struct UatScenarios {
    /// Available scenarios by category
    scenarios: HashMap<TestCategory, Vec<UatScenario>>,
    /// Scenario execution history
    execution_history: Vec<ScenarioExecution>,
}

/// UAT Scenario Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatScenario {
    /// Scenario identifier
    pub id: String,
    /// Scenario name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Scenario category
    pub category: TestCategory,
    /// Target user persona
    pub persona: UserPersona,
    /// Scenario complexity level
    pub complexity: ComplexityLevel,
    /// Prerequisites for scenario
    pub prerequisites: Vec<String>,
    /// Scenario steps
    pub steps: Vec<ScenarioStep>,
    /// Expected outcomes
    pub expected_outcomes: Vec<String>,
    /// Success criteria
    pub success_criteria: Vec<String>,
    /// Estimated duration
    pub estimated_duration: Duration,
    /// Required users count
    pub required_users: usize,
    /// Test data requirements
    pub test_data: Vec<TestDataRequirement>,
}

/// Scenario complexity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityLevel {
    /// Simple single-user scenarios
    Simple,
    /// Medium complexity with multiple interactions
    Medium,
    /// Complex multi-user scenarios
    Complex,
    /// Advanced scenarios with edge cases
    Advanced,
}

/// Individual scenario step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    /// Step number
    pub step_number: usize,
    /// Step description
    pub description: String,
    /// User performing the step
    pub user: String,
    /// Expected action/behavior
    pub action: String,
    /// Expected result of the step
    pub expected_result: String,
    /// Step timeout
    pub timeout: Duration,
    /// Whether step requires manual validation
    pub manual_validation: bool,
}

/// Test data requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDataRequirement {
    /// Data type identifier
    pub data_type: String,
    /// Data description
    pub description: String,
    /// Required quantity
    pub quantity: usize,
    /// Data generation method
    pub generation_method: DataGenerationMethod,
}

/// Data generation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataGenerationMethod {
    /// Generate data automatically
    Automatic,
    /// Use predefined test data
    Predefined,
    /// Requires manual data creation
    Manual,
    /// Use existing production-like data
    ProductionLike,
}

/// Scenario execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioExecution {
    /// Execution identifier
    pub execution_id: String,
    /// Executed scenario
    pub scenario: UatScenario,
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Execution end time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Execution status
    pub status: ExecutionStatus,
    /// Step results
    pub step_results: Vec<StepResult>,
    /// Overall result
    pub result: UatResult,
    /// User feedback
    pub user_feedback: Vec<UserFeedback>,
    /// Performance metrics
    pub performance_metrics: ScenarioMetrics,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Scenario is preparing to execute
    Preparing,
    /// Scenario is currently running
    Running,
    /// Scenario is paused
    Paused,
    /// Scenario completed successfully
    Completed,
    /// Scenario failed
    Failed,
    /// Scenario was cancelled
    Cancelled,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step that was executed
    pub step: ScenarioStep,
    /// Step execution result
    pub result: UatResult,
    /// Execution duration
    pub duration: Duration,
    /// Any issues encountered
    pub issues: Vec<String>,
    /// User observations
    pub observations: Vec<String>,
}

/// User feedback for scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    /// User providing feedback
    pub user_id: String,
    /// User persona
    pub persona: UserPersona,
    /// Satisfaction rating (1-10)
    pub satisfaction_rating: f64,
    /// Ease of use rating (1-10)
    pub ease_of_use_rating: f64,
    /// Feature completeness rating (1-10)
    pub completeness_rating: f64,
    /// Overall experience rating (1-10)
    pub overall_rating: f64,
    /// Written feedback
    pub comments: String,
    /// Suggested improvements
    pub suggestions: Vec<String>,
    /// Encountered issues
    pub issues_encountered: Vec<String>,
}

/// Scenario performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioMetrics {
    /// Total execution time
    pub total_duration: Duration,
    /// Time per step
    pub step_durations: Vec<Duration>,
    /// User task completion time
    pub task_completion_times: Vec<Duration>,
    /// Error count
    pub error_count: usize,
    /// User hesitation points
    pub hesitation_points: Vec<String>,
    /// Help requests
    pub help_requests: usize,
}

impl UatScenarios {
    /// Create new scenarios manager
    pub fn new() -> Self {
        let mut scenarios = HashMap::new();
        scenarios.insert(TestCategory::Functional, Vec::new());
        scenarios.insert(TestCategory::Usability, Vec::new());
        scenarios.insert(TestCategory::Compatibility, Vec::new());
        scenarios.insert(TestCategory::Integration, Vec::new());
        scenarios.insert(TestCategory::Performance, Vec::new());

        Self {
            scenarios,
            execution_history: Vec::new(),
        }
    }

    /// Initialize with default scenarios
    pub fn with_default_scenarios() -> Self {
        let mut manager = Self::new();
        manager.load_default_scenarios();
        manager
    }

    /// Load default test scenarios
    pub fn load_default_scenarios(&mut self) {
        self.load_functional_scenarios();
        self.load_usability_scenarios();
        self.load_compatibility_scenarios();
        self.load_integration_scenarios();
        self.load_performance_scenarios();
    }

    /// Load functional testing scenarios
    fn load_functional_scenarios(&mut self) {
        let mut scenarios = Vec::new();

        // New User Onboarding Scenario
        scenarios.push(UatScenario {
            id: "FUNC-SCENARIO-001".to_string(),
            name: "New User Onboarding Journey".to_string(),
            description: "Complete new user experience from registration to first conversation"
                .to_string(),
            category: TestCategory::Functional,
            persona: UserPersona::Novice,
            complexity: ComplexityLevel::Medium,
            prerequisites: vec!["Server running".to_string(), "Clean database".to_string()],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "User discovers lair-chat application".to_string(),
                    user: "NewUser".to_string(),
                    action: "Navigate to application homepage".to_string(),
                    expected_result: "Homepage loads with clear value proposition".to_string(),
                    timeout: Duration::from_secs(10),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "User begins registration process".to_string(),
                    user: "NewUser".to_string(),
                    action: "Click registration button and fill form".to_string(),
                    expected_result: "Registration form is intuitive and clear".to_string(),
                    timeout: Duration::from_secs(30),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "User completes registration".to_string(),
                    user: "NewUser".to_string(),
                    action: "Submit registration with valid information".to_string(),
                    expected_result: "Account created successfully with confirmation".to_string(),
                    timeout: Duration::from_secs(15),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 4,
                    description: "User first login".to_string(),
                    user: "NewUser".to_string(),
                    action: "Login with newly created credentials".to_string(),
                    expected_result: "Successful login with welcome experience".to_string(),
                    timeout: Duration::from_secs(20),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 5,
                    description: "User explores interface".to_string(),
                    user: "NewUser".to_string(),
                    action: "Navigate through main interface sections".to_string(),
                    expected_result: "Interface is intuitive and discoverable".to_string(),
                    timeout: Duration::from_secs(60),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 6,
                    description: "User joins first room".to_string(),
                    user: "NewUser".to_string(),
                    action: "Discover and join a public room".to_string(),
                    expected_result: "Room joining process is straightforward".to_string(),
                    timeout: Duration::from_secs(30),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 7,
                    description: "User sends first message".to_string(),
                    user: "NewUser".to_string(),
                    action: "Compose and send first message in room".to_string(),
                    expected_result: "Message sending is intuitive and immediate".to_string(),
                    timeout: Duration::from_secs(15),
                    manual_validation: false,
                },
            ],
            expected_outcomes: vec![
                "User successfully creates account".to_string(),
                "User can navigate interface confidently".to_string(),
                "User can participate in chat conversations".to_string(),
            ],
            success_criteria: vec![
                "Registration completes without confusion".to_string(),
                "User finds interface intuitive".to_string(),
                "First message sent within 10 minutes".to_string(),
            ],
            estimated_duration: Duration::from_secs(600), // 10 minutes
            required_users: 1,
            test_data: vec![TestDataRequirement {
                data_type: "public_rooms".to_string(),
                description: "Pre-existing public rooms for joining".to_string(),
                quantity: 3,
                generation_method: DataGenerationMethod::Automatic,
            }],
        });

        // Multi-User Chat Scenario
        scenarios.push(UatScenario {
            id: "FUNC-SCENARIO-002".to_string(),
            name: "Multi-User Chat Conversation".to_string(),
            description: "Multiple users engaging in real-time conversation".to_string(),
            category: TestCategory::Functional,
            persona: UserPersona::Regular,
            complexity: ComplexityLevel::Complex,
            prerequisites: vec![
                "Multiple user accounts".to_string(),
                "Shared chat room".to_string(),
            ],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Users join common room".to_string(),
                    user: "All".to_string(),
                    action: "All users join the same chat room".to_string(),
                    expected_result: "All users appear in room member list".to_string(),
                    timeout: Duration::from_secs(30),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Initiate conversation".to_string(),
                    user: "User1".to_string(),
                    action: "Send greeting message to room".to_string(),
                    expected_result: "Message appears for all users immediately".to_string(),
                    timeout: Duration::from_secs(5),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Multiple responses".to_string(),
                    user: "User2,User3".to_string(),
                    action: "Users respond to greeting simultaneously".to_string(),
                    expected_result: "All messages appear in correct order".to_string(),
                    timeout: Duration::from_secs(10),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 4,
                    description: "Rapid conversation flow".to_string(),
                    user: "All".to_string(),
                    action: "Users engage in rapid back-and-forth conversation".to_string(),
                    expected_result: "Real-time message flow without delays".to_string(),
                    timeout: Duration::from_secs(120),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 5,
                    description: "Special content testing".to_string(),
                    user: "User1".to_string(),
                    action: "Send messages with emojis, links, and formatting".to_string(),
                    expected_result: "Special content renders correctly for all users".to_string(),
                    timeout: Duration::from_secs(30),
                    manual_validation: true,
                },
            ],
            expected_outcomes: vec![
                "Seamless multi-user conversation".to_string(),
                "Real-time message synchronization".to_string(),
                "Proper message ordering".to_string(),
            ],
            success_criteria: vec![
                "Messages appear within 1 second".to_string(),
                "Message order is preserved".to_string(),
                "All users see identical conversation".to_string(),
            ],
            estimated_duration: Duration::from_secs(300), // 5 minutes
            required_users: 3,
            test_data: vec![TestDataRequirement {
                data_type: "test_messages".to_string(),
                description: "Variety of test message content".to_string(),
                quantity: 20,
                generation_method: DataGenerationMethod::Predefined,
            }],
        });

        // Direct Messaging Scenario
        scenarios.push(UatScenario {
            id: "FUNC-SCENARIO-003".to_string(),
            name: "Private Direct Messaging".to_string(),
            description: "One-on-one private messaging functionality".to_string(),
            category: TestCategory::Functional,
            persona: UserPersona::Regular,
            complexity: ComplexityLevel::Medium,
            prerequisites: vec!["Two user accounts".to_string()],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Initiate direct message".to_string(),
                    user: "User1".to_string(),
                    action: "Start new direct message with User2".to_string(),
                    expected_result: "DM interface opens correctly".to_string(),
                    timeout: Duration::from_secs(15),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Send private message".to_string(),
                    user: "User1".to_string(),
                    action: "Send first private message".to_string(),
                    expected_result: "Message delivered to recipient only".to_string(),
                    timeout: Duration::from_secs(10),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Receive and respond".to_string(),
                    user: "User2".to_string(),
                    action: "Receive notification and respond to DM".to_string(),
                    expected_result: "Notification received, response sent".to_string(),
                    timeout: Duration::from_secs(20),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 4,
                    description: "Verify privacy".to_string(),
                    user: "User3".to_string(),
                    action: "Verify DM content is not visible to other users".to_string(),
                    expected_result: "Private messages remain private".to_string(),
                    timeout: Duration::from_secs(15),
                    manual_validation: true,
                },
            ],
            expected_outcomes: vec![
                "Private messaging works reliably".to_string(),
                "Message privacy is maintained".to_string(),
                "Notifications work correctly".to_string(),
            ],
            success_criteria: vec![
                "DM delivery is immediate".to_string(),
                "Privacy is enforced".to_string(),
                "Notifications are timely".to_string(),
            ],
            estimated_duration: Duration::from_secs(180), // 3 minutes
            required_users: 3,
            test_data: vec![],
        });

        self.scenarios.insert(TestCategory::Functional, scenarios);
    }

    /// Load usability testing scenarios
    fn load_usability_scenarios(&mut self) {
        let mut scenarios = Vec::new();

        // User Interface Navigation Scenario
        scenarios.push(UatScenario {
            id: "USAB-SCENARIO-001".to_string(),
            name: "Interface Navigation Assessment".to_string(),
            description: "Evaluate user interface intuitiveness and navigation efficiency"
                .to_string(),
            category: TestCategory::Usability,
            persona: UserPersona::Novice,
            complexity: ComplexityLevel::Medium,
            prerequisites: vec!["User logged in".to_string()],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Explore main navigation".to_string(),
                    user: "TestUser".to_string(),
                    action: "Navigate through all main interface sections".to_string(),
                    expected_result: "User can find all major features intuitively".to_string(),
                    timeout: Duration::from_secs(180),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Task completion assessment".to_string(),
                    user: "TestUser".to_string(),
                    action: "Complete common tasks without guidance".to_string(),
                    expected_result: "Tasks completed efficiently".to_string(),
                    timeout: Duration::from_secs(300),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Feature discovery".to_string(),
                    user: "TestUser".to_string(),
                    action: "Discover advanced features through exploration".to_string(),
                    expected_result: "Advanced features are discoverable".to_string(),
                    timeout: Duration::from_secs(240),
                    manual_validation: true,
                },
            ],
            expected_outcomes: vec![
                "Interface is intuitive for new users".to_string(),
                "Common tasks are easily discoverable".to_string(),
                "User can work efficiently".to_string(),
            ],
            success_criteria: vec![
                "90% of tasks completed without help".to_string(),
                "Average task completion time under 2 minutes".to_string(),
                "User satisfaction rating above 7/10".to_string(),
            ],
            estimated_duration: Duration::from_secs(720), // 12 minutes
            required_users: 1,
            test_data: vec![TestDataRequirement {
                data_type: "usability_tasks".to_string(),
                description: "Predefined usability tasks for testing".to_string(),
                quantity: 10,
                generation_method: DataGenerationMethod::Predefined,
            }],
        });

        // Error Handling and Recovery Scenario
        scenarios.push(UatScenario {
            id: "USAB-SCENARIO-002".to_string(),
            name: "Error Handling User Experience".to_string(),
            description: "Evaluate how users handle and recover from errors".to_string(),
            category: TestCategory::Usability,
            persona: UserPersona::Regular,
            complexity: ComplexityLevel::Advanced,
            prerequisites: vec!["User logged in".to_string()],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Trigger validation errors".to_string(),
                    user: "TestUser".to_string(),
                    action: "Intentionally trigger form validation errors".to_string(),
                    expected_result: "Clear, helpful error messages appear".to_string(),
                    timeout: Duration::from_secs(60),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Network interruption simulation".to_string(),
                    user: "TestUser".to_string(),
                    action: "Simulate network connectivity issues".to_string(),
                    expected_result: "Graceful handling with user feedback".to_string(),
                    timeout: Duration::from_secs(90),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Recovery process".to_string(),
                    user: "TestUser".to_string(),
                    action: "Follow error recovery suggestions".to_string(),
                    expected_result: "User can successfully recover from errors".to_string(),
                    timeout: Duration::from_secs(120),
                    manual_validation: true,
                },
            ],
            expected_outcomes: vec![
                "Error messages are clear and actionable".to_string(),
                "Recovery paths are obvious".to_string(),
                "User maintains confidence after errors".to_string(),
            ],
            success_criteria: vec![
                "Error messages rated as helpful".to_string(),
                "Recovery success rate above 90%".to_string(),
                "User frustration remains low".to_string(),
            ],
            estimated_duration: Duration::from_secs(450), // 7.5 minutes
            required_users: 1,
            test_data: vec![],
        });

        self.scenarios.insert(TestCategory::Usability, scenarios);
    }

    /// Load compatibility testing scenarios
    fn load_compatibility_scenarios(&mut self) {
        let mut scenarios = Vec::new();

        // Cross-Platform Functionality Scenario
        scenarios.push(UatScenario {
            id: "COMPAT-SCENARIO-001".to_string(),
            name: "Cross-Platform Feature Parity".to_string(),
            description: "Verify consistent functionality across different platforms".to_string(),
            category: TestCategory::Compatibility,
            persona: UserPersona::Expert,
            complexity: ComplexityLevel::Complex,
            prerequisites: vec![
                "Multiple platform access".to_string(),
                "Same user account".to_string(),
            ],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Test core features on each platform".to_string(),
                    user: "TestUser".to_string(),
                    action: "Execute same tasks on Windows, macOS, and Linux".to_string(),
                    expected_result: "Consistent behavior across platforms".to_string(),
                    timeout: Duration::from_secs(600),
                    manual_validation: true,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Performance comparison".to_string(),
                    user: "TestUser".to_string(),
                    action: "Compare response times and resource usage".to_string(),
                    expected_result: "Performance within acceptable variance".to_string(),
                    timeout: Duration::from_secs(300),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "UI consistency check".to_string(),
                    user: "TestUser".to_string(),
                    action: "Compare user interface rendering and layout".to_string(),
                    expected_result: "Consistent visual appearance".to_string(),
                    timeout: Duration::from_secs(240),
                    manual_validation: true,
                },
            ],
            expected_outcomes: vec![
                "Feature parity across platforms".to_string(),
                "Consistent performance characteristics".to_string(),
                "Uniform user experience".to_string(),
            ],
            success_criteria: vec![
                "100% feature availability on all platforms".to_string(),
                "Performance variance less than 20%".to_string(),
                "UI consistency rating above 9/10".to_string(),
            ],
            estimated_duration: Duration::from_secs(1140), // 19 minutes
            required_users: 1,
            test_data: vec![],
        });

        self.scenarios
            .insert(TestCategory::Compatibility, scenarios);
    }

    /// Load integration testing scenarios
    fn load_integration_scenarios(&mut self) {
        let mut scenarios = Vec::new();

        // End-to-End Integration Scenario
        scenarios.push(UatScenario {
            id: "INTEG-SCENARIO-001".to_string(),
            name: "Complete System Integration".to_string(),
            description: "Test full system integration from user perspective".to_string(),
            category: TestCategory::Integration,
            persona: UserPersona::Expert,
            complexity: ComplexityLevel::Advanced,
            prerequisites: vec![
                "Full system deployment".to_string(),
                "All services running".to_string(),
            ],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "API integration validation".to_string(),
                    user: "TestUser".to_string(),
                    action: "Test all API endpoints through user interface".to_string(),
                    expected_result: "All APIs respond correctly".to_string(),
                    timeout: Duration::from_secs(180),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Database integration check".to_string(),
                    user: "TestUser".to_string(),
                    action: "Perform CRUD operations and verify persistence".to_string(),
                    expected_result: "Data persists correctly across sessions".to_string(),
                    timeout: Duration::from_secs(120),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Real-time communication validation".to_string(),
                    user: "TestUser".to_string(),
                    action: "Test WebSocket connections and real-time updates".to_string(),
                    expected_result: "Real-time features work seamlessly".to_string(),
                    timeout: Duration::from_secs(90),
                    manual_validation: false,
                },
            ],
            expected_outcomes: vec![
                "Complete system integration validated".to_string(),
                "All components work together seamlessly".to_string(),
                "Performance is acceptable under integration load".to_string(),
            ],
            success_criteria: vec![
                "All integration points functional".to_string(),
                "Data integrity maintained".to_string(),
                "Response times within SLA".to_string(),
            ],
            estimated_duration: Duration::from_secs(390), // 6.5 minutes
            required_users: 1,
            test_data: vec![],
        });

        self.scenarios.insert(TestCategory::Integration, scenarios);
    }

    /// Load performance testing scenarios
    fn load_performance_scenarios(&mut self) {
        let mut scenarios = Vec::new();

        // User Load Performance Scenario
        scenarios.push(UatScenario {
            id: "PERF-SCENARIO-001".to_string(),
            name: "Multi-User Performance Validation".to_string(),
            description: "Test system performance under realistic user load".to_string(),
            category: TestCategory::Performance,
            persona: UserPersona::Regular,
            complexity: ComplexityLevel::Complex,
            prerequisites: vec![
                "Performance monitoring enabled".to_string(),
                "Multiple user accounts".to_string(),
            ],
            steps: vec![
                ScenarioStep {
                    step_number: 1,
                    description: "Baseline performance measurement".to_string(),
                    user: "SingleUser".to_string(),
                    action: "Measure single-user performance baseline".to_string(),
                    expected_result: "Baseline metrics established".to_string(),
                    timeout: Duration::from_secs(60),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 2,
                    description: "Concurrent user simulation".to_string(),
                    user: "MultipleUsers".to_string(),
                    action: "Simulate 10 concurrent users performing typical tasks".to_string(),
                    expected_result: "System handles load gracefully".to_string(),
                    timeout: Duration::from_secs(300),
                    manual_validation: false,
                },
                ScenarioStep {
                    step_number: 3,
                    description: "Performance degradation assessment".to_string(),
                    user: "MultipleUsers".to_string(),
                    action: "Compare performance under load vs baseline".to_string(),
                    expected_result: "Performance degradation within acceptable limits".to_string(),
                    timeout: Duration::from_secs(120),
                    manual_validation: false,
                },
            ],
            expected_outcomes: vec![
                "System performs well under realistic load".to_string(),
                "Response times remain acceptable".to_string(),
                "No critical performance bottlenecks".to_string(),
            ],
            success_criteria: vec![
                "Response time increase less than 50%".to_string(),
                "No timeouts or failures under load".to_string(),
                "Resource utilization within limits".to_string(),
            ],
            estimated_duration: Duration::from_secs(480), // 8 minutes
            required_users: 10,
            test_data: vec![],
        });

        self.scenarios.insert(TestCategory::Performance, scenarios);
    }

    /// Get scenarios by category
    pub fn get_scenarios_by_category(&self, category: &TestCategory) -> Vec<&UatScenario> {
        self.scenarios
            .get(category)
            .map(|scenarios| scenarios.iter().collect())
            .unwrap_or_default()
    }

    /// Get scenarios by persona
    pub fn get_scenarios_by_persona(&self, persona: &UserPersona) -> Vec<&UatScenario> {
        self.scenarios
            .values()
            .flatten()
            .filter(|scenario| scenario.persona == *persona)
            .collect()
    }

    /// Get scenarios by complexity
    pub fn get_scenarios_by_complexity(&self, complexity: &ComplexityLevel) -> Vec<&UatScenario> {
        self.scenarios
            .values()
            .flatten()
            .filter(|scenario| scenario.complexity == *complexity)
            .collect()
    }

    /// Get all scenarios
    pub fn get_all_scenarios(&self) -> Vec<&UatScenario> {
        self.scenarios.values().flatten().collect()
    }

    /// Add custom scenario
    pub fn add_scenario(&mut self, scenario: UatScenario) {
        let category = scenario.category.clone();
        self.scenarios
            .entry(category)
            .or_insert_with(Vec::new)
            .push(scenario);
    }

    /// Execute scenario and record results
    pub async fn execute_scenario(
        &mut self,
        scenario_id: &str,
        user_count: usize,
    ) -> Result<ScenarioExecution, String> {
        // Find scenario
        let scenario = self
            .get_all_scenarios()
            .into_iter()
            .find(|s| s.id == scenario_id)
            .ok_or_else(|| format!("Scenario not found: {}", scenario_id))?
            .clone();

        let execution_id = format!("{}_{}", scenario_id, chrono::Utc::now().timestamp());
        let start_time = chrono::Utc::now();

        let mut execution = ScenarioExecution {
            execution_id: execution_id.clone(),
            scenario: scenario.clone(),
            start_time,
            end_time: None,
            status: ExecutionStatus::Running,
            step_results: Vec::new(),
            result: UatResult::Pass,
            user_feedback: Vec::new(),
            performance_metrics: ScenarioMetrics::default(),
        };

        // Execute each step
        for step in &scenario.steps {
            let step_start = std::time::Instant::now();

            // Simulate step execution
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            let step_duration = step_start.elapsed();
            let step_result = StepResult {
                step: step.clone(),
                result: UatResult::Pass,
                duration: step_duration,
                issues: Vec::new(),
                observations: Vec::new(),
            };

            execution.step_results.push(step_result);
            execution
                .performance_metrics
                .step_durations
                .push(step_duration);
        }

        execution.end_time = Some(chrono::Utc::now());
        execution.status = ExecutionStatus::Completed;
        execution.performance_metrics.total_duration =
            execution.performance_metrics.step_durations.iter().sum();

        // Store execution history
        self.execution_history.push(execution.clone());

        Ok(execution)
    }

    /// Get execution history
    pub fn get_execution_history(&self) -> &[ScenarioExecution] {
        &self.execution_history
    }

    /// Get scenario statistics
    pub fn get_scenario_statistics(&self) -> ScenarioStatistics {
        let total_scenarios = self.get_all_scenarios().len();
        let executions = &self.execution_history;

        let successful_executions = executions
            .iter()
            .filter(|e| matches!(e.status, ExecutionStatus::Completed))
            .count();

        let failed_executions = executions
            .iter()
            .filter(|e| matches!(e.status, ExecutionStatus::Failed))
            .count();

        let average_duration = if !executions.is_empty() {
            let total_duration: std::time::Duration = executions
                .iter()
                .map(|e| e.performance_metrics.total_duration)
                .sum();
            total_duration / executions.len() as u32
        } else {
            std::time::Duration::from_secs(0)
        };

        ScenarioStatistics {
            total_scenarios,
            executed_scenarios: executions.len(),
            successful_executions,
            failed_executions,
            average_execution_duration: average_duration,
            success_rate: if executions.is_empty() {
                0.0
            } else {
                successful_executions as f64 / executions.len() as f64
            },
        }
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.execution_history.clear();
    }

    /// Export scenarios to JSON
    pub fn export_scenarios(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.scenarios)
    }

    /// Import scenarios from JSON
    pub fn import_scenarios(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let imported_scenarios: HashMap<TestCategory, Vec<UatScenario>> =
            serde_json::from_str(json)?;

        for (category, scenarios) in imported_scenarios {
            self.scenarios.insert(category, scenarios);
        }

        Ok(())
    }
}

/// Scenario execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStatistics {
    /// Total number of available scenarios
    pub total_scenarios: usize,
    /// Number of executed scenarios
    pub executed_scenarios: usize,
    /// Number of successful executions
    pub successful_executions: usize,
    /// Number of failed executions
    pub failed_executions: usize,
    /// Average execution duration
    pub average_execution_duration: std::time::Duration,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

impl Default for UatScenarios {
    fn default() -> Self {
        Self::with_default_scenarios()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenarios_creation() {
        let scenarios = UatScenarios::new();
        assert_eq!(scenarios.scenarios.len(), 5); // 5 categories
        assert_eq!(scenarios.execution_history.len(), 0);
    }

    #[test]
    fn test_default_scenarios_loading() {
        let scenarios = UatScenarios::with_default_scenarios();

        // Check that scenarios were loaded for each category
        assert!(!scenarios
            .get_scenarios_by_category(&TestCategory::Functional)
            .is_empty());
        assert!(!scenarios
            .get_scenarios_by_category(&TestCategory::Usability)
            .is_empty());
        assert!(!scenarios
            .get_scenarios_by_category(&TestCategory::Compatibility)
            .is_empty());
        assert!(!scenarios
            .get_scenarios_by_category(&TestCategory::Integration)
            .is_empty());
        assert!(!scenarios
            .get_scenarios_by_category(&TestCategory::Performance)
            .is_empty());
    }

    #[test]
    fn test_scenario_filtering() {
        let scenarios = UatScenarios::with_default_scenarios();

        // Test filtering by persona
        let novice_scenarios = scenarios.get_scenarios_by_persona(&UserPersona::Novice);
        assert!(!novice_scenarios.is_empty());

        // Test filtering by complexity
        let simple_scenarios = scenarios.get_scenarios_by_complexity(&ComplexityLevel::Simple);
        let complex_scenarios = scenarios.get_scenarios_by_complexity(&ComplexityLevel::Complex);

        // All scenarios should have some complexity level
        let all_scenarios = scenarios.get_all_scenarios();
        assert!(!all_scenarios.is_empty());
    }

    #[test]
    fn test_scenario_statistics() {
        let scenarios = UatScenarios::with_default_scenarios();
        let stats = scenarios.get_scenario_statistics();

        assert!(stats.total_scenarios > 0);
        assert_eq!(stats.executed_scenarios, 0); // No executions yet
        assert_eq!(stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_scenario_execution() {
        let mut scenarios = UatScenarios::with_default_scenarios();
        let all_scenarios = scenarios.get_all_scenarios();

        if let Some(scenario) = all_scenarios.first() {
            let result = scenarios
                .execute_scenario(&scenario.id, scenario.required_users)
                .await;

            assert!(result.is_ok());

            let execution = result.unwrap();
            assert_eq!(execution.status, ExecutionStatus::Completed);
            assert!(!execution.step_results.is_empty());

            // Check that execution was recorded
            assert_eq!(scenarios.execution_history.len(), 1);
        }
    }

    #[test]
    fn test_scenario_serialization() {
        let scenarios = UatScenarios::with_default_scenarios();

        // Test export
        let json = scenarios.export_scenarios();
        assert!(json.is_ok());

        // Test import
        let mut new_scenarios = UatScenarios::new();
        let import_result = new_scenarios.import_scenarios(&json.unwrap());
        assert!(import_result.is_ok());

        // Verify scenarios were imported
        assert_eq!(
            new_scenarios.get_all_scenarios().len(),
            scenarios.get_all_scenarios().len()
        );
    }
}
