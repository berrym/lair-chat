//! Client-side authentication protocol handler for Lair-Chat
//! Handles authentication message serialization and communication.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::types::{AuthError, AuthResult, Credentials, DeviceInfo, Session, UserProfile};

/// Authentication request message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthRequest {
    /// Register a new user account
    #[serde(rename = "register")]
    Register {
        username: String,
        password: String,
        #[serde(default)]
        device: DeviceInfo,
    },

    /// Login with existing credentials
    #[serde(rename = "login")]
    Login {
        username: String,
        password: String,
        #[serde(default)]
        device: DeviceInfo,
    },

    /// Logout request
    #[serde(rename = "logout")]
    Logout {
        token: String,
    },

    /// Refresh session token
    #[serde(rename = "refresh")]
    RefreshToken {
        token: String,
    },
}

impl AuthRequest {
    /// Create a login request
    pub fn login(credentials: Credentials) -> Self {
        Self::Login {
            username: credentials.username,
            password: credentials.password,
            device: DeviceInfo::current(),
        }
    }

    /// Create a registration request
    pub fn register(credentials: Credentials) -> Self {
        Self::Register {
            username: credentials.username,
            password: credentials.password,
            device: DeviceInfo::current(),
        }
    }

    /// Create a logout request
    pub fn logout(token: String) -> Self {
        Self::Logout { token }
    }

    /// Create a token refresh request
    pub fn refresh(token: String) -> Self {
        Self::RefreshToken { token }
    }
}

/// Authentication response message
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AuthResponse {
    /// Successful authentication
    #[serde(rename = "success")]
    Success {
        user_id: Uuid,
        username: String,
        roles: Vec<String>,
        token: String,
        expires_at: u64,
    },

    /// Authentication failed
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        #[serde(default)]
        details: Option<String>,
    },

    /// Additional authentication step required
    #[serde(rename = "challenge")]
    Challenge {
        challenge_type: String,
        challenge_data: String,
    },
}

impl AuthResponse {
    /// Convert successful response into session and profile
    pub fn into_session_and_profile(self) -> AuthResult<(Session, UserProfile)> {
        match self {
            AuthResponse::Success {
                user_id,
                username,
                roles,
                token,
                expires_at,
            } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let session = Session {
                    id: Uuid::new_v4(),
                    token,
                    created_at: now,
                    expires_at,
                };

                let profile = UserProfile {
                    id: user_id,
                    username,
                    roles,
                };

                Ok((session, profile))
            }
            AuthResponse::Error { code, message, details } => {
                let error_msg = if let Some(detail) = details {
                    format!("{}: {} ({})", code, message, detail)
                } else {
                    format!("{}: {}", code, message)
                };
                Err(AuthError::AuthenticationFailed(error_msg))
            }
            AuthResponse::Challenge { .. } => {
                Err(AuthError::ProtocolError("Unexpected challenge response".into()))
            }
        }
    }
}

/// Handles encoding/decoding of authentication messages
pub struct AuthProtocol;

impl AuthProtocol {
    /// Encode an authentication request
    pub fn encode_request(request: &AuthRequest) -> AuthResult<String> {
        serde_json::to_string(request).map_err(|e| {
            AuthError::ProtocolError(format!("Failed to encode request: {}", e))
        })
    }

    /// Decode an authentication response
    pub fn decode_response(response: &str) -> AuthResult<AuthResponse> {
        serde_json::from_str(response).map_err(|e| {
            AuthError::ProtocolError(format!("Failed to decode response: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_encoding() {
        let credentials = Credentials {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let login_request = AuthRequest::login(credentials.clone());
        let encoded = AuthProtocol::encode_request(&login_request).unwrap();
        
        assert!(encoded.contains(r#""type":"login""#));
        assert!(encoded.contains(&credentials.username));
        assert!(encoded.contains(&credentials.password));
    }

    #[test]
    fn test_response_decoding() {
        let success_json = r#"{
            "type": "success",
            "user_id": "123e4567-e89b-12d3-a456-426614174000",
            "username": "testuser",
            "roles": ["user"],
            "token": "session123",
            "expires_at": 1234567890
        }"#;

        let response = AuthProtocol::decode_response(success_json).unwrap();
        let (session, profile) = response.into_session_and_profile().unwrap();

        assert_eq!(profile.username, "testuser");
        assert_eq!(session.token, "session123");
    }

    #[test]
    fn test_error_response() {
        let error_json = r#"{
            "type": "error",
            "code": "AUTH001",
            "message": "Invalid credentials",
            "details": "Username not found"
        }"#;

        let response = AuthProtocol::decode_response(error_json).unwrap();
        let result = response.into_session_and_profile();
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::AuthenticationFailed(_)));
    }
}