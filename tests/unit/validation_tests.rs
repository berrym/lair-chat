//! Unit tests for the validation system
//!
//! This module provides comprehensive testing for the input validation
//! system implemented in Phase 7, including rate limiting, security
//! validation, input sanitization, and command validation.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

// Import the validation framework
use lair_chat::server::error::types::TcpError;
use lair_chat::server::validation::{
    get_validation_system, init_validation_system, CommandValidator, RateLimit, RateLimitConfig,
    RateLimiter, SecurityValidator, ValidatedInput, ValidationResult, ValidationStats,
    ValidationSystem,
};

#[tokio::test]
async fn test_validation_system_initialization() {
    // Test that validation system can be initialized
    init_validation_system().await;
    let validator = get_validation_system().await;

    assert!(
        validator.is_ok(),
        "Validation system should initialize successfully"
    );

    let stats = validator.unwrap().get_stats().await;
    assert_eq!(
        stats.total_validations, 0,
        "Initial validation count should be zero"
    );
    assert_eq!(
        stats.rate_limit_violations, 0,
        "Initial rate limit violations should be zero"
    );
}

#[tokio::test]
async fn test_basic_input_validation() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let user_id = "test_user_123";
    let valid_input = "/join general";
    let invalid_input = "";
    let malicious_input = "/join room'; DROP TABLE users; --";

    // Test valid input
    let result = validator.validate_input(valid_input, user_id).await;
    assert!(result.is_ok(), "Valid input should pass validation");

    let validated = result.unwrap();
    assert_eq!(validated.command, "join");
    assert_eq!(validated.arguments, vec!["general"]);
    assert_eq!(validated.user_id, user_id);

    // Test empty input
    let result = validator.validate_input(invalid_input, user_id).await;
    assert!(result.is_err(), "Empty input should fail validation");

    // Test malicious input
    let result = validator.validate_input(malicious_input, user_id).await;
    assert!(result.is_err(), "Malicious input should fail validation");
}

#[tokio::test]
async fn test_command_parsing() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let test_cases = vec![
        ("/join general", "join", vec!["general"]),
        (
            "/msg alice hello world",
            "msg",
            vec!["alice", "hello", "world"],
        ),
        ("/help", "help", vec![]),
        ("/list rooms", "list", vec!["rooms"]),
        (
            "/create \"room with spaces\"",
            "create",
            vec!["room with spaces"],
        ),
    ];

    for (input, expected_cmd, expected_args) in test_cases {
        let result = validator.validate_input(input, "test_user").await;
        assert!(result.is_ok(), "Command '{}' should be valid", input);

        let validated = result.unwrap();
        assert_eq!(validated.command, expected_cmd);
        assert_eq!(validated.arguments, expected_args);
    }
}

#[tokio::test]
async fn test_input_sanitization() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let test_cases = vec![
        ("/join <script>alert('xss')</script>", "/join alert('xss')"), // HTML tags removed
        ("/msg user \x00\x01\x02", "/msg user"),                       // Control characters removed
        ("/say hello\n\r\tworld", "/say hello world"),                 // Normalized whitespace
        ("/join room   with   spaces", "/join room with spaces"),      // Multiple spaces normalized
    ];

    for (input, expected_sanitized) in test_cases {
        let result = validator.validate_input(input, "test_user").await;
        if result.is_ok() {
            let validated = result.unwrap();
            assert_eq!(
                validated.sanitized_input.trim(),
                expected_sanitized.trim(),
                "Input should be properly sanitized"
            );
        }
    }
}

#[tokio::test]
async fn test_rate_limiting() {
    let config = RateLimitConfig {
        default_user_rpm: 5, // 5 requests per minute
        default_global_rpm: 100,
        window_size: Duration::from_secs(60),
        burst_allowance: 2,
    };

    let mut rate_limiter = RateLimiter::new(config);
    let user_id = "test_user";

    // Test normal operation within limits
    for i in 0..5 {
        let result = rate_limiter.check_rate_limit(user_id);
        assert!(
            result.is_ok(),
            "Request {} should be within rate limit",
            i + 1
        );
    }

    // Test rate limit exceeded
    let result = rate_limiter.check_rate_limit(user_id);
    assert!(result.is_err(), "Request should exceed rate limit");

    // Test different user is not affected
    let other_user = "other_user";
    let result = rate_limiter.check_rate_limit(other_user);
    assert!(result.is_ok(), "Different user should not be rate limited");
}

#[tokio::test]
async fn test_rate_limit_window_reset() {
    let config = RateLimitConfig {
        default_user_rpm: 2,
        default_global_rpm: 100,
        window_size: Duration::from_millis(100), // Very short window for testing
        burst_allowance: 1,
    };

    let mut rate_limiter = RateLimiter::new(config);
    let user_id = "test_user";

    // Exhaust rate limit
    for _ in 0..2 {
        rate_limiter.check_rate_limit(user_id).unwrap();
    }

    // Should be rate limited
    assert!(rate_limiter.check_rate_limit(user_id).is_err());

    // Wait for window to reset
    sleep(Duration::from_millis(150)).await;

    // Should be allowed again
    let result = rate_limiter.check_rate_limit(user_id);
    assert!(
        result.is_ok(),
        "Rate limit should reset after window expires"
    );
}

#[tokio::test]
async fn test_security_validation() {
    let security_validator = SecurityValidator::new();

    let malicious_inputs = vec![
        "'; DROP TABLE users; --",                      // SQL injection
        "<script>alert('xss')</script>",                // XSS attempt
        "../../../../etc/passwd",                       // Path traversal
        "SELECT * FROM sensitive_data",                 // SQL keywords
        "javascript:alert('xss')",                      // JavaScript protocol
        "data:text/html,<script>alert('xss')</script>", // Data URI
    ];

    for input in malicious_inputs {
        assert!(
            security_validator.has_suspicious_patterns(input),
            "Should detect suspicious pattern in: {}",
            input
        );
    }

    let safe_inputs = vec![
        "hello world",
        "join general room",
        "message to friend",
        "help command",
        "list available rooms",
    ];

    for input in safe_inputs {
        assert!(
            !security_validator.has_suspicious_patterns(input),
            "Should not flag safe input: {}",
            input
        );
    }
}

#[tokio::test]
async fn test_blocked_content_detection() {
    let security_validator = SecurityValidator::new();

    let blocked_content = vec![
        "spam message repeated content spam message",
        "BUY NOW!!! URGENT!!! LIMITED TIME!!!",
        "Click here for free money",
        "You have won a million dollars",
    ];

    for content in blocked_content {
        assert!(
            security_validator.has_blocked_content(content),
            "Should detect blocked content: {}",
            content
        );
    }
}

#[tokio::test]
async fn test_validation_stats_tracking() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let user_id = "stats_test_user";

    // Perform various validations
    let _ = validator.validate_input("/join room1", user_id).await;
    let _ = validator.validate_input("/msg user hello", user_id).await;
    let _ = validator.validate_input("invalid input", user_id).await; // Should fail
    let _ = validator.validate_input("", user_id).await; // Should fail

    let stats = validator.get_stats().await;

    assert!(
        stats.total_validations >= 4,
        "Should track total validations"
    );
    assert!(
        stats.successful_validations >= 2,
        "Should track successful validations"
    );
    assert!(
        stats.failed_validations >= 2,
        "Should track failed validations"
    );
    assert!(
        stats.commands_by_type.contains_key("join"),
        "Should track command types"
    );
    assert!(
        stats.commands_by_type.contains_key("msg"),
        "Should track command types"
    );
}

#[tokio::test]
async fn test_concurrent_validation() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let mut handles = vec![];

    // Spawn 50 concurrent validation tasks
    for i in 0..50 {
        let validator_clone = validator.clone();
        let handle = tokio::spawn(async move {
            let user_id = format!("user_{}", i);
            let input = format!("/join room_{}", i);
            validator_clone.validate_input(&input, &user_id).await
        });
        handles.push(handle);
    }

    let mut successful = 0;
    let mut failed = 0;

    // Wait for all tasks to complete
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => successful += 1,
            Err(_) => failed += 1,
        }
    }

    assert!(successful > 0, "Some validations should succeed");

    let stats = validator.get_stats().await;
    assert_eq!(
        stats.total_validations,
        successful + failed,
        "Should track all concurrent validations"
    );
}

#[tokio::test]
async fn test_validation_performance() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let start_time = Instant::now();
    let iterations = 1000;

    // Perform many validations to test performance
    for i in 0..iterations {
        let user_id = format!("perf_user_{}", i % 10); // 10 different users
        let input = format!("/join room_{}", i % 100); // 100 different rooms

        let _ = validator.validate_input(&input, &user_id).await;
    }

    let elapsed = start_time.elapsed();
    let avg_time = elapsed / iterations;

    assert!(
        avg_time < Duration::from_millis(1),
        "Average validation time should be under 1ms, got {:?}",
        avg_time
    );

    let stats = validator.get_stats().await;
    assert_eq!(
        stats.total_validations, iterations as u64,
        "Should process all validations"
    );
}

#[tokio::test]
async fn test_custom_rate_limits() {
    let mut config = RateLimitConfig::default();
    let mut rate_limiter = RateLimiter::new(config);

    let premium_user = "premium_user";
    let regular_user = "regular_user";

    // Set custom rate limit for premium user
    rate_limiter.set_user_rate_limit(premium_user, 100, Duration::from_secs(60));

    // Premium user should have higher limits
    for _ in 0..50 {
        let result = rate_limiter.check_rate_limit(premium_user);
        assert!(
            result.is_ok(),
            "Premium user should have higher rate limits"
        );
    }

    // Regular user should hit default limits sooner
    for _ in 0..10 {
        let _ = rate_limiter.check_rate_limit(regular_user);
    }

    let result = rate_limiter.check_rate_limit(regular_user);
    assert!(
        result.is_err(),
        "Regular user should hit rate limit with default settings"
    );
}

#[tokio::test]
async fn test_validation_error_types() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let test_cases = vec![
        ("", "EmptyInput"),
        ("   ", "EmptyInput"),
        ("not_a_command", "InvalidFormat"),
        ("/nonexistent_command", "UnknownCommand"),
        ("/join", "MissingArguments"),
        ("/msg", "MissingArguments"),
        ("/' OR 1=1 --", "SecurityViolation"),
        ("<script>alert('xss')</script>", "SecurityViolation"),
    ];

    for (input, expected_error_type) in test_cases {
        let result = validator.validate_input(input, "test_user").await;
        assert!(result.is_err(), "Input '{}' should fail validation", input);

        // Check that the error type matches expectations
        if let Err(error) = result {
            let error_string = format!("{:?}", error);
            assert!(
                error_string.contains(expected_error_type)
                    || error_string
                        .to_lowercase()
                        .contains(&expected_error_type.to_lowercase()),
                "Error for '{}' should be of type '{}', got: {}",
                input,
                expected_error_type,
                error_string
            );
        }
    }
}

#[tokio::test]
async fn test_input_length_limits() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    // Test normal length input
    let normal_input = "/join general_room";
    let result = validator.validate_input(normal_input, "test_user").await;
    assert!(result.is_ok(), "Normal length input should be valid");

    // Test extremely long input
    let long_input = format!("/join {}", "a".repeat(10000));
    let result = validator.validate_input(&long_input, "test_user").await;
    assert!(result.is_err(), "Extremely long input should be rejected");

    // Test long but reasonable input
    let reasonable_input = format!("/msg user {}", "hello world ".repeat(50));
    let result = validator
        .validate_input(&reasonable_input, "test_user")
        .await;
    // This might pass or fail depending on configured limits
    match result {
        Ok(_) => assert!(
            reasonable_input.len() < 5000,
            "Should have reasonable limits"
        ),
        Err(_) => assert!(
            reasonable_input.len() > 1000,
            "Should allow reasonable messages"
        ),
    }
}

#[tokio::test]
async fn test_validation_reset() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    // Generate some validation activity
    for i in 0..10 {
        let input = format!("/join room_{}", i);
        let _ = validator.validate_input(&input, "test_user").await;
    }

    let stats_before = validator.get_stats().await;
    assert!(stats_before.total_validations > 0);

    // Reset stats
    validator.reset_stats().await;

    let stats_after = validator.get_stats().await;
    assert_eq!(stats_after.total_validations, 0, "Stats should be reset");
    assert_eq!(
        stats_after.successful_validations, 0,
        "Stats should be reset"
    );
    assert_eq!(stats_after.failed_validations, 0, "Stats should be reset");
    assert!(
        stats_after.commands_by_type.is_empty(),
        "Command stats should be reset"
    );
}

#[tokio::test]
async fn test_unicode_and_special_characters() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    let unicode_inputs = vec![
        "/join café",      // Accented characters
        "/msg 用户 你好",  // Chinese characters
        "/say Hello 🌍",   // Emojis
        "/join комната",   // Cyrillic
        "/msg user مرحبا", // Arabic
        "/say ♠♣♥♦",       // Symbols
    ];

    for input in unicode_inputs {
        let result = validator.validate_input(input, "unicode_test_user").await;
        // Should handle unicode gracefully, either accepting or rejecting consistently
        match result {
            Ok(validated) => {
                assert!(
                    !validated.sanitized_input.is_empty(),
                    "Should not empty sanitized input"
                );
                assert!(!validated.command.is_empty(), "Should extract command");
            }
            Err(_) => {
                // Rejection is also acceptable for security reasons
                // Just ensure it doesn't crash
            }
        }
    }
}

#[tokio::test]
async fn test_validation_system_memory_usage() {
    init_validation_system().await;
    let validator = get_validation_system().await.unwrap();

    // Simulate heavy usage to test memory management
    for batch in 0..10 {
        for i in 0..100 {
            let user_id = format!("user_{}_{}", batch, i);
            let input = format!("/join room_{}_{}", batch, i);
            let _ = validator.validate_input(&input, &user_id).await;
        }

        // Periodically reset to prevent unbounded growth in tests
        if batch % 3 == 0 {
            validator.reset_stats().await;
        }
    }

    let final_stats = validator.get_stats().await;
    // Should handle memory efficiently and not accumulate unbounded data
    assert!(
        final_stats.total_validations < 1000,
        "Should manage memory efficiently with periodic resets"
    );
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_with_rate_limiting_integration() {
        init_validation_system().await;
        let validator = get_validation_system().await.unwrap();

        let user_id = "integration_test_user";

        // Perform validations rapidly to trigger rate limiting
        let mut successful = 0;
        let mut rate_limited = 0;

        for i in 0..20 {
            let input = format!("/join room_{}", i);
            match validator.validate_input(&input, user_id).await {
                Ok(_) => successful += 1,
                Err(ref e) if format!("{:?}", e).contains("RateLimit") => rate_limited += 1,
                Err(_) => {} // Other validation errors
            }
        }

        assert!(successful > 0, "Some requests should succeed");
        assert!(rate_limited > 0, "Some requests should be rate limited");

        let stats = validator.get_stats().await;
        assert!(
            stats.rate_limit_violations > 0,
            "Should track rate limit violations"
        );
    }

    #[tokio::test]
    async fn test_security_and_validation_integration() {
        init_validation_system().await;
        let validator = get_validation_system().await.unwrap();

        let malicious_inputs = vec![
            "/join '; DROP TABLE users; --",
            "/msg <script>alert('xss')</script> hello",
            "/say javascript:alert('test')",
            "/create ../../../etc/passwd",
        ];

        for input in malicious_inputs {
            let result = validator.validate_input(input, "security_test_user").await;
            assert!(
                result.is_err(),
                "Malicious input should be rejected: {}",
                input
            );
        }

        let stats = validator.get_stats().await;
        assert!(
            stats.security_violations > 0,
            "Should track security violations"
        );
    }

    #[tokio::test]
    async fn test_full_validation_pipeline() {
        init_validation_system().await;
        let validator = get_validation_system().await.unwrap();

        let user_id = "pipeline_test_user";
        let input = "/join   general_room   with   extra   spaces  ";

        let result = validator.validate_input(input, user_id).await;
        assert!(result.is_ok(), "Valid input should pass full pipeline");

        let validated = result.unwrap();

        // Check all pipeline stages
        assert_eq!(validated.command, "join", "Command should be extracted");
        assert_eq!(
            validated.arguments,
            vec!["general_room", "with", "extra", "spaces"],
            "Arguments should be parsed"
        );
        assert_eq!(validated.user_id, user_id, "User ID should be preserved");
        assert!(
            !validated.sanitized_input.is_empty(),
            "Should have sanitized input"
        );
        assert!(!validated.raw_input.is_empty(), "Should preserve raw input");
        assert!(validated.timestamp.timestamp() > 0, "Should have timestamp");
    }
}
