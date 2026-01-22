//! HTTP API integration tests.
//!
//! These tests verify the HTTP adapter works correctly end-to-end,
//! from HTTP request to database and back.

use std::sync::Arc;

use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum_test::TestServer;
use serde_json::{json, Value};

use lair_chat_server::adapters::http::routes::create_router;
use lair_chat_server::core::engine::ChatEngine;
use lair_chat_server::storage::sqlite::SqliteStorage;

/// Create a test server with an in-memory database.
async fn create_test_server() -> TestServer {
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage)));
    let router = create_router(engine);
    TestServer::new(router).unwrap()
}

/// Helper to create authorization header value.
fn auth_header(session_id: &str) -> (HeaderName, HeaderValue) {
    (
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", session_id)).unwrap(),
    )
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[tokio::test]
async fn test_health_check() {
    let server = create_test_server().await;

    let response = server.get("/health").await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_readiness_check() {
    let server = create_test_server().await;

    let response = server.get("/ready").await;

    response.assert_status_ok();
    let body: Value = response.json();
    // Readiness returns "ready" field, not "status"
    assert_eq!(body["ready"], true);
}

// ============================================================================
// Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_success() {
    let server = create_test_server().await;

    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: Value = response.json();
    assert_eq!(body["user"]["username"], "testuser");
    assert_eq!(body["user"]["email"], "test@example.com");
    assert!(body["token"].as_str().is_some());
    assert!(body["session"]["id"].as_str().is_some());
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let server = create_test_server().await;

    // First registration
    server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "duplicate",
            "email": "first@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    // Second registration with same username
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "duplicate",
            "email": "second@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "username_taken");
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let server = create_test_server().await;

    // First registration
    server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "user1",
            "email": "same@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    // Second registration with same email
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "user2",
            "email": "same@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "email_taken");
}

#[tokio::test]
async fn test_register_weak_password() {
    let server = create_test_server().await;

    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "weak"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "password_too_weak");
}

#[tokio::test]
async fn test_register_invalid_username() {
    let server = create_test_server().await;

    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "ab",  // too short
            "email": "test@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "username_invalid");
}

// ============================================================================
// Login Tests
// ============================================================================

#[tokio::test]
async fn test_login_success_with_username() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "logintest",
            "email": "login@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    // Login with username
    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "identifier": "logintest",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["user"]["username"], "logintest");
    assert!(body["token"].as_str().is_some());
}

#[tokio::test]
async fn test_login_success_with_email() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "emailtest",
            "email": "emaillogin@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    // Login with email
    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "identifier": "emaillogin@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["user"]["email"], "emaillogin@example.com");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "wrongpass",
            "email": "wrongpass@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    // Login with wrong password
    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "identifier": "wrongpass",
            "password": "WrongPassword!"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "invalid_credentials");
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let server = create_test_server().await;

    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({
            "identifier": "nonexistent",
            "password": "SomePass123!"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
    let body: Value = response.json();
    assert_eq!(body["error"]["code"], "invalid_credentials");
}

// ============================================================================
// Session Tests
// ============================================================================

#[tokio::test]
async fn test_logout() {
    let server = create_test_server().await;

    // Register and get session
    let reg_response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "logouttest",
            "email": "logout@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    let body: Value = reg_response.json();
    let session_id = body["session"]["id"].as_str().unwrap();
    let (name, value) = auth_header(session_id);

    // Logout
    let response = server
        .post("/api/v1/auth/logout")
        .add_header(name.clone(), value.clone())
        .await;

    response.assert_status_ok();

    // Try to use the session - should fail
    let response = server.get("/api/v1/users/me").add_header(name, value).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Room Tests
// ============================================================================

async fn register_and_get_session(server: &TestServer, username: &str) -> String {
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": username,
            "email": format!("{}@example.com", username),
            "password": "SecurePass123!"
        }))
        .await;

    let body: Value = response.json();
    body["session"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_create_room() {
    let server = create_test_server().await;
    let session = register_and_get_session(&server, "roomcreator").await;
    let (name, value) = auth_header(&session);

    let response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Test Room",
            "description": "A test room"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: Value = response.json();
    assert_eq!(body["room"]["name"], "Test Room");
    assert!(body["room"]["id"].as_str().is_some());
}

#[tokio::test]
async fn test_list_rooms() {
    let server = create_test_server().await;
    let session = register_and_get_session(&server, "roomlister").await;

    // Create a few rooms
    for i in 1..=3 {
        let (name, value) = auth_header(&session);
        server
            .post("/api/v1/rooms")
            .add_header(name, value)
            .json(&json!({
                "name": format!("Room {}", i)
            }))
            .await;
    }

    // List rooms
    let (name, value) = auth_header(&session);
    let response = server.get("/api/v1/rooms").add_header(name, value).await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert!(body["rooms"].as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn test_join_and_leave_room() {
    let server = create_test_server().await;

    // Create room with first user
    let owner_session = register_and_get_session(&server, "roomowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Join Test Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Second user joins
    let joiner_session = register_and_get_session(&server, "roomjoiner").await;
    let (name, value) = auth_header(&joiner_session);
    let join_response = server
        .post(&format!("/api/v1/rooms/{}/join", room_id))
        .add_header(name, value)
        .await;

    join_response.assert_status_ok();

    // Second user leaves
    let (name, value) = auth_header(&joiner_session);
    let leave_response = server
        .post(&format!("/api/v1/rooms/{}/leave", room_id))
        .add_header(name, value)
        .await;

    leave_response.assert_status_ok();
}

#[tokio::test]
async fn test_get_room_members() {
    let server = create_test_server().await;

    // Create room
    let session = register_and_get_session(&server, "memberlister").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Members Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Get members - need to be a member to get members
    let (name, value) = auth_header(&session);
    let response = server
        .get(&format!("/api/v1/rooms/{}/members", room_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 1); // Just the creator
}

// ============================================================================
// Message Tests
// ============================================================================

#[tokio::test]
async fn test_send_message() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_session(&server, "messagesender").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Message Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Send message - target needs "type" field
    let (name, value) = auth_header(&session);
    let response = server
        .post("/api/v1/messages")
        .add_header(name, value)
        .json(&json!({
            "target": { "type": "room", "room_id": room_id },
            "content": "Hello, world!"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: Value = response.json();
    assert_eq!(body["message"]["content"], "Hello, world!");
}

#[tokio::test]
async fn test_get_messages() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_session(&server, "messagegetter").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Get Messages Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Send some messages
    for i in 1..=5 {
        let (name, value) = auth_header(&session);
        server
            .post("/api/v1/messages")
            .add_header(name, value)
            .json(&json!({
                "target": { "type": "room", "room_id": room_id },
                "content": format!("Message {}", i)
            }))
            .await;
    }

    // Get messages - use target_type and target_id query params
    let (name, value) = auth_header(&session);
    let response = server
        .get("/api/v1/messages")
        .add_query_param("target_type", "room")
        .add_query_param("target_id", room_id)
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["messages"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_edit_message() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_session(&server, "messageeditor").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Edit Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Send message
    let (name, value) = auth_header(&session);
    let send_response = server
        .post("/api/v1/messages")
        .add_header(name, value)
        .json(&json!({
            "target": { "type": "room", "room_id": room_id },
            "content": "Original message"
        }))
        .await;

    let body: Value = send_response.json();
    let message_id = body["message"]["id"].as_str().unwrap();

    // Edit message
    let (name, value) = auth_header(&session);
    let response = server
        .patch(&format!("/api/v1/messages/{}", message_id))
        .add_header(name, value)
        .json(&json!({
            "content": "Edited message"
        }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["message"]["content"], "Edited message");
    assert_eq!(body["message"]["is_edited"], true);
}

#[tokio::test]
async fn test_delete_message() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_session(&server, "messagedeleter").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Delete Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Send message
    let (name, value) = auth_header(&session);
    let send_response = server
        .post("/api/v1/messages")
        .add_header(name, value)
        .json(&json!({
            "target": { "type": "room", "room_id": room_id },
            "content": "To be deleted"
        }))
        .await;

    let body: Value = send_response.json();
    let message_id = body["message"]["id"].as_str().unwrap();

    // Delete message
    let (name, value) = auth_header(&session);
    let response = server
        .delete(&format!("/api/v1/messages/{}", message_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
}

// ============================================================================
// User Tests
// ============================================================================

#[tokio::test]
async fn test_get_current_user() {
    let server = create_test_server().await;
    let session = register_and_get_session(&server, "currentuser").await;
    let (name, value) = auth_header(&session);

    let response = server.get("/api/v1/users/me").add_header(name, value).await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["user"]["username"], "currentuser");
}

#[tokio::test]
async fn test_list_users() {
    let server = create_test_server().await;

    // Create multiple users
    for i in 1..=3 {
        register_and_get_session(&server, &format!("listuser{}", i)).await;
    }

    let session = register_and_get_session(&server, "listusers").await;
    let (name, value) = auth_header(&session);

    let response = server.get("/api/v1/users").add_header(name, value).await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert!(body["users"].as_array().unwrap().len() >= 4);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_unauthenticated_request() {
    let server = create_test_server().await;

    let response = server.get("/api/v1/users/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_session() {
    let server = create_test_server().await;
    let (name, value) = auth_header("invalid-session-id");

    let response = server.get("/api/v1/users/me").add_header(name, value).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Admin Tests
// ============================================================================

#[tokio::test]
async fn test_admin_stats_requires_permission() {
    let server = create_test_server().await;
    let session = register_and_get_session(&server, "regularuser").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get("/api/v1/admin/stats")
        .add_header(name, value)
        .await;

    // Regular users get 403 Forbidden for admin endpoints
    response.assert_status(StatusCode::FORBIDDEN);
}
