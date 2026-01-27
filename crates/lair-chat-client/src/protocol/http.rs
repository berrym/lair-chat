//! HTTP client for REST API authentication.
//!
//! This module provides HTTP client functionality for authentication operations.
//! Per ADR-013, authentication is handled via HTTP, and the returned JWT token
//! is then used to authenticate the TCP connection.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

use super::messages::{Session, User};

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

/// Authentication response from the server.
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub user: Option<User>,
    pub session: Option<Session>,
    pub token: Option<String>,
    pub error: Option<ErrorInfo>,
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

/// List users response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ListUsersResponse {
    pub users: Vec<User>,
    pub online_user_ids: Vec<String>,
    pub has_more: bool,
    pub total_count: u64,
}

impl HttpClient {
    /// Create a new HTTP client.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
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
    pub async fn login(
        &mut self,
        identifier: &str,
        password: &str,
    ) -> Result<AuthResponse, HttpError> {
        let url = format!("{}/api/v1/auth/login", self.base_url);
        debug!("HTTP login to {}", url);

        let request = LoginRequest {
            identifier: identifier.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let auth_response: AuthResponse = response.json().await?;

        if !auth_response.success {
            let error_msg = auth_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Login failed".to_string());
            return Err(HttpError::AuthenticationFailed(error_msg));
        }

        // Store the token
        if let Some(ref token) = auth_response.token {
            self.token = Some(token.clone());
            info!("Login successful, token obtained");
        } else if !status.is_success() {
            return Err(HttpError::AuthenticationFailed(
                "No token in response".to_string(),
            ));
        }

        Ok(auth_response)
    }

    /// Register a new account.
    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, HttpError> {
        let url = format!("{}/api/v1/auth/register", self.base_url);
        debug!("HTTP register to {}", url);

        let request = RegisterRequest {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let auth_response: AuthResponse = response.json().await?;

        if !auth_response.success {
            let error_msg = auth_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Registration failed".to_string());
            return Err(HttpError::AuthenticationFailed(error_msg));
        }

        // Store the token
        if let Some(ref token) = auth_response.token {
            self.token = Some(token.clone());
            info!("Registration successful, token obtained");
        } else if !status.is_success() {
            return Err(HttpError::AuthenticationFailed(
                "No token in response".to_string(),
            ));
        }

        Ok(auth_response)
    }

    /// Logout (invalidate token).
    pub async fn logout(&mut self) -> Result<(), HttpError> {
        let Some(token) = &self.token else {
            return Ok(());
        };

        let url = format!("{}/api/v1/auth/logout", self.base_url);
        debug!("HTTP logout to {}", url);

        let _ = self
            .client
            .post(&url)
            .bearer_auth(token)
            .send()
            .await?;

        self.token = None;
        info!("Logout successful");
        Ok(())
    }

    /// List rooms (optional, for future use).
    #[allow(dead_code)]
    pub async fn list_rooms(&self) -> Result<ListRoomsResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/rooms", self.base_url);
        debug!("HTTP list rooms from {}", url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(HttpError::RequestFailed(format!(
                "Status {}",
                response.status()
            )));
        }

        let rooms_response: ListRoomsResponse = response.json().await?;
        Ok(rooms_response)
    }

    /// List users (optional, for future use).
    #[allow(dead_code)]
    pub async fn list_users(&self) -> Result<ListUsersResponse, HttpError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| HttpError::AuthenticationFailed("Not authenticated".to_string()))?;

        let url = format!("{}/api/v1/users", self.base_url);
        debug!("HTTP list users from {}", url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(HttpError::RequestFailed(format!(
                "Status {}",
                response.status()
            )));
        }

        let users_response: ListUsersResponse = response.json().await?;
        Ok(users_response)
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
