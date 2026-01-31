//! WebSocket protocol integration tests.
//!
//! These tests verify the WebSocket adapter works correctly end-to-end,
//! using the same JSON message format as TCP but over WebSocket frames.

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use lair_chat_server::adapters::http::HttpConfig;
use lair_chat_server::adapters::http::HttpServer;
use lair_chat_server::core::engine::ChatEngine;
use lair_chat_server::storage::sqlite::SqliteStorage;

/// Type alias for the WebSocket stream.
type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Default test timeout.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Test JWT secret for tests.
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-websocket-tests-only";

/// Test helper to find an available port.
async fn get_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Create a test HTTP server with WebSocket support.
async fn create_test_server() -> (HttpServer, u16, Arc<ChatEngine<SqliteStorage>>) {
    let port = get_available_port().await;
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));

    let config = HttpConfig { port, tls: None };

    let server = HttpServer::start(config, engine.clone()).await.unwrap();
    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    (server, port, engine)
}

/// WebSocket test client.
struct WsTestClient {
    ws: WsStream,
}

impl WsTestClient {
    /// Connect to WebSocket server.
    async fn connect(port: u16) -> Self {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (ws, _): (WsStream, _) = connect_async(&url).await.expect("Failed to connect");
        Self { ws }
    }

    /// Connect with pre-authentication token.
    async fn connect_with_token(port: u16, token: &str) -> Self {
        let url = format!("ws://127.0.0.1:{}/ws?token={}", port, token);
        let (ws, _): (WsStream, _) = connect_async(&url).await.expect("Failed to connect");
        Self { ws }
    }

    /// Send a JSON message.
    async fn send(&mut self, msg: &Value) {
        let json = serde_json::to_string(msg).unwrap();
        self.ws.send(Message::Text(json)).await.unwrap();
    }

    /// Receive a JSON message.
    async fn recv(&mut self) -> Value {
        loop {
            match self.ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(&text).unwrap();
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                    // Skip ping/pong frames
                    continue;
                }
                Some(Ok(Message::Close(_))) => {
                    panic!("Connection closed unexpectedly");
                }
                Some(Err(e)) => {
                    panic!("WebSocket error: {}", e);
                }
                None => {
                    panic!("WebSocket stream ended");
                }
                _ => continue,
            }
        }
    }

    /// Check if a message type is an event.
    fn is_event(msg: &Value) -> bool {
        let event_types = [
            "message_received",
            "message_edited",
            "message_deleted",
            "user_joined_room",
            "user_left_room",
            "room_updated",
            "room_deleted",
            "user_online",
            "user_offline",
            "user_typing",
            "invitation_received",
            "server_notice",
        ];
        if let Some(msg_type) = msg.get("type").and_then(|t| t.as_str()) {
            event_types.contains(&msg_type)
        } else {
            false
        }
    }

    /// Send a message and receive the response, skipping events.
    async fn request(&mut self, msg: &Value) -> Value {
        self.send(msg).await;
        loop {
            let response = self.recv().await;
            if !Self::is_event(&response) {
                return response;
            }
        }
    }

    /// Complete the handshake.
    async fn handshake(&mut self) -> Value {
        // Receive server hello
        let server_hello = self.recv().await;
        assert_eq!(server_hello["type"], "server_hello");

        // Send client hello
        self.send(&json!({
            "type": "client_hello",
            "version": "1.0",
            "client_name": "ws-test-client"
        }))
        .await;

        server_hello
    }

    /// Register a new user.
    async fn register(&mut self, username: &str, email: &str, password: &str) -> Value {
        self.request(&json!({
            "type": "register",
            "username": username,
            "email": email,
            "password": password
        }))
        .await
    }

    /// Login with credentials.
    async fn login(&mut self, identifier: &str, password: &str) -> Value {
        self.request(&json!({
            "type": "login",
            "identifier": identifier,
            "password": password
        }))
        .await
    }

    /// Authenticate with JWT token.
    async fn authenticate(&mut self, token: &str) -> Value {
        self.request(&json!({
            "type": "authenticate",
            "token": token
        }))
        .await
    }
}

// ============================================================================
// Connection and Handshake Tests
// ============================================================================

#[tokio::test]
async fn test_ws_server_sends_hello_on_connect() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        let hello = client.recv().await;

        assert_eq!(hello["type"], "server_hello");
        assert!(hello["version"].is_string());
        assert!(hello["server_name"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_handshake_success() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        let hello = client.handshake().await;

        assert_eq!(hello["type"], "server_hello");
        assert_eq!(hello["version"], "1.0");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_version_mismatch() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;

        // Get server hello
        let _hello = client.recv().await;

        // Send client hello with incompatible version
        client
            .send(&json!({
                "type": "client_hello",
                "version": "99.0",
                "client_name": "bad-client"
            }))
            .await;

        // Should receive error
        let response = client.recv().await;
        assert_eq!(response["type"], "error");
        assert_eq!(response["code"], "version_mismatch");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_ws_register_success() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;

        let response = client
            .register("wsuser", "wsuser@example.com", "SecurePass123!")
            .await;

        assert_eq!(response["type"], "register_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "wsuser");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_login_success() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;

        // Register first
        client
            .register("wslogin", "wslogin@example.com", "SecurePass123!")
            .await;

        // Create a new connection to test login
        let mut client2 = WsTestClient::connect(port).await;
        client2.handshake().await;

        let response = client2.login("wslogin", "SecurePass123!").await;

        assert_eq!(response["type"], "login_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "wslogin");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_authenticate_with_token() {
    let (server, port, engine) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Register via engine to get a JWT
        let (_, _, token) = engine
            .register("wsauthuser", "wsauth@example.com", "SecurePass123!")
            .await
            .unwrap();

        // Connect via WebSocket and authenticate
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;

        let response = client.authenticate(&token).await;

        assert_eq!(response["type"], "authenticate_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "wsauthuser");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_pre_auth_with_token() {
    let (server, port, engine) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Register via engine to get a JWT
        let (_, _, token) = engine
            .register("wspreauth", "wspreauth@example.com", "SecurePass123!")
            .await
            .unwrap();

        // Connect with pre-auth token
        let mut client = WsTestClient::connect_with_token(port, &token).await;

        // Should receive server hello
        let hello = client.recv().await;
        assert_eq!(hello["type"], "server_hello");

        // Should receive auth response immediately
        let auth_response = client.recv().await;
        assert_eq!(auth_response["type"], "authenticate_response");
        assert_eq!(auth_response["success"], true);
        assert_eq!(auth_response["user"]["username"], "wspreauth");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_pre_auth_invalid_token() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Connect with invalid token
        let mut client = WsTestClient::connect_with_token(port, "invalid-token").await;

        // Should receive server hello
        let hello = client.recv().await;
        assert_eq!(hello["type"], "server_hello");

        // Should receive failed auth response
        let auth_response = client.recv().await;
        assert_eq!(auth_response["type"], "authenticate_response");
        assert_eq!(auth_response["success"], false);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Ping/Pong Tests
// ============================================================================

#[tokio::test]
async fn test_ws_ping_pong_before_auth() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;

        // Ping before auth should work
        let pong = client.request(&json!({"type": "ping"})).await;
        assert_eq!(pong["type"], "pong");
        assert!(pong["server_time"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_ping_pong_after_auth() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wspinguser", "wsping@example.com", "SecurePass123!")
            .await;

        let pong = client.request(&json!({"type": "ping"})).await;
        assert_eq!(pong["type"], "pong");
        assert!(pong["server_time"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Room and Message Tests
// ============================================================================

#[tokio::test]
async fn test_ws_create_room() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsroomuser", "wsroom@example.com", "SecurePass123!")
            .await;

        let response = client
            .request(&json!({
                "type": "create_room",
                "name": "WebSocket Room",
                "description": "A room created via WebSocket"
            }))
            .await;

        assert_eq!(response["type"], "create_room_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["room"]["name"], "WebSocket Room");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_send_message() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsmsguser", "wsmsg@example.com", "SecurePass123!")
            .await;

        // Create room
        let room_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Message Test Room"
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Send message
        let msg_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "Hello from WebSocket!"
            }))
            .await;

        assert_eq!(msg_response["type"], "send_message_response");
        assert_eq!(msg_response["success"], true);
        assert_eq!(msg_response["message"]["content"], "Hello from WebSocket!");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_get_messages() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsgetmsg", "wsgetmsg@example.com", "SecurePass123!")
            .await;

        // Create room
        let room_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Get Messages Room"
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Send a few messages
        for i in 1..=3 {
            client
                .request(&json!({
                    "type": "send_message",
                    "target": {"type": "room", "room_id": room_id},
                    "content": format!("Message {}", i)
                }))
                .await;
        }

        // Get messages
        let response = client
            .request(&json!({
                "type": "get_messages",
                "target": {"type": "room", "room_id": room_id},
                "limit": 50
            }))
            .await;

        assert_eq!(response["type"], "get_messages_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["messages"].as_array().unwrap().len(), 3);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_edit_message() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsedit", "wsedit@example.com", "SecurePass123!")
            .await;

        // Create room
        let room_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Edit Test Room"
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Send message
        let msg_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "Original"
            }))
            .await;
        let message_id = msg_response["message"]["id"].as_str().unwrap();

        // Edit message
        let edit_response = client
            .request(&json!({
                "type": "edit_message",
                "message_id": message_id,
                "content": "Edited"
            }))
            .await;

        assert_eq!(edit_response["type"], "edit_message_response");
        assert_eq!(edit_response["success"], true);
        assert_eq!(edit_response["message"]["content"], "Edited");
        assert_eq!(edit_response["message"]["edited"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_delete_message() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsdelete", "wsdelete@example.com", "SecurePass123!")
            .await;

        // Create room
        let room_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Delete Test Room"
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Send message
        let msg_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "To be deleted"
            }))
            .await;
        let message_id = msg_response["message"]["id"].as_str().unwrap();

        // Delete message
        let delete_response = client
            .request(&json!({
                "type": "delete_message",
                "message_id": message_id
            }))
            .await;

        assert_eq!(delete_response["type"], "delete_message_response");
        assert_eq!(delete_response["success"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Room Operations Tests
// ============================================================================

#[tokio::test]
async fn test_ws_join_room() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Owner creates room
        let mut owner = WsTestClient::connect(port).await;
        owner.handshake().await;
        owner
            .register("wsjoinowner", "wsjoinowner@example.com", "SecurePass123!")
            .await;

        let room_response = owner
            .request(&json!({
                "type": "create_room",
                "name": "Join Test Room",
                "settings": {"public": true}
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Joiner joins
        let mut joiner = WsTestClient::connect(port).await;
        joiner.handshake().await;
        joiner
            .register("wsjoiner", "wsjoiner@example.com", "SecurePass123!")
            .await;

        let join_response = joiner
            .request(&json!({
                "type": "join_room",
                "room_id": room_id
            }))
            .await;

        assert_eq!(join_response["type"], "join_room_response");
        assert_eq!(join_response["success"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_leave_room() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Owner creates room
        let mut owner = WsTestClient::connect(port).await;
        owner.handshake().await;
        owner
            .register("wsleaveowner", "wsleaveowner@example.com", "SecurePass123!")
            .await;

        let room_response = owner
            .request(&json!({
                "type": "create_room",
                "name": "Leave Test Room",
                "settings": {"public": true}
            }))
            .await;
        let room_id = room_response["room"]["id"].as_str().unwrap();

        // Joiner joins then leaves
        let mut joiner = WsTestClient::connect(port).await;
        joiner.handshake().await;
        joiner
            .register("wsleaver", "wsleaver@example.com", "SecurePass123!")
            .await;

        joiner
            .request(&json!({
                "type": "join_room",
                "room_id": room_id
            }))
            .await;

        let leave_response = joiner
            .request(&json!({
                "type": "leave_room",
                "room_id": room_id
            }))
            .await;

        assert_eq!(leave_response["type"], "leave_room_response");
        assert_eq!(leave_response["success"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_list_rooms() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wslistroom", "wslistroom@example.com", "SecurePass123!")
            .await;

        // Create some rooms
        for i in 1..=3 {
            client
                .request(&json!({
                    "type": "create_room",
                    "name": format!("Room {}", i)
                }))
                .await;
        }

        let response = client
            .request(&json!({
                "type": "list_rooms"
            }))
            .await;

        assert_eq!(response["type"], "list_rooms_response");
        assert_eq!(response["success"], true);
        assert!(response["rooms"].as_array().unwrap().len() >= 3);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// User Operations Tests
// ============================================================================

#[tokio::test]
async fn test_ws_get_current_user() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wscurrent", "wscurrent@example.com", "SecurePass123!")
            .await;

        let response = client
            .request(&json!({
                "type": "get_current_user"
            }))
            .await;

        assert_eq!(response["type"], "get_current_user_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "wscurrent");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_list_users() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wslistuser1", "wslist1@example.com", "SecurePass123!")
            .await;

        // Create more users
        let mut client2 = WsTestClient::connect(port).await;
        client2.handshake().await;
        client2
            .register("wslistuser2", "wslist2@example.com", "SecurePass123!")
            .await;

        let response = client
            .request(&json!({
                "type": "list_users"
            }))
            .await;

        assert_eq!(response["type"], "list_users_response");
        assert_eq!(response["success"], true);
        assert!(response["users"].as_array().unwrap().len() >= 2);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_ws_unauthorized_before_auth() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;

        // Try to create room without auth
        let response = client
            .request(&json!({
                "type": "create_room",
                "name": "Unauthorized Room"
            }))
            .await;

        assert_eq!(response["type"], "error");
        assert_eq!(response["code"], "unauthorized");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_key_exchange_not_supported() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wskeyex", "wskeyex@example.com", "SecurePass123!")
            .await;

        // Try key exchange - not supported on WebSocket
        let response = client
            .request(&json!({
                "type": "key_exchange",
                "public_key": "some-key"
            }))
            .await;

        assert_eq!(response["type"], "error");
        assert_eq!(response["code"], "not_supported");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_logout() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wslogout", "wslogout@example.com", "SecurePass123!")
            .await;

        let response = client.request(&json!({"type": "logout"})).await;

        assert_eq!(response["type"], "logout_response");
        assert_eq!(response["success"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Direct Message Tests
// ============================================================================

#[tokio::test]
async fn test_ws_direct_message() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // User 1
        let mut user1 = WsTestClient::connect(port).await;
        user1.handshake().await;
        let reg1 = user1
            .register("wsdm1", "wsdm1@example.com", "SecurePass123!")
            .await;
        let user1_id = reg1["user"]["id"].as_str().unwrap();

        // User 2
        let mut user2 = WsTestClient::connect(port).await;
        user2.handshake().await;
        let reg2 = user2
            .register("wsdm2", "wsdm2@example.com", "SecurePass123!")
            .await;
        let user2_id = reg2["user"]["id"].as_str().unwrap();

        // User1 sends DM to User2
        let send_response = user1
            .request(&json!({
                "type": "send_message",
                "target": {"type": "dm", "recipient": user2_id},
                "content": "Hello via WebSocket DM!"
            }))
            .await;

        assert_eq!(send_response["type"], "send_message_response");
        assert_eq!(send_response["success"], true);

        // User2 retrieves DM
        let get_response = user2
            .request(&json!({
                "type": "get_messages",
                "target": {"type": "dm", "recipient": user1_id},
                "limit": 50
            }))
            .await;

        assert_eq!(get_response["type"], "get_messages_response");
        assert_eq!(get_response["success"], true);
        assert_eq!(get_response["messages"].as_array().unwrap().len(), 1);
        assert_eq!(
            get_response["messages"][0]["content"],
            "Hello via WebSocket DM!"
        );
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_ws_list_invitations() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = WsTestClient::connect(port).await;
        client.handshake().await;
        client
            .register("wsinvlist", "wsinvlist@example.com", "SecurePass123!")
            .await;

        let response = client.request(&json!({"type": "list_invitations"})).await;

        assert_eq!(response["type"], "list_invitations_response");
        assert_eq!(response["success"], true);
        // Should be empty initially
        assert!(response["invitations"].as_array().unwrap().is_empty());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_ws_invalid_json() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (mut ws, _): (WsStream, _) = connect_async(&url).await.unwrap();

        // Skip server hello
        ws.next().await;

        // Send invalid JSON
        ws.send(Message::Text("not valid json".to_string()))
            .await
            .unwrap();

        // Should receive error
        if let Some(Ok(Message::Text(text))) = ws.next().await {
            let response: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(response["type"], "error");
            assert_eq!(response["code"], "invalid_message");
        }
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ws_binary_message_rejected() {
    let (server, port, _) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (mut ws, _): (WsStream, _) = connect_async(&url).await.unwrap();

        // Skip server hello
        ws.next().await;

        // Send binary (not supported)
        ws.send(Message::Binary(vec![1, 2, 3])).await.unwrap();

        // Should receive error
        if let Some(Ok(Message::Text(text))) = ws.next().await {
            let response: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(response["type"], "error");
            assert_eq!(response["code"], "unsupported");
        }
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}
