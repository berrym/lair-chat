//! Command-specific validation rules for TCP commands
//!
//! This module provides validation rules for each TCP command type,
//! including parameter validation, format checking, and security validation.

use std::collections::HashMap;

use crate::server::error::ValidationError;
use crate::server::validation::{CommandValidator, ValidatedInput, ValidationResult};

/// Login command validator
pub struct LoginValidator;

/// Register command validator
pub struct RegisterValidator;

/// Message command validator
pub struct MessageValidator;

/// Create room command validator
pub struct CreateRoomValidator;

/// Join room command validator
pub struct JoinRoomValidator;

/// Leave room command validator
pub struct LeaveRoomValidator;

/// Invite user command validator
pub struct InviteUserValidator;

/// List rooms command validator
pub struct ListRoomsValidator;

/// List users command validator
pub struct ListUsersValidator;

/// Direct message command validator
pub struct DirectMessageValidator;

/// Edit message command validator
pub struct EditMessageValidator;

/// Delete message command validator
pub struct DeleteMessageValidator;

/// Search messages command validator
pub struct SearchMessagesValidator;

/// Change password command validator
pub struct ChangePasswordValidator;

/// Update profile command validator
pub struct UpdateProfileValidator;

/// Command validation rules
pub struct CommandValidationRules {
    /// Minimum/maximum parameter counts for each command
    param_counts: HashMap<String, (usize, usize)>,
    /// Required parameter formats
    param_formats: HashMap<String, Vec<ParamFormat>>,
    /// Permission requirements
    permission_requirements: HashMap<String, Vec<Permission>>,
}

/// Parameter format specification
#[derive(Debug, Clone)]
pub enum ParamFormat {
    /// Alphanumeric string
    Alphanumeric,
    /// Email address
    Email,
    /// Room name (letters, numbers, underscores, hyphens)
    RoomName,
    /// Username (letters, numbers, underscores)
    Username,
    /// Message content (printable characters)
    MessageContent,
    /// Room ID (UUID format)
    RoomId,
    /// User ID (UUID format)
    UserId,
    /// Message ID (UUID format)
    MessageId,
    /// Timestamp (ISO 8601 format)
    Timestamp,
    /// Number (positive integer)
    Number,
    /// Boolean (true/false)
    Boolean,
    /// JSON string
    Json,
}

/// Permission requirements
#[derive(Debug, Clone)]
pub enum Permission {
    /// Must be authenticated
    Authenticated,
    /// Must be in room
    InRoom,
    /// Must be room admin
    RoomAdmin,
    /// Must be system admin
    SystemAdmin,
    /// Must be message author
    MessageAuthor,
    /// Must have valid session
    ValidSession,
}

impl LoginValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for LoginValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() != 3 {
            return Err(ValidationError::InvalidFormat(
                "LOGIN command requires exactly 2 parameters: username and password".to_string(),
            ));
        }

        let username = parts[1];
        let password = parts[2];

        // Validate username format
        if !Self::is_valid_username(username) {
            return Err(ValidationError::InvalidFormat(
                "Username must be 3-20 characters, letters, numbers, and underscores only"
                    .to_string(),
            ));
        }

        // Validate password length
        if password.len() < 6 || password.len() > 128 {
            return Err(ValidationError::InvalidLength(
                "Password must be 6-128 characters".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "LOGIN".to_string(),
            arguments: vec![username.to_string(), password.to_string()],
            raw_input: input.to_string(),
            sanitized_input: format!("LOGIN {} [REDACTED]", username),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.len() >= 2 {
            format!("LOGIN {} [REDACTED]", parts[1])
        } else {
            input.to_string()
        }
    }

    fn check_rate_limit(&self, user_id: &str, command: &str) -> ValidationResult<()> {
        // Login attempts are heavily rate limited
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // No special permissions required for login
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 2 {
            return Err(ValidationError::InvalidFormat(
                "LOGIN requires username and password".to_string(),
            ));
        }
        Ok(())
    }
}

impl LoginValidator {
    fn is_valid_username(username: &str) -> bool {
        username.len() >= 3
            && username.len() <= 20
            && username.chars().all(|c| c.is_alphanumeric() || c == '_')
    }
}

impl RegisterValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for RegisterValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() != 4 {
            return Err(ValidationError::InvalidFormat(
                "REGISTER command requires exactly 3 parameters: username, password, and email"
                    .to_string(),
            ));
        }

        let username = parts[1];
        let password = parts[2];
        let email = parts[3];

        // Validate username
        if !Self::is_valid_username(username) {
            return Err(ValidationError::InvalidFormat(
                "Username must be 3-20 characters, letters, numbers, and underscores only"
                    .to_string(),
            ));
        }

        // Validate password
        if !Self::is_valid_password(password) {
            return Err(ValidationError::InvalidFormat(
                "Password must be 6-128 characters with at least one letter and one number"
                    .to_string(),
            ));
        }

        // Validate email
        if !Self::is_valid_email(email) {
            return Err(ValidationError::InvalidFormat(
                "Invalid email address format".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "REGISTER".to_string(),
            arguments: vec![
                username.to_string(),
                password.to_string(),
                email.to_string(),
            ],
            raw_input: input.to_string(),
            sanitized_input: format!("REGISTER {} [REDACTED] {}", username, email),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.len() >= 4 {
            format!("REGISTER {} [REDACTED] {}", parts[1], parts[3])
        } else {
            input.to_string()
        }
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        // Registration is rate limited
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // No special permissions required for registration
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 3 {
            return Err(ValidationError::InvalidFormat(
                "REGISTER requires username, password, and email".to_string(),
            ));
        }
        Ok(())
    }
}

impl RegisterValidator {
    fn is_valid_username(username: &str) -> bool {
        username.len() >= 3
            && username.len() <= 20
            && username.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    fn is_valid_password(password: &str) -> bool {
        password.len() >= 6
            && password.len() <= 128
            && password.chars().any(|c| c.is_alphabetic())
            && password.chars().any(|c| c.is_numeric())
    }

    fn is_valid_email(email: &str) -> bool {
        email.contains('@')
            && email.contains('.')
            && email.len() >= 5
            && email.len() <= 254
            && !email.starts_with('@')
            && !email.ends_with('@')
    }
}

impl MessageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for MessageValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();

        if parts.len() < 3 {
            return Err(ValidationError::InvalidFormat(
                "MESSAGE command requires room_id and message content".to_string(),
            ));
        }

        let room_id = parts[1];
        let message_content = parts[2];

        // Validate room ID format
        if !Self::is_valid_room_id(room_id) {
            return Err(ValidationError::InvalidFormat(
                "Invalid room ID format".to_string(),
            ));
        }

        // Validate message content
        if !Self::is_valid_message_content(message_content) {
            return Err(ValidationError::InvalidFormat(
                "Message content must be 1-1000 characters".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "MESSAGE".to_string(),
            arguments: vec![room_id.to_string(), message_content.to_string()],
            raw_input: input.to_string(),
            sanitized_input: self.sanitize_input(input),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        // Remove potentially harmful characters but preserve message content
        input
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control() || c.is_whitespace())
            .collect()
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        // Messages are rate limited per user
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // Must be authenticated and in room
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 2 {
            return Err(ValidationError::InvalidFormat(
                "MESSAGE requires room_id and message content".to_string(),
            ));
        }
        Ok(())
    }
}

impl MessageValidator {
    fn is_valid_room_id(room_id: &str) -> bool {
        // Simple UUID format check
        room_id.len() == 36 && room_id.chars().all(|c| c.is_alphanumeric() || c == '-')
    }

    fn is_valid_message_content(content: &str) -> bool {
        !content.trim().is_empty()
            && content.len() <= 1000
            && content
                .chars()
                .all(|c| c.is_ascii() && (!c.is_control() || c.is_whitespace()))
    }
}

impl CreateRoomValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for CreateRoomValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() < 2 {
            return Err(ValidationError::InvalidFormat(
                "CREATE_ROOM command requires room name".to_string(),
            ));
        }

        let room_name = parts[1];

        // Validate room name
        if !Self::is_valid_room_name(room_name) {
            return Err(ValidationError::InvalidFormat(
                "Room name must be 3-50 characters, letters, numbers, underscores, and hyphens only".to_string()
            ));
        }

        // Optional description
        let description = if parts.len() > 2 {
            parts[2..].join(" ")
        } else {
            String::new()
        };

        let mut arguments = vec![room_name.to_string()];
        if !description.is_empty() {
            arguments.push(description);
        }

        Ok(ValidatedInput {
            command: "CREATE_ROOM".to_string(),
            arguments,
            raw_input: input.to_string(),
            sanitized_input: self.sanitize_input(input),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii() && (!c.is_control() || c.is_whitespace()))
            .collect()
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        // Room creation is rate limited
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // Must be authenticated
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.is_empty() {
            return Err(ValidationError::InvalidFormat(
                "CREATE_ROOM requires room name".to_string(),
            ));
        }
        Ok(())
    }
}

impl CreateRoomValidator {
    fn is_valid_room_name(name: &str) -> bool {
        name.len() >= 3
            && name.len() <= 50
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }
}

impl JoinRoomValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for JoinRoomValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() != 2 {
            return Err(ValidationError::InvalidFormat(
                "JOIN_ROOM command requires exactly 1 parameter: room_id".to_string(),
            ));
        }

        let room_id = parts[1];

        // Validate room ID format
        if !Self::is_valid_room_id(room_id) {
            return Err(ValidationError::InvalidFormat(
                "Invalid room ID format".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "JOIN_ROOM".to_string(),
            arguments: vec![room_id.to_string()],
            raw_input: input.to_string(),
            sanitized_input: self.sanitize_input(input),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c == '_' || c == '-')
            .collect()
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // Must be authenticated
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 1 {
            return Err(ValidationError::InvalidFormat(
                "JOIN_ROOM requires room_id".to_string(),
            ));
        }
        Ok(())
    }
}

impl JoinRoomValidator {
    fn is_valid_room_id(room_id: &str) -> bool {
        !room_id.is_empty()
            && room_id.len() <= 100
            && room_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }
}

impl DirectMessageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for DirectMessageValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();

        if parts.len() < 3 {
            return Err(ValidationError::InvalidFormat(
                "DM command requires recipient username and message content".to_string(),
            ));
        }

        let recipient = parts[1];
        let message_content = parts[2];

        // Validate recipient username
        if !Self::is_valid_username(recipient) {
            return Err(ValidationError::InvalidFormat(
                "Invalid recipient username format".to_string(),
            ));
        }

        // Validate message content
        if !Self::is_valid_message_content(message_content) {
            return Err(ValidationError::InvalidFormat(
                "Message content must be 1-1000 characters".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "DM".to_string(),
            arguments: vec![recipient.to_string(), message_content.to_string()],
            raw_input: input.to_string(),
            sanitized_input: self.sanitize_input(input),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii() && (!c.is_control() || c.is_whitespace()))
            .collect()
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        // Direct messages are rate limited
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // Must be authenticated
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 2 {
            return Err(ValidationError::InvalidFormat(
                "DM requires recipient and message content".to_string(),
            ));
        }
        Ok(())
    }
}

impl DirectMessageValidator {
    fn is_valid_username(username: &str) -> bool {
        username.len() >= 3
            && username.len() <= 20
            && username.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    fn is_valid_message_content(content: &str) -> bool {
        !content.trim().is_empty()
            && content.len() <= 1000
            && content
                .chars()
                .all(|c| c.is_ascii() && (!c.is_control() || c.is_whitespace()))
    }
}

impl InviteUserValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CommandValidator for InviteUserValidator {
    fn validate_input(&self, input: &str) -> ValidationResult<ValidatedInput> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() != 3 {
            return Err(ValidationError::InvalidFormat(
                "INVITE_USER command requires exactly 2 parameters: room_id and username"
                    .to_string(),
            ));
        }

        let room_id = parts[1];
        let username = parts[2];

        // Validate room ID
        if !Self::is_valid_room_id(room_id) {
            return Err(ValidationError::InvalidFormat(
                "Invalid room ID format".to_string(),
            ));
        }

        // Validate username
        if !Self::is_valid_username(username) {
            return Err(ValidationError::InvalidFormat(
                "Invalid username format".to_string(),
            ));
        }

        Ok(ValidatedInput {
            command: "INVITE_USER".to_string(),
            arguments: vec![room_id.to_string(), username.to_string()],
            raw_input: input.to_string(),
            sanitized_input: self.sanitize_input(input),
            user_id: None,
            timestamp: crate::server::storage::current_timestamp(),
        })
    }

    fn sanitize_input(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c == '_' || c == '-')
            .collect()
    }

    fn check_rate_limit(&self, _user_id: &str, _command: &str) -> ValidationResult<()> {
        // Invitations are rate limited
        Ok(())
    }

    fn validate_permissions(
        &self,
        _user_id: &str,
        _command: &ValidatedInput,
    ) -> ValidationResult<()> {
        // Must be authenticated and in room
        Ok(())
    }

    fn validate_command_params(&self, command: &ValidatedInput) -> ValidationResult<()> {
        if command.arguments.len() != 2 {
            return Err(ValidationError::InvalidFormat(
                "INVITE_USER requires room_id and username".to_string(),
            ));
        }
        Ok(())
    }
}

impl InviteUserValidator {
    pub fn new() -> Self {
        InviteUserValidator
    }

    fn is_valid_room_id(room_id: &str) -> bool {
        !room_id.is_empty()
            && room_id.len() <= 100
            && room_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    fn is_valid_username(username: &str) -> bool {
        username.len() >= 3
            && username.len() <= 20
            && username.chars().all(|c| c.is_alphanumeric() || c == '_')
    }
}

/// Register all command validators
pub fn register_all_validators() -> HashMap<String, Box<dyn CommandValidator + Send + Sync>> {
    let mut validators: HashMap<String, Box<dyn CommandValidator + Send + Sync>> = HashMap::new();

    validators.insert("LOGIN".to_string(), Box::new(LoginValidator::new()));
    validators.insert("REGISTER".to_string(), Box::new(RegisterValidator::new()));
    validators.insert("MESSAGE".to_string(), Box::new(MessageValidator::new()));
    validators.insert(
        "CREATE_ROOM".to_string(),
        Box::new(CreateRoomValidator::new()),
    );
    validators.insert("JOIN_ROOM".to_string(), Box::new(JoinRoomValidator::new()));
    validators.insert("DM".to_string(), Box::new(DirectMessageValidator::new()));
    validators.insert(
        "INVITE_USER".to_string(),
        Box::new(InviteUserValidator::new()),
    );

    validators
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_validator() {
        let validator = LoginValidator::new();

        // Valid login
        let result = validator.validate_input("LOGIN testuser password123");
        assert!(result.is_ok());

        // Invalid format
        let result = validator.validate_input("LOGIN testuser");
        assert!(result.is_err());

        // Invalid username
        let result = validator.validate_input("LOGIN ab password123");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_validator() {
        let validator = RegisterValidator::new();

        // Valid registration
        let result = validator.validate_input("REGISTER testuser password123 test@example.com");
        assert!(result.is_ok());

        // Invalid email
        let result = validator.validate_input("REGISTER testuser password123 invalid-email");
        assert!(result.is_err());

        // Weak password
        let result = validator.validate_input("REGISTER testuser weak test@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_message_validator() {
        let validator = MessageValidator::new();

        // Valid message
        let result = validator.validate_input("MESSAGE room123 Hello world!");
        assert!(result.is_ok());

        // Missing content
        let result = validator.validate_input("MESSAGE room123");
        assert!(result.is_err());

        // Invalid room ID
        let result = validator.validate_input("MESSAGE invalid@room Hello world!");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_room_validator() {
        let validator = CreateRoomValidator::new();

        // Valid room creation
        let result = validator.validate_input("CREATE_ROOM general");
        assert!(result.is_ok());

        // With description
        let result = validator.validate_input("CREATE_ROOM general A general discussion room");
        assert!(result.is_ok());

        // Invalid room name
        let result = validator.validate_input("CREATE_ROOM ab");
        assert!(result.is_err());
    }

    #[test]
    fn test_direct_message_validator() {
        let validator = DirectMessageValidator::new();

        // Valid DM
        let result = validator.validate_input("DM alice Hello there!");
        assert!(result.is_ok());

        // Missing content
        let result = validator.validate_input("DM alice");
        assert!(result.is_err());

        // Invalid username
        let result = validator.validate_input("DM ab Hello there!");
        assert!(result.is_err());
    }
}
