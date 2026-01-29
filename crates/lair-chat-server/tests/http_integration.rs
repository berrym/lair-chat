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

/// Test JWT secret for tests.
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-integration-tests-only";

/// Create a test server with an in-memory database.
async fn create_test_server() -> TestServer {
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));
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

    // Register and get token
    let reg_response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": "logouttest",
            "email": "logout@example.com",
            "password": "SecurePass123!"
        }))
        .await;

    let body: Value = reg_response.json();
    let token = body["token"].as_str().unwrap();
    let (name, value) = auth_header(token);

    // Logout
    let response = server
        .post("/api/v1/auth/logout")
        .add_header(name.clone(), value.clone())
        .await;

    response.assert_status_ok();

    // Try to use the token - should fail (session invalidated)
    let response = server.get("/api/v1/users/me").add_header(name, value).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Room Tests
// ============================================================================

/// Register a user and get their JWT token for authentication.
async fn register_and_get_token(server: &TestServer, username: &str) -> String {
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": username,
            "email": format!("{}@example.com", username),
            "password": "SecurePass123!"
        }))
        .await;

    let body: Value = response.json();
    body["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_create_room() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "roomcreator").await;
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
    let session = register_and_get_token(&server, "roomlister").await;

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
    let owner_session = register_and_get_token(&server, "roomowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Join Test Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Second user joins
    let joiner_session = register_and_get_token(&server, "roomjoiner").await;
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
    let session = register_and_get_token(&server, "memberlister").await;
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
    let session = register_and_get_token(&server, "messagesender").await;
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
    let session = register_and_get_token(&server, "messagegetter").await;
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
    let session = register_and_get_token(&server, "messageeditor").await;
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
    assert_eq!(body["message"]["edited"], true);
}

#[tokio::test]
async fn test_delete_message() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_token(&server, "messagedeleter").await;
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
    let session = register_and_get_token(&server, "currentuser").await;
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
        register_and_get_token(&server, &format!("listuser{}", i)).await;
    }

    let session = register_and_get_token(&server, "listusers").await;
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
// Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_create_invitation() {
    let server = create_test_server().await;

    // Create room owner and private room
    let owner_session = register_and_get_token(&server, "inviteowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Private Room",
            "settings": { "public": false }
        }))
        .await;

    create_response.assert_status(StatusCode::CREATED);
    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Create invitee
    let invitee_info = register_and_get_full_info(&server, "invitee").await;
    let invitee_id = invitee_info["user"]["id"].as_str().unwrap();

    // Owner invites invitee
    let (name, value) = auth_header(&owner_session);
    let response = server
        .post("/api/v1/invitations")
        .add_header(name, value)
        .json(&json!({
            "room_id": room_id,
            "user_id": invitee_id
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: Value = response.json();
    assert!(body["invitation"]["id"].as_str().is_some());
}

#[tokio::test]
async fn test_list_invitations() {
    let server = create_test_server().await;

    // Create room owner and room
    let owner_session = register_and_get_token(&server, "listinvowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "List Invitations Room",
            "settings": { "public": false }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Create invitee
    let invitee_info = register_and_get_full_info(&server, "listinvitee").await;
    let invitee_id = invitee_info["user"]["id"].as_str().unwrap();
    let invitee_session = invitee_info["token"].as_str().unwrap();

    // Owner sends invitation
    let (name, value) = auth_header(&owner_session);
    server
        .post("/api/v1/invitations")
        .add_header(name, value)
        .json(&json!({
            "room_id": room_id,
            "user_id": invitee_id
        }))
        .await;

    // Invitee lists their invitations
    let (name, value) = auth_header(invitee_session);
    let response = server
        .get("/api/v1/invitations")
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let invitations = body["invitations"].as_array().unwrap();
    assert!(!invitations.is_empty());
}

#[tokio::test]
async fn test_accept_invitation_public_room() {
    let server = create_test_server().await;

    // Create room owner and a public room
    let owner_session = register_and_get_token(&server, "acceptinvowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Accept Invitation Room",
            "settings": { "public": true }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Create invitee
    let invitee_info = register_and_get_full_info(&server, "acceptinvitee").await;
    let invitee_id = invitee_info["user"]["id"].as_str().unwrap();
    let invitee_session = invitee_info["token"].as_str().unwrap();

    // Owner sends invitation (even to a public room, for testing)
    let (name, value) = auth_header(&owner_session);
    let invite_response = server
        .post("/api/v1/invitations")
        .add_header(name, value)
        .json(&json!({
            "room_id": room_id,
            "user_id": invitee_id
        }))
        .await;

    let body: Value = invite_response.json();
    let invitation_id = body["invitation"]["id"].as_str().unwrap();

    // Invitee accepts invitation
    let (name, value) = auth_header(invitee_session);
    let response = server
        .post(&format!("/api/v1/invitations/{}/accept", invitation_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert!(body["membership"]["room_id"].as_str().is_some());
}

/// Test accepting an invitation to a private room.
///
/// BUG: This test currently fails due to a race condition in accept_invitation().
/// See: https://github.com/berrym/lair-chat/issues/12
///
/// The bug: accept_invitation() marks the invitation as "accepted" before calling
/// join(), but join() checks for a "pending" invitation for private rooms.
#[tokio::test]
#[ignore = "Known bug: accept_invitation fails for private rooms (issue #12)"]
async fn test_accept_invitation_private_room() {
    let server = create_test_server().await;

    // Create room owner and a PRIVATE room
    let owner_session = register_and_get_token(&server, "privacceptowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Private Invitation Room",
            "settings": { "public": false }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Create invitee
    let invitee_info = register_and_get_full_info(&server, "privacceptinvitee").await;
    let invitee_id = invitee_info["user"]["id"].as_str().unwrap();
    let invitee_session = invitee_info["token"].as_str().unwrap();

    // Owner sends invitation to private room
    let (name, value) = auth_header(&owner_session);
    let invite_response = server
        .post("/api/v1/invitations")
        .add_header(name, value)
        .json(&json!({
            "room_id": room_id,
            "user_id": invitee_id
        }))
        .await;

    let body: Value = invite_response.json();
    let invitation_id = body["invitation"]["id"].as_str().unwrap();

    // Invitee accepts invitation - THIS SHOULD SUCCEED but currently fails
    let (name, value) = auth_header(invitee_session);
    let response = server
        .post(&format!("/api/v1/invitations/{}/accept", invitation_id))
        .add_header(name, value)
        .await;

    // Expected: 200 OK with membership
    // Actual: 403 Forbidden with "room_private" error
    response.assert_status_ok();
    let body: Value = response.json();
    assert!(body["membership"]["room_id"].as_str().is_some());
}

#[tokio::test]
async fn test_decline_invitation() {
    let server = create_test_server().await;

    // Create room owner and room
    let owner_session = register_and_get_token(&server, "declineinvowner").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Decline Invitation Room",
            "settings": { "public": false }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Create invitee
    let invitee_info = register_and_get_full_info(&server, "declineinvitee").await;
    let invitee_id = invitee_info["user"]["id"].as_str().unwrap();
    let invitee_session = invitee_info["token"].as_str().unwrap();

    // Owner sends invitation
    let (name, value) = auth_header(&owner_session);
    let invite_response = server
        .post("/api/v1/invitations")
        .add_header(name, value)
        .json(&json!({
            "room_id": room_id,
            "user_id": invitee_id
        }))
        .await;

    let body: Value = invite_response.json();
    let invitation_id = body["invitation"]["id"].as_str().unwrap();

    // Invitee declines invitation
    let (name, value) = auth_header(invitee_session);
    let response = server
        .post(&format!("/api/v1/invitations/{}/decline", invitation_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_accept_invitation_not_found() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "notfoundinvitee").await;
    let (name, value) = auth_header(&session);

    let response = server
        .post("/api/v1/invitations/00000000-0000-0000-0000-000000000000/accept")
        .add_header(name, value)
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

// Helper function to get full user info including ID
async fn register_and_get_full_info(server: &TestServer, username: &str) -> Value {
    let response = server
        .post("/api/v1/auth/register")
        .json(&json!({
            "username": username,
            "email": format!("{}@example.com", username),
            "password": "SecurePass123!"
        }))
        .await;
    response.json()
}

// ============================================================================
// Room Handler Tests (Extended)
// ============================================================================

#[tokio::test]
async fn test_get_room() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_token(&server, "getroomuser").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Get Room Test",
            "description": "A test room"
        }))
        .await;

    create_response.assert_status(StatusCode::CREATED);
    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Get the room
    let (name, value) = auth_header(&session);
    let response = server
        .get(&format!("/api/v1/rooms/{}", room_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["room"]["name"], "Get Room Test");
    assert!(body["membership"].is_object()); // Creator should be a member
    assert!(body["member_count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_get_room_not_found() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "roomnotfound").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get("/api/v1/rooms/00000000-0000-0000-0000-000000000000")
        .add_header(name, value)
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_room() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_token(&server, "updateroomuser").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Original Name",
            "settings": { "public": true }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Update the room
    let (name, value) = auth_header(&session);
    let response = server
        .patch(&format!("/api/v1/rooms/{}", room_id))
        .add_header(name, value)
        .json(&json!({
            "name": "Updated Name",
            "description": "New description"
        }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["room"]["name"], "Updated Name");
}

#[tokio::test]
async fn test_update_room_not_owner() {
    let server = create_test_server().await;

    // Create owner and room
    let owner_session = register_and_get_token(&server, "roomowner2").await;
    let (name, value) = auth_header(&owner_session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({
            "name": "Owner's Room",
            "settings": { "public": true }
        }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Non-owner tries to update
    let other_session = register_and_get_token(&server, "notowner").await;
    let (name, value) = auth_header(&other_session);
    let response = server
        .patch(&format!("/api/v1/rooms/{}", room_id))
        .add_header(name, value)
        .json(&json!({ "name": "Hacked Name" }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_room() {
    let server = create_test_server().await;

    // Create user and room
    let session = register_and_get_token(&server, "deleteroomuser").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Room To Delete" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Delete the room
    let (name, value) = auth_header(&session);
    let response = server
        .delete(&format!("/api/v1/rooms/{}", room_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();

    // Verify room is deleted
    let (name, value) = auth_header(&session);
    let get_response = server
        .get(&format!("/api/v1/rooms/{}", room_id))
        .add_header(name, value)
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_rooms_joined_only() {
    let server = create_test_server().await;

    // Create user and some rooms
    let session = register_and_get_token(&server, "joinedlistuser").await;
    let (name, value) = auth_header(&session);

    // Create a room (user auto-joins)
    server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Joined Room" }))
        .await;

    // List joined rooms only
    let (name, value) = auth_header(&session);
    let response = server
        .get("/api/v1/rooms")
        .add_query_param("joined_only", "true")
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let rooms = body["rooms"].as_array().unwrap();
    assert!(!rooms.is_empty());
    // All rooms should have is_member = true
    for room in rooms {
        assert_eq!(room["is_member"], true);
    }
}

#[tokio::test]
async fn test_list_rooms_with_limit_public() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "paginationuser").await;

    // Create multiple public rooms
    for i in 1..=5 {
        let (name, value) = auth_header(&session);
        server
            .post("/api/v1/rooms")
            .add_header(name, value)
            .json(&json!({
                "name": format!("Pagination Room {}", i),
                "settings": { "public": true }
            }))
            .await;
    }

    // List public rooms with limit - pagination works for public rooms
    let (name, value) = auth_header(&session);
    let response = server
        .get("/api/v1/rooms")
        .add_query_param("limit", "2")
        .add_query_param("joined_only", "false")
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let rooms = body["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), 2);
    assert!(body["has_more"].is_boolean());
    assert!(body["total_count"].as_u64().is_some());
}

/// Test pagination for joined rooms (joined_only=true).
///
/// BUG: This test currently fails because pagination is not applied to joined_only queries.
/// See: https://github.com/berrym/lair-chat/issues/13
///
/// The bug: list_rooms handler calls list_user_rooms() without passing pagination
/// when joined_only=true.
#[tokio::test]
#[ignore = "Known bug: pagination not applied for joined_only queries (issue #13)"]
async fn test_list_rooms_with_limit_joined_only() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "joinedpaginationuser").await;

    // Create multiple rooms (user auto-joins as owner)
    for i in 1..=5 {
        let (name, value) = auth_header(&session);
        server
            .post("/api/v1/rooms")
            .add_header(name, value)
            .json(&json!({ "name": format!("Joined Pagination Room {}", i) }))
            .await;
    }

    // List joined rooms with limit - THIS SHOULD RETURN ONLY 2 ROOMS
    let (name, value) = auth_header(&session);
    let response = server
        .get("/api/v1/rooms")
        .add_query_param("limit", "2")
        .add_query_param("joined_only", "true")
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let rooms = body["rooms"].as_array().unwrap();
    // Expected: 2 rooms (respecting limit)
    // Actual: 5 rooms (limit ignored for joined_only)
    assert_eq!(
        rooms.len(),
        2,
        "Pagination limit should be applied to joined_only queries"
    );
}

#[tokio::test]
async fn test_join_room_already_member() {
    let server = create_test_server().await;

    // Create user and room (auto-joins)
    let session = register_and_get_token(&server, "alreadymember").await;
    let (name, value) = auth_header(&session);
    let create_response = server
        .post("/api/v1/rooms")
        .add_header(name, value)
        .json(&json!({ "name": "Already Member Room" }))
        .await;

    let body: Value = create_response.json();
    let room_id = body["room"]["id"].as_str().unwrap();

    // Try to join again
    let (name, value) = auth_header(&session);
    let response = server
        .post(&format!("/api/v1/rooms/{}/join", room_id))
        .add_header(name, value)
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

// ============================================================================
// User Handler Tests (Extended)
// ============================================================================

#[tokio::test]
async fn test_get_user_by_id() {
    let server = create_test_server().await;

    // Create a user
    let user_info = register_and_get_full_info(&server, "targetuser").await;
    let user_id = user_info["user"]["id"].as_str().unwrap();

    // Another user looks them up
    let session = register_and_get_token(&server, "lookupuser").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get(&format!("/api/v1/users/{}", user_id))
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["user"]["username"], "targetuser");
}

#[tokio::test]
async fn test_get_user_not_found() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "usernotfoundtest").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get("/api/v1/users/00000000-0000-0000-0000-000000000000")
        .add_header(name, value)
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_users_with_pagination() {
    let server = create_test_server().await;

    // Create multiple users
    for i in 1..=5 {
        register_and_get_token(&server, &format!("paginateuser{}", i)).await;
    }

    let session = register_and_get_token(&server, "userpaginator").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get("/api/v1/users")
        .add_query_param("limit", "3")
        .add_header(name, value)
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 3);
    assert_eq!(body["has_more"], true);
}

// ============================================================================
// Admin Tests
// ============================================================================

#[tokio::test]
async fn test_admin_stats_requires_permission() {
    let server = create_test_server().await;
    let session = register_and_get_token(&server, "regularuser").await;
    let (name, value) = auth_header(&session);

    let response = server
        .get("/api/v1/admin/stats")
        .add_header(name, value)
        .await;

    // Regular users get 403 Forbidden for admin endpoints
    response.assert_status(StatusCode::FORBIDDEN);
}
