//! HTTPS/TLS integration tests.
//!
//! These tests verify that TLS configuration works correctly for the HTTP server.
//! Note: These tests focus on configuration validation and error handling,
//! not actual TLS handshakes (which would require real certificates).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lair_chat_server::adapters::http::{HttpConfig, HttpServer, TlsConfig};
use lair_chat_server::core::engine::ChatEngine;
use lair_chat_server::storage::sqlite::SqliteStorage;

// Mutex to serialize environment variable tests
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Test JWT secret for tests.
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-integration-tests-only";

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_http_config_default() {
    let config = HttpConfig::default();
    assert_eq!(config.port, 8082);
    assert!(config.tls.is_none());
}

#[test]
fn test_http_config_with_tls() {
    let config = HttpConfig {
        port: 8443,
        tls: Some(TlsConfig {
            cert_path: PathBuf::from("/path/to/cert.pem"),
            key_path: PathBuf::from("/path/to/key.pem"),
        }),
    };
    assert_eq!(config.port, 8443);
    assert!(config.tls.is_some());

    let tls = config.tls.unwrap();
    assert_eq!(tls.cert_path, PathBuf::from("/path/to/cert.pem"));
    assert_eq!(tls.key_path, PathBuf::from("/path/to/key.pem"));
}

// ============================================================================
// Server Startup Tests (HTTP Mode)
// ============================================================================

#[tokio::test]
async fn test_http_server_starts_without_tls() {
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig {
        port: 0, // Use any available port
        tls: None,
    };

    // Server should start successfully in HTTP mode
    let result = HttpServer::start(config, engine).await;
    assert!(result.is_ok(), "HTTP server should start without TLS");

    let server = result.unwrap();
    server.shutdown().await;
}

// ============================================================================
// TLS Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_https_server_fails_with_missing_cert() {
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig {
        port: 0,
        tls: Some(TlsConfig {
            cert_path: PathBuf::from("/nonexistent/cert.pem"),
            key_path: PathBuf::from("/nonexistent/key.pem"),
        }),
    };

    // Server should fail to start with missing certificate
    let result = HttpServer::start(config, engine).await;
    assert!(result.is_err(), "Should fail with missing certificate");

    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("certificate")
            || err_msg.contains("cert")
            || err_msg.contains("not found"),
        "Error should mention certificate issue: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_https_server_fails_with_missing_key() {
    // Create a temporary file to serve as a "certificate"
    let temp_dir = tempfile::tempdir().unwrap();
    let cert_path = temp_dir.path().join("cert.pem");

    // Write a dummy certificate (invalid, but file exists)
    std::fs::write(
        &cert_path,
        "-----BEGIN CERTIFICATE-----\nDUMMY\n-----END CERTIFICATE-----\n",
    )
    .unwrap();

    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig {
        port: 0,
        tls: Some(TlsConfig {
            cert_path,
            key_path: PathBuf::from("/nonexistent/key.pem"),
        }),
    };

    // Server should fail to start with missing key
    let result = HttpServer::start(config, engine).await;
    assert!(result.is_err(), "Should fail with missing key");
}

#[tokio::test]
async fn test_https_server_fails_with_invalid_cert() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cert_path = temp_dir.path().join("cert.pem");
    let key_path = temp_dir.path().join("key.pem");

    // Write invalid certificate and key content
    std::fs::write(&cert_path, "not a valid certificate").unwrap();
    std::fs::write(&key_path, "not a valid key").unwrap();

    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig {
        port: 0,
        tls: Some(TlsConfig {
            cert_path,
            key_path,
        }),
    };

    // Server should fail to start with invalid certificate
    let result = HttpServer::start(config, engine).await;
    assert!(result.is_err(), "Should fail with invalid certificate");
}

#[tokio::test]
async fn test_https_server_fails_with_empty_cert() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cert_path = temp_dir.path().join("cert.pem");
    let key_path = temp_dir.path().join("key.pem");

    // Write empty files
    std::fs::write(&cert_path, "").unwrap();
    std::fs::write(&key_path, "").unwrap();

    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig {
        port: 0,
        tls: Some(TlsConfig {
            cert_path,
            key_path,
        }),
    };

    // Server should fail to start with empty certificate
    let result = HttpServer::start(config, engine).await;
    assert!(result.is_err(), "Should fail with empty certificate file");
}

// ============================================================================
// Environment Variable Tests
// ============================================================================
//
// Note: These tests modify global environment variables and must run serially.
// We use a mutex to ensure they don't interfere with each other.

/// Helper to run an env test with proper setup/cleanup
fn run_env_test<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Clear all TLS env vars before test
    std::env::remove_var("LAIR_TLS_ENABLED");
    std::env::remove_var("LAIR_TLS_CERT_PATH");
    std::env::remove_var("LAIR_TLS_KEY_PATH");

    f();

    // Clean up after test
    std::env::remove_var("LAIR_TLS_ENABLED");
    std::env::remove_var("LAIR_TLS_CERT_PATH");
    std::env::remove_var("LAIR_TLS_KEY_PATH");
}

#[test]
fn test_tls_env_parsing_disabled() {
    run_env_test(|| {
        let config = lair_chat_server::config::Config::from_env().unwrap();
        assert!(
            config.http.tls.is_none(),
            "TLS should be disabled by default"
        );
    });
}

#[test]
fn test_tls_env_parsing_enabled_without_paths() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "true");

        let result = lair_chat_server::config::Config::from_env();
        assert!(result.is_err(), "Should fail without cert path");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("LAIR_TLS_CERT_PATH"),
            "Error should mention missing LAIR_TLS_CERT_PATH: {}",
            err_msg
        );
    });
}

#[test]
fn test_tls_env_parsing_enabled_without_key() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "true");
        std::env::set_var("LAIR_TLS_CERT_PATH", "/path/to/cert.pem");

        let result = lair_chat_server::config::Config::from_env();
        assert!(result.is_err(), "Should fail without key path");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("LAIR_TLS_KEY_PATH"),
            "Error should mention missing LAIR_TLS_KEY_PATH: {}",
            err_msg
        );
    });
}

#[test]
fn test_tls_env_parsing_full_config() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "true");
        std::env::set_var("LAIR_TLS_CERT_PATH", "/path/to/cert.pem");
        std::env::set_var("LAIR_TLS_KEY_PATH", "/path/to/key.pem");

        let config = lair_chat_server::config::Config::from_env().unwrap();
        assert!(config.http.tls.is_some(), "TLS should be enabled");

        let tls = config.http.tls.unwrap();
        assert_eq!(tls.cert_path, PathBuf::from("/path/to/cert.pem"));
        assert_eq!(tls.key_path, PathBuf::from("/path/to/key.pem"));
    });
}

#[test]
fn test_tls_env_parsing_false_value() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "false");

        let config = lair_chat_server::config::Config::from_env().unwrap();
        assert!(
            config.http.tls.is_none(),
            "TLS should be disabled when set to false"
        );
    });
}

#[test]
fn test_tls_env_parsing_zero_value() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "0");

        let config = lair_chat_server::config::Config::from_env().unwrap();
        assert!(
            config.http.tls.is_none(),
            "TLS should be disabled when set to 0"
        );
    });
}

#[test]
fn test_tls_env_parsing_one_value() {
    run_env_test(|| {
        std::env::set_var("LAIR_TLS_ENABLED", "1");
        std::env::set_var("LAIR_TLS_CERT_PATH", "/path/to/cert.pem");
        std::env::set_var("LAIR_TLS_KEY_PATH", "/path/to/key.pem");

        let config = lair_chat_server::config::Config::from_env().unwrap();
        assert!(
            config.http.tls.is_some(),
            "TLS should be enabled when set to 1"
        );
    });
}
