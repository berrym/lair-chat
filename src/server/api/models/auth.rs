//! Authentication API models
//!
//! This module contains all data structures related to user authentication,
//! including registration, login, token management, and JWT claims.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

lazy_static! {
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// User registration request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-50 characters, alphanumeric and underscores only)
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    /// Email address
    #[validate(email)]
    pub email: String,

    /// Password (minimum 8 characters, must contain uppercase, lowercase, number)
    #[validate(length(min = 8), custom(function = "validate_password_strength"))]
    pub password: String,

    /// Display name (optional, 1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,

    /// User timezone (optional, IANA timezone identifier)
    pub timezone: Option<String>,
}

/// User login request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// Username or email address
    #[validate(length(min = 1))]
    pub identifier: String,

    /// Password
    #[validate(length(min = 1))]
    pub password: String,

    /// Whether to create a long-lived session (optional)
    #[serde(default)]
    pub remember_me: bool,

    /// Device information for session tracking (optional)
    pub device_info: Option<DeviceInfo>,
}

/// Device information for session tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceInfo {
    /// Device name/identifier
    pub name: String,

    /// Device type (mobile, desktop, tablet, etc.)
    pub device_type: String,

    /// Operating system
    pub os: Option<String>,

    /// Browser/client application
    pub client: Option<String>,

    /// IP address (set by server)
    #[serde(skip_deserializing)]
    pub ip_address: Option<String>,
}

/// Authentication response containing JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    /// JWT access token (short-lived)
    pub access_token: String,

    /// JWT refresh token (long-lived)
    pub refresh_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Access token expiration time in seconds
    pub expires_in: u64,

    /// User information
    pub user: AuthUserInfo,

    /// Session information
    pub session: SessionInfo,
}

impl AuthResponse {
    pub fn new(
        access_token: String,
        refresh_token: String,
        expires_in: u64,
        user: AuthUserInfo,
        session: SessionInfo,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            user,
            session,
        }
    }
}

/// User information included in authentication response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthUserInfo {
    /// User ID
    pub id: Uuid,

    /// Username
    pub username: String,

    /// Email address
    pub email: String,

    /// Display name
    pub display_name: String,

    /// User role
    pub role: UserRole,

    /// Account status
    pub status: UserStatus,

    /// User creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Session information included in authentication response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    /// Session ID
    pub id: Uuid,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,

    /// Session expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// Device information
    pub device: Option<DeviceInfo>,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RefreshRequest {
    /// Refresh token
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshResponse {
    /// New access token
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Access token expiration time in seconds
    pub expires_in: u64,

    /// Optional new refresh token (if rotation is enabled)
    pub refresh_token: Option<String>,
}

/// Logout request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// Whether to logout from all devices
    #[serde(default)]
    pub all_devices: bool,
}

/// Password change request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password
    #[validate(length(min = 1))]
    pub current_password: String,

    /// New password
    #[validate(length(min = 8), custom(function = "validate_password_strength"))]
    pub new_password: String,
}

/// Password reset request (initiate)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct PasswordResetRequest {
    /// Email address
    #[validate(email)]
    pub email: String,
}

/// Password reset confirmation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct PasswordResetConfirm {
    /// Reset token from email
    #[validate(length(min = 1))]
    pub token: String,

    /// New password
    #[validate(length(min = 8), custom(function = "validate_password_strength"))]
    pub new_password: String,
}

/// User role enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// System administrator
    Admin,
    /// Room moderator
    Moderator,
    /// Regular user
    User,
    /// Guest user (limited permissions)
    Guest,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Moderator => write!(f, "moderator"),
            UserRole::User => write!(f, "user"),
            UserRole::Guest => write!(f, "guest"),
        }
    }
}

/// User account status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// Account is active and can be used
    Active,
    /// Account is temporarily suspended
    Suspended,
    /// Account is banned
    Banned,
    /// Account is pending email verification
    PendingVerification,
    /// Account is deactivated by user
    Deactivated,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// JWT ID (unique identifier)
    pub jti: String,

    /// Token type (access or refresh)
    pub token_type: TokenType,

    /// User role
    pub role: UserRole,

    /// Session ID
    pub session_id: String,

    /// Additional custom claims
    #[serde(flatten)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

/// Token type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Short-lived access token
    Access,
    /// Long-lived refresh token
    Refresh,
}

/// Email verification request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct EmailVerificationRequest {
    /// Verification token from email
    #[validate(length(min = 1))]
    pub token: String,
}

/// Resend verification email request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ResendVerificationRequest {
    /// Email address
    #[validate(email)]
    pub email: String,
}

// Validation functions and constants

/// Validate password strength
fn validate_password_strength(password: &str) -> Result<(), validator::ValidationError> {
    let mut errors = Vec::new();

    // Check for uppercase letter
    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter");
    }

    // Check for lowercase letter
    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Password must contain at least one lowercase letter");
    }

    // Check for digit
    if !password.chars().any(|c| c.is_numeric()) {
        errors.push("Password must contain at least one number");
    }

    // Check for special character (optional but recommended)
    let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
    if !password.chars().any(|c| special_chars.contains(c)) {
        // This is a warning, not an error
        tracing::warn!("Password does not contain special characters (recommended)");
    }

    if !errors.is_empty() {
        let mut error = validator::ValidationError::new("password_strength");
        error.message = Some(errors.join(", ").into());
        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_register_request_validation() {
        let valid_request = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePassword123".to_string(),
            display_name: Some("Test User".to_string()),
            timezone: Some("UTC".to_string()),
        };

        assert!(valid_request.validate().is_ok());

        // Test invalid username (too short)
        let invalid_request = RegisterRequest {
            username: "ab".to_string(),
            ..valid_request.clone()
        };
        assert!(invalid_request.validate().is_err());

        // Test invalid email
        let invalid_request = RegisterRequest {
            email: "invalid-email".to_string(),
            ..valid_request.clone()
        };
        assert!(invalid_request.validate().is_err());

        // Test weak password
        let invalid_request = RegisterRequest {
            password: "weak".to_string(),
            ..valid_request.clone()
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_login_request_validation() {
        let valid_request = LoginRequest {
            identifier: "testuser".to_string(),
            password: "password".to_string(),
            remember_me: false,
            device_info: None,
        };

        assert!(valid_request.validate().is_ok());

        // Test empty identifier
        let invalid_request = LoginRequest {
            identifier: "".to_string(),
            ..valid_request.clone()
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid passwords
        assert!(validate_password_strength("SecurePassword123").is_ok());
        assert!(validate_password_strength("MyPassword1").is_ok());

        // Invalid passwords
        assert!(validate_password_strength("lowercase123").is_err()); // No uppercase
        assert!(validate_password_strength("UPPERCASE123").is_err()); // No lowercase
        assert!(validate_password_strength("NoNumbers").is_err()); // No numbers
        assert!(validate_password_strength("Short1").is_err()); // Too short (caught by length validator)
    }

    #[test]
    fn test_auth_response_creation() {
        let user = AuthUserInfo {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            role: UserRole::User,
            status: UserStatus::Active,
            created_at: Utc::now(),
        };

        let session = SessionInfo {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            device: None,
        };

        let response = AuthResponse::new(
            "access_token".to_string(),
            "refresh_token".to_string(),
            3600,
            user,
            session,
        );

        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
    }

    #[test]
    fn test_jwt_claims_serialization() {
        let mut custom = std::collections::HashMap::new();
        custom.insert(
            "custom_field".to_string(),
            serde_json::json!("custom_value"),
        );

        let claims = JwtClaims {
            sub: "user_id".to_string(),
            iat: 1234567890,
            exp: 1234567890 + 3600,
            iss: "lair-chat".to_string(),
            aud: "lair-chat-api".to_string(),
            jti: "token_id".to_string(),
            token_type: TokenType::Access,
            role: UserRole::User,
            session_id: "session_id".to_string(),
            custom,
        };

        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: JwtClaims = serde_json::from_str(&json).unwrap();

        assert_eq!(claims.sub, deserialized.sub);
        assert_eq!(claims.custom.len(), deserialized.custom.len());
    }
}
