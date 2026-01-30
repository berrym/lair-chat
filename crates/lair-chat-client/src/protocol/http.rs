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

/// List invitations response.
#[derive(Debug, Deserialize)]
pub struct ListInvitationsResponse {
    pub invitations: Vec<super::messages::Invitation>,
    /// Pagination support (for future use).
    #[serde(default)]
    #[allow(dead_code)]
    pub has_more: bool,
}

/// Accept invitation response (returns the new membership and room).
#[derive(Debug, Deserialize)]
pub struct AcceptInvitationResponse {
    /// The membership record for the newly joined room.
    #[allow(dead_code)]
    pub membership: super::messages::RoomMembership,
    /// The room that was joined.
    pub room: super::messages::Room,
}

/// Decline invitation response.
#[derive(Debug, Deserialize)]
pub struct DeclineInvitationResponse {
    /// Whether the decline was successful.
    #[serde(default)]
    #[allow(dead_code)]
    pub success: bool,
}

/// Create invitation request.
#[derive(Debug, Serialize)]
pub struct CreateInvitationRequest {
    pub room_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Create invitation response.
#[derive(Debug, Deserialize)]
pub struct CreateInvitationResponse {
    /// The created invitation with enriched data.
    #[allow(dead_code)]
    pub invitation: super::messages::Invitation,
}

/// Get room members response (new format with enriched members).
#[derive(Debug, Deserialize)]
pub struct GetRoomMembersResponse {
    pub members: Vec<super::messages::RoomMember>,
    #[allow(dead_code)]
    pub total: u32,
}

/// Update member role request.
#[derive(Debug, serde::Serialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

/// Update member role response.
#[derive(Debug, Deserialize)]
pub struct UpdateRoleResponse {
    pub member: super::messages::RoomMember,
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

    /// List pending invitations for the current user.
    pub async fn list_invitations(&self) -> Result<ListInvitationsResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/invitations", self.base_url);
        debug!("HTTP GET {} (list invitations)", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        let invitations_response: ListInvitationsResponse = response.json().await.map_err(|e| {
            HttpError::InvalidResponse(format!("Failed to parse invitations response: {}", e))
        })?;
        debug!(
            "HTTP list_invitations returned {} invitations",
            invitations_response.invitations.len()
        );
        Ok(invitations_response)
    }

    /// Accept an invitation.
    pub async fn accept_invitation(
        &self,
        invitation_id: &str,
    ) -> Result<AcceptInvitationResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!(
            "{}/api/v1/invitations/{}/accept",
            self.base_url, invitation_id
        );
        debug!("HTTP accept invitation at {}", url);

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

        let accept_response: AcceptInvitationResponse = response.json().await?;
        info!("Invitation accepted");
        Ok(accept_response)
    }

    /// Decline an invitation.
    pub async fn decline_invitation(
        &self,
        invitation_id: &str,
    ) -> Result<DeclineInvitationResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!(
            "{}/api/v1/invitations/{}/decline",
            self.base_url, invitation_id
        );
        debug!("HTTP decline invitation at {}", url);

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

        let decline_response: DeclineInvitationResponse = response.json().await?;
        info!("Invitation declined");
        Ok(decline_response)
    }

    /// Create an invitation to a room.
    pub async fn create_invitation(
        &self,
        room_id: &str,
        user_id: &str,
        message: Option<&str>,
    ) -> Result<CreateInvitationResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/invitations", self.base_url);
        debug!("HTTP create invitation at {}", url);

        let request = CreateInvitationRequest {
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
            message: message.map(|s| s.to_string()),
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

        let invitation_response: CreateInvitationResponse = response.json().await?;
        info!("Invitation created");
        Ok(invitation_response)
    }

    /// Get members of a room with their roles.
    pub async fn get_room_members(
        &self,
        room_id: &str,
    ) -> Result<GetRoomMembersResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/rooms/{}/members", self.base_url, room_id);
        debug!("HTTP get room members from {}", url);

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

        let members_response: GetRoomMembersResponse = response.json().await?;
        Ok(members_response)
    }

    /// Change a member's role in a room.
    ///
    /// Only room owners can change roles.
    pub async fn change_member_role(
        &self,
        room_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<UpdateRoleResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!(
            "{}/api/v1/rooms/{}/members/{}/role",
            self.base_url, room_id, user_id
        );
        debug!("HTTP change member role at {}", url);

        let request = UpdateRoleRequest {
            role: role.to_string(),
        };

        let response = self
            .client
            .put(&url)
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

        let role_response: UpdateRoleResponse = response.json().await?;
        info!("Member role changed to {}", role);
        Ok(role_response)
    }

    /// Remove a member from a room (kick).
    ///
    /// Owners can kick anyone except themselves.
    /// Moderators can kick regular members only.
    pub async fn kick_member(&self, room_id: &str, user_id: &str) -> Result<(), HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!(
            "{}/api/v1/rooms/{}/members/{}",
            self.base_url, room_id, user_id
        );
        debug!("HTTP kick member at {}", url);

        let response = self.client.delete(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let error_response: ErrorResponse = response.json().await.map_err(|e| {
                HttpError::InvalidResponse(format!("Failed to parse error response: {}", e))
            })?;
            return Err(HttpError::ServerError {
                code: error_response.error.code,
                message: error_response.error.message,
            });
        }

        info!("Member kicked from room");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HttpClientConfig Tests
    // ========================================================================

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8082");
        assert!(!config.skip_tls_verify);
    }

    #[test]
    fn test_http_client_config_custom() {
        let config = HttpClientConfig {
            base_url: "https://example.com:9000".to_string(),
            skip_tls_verify: true,
        };
        assert_eq!(config.base_url, "https://example.com:9000");
        assert!(config.skip_tls_verify);
    }

    // ========================================================================
    // HttpClient Creation Tests
    // ========================================================================

    #[test]
    fn test_http_client_new() {
        let client = HttpClient::new("http://localhost:8082");
        assert!(client.token().is_none());
        assert_eq!(client.base_url, "http://localhost:8082");
    }

    #[test]
    fn test_http_client_with_config() {
        let config = HttpClientConfig {
            base_url: "http://test:1234".to_string(),
            skip_tls_verify: false,
        };
        let client = HttpClient::with_config(config);
        assert!(client.token().is_none());
        assert_eq!(client.base_url, "http://test:1234");
    }

    // ========================================================================
    // Token Management Tests
    // ========================================================================

    #[test]
    fn test_set_token() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.set_token("test-token");
        assert_eq!(client.token(), Some("test-token"));
    }

    #[test]
    fn test_set_token_overwrites() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.set_token("token-1");
        client.set_token("token-2");
        assert_eq!(client.token(), Some("token-2"));
    }

    #[test]
    fn test_clear_token() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.set_token("test-token");
        client.clear_token();
        assert!(client.token().is_none());
    }

    #[test]
    fn test_clear_token_when_none() {
        let mut client = HttpClient::new("http://localhost:8082");
        client.clear_token(); // Should not panic
        assert!(client.token().is_none());
    }

    // ========================================================================
    // SessionInfo Tests
    // ========================================================================

    #[test]
    fn test_session_info_to_session_valid() {
        let session_info = SessionInfo {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            expires_at: "2026-12-31T23:59:59Z".to_string(),
        };

        let session = session_info.to_session().unwrap();
        assert_eq!(
            session.id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_session_info_to_session_invalid_uuid() {
        let session_info = SessionInfo {
            id: "not-a-valid-uuid".to_string(),
            expires_at: "2026-12-31T23:59:59Z".to_string(),
        };

        let result = session_info.to_session();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::InvalidResponse(_)));
    }

    #[test]
    fn test_session_info_to_session_invalid_date() {
        let session_info = SessionInfo {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            expires_at: "not-a-valid-date".to_string(),
        };

        let result = session_info.to_session();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::InvalidResponse(_)));
    }

    // ========================================================================
    // Request Struct Serialization Tests
    // ========================================================================

    #[test]
    fn test_login_request_serialization() {
        let request = LoginRequest {
            identifier: "testuser".to_string(),
            password: "secret123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"identifier\":\"testuser\""));
        assert!(json.contains("\"password\":\"secret123\""));
    }

    #[test]
    fn test_register_request_serialization() {
        let request = RegisterRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"username\":\"newuser\""));
        assert!(json.contains("\"email\":\"new@example.com\""));
        assert!(json.contains("\"password\":\"password123\""));
    }

    #[test]
    fn test_create_room_request_serialization_minimal() {
        let request = CreateRoomRequest {
            name: "test-room".to_string(),
            description: None,
            settings: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"test-room\""));
        // Optional fields should be skipped
        assert!(!json.contains("description"));
        assert!(!json.contains("settings"));
    }

    #[test]
    fn test_create_room_request_serialization_full() {
        let request = CreateRoomRequest {
            name: "test-room".to_string(),
            description: Some("A test room".to_string()),
            settings: Some(RoomSettingsRequest {
                public: Some(true),
                max_members: Some(50),
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"test-room\""));
        assert!(json.contains("\"description\":\"A test room\""));
        assert!(json.contains("\"public\":true"));
        assert!(json.contains("\"max_members\":50"));
    }

    #[test]
    fn test_room_settings_request_serialization_partial() {
        let settings = RoomSettingsRequest {
            public: Some(false),
            max_members: None,
        };

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"public\":false"));
        assert!(!json.contains("max_members"));
    }

    // ========================================================================
    // Response Struct Deserialization Tests
    // ========================================================================

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{"error": {"code": "unauthorized", "message": "Invalid token"}}"#;
        let response: ErrorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.error.code, "unauthorized");
        assert_eq!(response.error.message, "Invalid token");
    }

    #[test]
    fn test_list_rooms_response_deserialization() {
        let json = r#"{
            "rooms": [
                {
                    "room": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "name": "general",
                        "owner": "550e8400-e29b-41d4-a716-446655440001",
                        "settings": {"public": true, "moderated": false},
                        "created_at": "2026-01-01T00:00:00Z"
                    },
                    "member_count": 5,
                    "is_member": true
                }
            ],
            "has_more": false,
            "total_count": 1
        }"#;

        let response: ListRoomsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.rooms.len(), 1);
        assert_eq!(response.rooms[0].room.name, "general");
        assert_eq!(response.rooms[0].member_count, 5);
        assert!(response.rooms[0].is_member);
        assert!(!response.has_more);
        assert_eq!(response.total_count, 1);
    }

    #[test]
    fn test_list_users_response_deserialization() {
        let json = r#"{
            "users": [
                {
                    "user": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "username": "alice",
                        "email": "alice@example.com",
                        "role": "user",
                        "created_at": "2026-01-01T00:00:00Z"
                    },
                    "online": true
                }
            ],
            "has_more": false,
            "total_count": 1
        }"#;

        let response: ListUsersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.users.len(), 1);
        assert_eq!(response.users[0].user.username, "alice");
        assert!(response.users[0].online);
    }

    #[test]
    fn test_create_room_response_deserialization() {
        let json = r#"{
            "room": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "new-room",
                "owner": "550e8400-e29b-41d4-a716-446655440001",
                "settings": {"public": true, "moderated": false},
                "created_at": "2026-01-01T00:00:00Z"
            }
        }"#;

        let response: CreateRoomResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.room.name, "new-room");
    }

    #[test]
    fn test_join_room_response_deserialization() {
        let json = r#"{
            "room": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "joined-room",
                "owner": "550e8400-e29b-41d4-a716-446655440001",
                "settings": {"public": false, "moderated": true},
                "created_at": "2026-01-01T00:00:00Z"
            }
        }"#;

        let response: JoinRoomResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.room.name, "joined-room");
    }

    #[test]
    fn test_get_messages_response_deserialization() {
        let json = r#"{
            "messages": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "author": "550e8400-e29b-41d4-a716-446655440001",
                    "target": {"type": "room", "room_id": "550e8400-e29b-41d4-a716-446655440002"},
                    "content": "Hello, world!",
                    "edited": false,
                    "created_at": "2026-01-01T00:00:00Z"
                }
            ],
            "has_more": true
        }"#;

        let response: GetMessagesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(response.messages[0].content, "Hello, world!");
        assert!(response.has_more);
    }

    #[test]
    fn test_get_messages_response_empty() {
        let json = r#"{"messages": [], "has_more": false}"#;

        let response: GetMessagesResponse = serde_json::from_str(json).unwrap();
        assert!(response.messages.is_empty());
        assert!(!response.has_more);
    }

    // ========================================================================
    // Authentication Required Tests (without network)
    // ========================================================================

    #[tokio::test]
    async fn test_list_rooms_requires_auth() {
        let client = HttpClient::new("http://localhost:8082");
        // No token set

        let result = client.list_rooms().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn test_list_users_requires_auth() {
        let client = HttpClient::new("http://localhost:8082");

        let result = client.list_users().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn test_create_room_requires_auth() {
        let client = HttpClient::new("http://localhost:8082");

        let result = client.create_room("test-room", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn test_join_room_requires_auth() {
        let client = HttpClient::new("http://localhost:8082");

        let result = client
            .join_room("550e8400-e29b-41d4-a716-446655440000")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn test_get_messages_requires_auth() {
        let client = HttpClient::new("http://localhost:8082");

        let result = client
            .get_messages("room", "550e8400-e29b-41d4-a716-446655440000", Some(50))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn test_logout_without_token_succeeds() {
        let mut client = HttpClient::new("http://localhost:8082");
        // No token set - logout should succeed silently

        let result = client.logout().await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // HttpError Tests
    // ========================================================================

    #[test]
    fn test_http_error_display_request_failed() {
        let err = HttpError::RequestFailed("timeout".to_string());
        assert_eq!(format!("{}", err), "Request failed: timeout");
    }

    #[test]
    fn test_http_error_display_invalid_response() {
        let err = HttpError::InvalidResponse("bad json".to_string());
        assert_eq!(format!("{}", err), "Invalid response: bad json");
    }

    #[test]
    fn test_http_error_display_authentication_failed() {
        let err = HttpError::AuthenticationFailed("expired".to_string());
        assert_eq!(format!("{}", err), "Authentication failed: expired");
    }

    #[test]
    fn test_http_error_display_server_error() {
        let err = HttpError::ServerError {
            code: "not_found".to_string(),
            message: "Room not found".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Server returned error: not_found - Room not found"
        );
    }

    // ========================================================================
    // UserWithStatus Tests
    // ========================================================================

    #[test]
    fn test_user_with_status_online() {
        let json = r#"{
            "user": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "username": "bob",
                "email": "bob@example.com",
                "role": "user",
                "created_at": "2026-01-01T00:00:00Z"
            },
            "online": true
        }"#;

        let user_status: UserWithStatus = serde_json::from_str(json).unwrap();
        assert_eq!(user_status.user.username, "bob");
        assert!(user_status.online);
    }

    #[test]
    fn test_user_with_status_offline() {
        let json = r#"{
            "user": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "username": "charlie",
                "email": "charlie@example.com",
                "role": "admin",
                "created_at": "2026-01-01T00:00:00Z"
            },
            "online": false
        }"#;

        let user_status: UserWithStatus = serde_json::from_str(json).unwrap();
        assert_eq!(user_status.user.username, "charlie");
        assert_eq!(user_status.user.role, "admin");
        assert!(!user_status.online);
    }

    // ========================================================================
    // RoomListItem Tests
    // ========================================================================

    #[test]
    fn test_room_list_item_deserialization() {
        let json = r#"{
            "room": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "developers",
                "owner": "550e8400-e29b-41d4-a716-446655440001",
                "settings": {"public": true, "moderated": false},
                "created_at": "2026-01-01T00:00:00Z"
            },
            "member_count": 42,
            "is_member": false
        }"#;

        let item: RoomListItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.room.name, "developers");
        assert_eq!(item.member_count, 42);
        assert!(!item.is_member);
    }

    // ========================================================================
    // AuthResponse Tests
    // ========================================================================

    #[test]
    fn test_auth_response_deserialization() {
        let json = r#"{
            "user": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "username": "testuser",
                "email": "test@example.com",
                "role": "user",
                "created_at": "2026-01-01T00:00:00Z"
            },
            "session": {
                "id": "660e8400-e29b-41d4-a716-446655440000",
                "expires_at": "2026-12-31T23:59:59Z"
            },
            "token": "jwt-token-here"
        }"#;

        let response: AuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.user.username, "testuser");
        assert_eq!(response.token, "jwt-token-here");
        assert_eq!(response.session.id, "660e8400-e29b-41d4-a716-446655440000");
    }
}
