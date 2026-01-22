//! TCP protocol integration tests.
//!
//! These tests verify the TCP adapter works correctly end-to-end,
//! using the length-prefixed JSON wire protocol.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::timeout;

use lair_chat_server::adapters::tcp::{TcpConfig, TcpServer};
use lair_chat_server::core::engine::ChatEngine;
use lair_chat_server::storage::sqlite::SqliteStorage;

/// Default test timeout.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Test helper to find an available port.
async fn get_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Test helper to create a server with in-memory database.
async fn create_test_server() -> (TcpServer, u16) {
    let port = get_available_port().await;
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage)));
    let config = TcpConfig { port };
    let server = TcpServer::start(config, engine).await.unwrap();
    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    (server, port)
}

/// TCP test client that speaks the wire protocol.
struct TestClient {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
}

impl TestClient {
    /// Connect to the server.
    async fn connect(port: u16) -> Self {
        let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let (read_half, write_half) = stream.into_split();
        Self {
            reader: BufReader::new(read_half),
            writer: BufWriter::new(write_half),
        }
    }

    /// Send a JSON message using length-prefixed framing.
    async fn send(&mut self, msg: &Value) {
        let json = serde_json::to_string(msg).unwrap();
        let bytes = json.as_bytes();
        let len = bytes.len() as u32;

        self.writer.write_all(&len.to_be_bytes()).await.unwrap();
        self.writer.write_all(bytes).await.unwrap();
        self.writer.flush().await.unwrap();
    }

    /// Receive a JSON message using length-prefixed framing.
    async fn recv(&mut self) -> Value {
        let mut len_bytes = [0u8; 4];
        self.reader.read_exact(&mut len_bytes).await.unwrap();
        let len = u32::from_be_bytes(len_bytes);

        let mut payload = vec![0u8; len as usize];
        self.reader.read_exact(&mut payload).await.unwrap();

        let json = String::from_utf8(payload).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// Send a message and receive the response.
    async fn request(&mut self, msg: &Value) -> Value {
        self.send(msg).await;
        self.recv().await
    }

    /// Complete the handshake.
    async fn handshake(&mut self) -> Value {
        // First, receive the server hello
        let server_hello = self.recv().await;
        assert_eq!(server_hello["type"], "server_hello");

        // Send client hello
        self.send(&json!({
            "type": "client_hello",
            "version": "1.0",
            "client_name": "test-client"
        }))
        .await;

        server_hello
    }

    /// Register a new user and return the response.
    async fn register(&mut self, username: &str, email: &str, password: &str) -> Value {
        self.request(&json!({
            "type": "register",
            "username": username,
            "email": email,
            "password": password
        }))
        .await
    }

    /// Login and return the response.
    async fn login(&mut self, identifier: &str, password: &str) -> Value {
        self.request(&json!({
            "type": "login",
            "identifier": identifier,
            "password": password
        }))
        .await
    }
}

// ============================================================================
// Handshake Tests
// ============================================================================

#[tokio::test]
async fn test_server_sends_hello_on_connect() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
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
async fn test_handshake_success() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        let server_hello = client.handshake().await;

        assert_eq!(server_hello["type"], "server_hello");
        assert_eq!(server_hello["version"], "1.0");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_ping_pong_before_auth() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        // Send ping
        let pong = client.request(&json!({"type": "ping"})).await;

        assert_eq!(pong["type"], "pong");
        assert!(pong["server_time"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_success() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        let response = client
            .register("testuser", "test@example.com", "SecurePass123!")
            .await;

        assert_eq!(response["type"], "register_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "testuser");
        assert!(response["session"]["id"].is_string());
        assert!(response["token"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // First client registers successfully
        let mut client1 = TestClient::connect(port).await;
        client1.handshake().await;
        client1
            .register("duplicate", "first@example.com", "SecurePass123!")
            .await;

        // Second client tries to register with the same username
        let mut client2 = TestClient::connect(port).await;
        client2.handshake().await;
        let response = client2
            .register("duplicate", "second@example.com", "SecurePass123!")
            .await;

        assert_eq!(response["success"], false);
        assert!(response["error"]["code"].as_str().is_some());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_register_weak_password() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        let response = client
            .register("testuser", "test@example.com", "weak")
            .await;

        assert_eq!(response["success"], false);
        assert_eq!(response["error"]["code"], "password_too_weak");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Login Tests
// ============================================================================

#[tokio::test]
async fn test_login_success() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        // Register first
        client
            .register("logintest", "login@example.com", "SecurePass123!")
            .await;

        // New connection to login
        let mut client2 = TestClient::connect(port).await;
        client2.handshake().await;

        let response = client2.login("logintest", "SecurePass123!").await;

        assert_eq!(response["type"], "login_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "logintest");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_login_wrong_password() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        // Register
        client
            .register("wrongpass", "wrong@example.com", "SecurePass123!")
            .await;

        // New connection with wrong password
        let mut client2 = TestClient::connect(port).await;
        client2.handshake().await;

        let response = client2.login("wrongpass", "WrongPassword!").await;

        assert_eq!(response["success"], false);
        assert_eq!(response["error"]["code"], "invalid_credentials");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        let response = client.login("nonexistent", "SomePass123!").await;

        assert_eq!(response["success"], false);
        assert_eq!(response["error"]["code"], "invalid_credentials");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Authenticated Operations Tests
// ============================================================================

#[tokio::test]
async fn test_create_room() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("roomcreator", "room@example.com", "SecurePass123!")
            .await;

        let response = client
            .request(&json!({
                "type": "create_room",
                "name": "Test Room",
                "description": "A test room"
            }))
            .await;

        eprintln!(
            "create_room response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );

        assert_eq!(response["type"], "create_room_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["room"]["name"], "Test Room");
        assert!(response["room"]["id"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_list_rooms() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("roomlister", "lister@example.com", "SecurePass123!")
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
                "type": "list_rooms",
                "limit": 50
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

#[tokio::test]
async fn test_join_room() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // First user creates a room
        let mut client1 = TestClient::connect(port).await;
        client1.handshake().await;
        client1
            .register("owner", "owner@example.com", "SecurePass123!")
            .await;

        let create_response = client1
            .request(&json!({
                "type": "create_room",
                "name": "Join Test Room"
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Second user joins
        let mut client2 = TestClient::connect(port).await;
        client2.handshake().await;
        client2
            .register("joiner", "joiner@example.com", "SecurePass123!")
            .await;

        let response = client2
            .request(&json!({
                "type": "join_room",
                "room_id": room_id
            }))
            .await;

        assert_eq!(response["type"], "join_room_response");
        assert_eq!(response["success"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_send_message() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("messager", "msg@example.com", "SecurePass123!")
            .await;

        // Create room
        let create_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Message Room"
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Send message
        let response = client
            .request(&json!({
                "type": "send_message",
                "target": {
                    "type": "room",
                    "room_id": room_id
                },
                "content": "Hello, world!"
            }))
            .await;

        assert_eq!(response["type"], "send_message_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["message"]["content"], "Hello, world!");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_get_messages() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("getmsg", "getmsg@example.com", "SecurePass123!")
            .await;

        // Create room
        let create_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Get Messages Room"
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Send some messages
        for i in 1..=5 {
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
        assert_eq!(response["messages"].as_array().unwrap().len(), 5);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_get_current_user() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("currentuser", "current@example.com", "SecurePass123!")
            .await;

        let response = client.request(&json!({"type": "get_current_user"})).await;

        assert_eq!(response["type"], "get_current_user_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "currentuser");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_list_users() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("listusers", "list@example.com", "SecurePass123!")
            .await;

        let response = client
            .request(&json!({
                "type": "list_users",
                "limit": 50
            }))
            .await;

        assert_eq!(response["type"], "list_users_response");
        assert_eq!(response["success"], true);
        assert!(response["users"].as_array().unwrap().len() >= 1);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_unauthorized_operation() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;

        // Try to create room without authenticating
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
async fn test_logout() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("logouttest", "logout@example.com", "SecurePass123!")
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
// Keepalive Tests
// ============================================================================

#[tokio::test]
async fn test_ping_pong_authenticated() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("pinguser", "ping@example.com", "SecurePass123!")
            .await;

        let pong = client.request(&json!({"type": "ping"})).await;

        assert_eq!(pong["type"], "pong");
        assert!(pong["server_time"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}
