//! User domain types.
//!
//! See [DOMAIN_MODEL.md](../../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::ValidationError;

// ============================================================================
// UserId
// ============================================================================

/// Unique identifier for a user account.
///
/// Uses UUID v4 for globally unique, non-guessable IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(Uuid);

impl UserId {
    /// Create a new random UserId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a UserId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a UserId from a string.
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| ValidationError::InvalidFormat {
                reason: "invalid UUID format".into(),
            })
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// Username
// ============================================================================

/// Validated username.
///
/// # Rules
/// - 3-32 characters
/// - Alphanumeric and underscore only
/// - Cannot start with underscore
/// - Case-insensitive for uniqueness, preserves original case
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Username(String);

impl Username {
    /// Minimum username length.
    pub const MIN_LENGTH: usize = 3;
    /// Maximum username length.
    pub const MAX_LENGTH: usize = 32;

    /// Create a new username with validation.
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();

        if s.len() < Self::MIN_LENGTH {
            return Err(ValidationError::TooShort {
                min: Self::MIN_LENGTH,
                actual: s.len(),
            });
        }
        if s.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                max: Self::MAX_LENGTH,
                actual: s.len(),
            });
        }
        if s.starts_with('_') {
            return Err(ValidationError::InvalidFormat {
                reason: "cannot start with underscore".into(),
            });
        }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ValidationError::InvalidFormat {
                reason: "must contain only alphanumeric characters or underscore".into(),
            });
        }

        Ok(Self(s))
    }

    /// Create a username without validation (use only for data from trusted sources).
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the username as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get lowercase version for comparison.
    pub fn normalized(&self) -> String {
        self.0.to_lowercase()
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Username {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Username> for String {
    fn from(username: Username) -> Self {
        username.0
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// Email
// ============================================================================

/// Validated email address.
///
/// # Rules
/// - Must contain exactly one @
/// - Must have non-empty local and domain parts
/// - Domain must contain at least one dot
/// - Maximum 254 characters
/// - Stored in lowercase
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Email(String);

impl Email {
    /// Maximum email length per RFC 5321.
    pub const MAX_LENGTH: usize = 254;

    /// Create a new email with validation.
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();

        if s.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                max: Self::MAX_LENGTH,
                actual: s.len(),
            });
        }

        if !s.contains('@') {
            return Err(ValidationError::InvalidFormat {
                reason: "missing @ symbol".into(),
            });
        }

        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 {
            return Err(ValidationError::InvalidFormat {
                reason: "must contain exactly one @ symbol".into(),
            });
        }

        let (local, domain) = (parts[0], parts[1]);

        if local.is_empty() {
            return Err(ValidationError::InvalidFormat {
                reason: "local part cannot be empty".into(),
            });
        }

        if domain.is_empty() {
            return Err(ValidationError::InvalidFormat {
                reason: "domain cannot be empty".into(),
            });
        }

        if !domain.contains('.') {
            return Err(ValidationError::InvalidFormat {
                reason: "domain must contain a dot".into(),
            });
        }

        // Normalize to lowercase
        Ok(Self(s.to_lowercase()))
    }

    /// Create an email without validation (use only for data from trusted sources).
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the email as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the local part (before @).
    pub fn local_part(&self) -> &str {
        self.0.split('@').next().unwrap()
    }

    /// Get the domain part (after @).
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap()
    }
}

impl Display for Email {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// Role
// ============================================================================

/// User permission level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Regular user - can chat, join rooms, send DMs.
    #[default]
    User,
    /// Moderator - can moderate rooms they're assigned to.
    Moderator,
    /// Administrator - full system access.
    Admin,
}

impl Role {
    /// Check if this role has at least the given permission level.
    ///
    /// Permission hierarchy: Admin > Moderator > User
    pub fn has_permission(&self, required: Role) -> bool {
        matches!(
            (self, required),
            (Role::Admin, _)
                | (Role::Moderator, Role::Moderator | Role::User)
                | (Role::User, Role::User)
        )
    }

    /// Check if this role is admin.
    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }

    /// Check if this role is at least moderator.
    pub fn is_moderator(&self) -> bool {
        matches!(self, Role::Admin | Role::Moderator)
    }

    /// Get the role as a string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Moderator => "moderator",
            Role::Admin => "admin",
        }
    }

    /// Parse a role from a database string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => Role::Admin,
            "moderator" => Role::Moderator,
            _ => Role::User,
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Moderator => write!(f, "moderator"),
            Role::Admin => write!(f, "admin"),
        }
    }
}

// ============================================================================
// User
// ============================================================================

/// A registered user account.
///
/// Note: Password hash is NOT part of this type. It's stored separately
/// in the storage layer and never exposed to application code outside
/// of authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier.
    pub id: UserId,
    /// Unique username.
    pub username: Username,
    /// Email address.
    pub email: Email,
    /// User's role determining permissions.
    pub role: Role,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last profile update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Last time the user was seen online.
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user with the given details.
    pub fn new(username: Username, email: Email, role: Role) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(),
            username,
            email,
            role,
            created_at: now,
            updated_at: now,
            last_seen_at: None,
        }
    }

    /// Create a new admin user.
    pub fn new_admin(username: Username, email: Email) -> Self {
        Self::new(username, email, Role::Admin)
    }

    /// Check if user has the required permission level.
    pub fn has_permission(&self, required: Role) -> bool {
        self.role.has_permission(required)
    }

    /// Check if user is admin.
    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// Check if user is at least moderator.
    pub fn is_moderator(&self) -> bool {
        self.role.is_moderator()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);

        let parsed = UserId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);

        assert!(UserId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn test_username_valid() {
        assert!(Username::new("alice").is_ok());
        assert!(Username::new("Alice123").is_ok());
        assert!(Username::new("user_name").is_ok());
        assert!(Username::new("abc").is_ok()); // minimum length
        assert!(Username::new("a".repeat(32)).is_ok()); // maximum length
    }

    #[test]
    fn test_username_invalid() {
        // Too short
        assert!(Username::new("ab").is_err());
        assert!(Username::new("").is_err());

        // Too long
        assert!(Username::new("a".repeat(33)).is_err());

        // Starts with underscore
        assert!(Username::new("_alice").is_err());

        // Invalid characters
        assert!(Username::new("alice!").is_err());
        assert!(Username::new("al ice").is_err());
        assert!(Username::new("alice@bob").is_err());
    }

    #[test]
    fn test_username_normalized() {
        let username = Username::new("Alice").unwrap();
        assert_eq!(username.as_str(), "Alice");
        assert_eq!(username.normalized(), "alice");
    }

    #[test]
    fn test_email_valid() {
        assert!(Email::new("user@example.com").is_ok());
        assert!(Email::new("USER@EXAMPLE.COM").is_ok());
        assert!(Email::new("user.name@sub.domain.com").is_ok());
    }

    #[test]
    fn test_email_invalid() {
        // Missing @
        assert!(Email::new("userexample.com").is_err());

        // Empty local part
        assert!(Email::new("@example.com").is_err());

        // Empty domain
        assert!(Email::new("user@").is_err());

        // No dot in domain
        assert!(Email::new("user@localhost").is_err());

        // Multiple @
        assert!(Email::new("user@name@example.com").is_err());
    }

    #[test]
    fn test_email_normalized() {
        let email = Email::new("User@Example.COM").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_role_permissions() {
        assert!(Role::Admin.has_permission(Role::Admin));
        assert!(Role::Admin.has_permission(Role::Moderator));
        assert!(Role::Admin.has_permission(Role::User));

        assert!(!Role::Moderator.has_permission(Role::Admin));
        assert!(Role::Moderator.has_permission(Role::Moderator));
        assert!(Role::Moderator.has_permission(Role::User));

        assert!(!Role::User.has_permission(Role::Admin));
        assert!(!Role::User.has_permission(Role::Moderator));
        assert!(Role::User.has_permission(Role::User));
    }

    #[test]
    fn test_user_creation() {
        let username = Username::new("alice").unwrap();
        let email = Email::new("alice@example.com").unwrap();
        let user = User::new(username.clone(), email.clone(), Role::User);

        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert_eq!(user.role, Role::User);
    }

    #[test]
    fn test_user_admin() {
        let username = Username::new("admin").unwrap();
        let email = Email::new("admin@example.com").unwrap();
        let user = User::new_admin(username, email);

        assert!(user.is_admin());
        assert!(user.is_moderator());
        assert!(user.has_permission(Role::Admin));
    }

    #[test]
    fn test_user_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = UserId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_user_id_default() {
        let id = UserId::default();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn test_user_id_display() {
        let id = UserId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
        assert!(UserId::parse(&display).is_ok());
    }

    #[test]
    fn test_user_id_from_trait() {
        let uuid = Uuid::new_v4();
        let id: UserId = uuid.into();
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_username_display() {
        let username = Username::new("testuser").unwrap();
        assert_eq!(format!("{}", username), "testuser");
    }

    #[test]
    fn test_username_as_str() {
        let username = Username::new("testuser").unwrap();
        assert_eq!(username.as_str(), "testuser");
    }

    #[test]
    fn test_username_as_ref() {
        let username = Username::new("testuser").unwrap();
        let s: &str = username.as_ref();
        assert_eq!(s, "testuser");
    }

    #[test]
    fn test_username_new_unchecked() {
        let username = Username::new_unchecked("unchecked");
        assert_eq!(username.as_str(), "unchecked");
    }

    #[test]
    fn test_username_try_from() {
        let valid: Result<Username, _> = "validuser".to_string().try_into();
        assert!(valid.is_ok());

        let invalid: Result<Username, _> = "ab".to_string().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_username_into_string() {
        let username = Username::new("testuser").unwrap();
        let s: String = username.into();
        assert_eq!(s, "testuser");
    }

    #[test]
    fn test_email_display() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(format!("{}", email), "user@example.com");
    }

    #[test]
    fn test_email_as_str() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_local_part() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(email.local_part(), "user");
    }

    #[test]
    fn test_email_domain() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn test_email_as_ref() {
        let email = Email::new("user@example.com").unwrap();
        let s: &str = email.as_ref();
        assert_eq!(s, "user@example.com");
    }

    #[test]
    fn test_email_new_unchecked() {
        let email = Email::new_unchecked("unchecked@test.com");
        assert_eq!(email.as_str(), "unchecked@test.com");
    }

    #[test]
    fn test_email_try_from() {
        let valid: Result<Email, _> = "valid@test.com".to_string().try_into();
        assert!(valid.is_ok());

        let invalid: Result<Email, _> = "invalid".to_string().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_email_into_string() {
        let email = Email::new("user@example.com").unwrap();
        let s: String = email.into();
        assert_eq!(s, "user@example.com");
    }

    #[test]
    fn test_email_too_long() {
        // Email max length is 254 characters
        let long_local = "a".repeat(250);
        let email = format!("{}@example.com", long_local);
        assert!(Email::new(email).is_err());
    }

    #[test]
    fn test_role_is_admin() {
        assert!(Role::Admin.is_admin());
        assert!(!Role::Moderator.is_admin());
        assert!(!Role::User.is_admin());
    }

    #[test]
    fn test_role_is_moderator() {
        assert!(Role::Admin.is_moderator());
        assert!(Role::Moderator.is_moderator());
        assert!(!Role::User.is_moderator());
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Moderator.as_str(), "moderator");
        assert_eq!(Role::User.as_str(), "user");
    }

    #[test]
    fn test_role_parse() {
        assert_eq!(Role::parse("admin"), Role::Admin);
        assert_eq!(Role::parse("ADMIN"), Role::Admin);
        assert_eq!(Role::parse("moderator"), Role::Moderator);
        assert_eq!(Role::parse("user"), Role::User);
        assert_eq!(Role::parse("unknown"), Role::User);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(format!("{}", Role::Admin), "admin");
        assert_eq!(format!("{}", Role::Moderator), "moderator");
        assert_eq!(format!("{}", Role::User), "user");
    }

    #[test]
    fn test_role_default() {
        let role = Role::default();
        assert_eq!(role, Role::User);
    }

    #[test]
    fn test_role_serialization() {
        let admin = Role::Admin;
        let json = serde_json::to_string(&admin).unwrap();
        assert_eq!(json, "\"admin\"");

        let moderator = Role::Moderator;
        let json = serde_json::to_string(&moderator).unwrap();
        assert_eq!(json, "\"moderator\"");

        let user = Role::User;
        let json = serde_json::to_string(&user).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_user_serialization() {
        let username = Username::new("testuser").unwrap();
        let email = Email::new("test@example.com").unwrap();
        let user = User::new(username, email, Role::User);

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"email\":\"test@example.com\""));
        assert!(json.contains("\"role\":\"user\""));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, user.id);
    }

    #[test]
    fn test_username_serialization() {
        let username = Username::new("testuser").unwrap();
        let json = serde_json::to_string(&username).unwrap();
        assert_eq!(json, "\"testuser\"");

        let deserialized: Username = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "testuser");
    }

    #[test]
    fn test_email_serialization() {
        let email = Email::new("user@example.com").unwrap();
        let json = serde_json::to_string(&email).unwrap();
        assert_eq!(json, "\"user@example.com\"");

        let deserialized: Email = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "user@example.com");
    }

    #[test]
    fn test_user_moderator() {
        let username = Username::new("moduser").unwrap();
        let email = Email::new("mod@example.com").unwrap();
        let user = User::new(username, email, Role::Moderator);

        assert!(!user.is_admin());
        assert!(user.is_moderator());
        assert!(user.has_permission(Role::Moderator));
        assert!(!user.has_permission(Role::Admin));
    }
}
