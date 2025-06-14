//! Client-side authentication protocol handler for Lair-Chat
//! Handles authentication message serialization and communication.

use super::types::{AuthError, AuthResult, Credentials, DeviceInfo, Session, UserProfile};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication request message (server-compatible format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthRequest {
    /// Login with existing credentials
    #[serde(rename = "login")]
    Login {
        username: String,
        password: String,
        #[serde(default)]
        fingerprint: String,
    },

    /// Register a new user account
    #[serde(rename = "register")]
    Register {
        username: String,
        password: String,
        #[serde(default)]
        fingerprint: String,
    },
}

impl AuthRequest {
    /// Create a login request
    pub fn login(credentials: Credentials) -> Self {
        Self::Login {
            username: credentials.username,
            password: credentials.password,
            fingerprint: DeviceInfo::current().fingerprint,
        }
    }

    /// Create a registration request
    pub fn register(credentials: Credentials) -> Self {
        Self::Register {
            username: credentials.username,
            password: credentials.password,
            fingerprint: DeviceInfo::current().fingerprint,
        }
    }
}

/// Authentication response message (server-compatible format)
#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub user: AuthUser,
    pub session: AuthSession,
}

/// User information in auth response
#[derive(Debug, Clone, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub roles: Vec<String>,
    pub created_at: u64,
    pub last_login: u64,
    pub status: String,
}

/// Session information in auth response
#[derive(Debug, Clone, Deserialize)]
pub struct AuthSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: u64,
    pub expires_at: u64,
    pub token: String,
    pub fingerprint: String,
}

impl AuthResponse {
    /// Convert the authentication response into session and profile
    pub fn into_session_and_profile(self) -> AuthResult<(Session, UserProfile)> {
        let session = Session {
            id: self.session.id,
            token: self.session.token,
            created_at: self.session.created_at,
            expires_at: self.session.expires_at,
        };

        let profile = UserProfile {
            id: self.user.id,
            username: self.user.username,
            roles: self.user.roles,
        };

        Ok((session, profile))
    }
}

/// Handles encoding/decoding of authentication messages
pub struct AuthProtocol;

impl AuthProtocol {
    /// Encode an authentication request
    pub fn encode_request(request: &AuthRequest) -> AuthResult<String> {
        serde_json::to_string(request)
            .map_err(|e| AuthError::ProtocolError(format!("Failed to encode request: {}", e)))
    }

    /// Decode an authentication response
    pub fn decode_response(response: &str) -> AuthResult<AuthResponse> {
        serde_json::from_str(response)
            .map_err(|e| AuthError::ProtocolError(format!("Failed to decode response: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_request_encoding() {
        let credentials = Credentials {
            username: "newuser".to_string(),
            password: "newpassword123".to_string(),
        };

        let register_request = AuthRequest::register(credentials.clone());
        let encoded = AuthProtocol::encode_request(&register_request).unwrap();

        assert!(encoded.contains(r#""is_registration":true"#));
        assert!(encoded.contains(&credentials.username));
        assert!(encoded.contains(&credentials.password));
    }

    #[test]
    fn test_response_decoding() {
        let success_json = r#"{
            "user": {
                "id": "01234567-89ab-cdef-0123-456789abcdef",
                "username": "testuser",
                "roles": ["user"],
                "created_at": 1234567890,
                "last_login": 1234567890,
                "status": "Active"
            },
            "session": {
                "id": "01234567-89ab-cdef-0123-456789abcdef",
                "user_id": "01234567-89ab-cdef-0123-456789abcdef",
                "created_at": 1234567890,
                "expires_at": 1234567890,
                "token": "test_token_123",
                "fingerprint": "test_fingerprint"
            }
        }"#;

        let response = AuthProtocol::decode_response(success_json).unwrap();
        let (session, profile) = response.into_session_and_profile().unwrap();

        assert_eq!(profile.username, "testuser");
        assert_eq!(session.token, "test_token_123");
    }
}
