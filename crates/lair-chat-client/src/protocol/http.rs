//! HTTP client for REST API authentication.
//!
//! This module provides HTTP client functionality for authentication operations.
//! Per ADR-013, authentication is handled via HTTP, and the returned JWT token
//! is then used to authenticate the TCP connection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::messages::{Session, SessionId, User};

/// HTTP client errors.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum HttpError {
    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Server returned error: {code} - {message}")]
    ServerError { code: String, message: String },
}

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Base URL for the HTTP API (e.g., "http://localhost:8082" or "https://localhost:8082").
    pub base_url: String,
    /// Skip TLS certificate verification (for development with self-signed certs).
    pub skip_tls_verify: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8082".to_string(),
            skip_tls_verify: false,
        }
    }
}

/// HTTP client for REST API operations.
pub struct HttpClient {
    base_url: String,
    client: reqwest::Client,
    token: Option<String>,
}

/// Login request body.
#[derive(Debug, Serialize)]
struct LoginRequest {
    identifier: String,
    password: String,
}

/// Register request body.
#[derive(Debug, Serialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

/// Authentication response from the server (matches server's AuthResponse format).
/// Server uses HTTP status codes for success/failure, not a `success` field.
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub user: User,
    pub session: SessionInfo,
    pub token: String,
}

/// Session info as returned by HTTP API (strings instead of typed IDs).
#[derive(Debug, Clone, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub expires_at: String,
}

impl SessionInfo {
    /// Convert to the typed Session struct used by the rest of the app.
    pub fn to_session(&self) -> Result<Session, HttpError> {
        let id: SessionId = Uuid::parse_str(&self.id).map_err(|e| {
            HttpError::InvalidResponse(format!("Invalid session ID '{}': {}", self.id, e))
        })?;

        let expires_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|e| {
                HttpError::InvalidResponse(format!(
                    "Invalid expires_at '{}': {}",
                    self.expires_at, e
                ))
            })?
            .with_timezone(&Utc);

        Ok(Session { id, expires_at })
    }
}

/// Error response from server (for non-2xx responses).
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
}

/// Error information from server.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// Room list item for HTTP responses.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RoomListItem {
    pub room: super::messages::Room,
    pub member_count: u64,
    pub is_member: bool,
}

/// List rooms response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ListRoomsResponse {
    pub rooms: Vec<RoomListItem>,
    pub has_more: bool,
    pub total_count: u64,
}

/// User with online status from HTTP API.
#[derive(Debug, Deserialize)]
pub struct UserWithStatus {
    pub user: User,
    pub online: bool,
}

/// List users response.
#[derive(Debug, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserWithStatus>,
    #[allow(dead_code)]
    pub has_more: bool,
    #[allow(dead_code)]
    pub total_count: u32,
}

/// Create room request body.
#[derive(Debug, Serialize)]
pub struct CreateRoomRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<RoomSettingsRequest>,
}

/// Room settings for create/update requests.
#[derive(Debug, Serialize)]
pub struct RoomSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u32>,
}

/// Create room response.
#[derive(Debug, Deserialize)]
pub struct CreateRoomResponse {
    pub room: super::messages::Room,
}

/// Join room response.
#[derive(Debug, Deserialize)]
pub struct JoinRoomResponse {
    pub room: super::messages::Room,
}

/// Get messages response.
#[derive(Debug, Deserialize)]
pub struct GetMessagesResponse {
    pub messages: Vec<super::messages::Message>,
    #[allow(dead_code)]
    pub has_more: bool,
}

impl HttpClient {
    /// Create a new HTTP client with default settings.
    #[allow(dead_code)]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_config(HttpClientConfig {
            base_url: base_url.into(),
            skip_tls_verify: false,
        })
    }

    /// Create a new HTTP client with custom configuration.
    pub fn with_config(config: HttpClientConfig) -> Self {
        let mut builder = reqwest::Client::builder();

        if config.skip_tls_verify {
            warn!("TLS certificate verification is disabled - DO NOT USE IN PRODUCTION");
            builder = builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        let client = builder.build().expect("Failed to build HTTP client");

        Self {
            base_url: config.base_url,
            client,
            token: None,
        }
    }

    /// Get the current JWT token.
    #[allow(dead_code)]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Set the JWT token for authenticated requests.
    #[allow(dead_code)]
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Clear the JWT token.
    pub fn clear_token(&mut self) {
        self.token = None;
    }

    /// Login with username/email and password.
    /// Returns user info, session, and JWT token on success.
    pub async fn login(
        &mut self,
        identifier: &str,
        password: &str,
    ) -> Result<(User, Session, String), HttpError> {
        let url = format!("{}/api/v1/auth/login", self.base_url);
        debug!("HTTP login to {}", url);

        let request = LoginRequest {
            identifier: identifier.to_string(),
            password: password.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        if !status.is_success() {
            // Parse error response
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let auth_response: AuthResponse = response.json().await.map_err(|e| {
            HttpError::InvalidResponse(format!("Failed to parse auth response: {}", e))
        })?;

        // Convert SessionInfo to Session
        let session = auth_response.session.to_session()?;

        // Store the token
        self.token = Some(auth_response.token.clone());
        info!("Login successful, token obtained");

        Ok((auth_response.user, session, auth_response.token))
    }

    /// Register a new account.
    /// Returns user info, session, and JWT token on success.
    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(User, Session, String), HttpError> {
        let url = format!("{}/api/v1/auth/register", self.base_url);
        debug!("HTTP register to {}", url);

        let request = RegisterRequest {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        if !status.is_success() {
            // Parse error response
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let auth_response: AuthResponse = response.json().await.map_err(|e| {
            HttpError::InvalidResponse(format!("Failed to parse auth response: {}", e))
        })?;

        // Convert SessionInfo to Session
        let session = auth_response.session.to_session()?;

        // Store the token
        self.token = Some(auth_response.token.clone());
        info!("Registration successful, token obtained");

        Ok((auth_response.user, session, auth_response.token))
    }

    /// Logout (invalidate token).
    pub async fn logout(&mut self) -> Result<(), HttpError> {
        let Some(token) = &self.token else {
            return Ok(());
        };

        let url = format!("{}/api/v1/auth/logout", self.base_url);
        debug!("HTTP logout to {}", url);

        let _ = self.client.post(&url).bearer_auth(token).send().await?;

        self.token = None;
        info!("Logout successful");
        Ok(())
    }

    /// List rooms.
    pub async fn list_rooms(&self) -> Result<ListRoomsResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/rooms", self.base_url);
        debug!("HTTP list rooms from {}", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            return Err(HttpError::RequestFailed(format!(
                "Status {}",
                response.status()
            )));
        }

        let rooms_response: ListRoomsResponse = response.json().await?;
        Ok(rooms_response)
    }

    /// List users.
    pub async fn list_users(&self) -> Result<ListUsersResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/users", self.base_url);
        debug!("HTTP list users from {}", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            return Err(HttpError::RequestFailed(format!(
                "Status {}",
                response.status()
            )));
        }

        let users_response: ListUsersResponse = response.json().await?;
        Ok(users_response)
    }

    /// Create a new room.
    pub async fn create_room(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<super::messages::Room, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/rooms", self.base_url);
        debug!("HTTP create room at {}", url);

        let request = CreateRoomRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            settings: None,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let room_response: CreateRoomResponse = response.json().await?;
        info!("Room created: {}", room_response.room.name);
        Ok(room_response.room)
    }

    /// Join a room.
    pub async fn join_room(&self, room_id: &str) -> Result<super::messages::Room, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/rooms/{}/join", self.base_url, room_id);
        debug!("HTTP join room at {}", url);

        let response = self.client.post(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let join_response: JoinRoomResponse = response.json().await?;
        info!("Joined room: {}", join_response.room.name);
        Ok(join_response.room)
    }

    /// Get messages for a room or direct message conversation.
    pub async fn get_messages(
        &self,
        target_type: &str,
        target_id: &str,
        limit: Option<u32>,
    ) -> Result<GetMessagesResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let limit = limit.unwrap_or(50);
        let url = format!(
            "{}/api/v1/messages?target_type={}&target_id={}&limit={}",
            self.base_url, target_type, target_id, limit
        );
        debug!("HTTP get messages from {}", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let messages_response: GetMessagesResponse = response.json().await?;
        Ok(messages_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_new() {
        let client = HttpClient::new("http://localhost:8082");
        assert!(client.token().is_none());
    }

    #[test]
    fn test_set_token() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.set_token("test-token");
        assert_eq!(client.token(), Some("test-token"));
    }

    #[test]
    fn test_clear_token() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.set_token("test-token");
        client.clear_token();
        assert!(client.token().is_none());
    }
}
