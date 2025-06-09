//! Protocol messages for Lair-Chat authentication
//! Defines the JSON message formats used in the authentication process.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client authentication request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthenticationMessage {
    /// Register a new user account
    #[serde(rename = "register")]
    Register {
        username: String,
        password: String,
        #[serde(default)]
        fingerprint: String,
    },

    /// Login with existing credentials
    #[serde(rename = "login")]
    Login {
        username: String,
        password: String,
        #[serde(default)]
        fingerprint: String,
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

/// Server authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthenticationResponse {
    /// Successful authentication
    #[serde(rename = "success")]
    Success {
        /// User details
        user_id: Uuid,
        username: String,
        roles: Vec<String>,
        /// Session details
        token: String,
        expires_at: u64,
    },

    /// Authentication failed
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<String>,
    },

    /// Additional authentication step required
    #[serde(rename = "challenge")]
    Challenge {
        challenge_type: String,
        challenge_data: String,
    },
}

impl AuthenticationResponse {
    /// Create a successful authentication response
    pub fn success(
        user_id: Uuid,
        username: String,
        roles: Vec<String>,
        token: String,
        expires_at: u64,
    ) -> Self {
        Self::Success {
            user_id,
            username,
            roles,
            token,
            expires_at,
        }
    }

    /// Create an error response
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create an error response with details
    pub fn error_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
            details: Some(details.into()),
        }
    }

    /// Create a challenge response
    pub fn challenge(challenge_type: impl Into<String>, challenge_data: impl Into<String>) -> Self {
        Self::Challenge {
            challenge_type: challenge_type.into(),
            challenge_data: challenge_data.into(),
        }
    }
}

/// Error codes returned in AuthenticationResponse::Error
pub mod error_codes {
    pub const INVALID_CREDENTIALS: &str = "AUTH001";
    pub const USER_NOT_FOUND: &str = "AUTH002";
    pub const USERNAME_TAKEN: &str = "AUTH003";
    pub const RATE_LIMITED: &str = "AUTH004";
    pub const SESSION_EXPIRED: &str = "AUTH005";
    pub const INVALID_TOKEN: &str = "AUTH006";
    pub const ACCOUNT_LOCKED: &str = "AUTH007";
    pub const INTERNAL_ERROR: &str = "AUTH999";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_message_serialization() {
        let login = AuthenticationMessage::Login {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            fingerprint: "device1".to_string(),
        };

        let json = serde_json::to_string(&login).unwrap();
        assert!(json.contains(r#""type":"login""#));
        assert!(json.contains(r#""username":"testuser""#));

        let parsed: AuthenticationMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AuthenticationMessage::Login { username, .. } => {
                assert_eq!(username, "testuser");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_auth_response_serialization() {
        let success = AuthenticationResponse::success(
            Uuid::new_v4(),
            "testuser".to_string(),
            vec!["user".to_string()],
            "session123".to_string(),
            1234567890,
        );

        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains(r#""type":"success""#));
        assert!(json.contains(r#""username":"testuser""#));

        let error = AuthenticationResponse::error(
            error_codes::INVALID_CREDENTIALS,
            "Invalid username or password",
        );

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(error_codes::INVALID_CREDENTIALS));
    }

    #[test]
    fn test_challenge_response() {
        let challenge = AuthenticationResponse::challenge(
            "totp",
            "Please enter your 2FA code",
        );

        let json = serde_json::to_string(&challenge).unwrap();
        assert!(json.contains(r#""type":"challenge""#));
        assert!(json.contains(r#""challenge_type":"totp""#));
    }
}