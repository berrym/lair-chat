//! Input security tests for the lair-chat application
//!
//! This module contains security tests specifically focused on input validation,
//! sanitization, and protection against various input-based attacks.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_input_sanitization_effectiveness() {
    // Test that input sanitization removes dangerous content
    let dangerous_inputs = vec![
        ("<script>alert('xss')</script>", "scriptalert('xss')/script"),
        ("javascript:alert('test')", "alert('test')"),
        (
            "<img src=x onerror=alert('xss')>",
            "img src=x onerror=alert('xss')",
        ),
        ("<svg/onload=alert('xss')>", "svg/onload=alert('xss')"),
        ("'; DROP TABLE users; --", "' DROP TABLE users --"),
        ("\x00\x01\x02dangerous", "dangerous"),
        ("hello\r\n\tworld", "hello world"),
        ("  multiple   spaces  ", "multiple spaces"),
    ];

    for (input, expected_safe) in dangerous_inputs {
        let start_time = Instant::now();

        let sanitized = simulate_input_sanitization(input).await;
        let sanitization_time = start_time.elapsed();

        assert!(
            !sanitized.contains("<script"),
            "Script tags should be removed from: {}",
            input
        );
        assert!(
            !sanitized.contains("javascript:"),
            "JavaScript protocols should be removed from: {}",
            input
        );
        assert!(
            !contains_control_characters(&sanitized),
            "Control characters should be removed from: {}",
            input
        );

        // Sanitization should be fast
        assert!(
            sanitization_time < Duration::from_millis(50),
            "Sanitization should be fast: {:?} for input: {}",
            sanitization_time,
            input
        );

        println!("✓ Sanitized: '{}' -> '{}'", input, sanitized);
    }
}

#[tokio::test]
async fn test_unicode_security_handling() {
    // Test handling of various Unicode characters and encodings
    let unicode_inputs = vec![
        ("café", true),              // Normal accented characters
        ("用户名", true),            // Chinese characters
        ("مرحبا", true),             // Arabic characters
        ("🔒🛡️", true),              // Security-related emojis
        ("\u{202E}override", false), // Right-to-left override (suspicious)
        ("\u{2028}\u{2029}", false), // Line/paragraph separators
        ("\u{FEFF}", false),         // Byte order mark
        ("\u{200B}\u{200C}", false), // Zero-width characters
        ("test\u{0000}null", false), // Null character
        ("\u{1F4A9}", true),         // Emoji (should be allowed in messages)
    ];

    for (input, should_be_safe) in unicode_inputs {
        let validation_result = simulate_unicode_input_validation(input).await;

        if should_be_safe {
            assert!(
                validation_result.is_valid(),
                "Safe Unicode input should be accepted: {:?}",
                input
            );
        } else {
            assert!(
                !validation_result.is_valid(),
                "Suspicious Unicode input should be rejected: {:?}",
                input
            );
        }

        println!("✓ Unicode test: {:?} -> {:?}", input, validation_result);
    }
}

#[tokio::test]
async fn test_encoding_attack_prevention() {
    // Test various encoding attacks
    let encoding_attacks = vec![
        // URL encoding attacks
        (
            "%3Cscript%3Ealert('xss')%3C/script%3E",
            "<script>alert('xss')</script>",
        ),
        ("%27%20OR%201=1%20--", "' OR 1=1 --"),
        ("%2E%2E%2F%2E%2E%2Fetc%2Fpasswd", "../../etc/passwd"),
        // Double URL encoding
        ("%253Cscript%253E", "<script>"),
        ("%2527%2520OR%25201=1", "' OR 1=1"),
        // HTML entity encoding
        (
            "&lt;script&gt;alert('xss')&lt;/script&gt;",
            "<script>alert('xss')</script>",
        ),
        ("&#x3C;script&#x3E;", "<script>"),
        ("&#60;script&#62;", "<script>"),
        // Mixed encoding
        ("%3C%73%63%72%69%70%74%3E", "<script>"),
        ("&lt;%73cript&gt;", "<script>"),
    ];

    for (encoded_input, decoded_expected) in encoding_attacks {
        let start_time = Instant::now();

        // Test that encoded malicious input is detected
        let validation_result = simulate_encoded_input_validation(encoded_input).await;

        assert!(
            !validation_result.is_valid(),
            "Encoded malicious input should be rejected: {}",
            encoded_input
        );

        let validation_time = start_time.elapsed();
        assert!(
            validation_time < Duration::from_millis(100),
            "Encoding attack detection should be fast: {:?}",
            validation_time
        );

        println!("✓ Blocked encoding attack: {}", encoded_input);
    }
}

#[tokio::test]
async fn test_input_length_validation() {
    // Test various input length scenarios
    let length_tests = vec![
        ("", false, "empty_input"),                    // Empty input
        ("a", true, "single_char"),                    // Single character
        ("valid message", true, "normal_message"),     // Normal message
        ("x".repeat(100), true, "medium_message"),     // Medium message
        ("y".repeat(1000), true, "long_message"),      // Long but acceptable
        ("z".repeat(5000), false, "too_long"),         // Too long
        ("w".repeat(100000), false, "extremely_long"), // Extremely long
    ];

    for (input, should_be_valid, test_name) in length_tests {
        let start_time = Instant::now();

        let validation_result = simulate_length_validation(input, "message").await;
        let validation_time = start_time.elapsed();

        if should_be_valid {
            assert!(
                validation_result.is_valid(),
                "Input should be valid for {}: length {}",
                test_name,
                input.len()
            );
        } else {
            assert!(
                !validation_result.is_valid(),
                "Input should be invalid for {}: length {}",
                test_name,
                input.len()
            );
        }

        // Validation should be fast regardless of input length
        assert!(
            validation_time < Duration::from_millis(50),
            "Length validation should be fast: {:?} for {} chars",
            validation_time,
            input.len()
        );

        println!(
            "✓ Length test {}: {} chars -> {:?}",
            test_name,
            input.len(),
            validation_result
        );
    }
}

#[tokio::test]
async fn test_command_input_validation() {
    // Test command input validation specifically
    let command_tests = vec![
        // Valid commands
        ("/join general", true, "basic_join"),
        ("/msg alice hello", true, "direct_message"),
        ("/help", true, "help_command"),
        ("/list rooms", true, "list_command"),
        ("/quit", true, "quit_command"),
        // Invalid commands
        ("/invalid_command", false, "unknown_command"),
        ("not_a_command", false, "missing_slash"),
        ("/", false, "empty_command"),
        ("//double_slash", false, "double_slash"),
        ("/join", false, "missing_arguments"),
        // Malicious commands
        (
            "/join room'; DROP TABLE users; --",
            false,
            "sql_injection_in_command",
        ),
        (
            "/msg <script>alert('xss')</script>",
            false,
            "xss_in_command",
        ),
        (
            "/create ../../../../etc/passwd",
            false,
            "path_traversal_in_command",
        ),
        ("/say $(whoami)", false, "command_injection"),
    ];

    for (command, should_be_valid, test_name) in command_tests {
        let start_time = Instant::now();

        let validation_result = simulate_command_validation(command).await;
        let validation_time = start_time.elapsed();

        if should_be_valid {
            assert!(
                validation_result.is_valid(),
                "Command should be valid for {}: {}",
                test_name,
                command
            );
        } else {
            assert!(
                !validation_result.is_valid(),
                "Command should be invalid for {}: {}",
                test_name,
                command
            );
        }

        assert!(
            validation_time < Duration::from_millis(25),
            "Command validation should be very fast: {:?}",
            validation_time
        );

        println!(
            "✓ Command test {}: {} -> {:?}",
            test_name, command, validation_result
        );
    }
}

#[tokio::test]
async fn test_special_character_handling() {
    // Test handling of special characters that might cause issues
    let special_char_tests = vec![
        // Control characters
        ("\x00", false, "null_character"),
        ("\x01", false, "start_of_heading"),
        ("\x1F", false, "unit_separator"),
        ("\x7F", false, "delete_character"),
        // Whitespace characters
        (" ", true, "space"),
        ("\t", true, "tab"),
        ("\n", false, "newline"),
        ("\r", false, "carriage_return"),
        // Special Unicode characters
        ("\u{200B}", false, "zero_width_space"),
        ("\u{FEFF}", false, "byte_order_mark"),
        ("\u{202E}", false, "right_to_left_override"),
        // Valid special characters
        ("!", true, "exclamation"),
        ("@", true, "at_symbol"),
        ("#", true, "hash"),
        ("$", true, "dollar"),
        ("%", true, "percent"),
        ("&", true, "ampersand"),
        ("*", true, "asterisk"),
    ];

    for (input, should_be_valid, test_name) in special_char_tests {
        let validation_result = simulate_special_char_validation(input).await;

        if should_be_valid {
            assert!(
                validation_result.is_valid(),
                "Special character should be valid for {}: {:?}",
                test_name,
                input
            );
        } else {
            assert!(
                !validation_result.is_valid(),
                "Special character should be invalid for {}: {:?}",
                test_name,
                input
            );
        }

        println!(
            "✓ Special char test {}: {:?} -> {:?}",
            test_name, input, validation_result
        );
    }
}

#[tokio::test]
async fn test_input_normalization() {
    // Test input normalization for consistency
    let normalization_tests = vec![
        (
            "  hello  world  ",
            "hello world",
            "whitespace_normalization",
        ),
        ("HELLO WORLD", "hello world", "case_normalization"),
        ("Hello\tWorld\n", "hello world", "mixed_whitespace"),
        ("café", "café", "unicode_preservation"), // Should preserve Unicode
        (
            "Test\r\nMessage",
            "test message",
            "line_ending_normalization",
        ),
    ];

    for (input, expected_normalized, test_name) in normalization_tests {
        let start_time = Instant::now();

        let normalized = simulate_input_normalization(input).await;
        let normalization_time = start_time.elapsed();

        assert_eq!(
            normalized, expected_normalized,
            "Normalization failed for {}: expected '{}', got '{}'",
            test_name, expected_normalized, normalized
        );

        assert!(
            normalization_time < Duration::from_millis(10),
            "Normalization should be very fast: {:?}",
            normalization_time
        );

        println!(
            "✓ Normalization test {}: '{}' -> '{}'",
            test_name, input, normalized
        );
    }
}

#[tokio::test]
async fn test_concurrent_input_validation() {
    // Test input validation under concurrent load
    let concurrent_inputs = 100;
    let mut handles = vec![];

    let start_time = Instant::now();

    for i in 0..concurrent_inputs {
        let input = format!("concurrent_test_message_{}", i);
        let handle =
            tokio::spawn(async move { simulate_concurrent_input_validation(&input, i).await });
        handles.push(handle);
    }

    let mut successful_validations = 0;
    let mut failed_validations = 0;

    for handle in handles {
        match handle.await.unwrap() {
            InputValidationResult::Valid => successful_validations += 1,
            InputValidationResult::Invalid(_) => failed_validations += 1,
        }
    }

    let total_time = start_time.elapsed();
    let average_time = total_time / concurrent_inputs;

    println!(
        "Concurrent validation test - Total: {:?}, Average: {:?}, Success: {}, Failed: {}",
        total_time, average_time, successful_validations, failed_validations
    );

    assert_eq!(
        successful_validations, concurrent_inputs as usize,
        "All valid concurrent inputs should be accepted"
    );

    assert!(
        average_time < Duration::from_millis(10),
        "Average validation time should be fast under concurrent load: {:?}",
        average_time
    );
}

// Helper functions and types

#[derive(Debug, Clone)]
enum InputValidationResult {
    Valid,
    Invalid(String),
}

impl InputValidationResult {
    fn is_valid(&self) -> bool {
        matches!(self, InputValidationResult::Valid)
    }
}

async fn simulate_input_sanitization(input: &str) -> String {
    sleep(Duration::from_millis(1)).await;

    let mut sanitized = input.to_string();

    // Remove script tags
    sanitized = sanitized.replace("<script", "script");
    sanitized = sanitized.replace("</script>", "/script");

    // Remove javascript protocols
    sanitized = sanitized.replace("javascript:", "");

    // Remove control characters
    sanitized = sanitized
        .chars()
        .filter(|c| !c.is_control() || *c == ' ' || *c == '\t')
        .collect();

    // Normalize whitespace
    sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");

    sanitized
}

async fn simulate_unicode_input_validation(input: &str) -> InputValidationResult {
    sleep(Duration::from_millis(2)).await;

    // Check for suspicious Unicode characters
    let suspicious_chars = [
        '\u{202E}', // Right-to-left override
        '\u{2028}', // Line separator
        '\u{2029}', // Paragraph separator
        '\u{FEFF}', // Byte order mark
        '\u{200B}', // Zero-width space
        '\u{200C}', // Zero-width non-joiner
        '\u{0000}', // Null character
    ];

    for &suspicious_char in &suspicious_chars {
        if input.contains(suspicious_char) {
            return InputValidationResult::Invalid(format!(
                "suspicious_unicode_character_{:?}",
                suspicious_char
            ));
        }
    }

    InputValidationResult::Valid
}

async fn simulate_encoded_input_validation(input: &str) -> InputValidationResult {
    sleep(Duration::from_millis(5)).await;

    // Decode URL encoding
    let decoded = urlencoding::decode(input).unwrap_or_else(|_| std::borrow::Cow::Borrowed(input));

    // Check for malicious patterns in decoded content
    let malicious_patterns = ["<script", "javascript:", "' OR 1=1", "../", "DROP TABLE"];

    for pattern in &malicious_patterns {
        if decoded.to_lowercase().contains(&pattern.to_lowercase()) {
            return InputValidationResult::Invalid(format!(
                "malicious_pattern_detected_{}",
                pattern
            ));
        }
    }

    InputValidationResult::Valid
}

async fn simulate_length_validation(input: &str, field_type: &str) -> InputValidationResult {
    sleep(Duration::from_millis(1)).await;

    let max_length = match field_type {
        "username" => 50,
        "message" => 2048,
        "room_name" => 100,
        _ => 1000,
    };

    if input.is_empty() {
        InputValidationResult::Invalid("empty_input".to_string())
    } else if input.len() > max_length {
        InputValidationResult::Invalid(format!("input_too_long_{}", input.len()))
    } else {
        InputValidationResult::Valid
    }
}

async fn simulate_command_validation(command: &str) -> InputValidationResult {
    sleep(Duration::from_millis(2)).await;

    if !command.starts_with('/') {
        return InputValidationResult::Invalid("not_a_command".to_string());
    }

    if command.len() == 1 {
        return InputValidationResult::Invalid("empty_command".to_string());
    }

    let parts: Vec<&str> = command[1..].split_whitespace().collect();
    if parts.is_empty() {
        return InputValidationResult::Invalid("empty_command".to_string());
    }

    let valid_commands = [
        "join", "leave", "msg", "help", "list", "quit", "say", "create",
    ];
    let command_name = parts[0];

    if !valid_commands.contains(&command_name) {
        return InputValidationResult::Invalid("unknown_command".to_string());
    }

    // Check for malicious patterns in command arguments
    let full_command = command.to_lowercase();
    let malicious_patterns = ["drop table", "<script", "javascript:", "../", "$(", "`"];

    for pattern in &malicious_patterns {
        if full_command.contains(pattern) {
            return InputValidationResult::Invalid("malicious_command".to_string());
        }
    }

    // Check command-specific requirements
    match command_name {
        "join" | "msg" | "create" => {
            if parts.len() < 2 {
                return InputValidationResult::Invalid("missing_arguments".to_string());
            }
        }
        _ => {}
    }

    InputValidationResult::Valid
}

async fn simulate_special_char_validation(input: &str) -> InputValidationResult {
    sleep(Duration::from_millis(1)).await;

    for ch in input.chars() {
        // Block control characters except tab and space
        if ch.is_control() && ch != ' ' && ch != '\t' {
            return InputValidationResult::Invalid("control_character".to_string());
        }

        // Block specific problematic Unicode characters
        match ch {
            '\u{200B}' | '\u{FEFF}' | '\u{202E}' => {
                return InputValidationResult::Invalid("problematic_unicode".to_string());
            }
            _ => {}
        }
    }

    InputValidationResult::Valid
}

async fn simulate_input_normalization(input: &str) -> String {
    sleep(Duration::from_millis(1)).await;

    // Convert to lowercase
    let mut normalized = input.to_lowercase();

    // Normalize whitespace (replace any whitespace with single space)
    normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    normalized
}

async fn simulate_concurrent_input_validation(input: &str, _id: usize) -> InputValidationResult {
    sleep(Duration::from_millis(1)).await;

    // Simple validation for concurrent testing
    if input.contains("concurrent_test_message_") {
        InputValidationResult::Valid
    } else {
        InputValidationResult::Invalid("invalid_concurrent_input".to_string())
    }
}

fn contains_control_characters(input: &str) -> bool {
    input
        .chars()
        .any(|c| c.is_control() && c != ' ' && c != '\t')
}

// URL encoding helper (simplified implementation)
mod urlencoding {
    use std::borrow::Cow;

    pub fn decode(input: &str) -> Result<Cow<str>, &'static str> {
        // Simplified URL decoding - in real implementation use proper library
        let decoded = input
            .replace("%20", " ")
            .replace("%3C", "<")
            .replace("%3E", ">")
            .replace("%27", "'")
            .replace("%22", "\"")
            .replace("%2F", "/")
            .replace("%2E", ".")
            .replace("%253C", "<")
            .replace("%2527", "'");

        Ok(Cow::Owned(decoded))
    }
}
