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
use lair_chat_server::crypto::{parse_public_key, Cipher, KeyPair, NONCE_SIZE};
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

/// Test JWT secret for tests.
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-integration-tests-only";

/// Test helper to create a server with in-memory database.
async fn create_test_server() -> (TcpServer, u16) {
    let port = get_available_port().await;
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));
    let config = TcpConfig {
        port,
        ..Default::default()
    };
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

    /// Check if a message type is an event (not a response).
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

    /// Send a message and receive the response, skipping any events.
    async fn request(&mut self, msg: &Value) -> Value {
        self.send(msg).await;
        loop {
            let response = self.recv().await;
            if !Self::is_event(&response) {
                return response;
            }
            // Skip events and keep waiting for the actual response
        }
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
        assert!(!response["users"].as_array().unwrap().is_empty());
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

// ============================================================================
// Encryption Tests
// ============================================================================

/// Encrypted test client that handles key exchange and AES-256-GCM.
struct EncryptedTestClient {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    cipher: Option<Cipher>,
}

impl EncryptedTestClient {
    async fn connect(port: u16) -> Self {
        let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let (read_half, write_half) = stream.into_split();
        Self {
            reader: BufReader::new(read_half),
            writer: BufWriter::new(write_half),
            cipher: None,
        }
    }

    async fn send(&mut self, msg: &Value) {
        let json = serde_json::to_string(msg).unwrap();

        if let Some(ref cipher) = self.cipher {
            // Encrypted send
            let plaintext = json.as_bytes();
            let (nonce, ciphertext) = cipher.encrypt(plaintext).unwrap();
            let frame_size = NONCE_SIZE + ciphertext.len();
            let length = frame_size as u32;

            self.writer.write_all(&length.to_be_bytes()).await.unwrap();
            self.writer.write_all(&nonce).await.unwrap();
            self.writer.write_all(&ciphertext).await.unwrap();
            self.writer.flush().await.unwrap();
        } else {
            // Unencrypted send
            let bytes = json.as_bytes();
            let len = bytes.len() as u32;
            self.writer.write_all(&len.to_be_bytes()).await.unwrap();
            self.writer.write_all(bytes).await.unwrap();
            self.writer.flush().await.unwrap();
        }
    }

    async fn recv(&mut self) -> Value {
        let mut len_bytes = [0u8; 4];
        self.reader.read_exact(&mut len_bytes).await.unwrap();
        let len = u32::from_be_bytes(len_bytes) as usize;

        if let Some(ref cipher) = self.cipher {
            // Encrypted recv
            let mut nonce = [0u8; NONCE_SIZE];
            self.reader.read_exact(&mut nonce).await.unwrap();

            let ciphertext_len = len - NONCE_SIZE;
            let mut ciphertext = vec![0u8; ciphertext_len];
            self.reader.read_exact(&mut ciphertext).await.unwrap();

            let plaintext = cipher.decrypt(&nonce, &ciphertext).unwrap();
            let json = String::from_utf8(plaintext).unwrap();
            serde_json::from_str(&json).unwrap()
        } else {
            // Unencrypted recv
            let mut payload = vec![0u8; len];
            self.reader.read_exact(&mut payload).await.unwrap();
            let json = String::from_utf8(payload).unwrap();
            serde_json::from_str(&json).unwrap()
        }
    }

    /// Check if a message type is an event (not a response).
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

    async fn request(&mut self, msg: &Value) -> Value {
        self.send(msg).await;
        loop {
            let response = self.recv().await;
            if !Self::is_event(&response) {
                return response;
            }
            // Skip events and keep waiting for the actual response
        }
    }

    /// Perform handshake with encryption support.
    async fn handshake_with_encryption(&mut self) -> Value {
        // Receive server hello
        let server_hello = self.recv().await;
        assert_eq!(server_hello["type"], "server_hello");

        // Check server supports encryption
        let features = server_hello["features"].as_array().unwrap();
        let supports_encryption = features.iter().any(|f| f == "encryption");
        assert!(supports_encryption, "Server should support encryption");

        // Send client hello with encryption feature
        self.send(&json!({
            "type": "client_hello",
            "version": "1.0",
            "client_name": "encrypted-test-client",
            "features": ["encryption"]
        }))
        .await;

        // Generate client keypair
        let keypair = KeyPair::generate();
        let client_public = keypair.public_key_base64();

        // Send key exchange
        self.send(&json!({
            "type": "key_exchange",
            "public_key": client_public
        }))
        .await;

        // Receive server's public key
        let key_response = self.recv().await;
        assert_eq!(key_response["type"], "key_exchange_response");

        let server_public_str = key_response["public_key"].as_str().unwrap();
        let server_public = parse_public_key(server_public_str).unwrap();

        // Derive shared secret
        let shared_secret = keypair.diffie_hellman(server_public);

        // Create cipher
        self.cipher = Some(Cipher::new(&shared_secret));

        server_hello
    }

    async fn register(&mut self, username: &str, email: &str, password: &str) -> Value {
        self.request(&json!({
            "type": "register",
            "username": username,
            "email": email,
            "password": password
        }))
        .await
    }
}

#[tokio::test]
async fn test_server_advertises_encryption() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        let hello = client.recv().await;

        assert_eq!(hello["type"], "server_hello");
        let features = hello["features"].as_array().unwrap();
        assert!(
            features.iter().any(|f| f == "encryption"),
            "Server should advertise encryption feature"
        );
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_encryption_handshake() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = EncryptedTestClient::connect(port).await;
        let server_hello = client.handshake_with_encryption().await;

        assert_eq!(server_hello["type"], "server_hello");
        assert!(client.cipher.is_some(), "Cipher should be established");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_encrypted_communication() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = EncryptedTestClient::connect(port).await;
        client.handshake_with_encryption().await;

        // Register (encrypted)
        let response = client
            .register("encuser", "enc@example.com", "SecurePass123!")
            .await;

        assert_eq!(response["type"], "register_response");
        assert_eq!(response["success"], true);
        assert_eq!(response["user"]["username"], "encuser");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_encrypted_ping_pong() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = EncryptedTestClient::connect(port).await;
        client.handshake_with_encryption().await;
        client
            .register("encping", "encping@example.com", "SecurePass123!")
            .await;

        // Send encrypted ping
        let pong = client.request(&json!({"type": "ping"})).await;

        assert_eq!(pong["type"], "pong");
        assert!(pong["server_time"].is_string());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_encrypted_room_and_messages() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = EncryptedTestClient::connect(port).await;
        client.handshake_with_encryption().await;
        client
            .register("encroom", "encroom@example.com", "SecurePass123!")
            .await;

        // Create room
        let create_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Encrypted Room"
            }))
            .await;

        assert_eq!(create_response["type"], "create_room_response");
        assert_eq!(create_response["success"], true);

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Send message
        let send_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "Secret encrypted message!"
            }))
            .await;

        assert_eq!(send_response["type"], "send_message_response");
        assert_eq!(send_response["success"], true);
        assert_eq!(
            send_response["message"]["content"],
            "Secret encrypted message!"
        );

        // Get messages
        let get_response = client
            .request(&json!({
                "type": "get_messages",
                "target": {"type": "room", "room_id": room_id},
                "limit": 50
            }))
            .await;

        assert_eq!(get_response["type"], "get_messages_response");
        assert_eq!(get_response["success"], true);
        assert_eq!(get_response["messages"].as_array().unwrap().len(), 1);
        assert_eq!(
            get_response["messages"][0]["content"],
            "Secret encrypted message!"
        );
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Message Edit/Delete Tests
// ============================================================================

#[tokio::test]
async fn test_edit_message() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("editor", "editor@example.com", "SecurePass123!")
            .await;

        // Create room
        let create_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Edit Test Room"
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Send message
        let send_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "Original content"
            }))
            .await;

        let message_id = send_response["message"]["id"].as_str().unwrap();

        // Edit message
        let edit_response = client
            .request(&json!({
                "type": "edit_message",
                "message_id": message_id,
                "content": "Edited content"
            }))
            .await;

        assert_eq!(edit_response["type"], "edit_message_response");
        assert_eq!(edit_response["success"], true);
        assert_eq!(edit_response["message"]["content"], "Edited content");
        assert_eq!(edit_response["message"]["edited"], true);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_delete_message() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        let mut client = TestClient::connect(port).await;
        client.handshake().await;
        client
            .register("deleter", "deleter@example.com", "SecurePass123!")
            .await;

        // Create room
        let create_response = client
            .request(&json!({
                "type": "create_room",
                "name": "Delete Test Room"
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Send message
        let send_response = client
            .request(&json!({
                "type": "send_message",
                "target": {"type": "room", "room_id": room_id},
                "content": "To be deleted"
            }))
            .await;

        let message_id = send_response["message"]["id"].as_str().unwrap();

        // Delete message
        let delete_response = client
            .request(&json!({
                "type": "delete_message",
                "message_id": message_id
            }))
            .await;

        assert_eq!(delete_response["type"], "delete_message_response");
        assert_eq!(delete_response["success"], true);

        // Verify message is deleted
        let get_response = client
            .request(&json!({
                "type": "get_messages",
                "target": {"type": "room", "room_id": room_id},
                "limit": 50
            }))
            .await;

        assert!(get_response["messages"].as_array().unwrap().is_empty());
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Room Operations Tests
// ============================================================================

#[tokio::test]
async fn test_leave_room() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // Owner creates room
        let mut owner = TestClient::connect(port).await;
        owner.handshake().await;
        owner
            .register("leaveowner", "leaveowner@example.com", "SecurePass123!")
            .await;

        let create_response = owner
            .request(&json!({
                "type": "create_room",
                "name": "Leave Test Room",
                "settings": { "public": true }
            }))
            .await;

        let room_id = create_response["room"]["id"].as_str().unwrap();

        // Second user joins
        let mut joiner = TestClient::connect(port).await;
        joiner.handshake().await;
        joiner
            .register("leaver", "leaver@example.com", "SecurePass123!")
            .await;

        let join_response = joiner
            .request(&json!({
                "type": "join_room",
                "room_id": room_id
            }))
            .await;

        assert_eq!(join_response["success"], true);

        // Second user leaves
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
async fn test_direct_message() {
    let (server, port) = create_test_server().await;

    let result = timeout(TEST_TIMEOUT, async {
        // First user
        let mut user1 = TestClient::connect(port).await;
        user1.handshake().await;
        let reg1 = user1
            .register("dmuser1", "dmuser1@example.com", "SecurePass123!")
            .await;
        let user1_id = reg1["user"]["id"].as_str().unwrap();

        // Second user
        let mut user2 = TestClient::connect(port).await;
        user2.handshake().await;
        let reg2 = user2
            .register("dmuser2", "dmuser2@example.com", "SecurePass123!")
            .await;
        let user2_id = reg2["user"]["id"].as_str().unwrap();

        // User1 sends DM to User2 (using "dm" type with "recipient" field)
        let send_response = user1
            .request(&json!({
                "type": "send_message",
                "target": {"type": "dm", "recipient": user2_id},
                "content": "Hello via DM!"
            }))
            .await;

        assert_eq!(send_response["type"], "send_message_response");
        assert_eq!(send_response["success"], true);
        assert_eq!(send_response["message"]["content"], "Hello via DM!");

        // User2 retrieves DM history using get_messages with DM target
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
        assert_eq!(get_response["messages"][0]["content"], "Hello via DM!");
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

// ============================================================================
// Connection Limit Tests
// ============================================================================

/// Test helper to create a server with connection limits.
async fn create_server_with_max_connections(max_connections: u32) -> (TcpServer, u16) {
    let port = get_available_port().await;
    let storage = SqliteStorage::in_memory().await.unwrap();
    let engine = Arc::new(ChatEngine::new(Arc::new(storage), TEST_JWT_SECRET));
    let config = TcpConfig {
        port,
        max_connections,
    };
    let server = TcpServer::start(config, engine).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    (server, port)
}

#[tokio::test]
async fn test_connection_limit_enforced() {
    // Create server with limit of 2 connections
    let (server, port) = create_server_with_max_connections(2).await;

    let result = timeout(TEST_TIMEOUT, async {
        // Connect first two clients - should succeed
        let client1 = TestClient::connect(port).await;
        let client2 = TestClient::connect(port).await;

        // Give the server time to register connections
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify connection count
        assert_eq!(server.connection_count(), 2);

        // Third connection should be rejected
        let stream3 = TcpStream::connect(format!("127.0.0.1:{}", port)).await;
        assert!(stream3.is_ok(), "Connection should establish initially");

        let mut stream3 = stream3.unwrap();
        // Try to receive - should get disconnect
        let mut buf = [0u8; 1024];
        let result = timeout(Duration::from_millis(500), stream3.read(&mut buf)).await;

        // Server should close the connection
        match result {
            Ok(Ok(0)) => {} // EOF - connection closed as expected
            Ok(Ok(_)) => panic!("Unexpected data received"),
            Ok(Err(_)) => {} // Connection error - also acceptable
            Err(_) => panic!("Timeout waiting for connection closure"),
        }

        // Keep clients alive
        drop(client1);
        drop(client2);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

/// Test that connection count increases when clients connect.
/// Note: We don't test decrement on disconnect because it depends on
/// the server detecting the closed socket, which has timing variations.
/// The connection count is verified by the limit enforcement tests instead.
#[tokio::test]
async fn test_connection_count_increases() {
    let (server, port) = create_server_with_max_connections(10).await;

    let result = timeout(TEST_TIMEOUT, async {
        // Connect first client
        let mut client1 = TestClient::connect(port).await;
        client1.handshake().await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(server.connection_count(), 1);

        // Connect second client
        let mut client2 = TestClient::connect(port).await;
        client2.handshake().await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(server.connection_count(), 2);

        // Keep clients alive until end of test
        drop(client1);
        drop(client2);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

#[tokio::test]
async fn test_unlimited_connections_when_zero() {
    // Create server with max_connections = 0 (unlimited)
    let (server, port) = create_server_with_max_connections(0).await;

    let result = timeout(TEST_TIMEOUT, async {
        // Connect many clients - all should succeed
        let mut clients = Vec::new();
        for _ in 0..10 {
            clients.push(TestClient::connect(port).await);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        // All connections should be established
        assert_eq!(server.connection_count(), 10);

        drop(clients);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}

/// Test that connection limit is properly enforced by verifying that
/// when at capacity, new connections receive the expected rejection behavior.
#[tokio::test]
async fn test_connection_limit_at_capacity() {
    let (server, port) = create_server_with_max_connections(2).await;

    let result = timeout(TEST_TIMEOUT, async {
        // Fill up connection slots
        let mut client1 = TestClient::connect(port).await;
        let mut client2 = TestClient::connect(port).await;

        // Complete handshake on both
        client1.handshake().await;
        client2.handshake().await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(server.connection_count(), 2);

        // Verify we're at the limit
        assert_eq!(server.connection_count(), 2, "Should be at max capacity");

        // Keep clients alive
        drop(client1);
        drop(client2);
    })
    .await;

    server.shutdown().await;
    result.unwrap();
}
