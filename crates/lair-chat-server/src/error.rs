//! # Error Types
//!
//! Unified error handling for the Lair Chat server.
//!
//! ## Error Categories
//!
//! - **Domain errors**: Business rule violations
//! - **Storage errors**: Database failures
//! - **Protocol errors**: Wire format issues
//! - **System errors**: Infrastructure failures
//!
//! ## Design
//!
//! All errors provide:
//! - `code`: Machine-readable error code
//! - `message`: User-safe message for display
//! - Internal details for logging (not exposed to users)

use thiserror::Error;

/// Main error type for the Lair Chat server
#[derive(Error, Debug)]
pub enum Error {
    // === Authentication Errors ===
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Session not found or expired")]
    SessionNotFound,

    #[error("Session expired")]
    SessionExpired,

    #[error("Account locked")]
    AccountLocked,

    #[error("Account banned")]
    AccountBanned,

    #[error("Invalid token: {reason}")]
    InvalidToken { reason: String },

    #[error("Token expired")]
    TokenExpired,

    // === Authorization Errors ===
    #[error("Permission denied")]
    PermissionDenied,

    #[error("Not a member of this room")]
    NotRoomMember,

    #[error("Not the author of this message")]
    NotMessageAuthor,

    // === Validation Errors ===
    #[error("Validation failed: {field} - {reason}")]
    ValidationFailed { field: String, reason: String },

    #[error("Username invalid: {reason}")]
    UsernameInvalid { reason: String },

    #[error("Username already taken")]
    UsernameTaken,

    #[error("Email invalid: {reason}")]
    EmailInvalid { reason: String },

    #[error("Email already registered")]
    EmailTaken,

    #[error("Password too weak: {reason}")]
    PasswordTooWeak { reason: String },

    #[error("Current password incorrect")]
    IncorrectPassword,

    #[error("Room name invalid: {reason}")]
    RoomNameInvalid { reason: String },

    #[error("Content invalid: {reason}")]
    ContentInvalid { reason: String },

    #[error("Content empty")]
    ContentEmpty,

    #[error("Content too long (max {max} characters)")]
    ContentTooLong { max: usize },

    // === Not Found Errors ===
    #[error("User not found")]
    UserNotFound,

    #[error("Room not found")]
    RoomNotFound,

    #[error("Message not found")]
    MessageNotFound,

    #[error("Invitation not found")]
    InvitationNotFound,

    // === Conflict Errors ===
    #[error("Already a member of this room")]
    AlreadyMember,

    #[error("Not the invitee")]
    NotInvitee,

    #[error("Room name already taken")]
    RoomNameTaken,

    #[error("Invitation already sent")]
    AlreadyInvited,

    #[error("Invitation already used")]
    InvitationUsed,

    // === State Errors ===
    #[error("Room is full")]
    RoomFull,

    #[error("Room requires invitation")]
    RoomPrivate,

    #[error("Cannot leave as only owner")]
    LastOwner,

    #[error("Message already deleted")]
    MessageDeleted,

    #[error("Invitation expired")]
    InvitationExpired,

    #[error("User blocked")]
    UserBlocked,

    // === Rate Limiting ===
    #[error("Rate limited, retry after {retry_after} seconds")]
    RateLimited { retry_after: u32 },

    // === Storage Errors ===
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // === System Errors ===
    #[error("Internal error")]
    Internal(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl Error {
    /// Get the error code for API responses
    pub fn code(&self) -> &'static str {
        match self {
            Error::InvalidCredentials => "invalid_credentials",
            Error::SessionNotFound => "session_not_found",
            Error::SessionExpired => "session_expired",
            Error::AccountLocked => "account_locked",
            Error::AccountBanned => "account_banned",
            Error::InvalidToken { .. } => "invalid_token",
            Error::TokenExpired => "token_expired",
            Error::PermissionDenied => "permission_denied",
            Error::NotRoomMember => "not_room_member",
            Error::NotMessageAuthor => "not_message_author",
            Error::ValidationFailed { .. } => "validation_failed",
            Error::UsernameInvalid { .. } => "username_invalid",
            Error::UsernameTaken => "username_taken",
            Error::EmailInvalid { .. } => "email_invalid",
            Error::EmailTaken => "email_taken",
            Error::PasswordTooWeak { .. } => "password_too_weak",
            Error::IncorrectPassword => "incorrect_password",
            Error::RoomNameInvalid { .. } => "room_name_invalid",
            Error::ContentInvalid { .. } => "content_invalid",
            Error::ContentEmpty => "content_empty",
            Error::ContentTooLong { .. } => "content_too_long",
            Error::UserNotFound => "user_not_found",
            Error::RoomNotFound => "room_not_found",
            Error::MessageNotFound => "message_not_found",
            Error::InvitationNotFound => "invitation_not_found",
            Error::AlreadyMember => "already_member",
            Error::NotInvitee => "not_invitee",
            Error::RoomNameTaken => "room_name_taken",
            Error::AlreadyInvited => "already_invited",
            Error::InvitationUsed => "invitation_used",
            Error::RoomFull => "room_full",
            Error::RoomPrivate => "room_private",
            Error::LastOwner => "last_owner",
            Error::MessageDeleted => "message_deleted",
            Error::InvitationExpired => "invitation_expired",
            Error::UserBlocked => "user_blocked",
            Error::RateLimited { .. } => "rate_limited",
            Error::Database(_) => "database_error",
            Error::Internal(_) => "internal_error",
            Error::Config(_) => "config_error",
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Error::InvalidCredentials => 401,
            Error::SessionNotFound | Error::SessionExpired => 401,
            Error::InvalidToken { .. } | Error::TokenExpired => 401,
            Error::AccountLocked | Error::AccountBanned => 403,
            Error::PermissionDenied | Error::NotRoomMember | Error::NotMessageAuthor => 403,
            Error::ValidationFailed { .. }
            | Error::UsernameInvalid { .. }
            | Error::EmailInvalid { .. }
            | Error::PasswordTooWeak { .. }
            | Error::IncorrectPassword
            | Error::RoomNameInvalid { .. }
            | Error::ContentInvalid { .. }
            | Error::ContentEmpty
            | Error::ContentTooLong { .. } => 400,
            Error::UsernameTaken | Error::EmailTaken => 409,
            Error::UserNotFound
            | Error::RoomNotFound
            | Error::MessageNotFound
            | Error::InvitationNotFound => 404,
            Error::AlreadyMember
            | Error::NotInvitee
            | Error::RoomNameTaken
            | Error::AlreadyInvited
            | Error::InvitationUsed => 409,
            Error::RoomFull
            | Error::RoomPrivate
            | Error::LastOwner
            | Error::MessageDeleted
            | Error::InvitationExpired
            | Error::UserBlocked => 409,
            Error::RateLimited { .. } => 429,
            Error::Database(_) | Error::Internal(_) => 500,
            Error::Config(_) => 500,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Error Code Tests
    // ========================================================================

    #[test]
    fn test_error_codes_authentication() {
        assert_eq!(Error::InvalidCredentials.code(), "invalid_credentials");
        assert_eq!(Error::SessionNotFound.code(), "session_not_found");
        assert_eq!(Error::SessionExpired.code(), "session_expired");
        assert_eq!(Error::AccountLocked.code(), "account_locked");
        assert_eq!(Error::AccountBanned.code(), "account_banned");
        assert_eq!(Error::InvalidToken { reason: "test".to_string() }.code(), "invalid_token");
        assert_eq!(Error::TokenExpired.code(), "token_expired");
    }

    #[test]
    fn test_error_codes_authorization() {
        assert_eq!(Error::PermissionDenied.code(), "permission_denied");
        assert_eq!(Error::NotRoomMember.code(), "not_room_member");
        assert_eq!(Error::NotMessageAuthor.code(), "not_message_author");
    }

    #[test]
    fn test_error_codes_validation() {
        assert_eq!(
            Error::ValidationFailed {
                field: "email".to_string(),
                reason: "invalid".to_string()
            }.code(),
            "validation_failed"
        );
        assert_eq!(Error::UsernameInvalid { reason: "too short".to_string() }.code(), "username_invalid");
        assert_eq!(Error::UsernameTaken.code(), "username_taken");
        assert_eq!(Error::EmailInvalid { reason: "invalid".to_string() }.code(), "email_invalid");
        assert_eq!(Error::EmailTaken.code(), "email_taken");
        assert_eq!(Error::PasswordTooWeak { reason: "too short".to_string() }.code(), "password_too_weak");
        assert_eq!(Error::IncorrectPassword.code(), "incorrect_password");
        assert_eq!(Error::RoomNameInvalid { reason: "empty".to_string() }.code(), "room_name_invalid");
        assert_eq!(Error::ContentInvalid { reason: "bad".to_string() }.code(), "content_invalid");
        assert_eq!(Error::ContentEmpty.code(), "content_empty");
        assert_eq!(Error::ContentTooLong { max: 1000 }.code(), "content_too_long");
    }

    #[test]
    fn test_error_codes_not_found() {
        assert_eq!(Error::UserNotFound.code(), "user_not_found");
        assert_eq!(Error::RoomNotFound.code(), "room_not_found");
        assert_eq!(Error::MessageNotFound.code(), "message_not_found");
        assert_eq!(Error::InvitationNotFound.code(), "invitation_not_found");
    }

    #[test]
    fn test_error_codes_conflict() {
        assert_eq!(Error::AlreadyMember.code(), "already_member");
        assert_eq!(Error::NotInvitee.code(), "not_invitee");
        assert_eq!(Error::RoomNameTaken.code(), "room_name_taken");
        assert_eq!(Error::AlreadyInvited.code(), "already_invited");
        assert_eq!(Error::InvitationUsed.code(), "invitation_used");
    }

    #[test]
    fn test_error_codes_state() {
        assert_eq!(Error::RoomFull.code(), "room_full");
        assert_eq!(Error::RoomPrivate.code(), "room_private");
        assert_eq!(Error::LastOwner.code(), "last_owner");
        assert_eq!(Error::MessageDeleted.code(), "message_deleted");
        assert_eq!(Error::InvitationExpired.code(), "invitation_expired");
        assert_eq!(Error::UserBlocked.code(), "user_blocked");
    }

    #[test]
    fn test_error_codes_rate_limit() {
        assert_eq!(Error::RateLimited { retry_after: 60 }.code(), "rate_limited");
    }

    #[test]
    fn test_error_codes_system() {
        assert_eq!(Error::Internal("test".to_string()).code(), "internal_error");
        assert_eq!(Error::Config("test".to_string()).code(), "config_error");
    }

    // ========================================================================
    // HTTP Status Code Tests
    // ========================================================================

    #[test]
    fn test_status_codes_401_unauthorized() {
        assert_eq!(Error::InvalidCredentials.status_code(), 401);
        assert_eq!(Error::SessionNotFound.status_code(), 401);
        assert_eq!(Error::SessionExpired.status_code(), 401);
        assert_eq!(Error::InvalidToken { reason: "test".to_string() }.status_code(), 401);
        assert_eq!(Error::TokenExpired.status_code(), 401);
    }

    #[test]
    fn test_status_codes_403_forbidden() {
        assert_eq!(Error::AccountLocked.status_code(), 403);
        assert_eq!(Error::AccountBanned.status_code(), 403);
        assert_eq!(Error::PermissionDenied.status_code(), 403);
        assert_eq!(Error::NotRoomMember.status_code(), 403);
        assert_eq!(Error::NotMessageAuthor.status_code(), 403);
    }

    #[test]
    fn test_status_codes_400_bad_request() {
        assert_eq!(
            Error::ValidationFailed {
                field: "x".to_string(),
                reason: "y".to_string()
            }.status_code(),
            400
        );
        assert_eq!(Error::UsernameInvalid { reason: "x".to_string() }.status_code(), 400);
        assert_eq!(Error::EmailInvalid { reason: "x".to_string() }.status_code(), 400);
        assert_eq!(Error::PasswordTooWeak { reason: "x".to_string() }.status_code(), 400);
        assert_eq!(Error::IncorrectPassword.status_code(), 400);
        assert_eq!(Error::RoomNameInvalid { reason: "x".to_string() }.status_code(), 400);
        assert_eq!(Error::ContentInvalid { reason: "x".to_string() }.status_code(), 400);
        assert_eq!(Error::ContentEmpty.status_code(), 400);
        assert_eq!(Error::ContentTooLong { max: 100 }.status_code(), 400);
    }

    #[test]
    fn test_status_codes_404_not_found() {
        assert_eq!(Error::UserNotFound.status_code(), 404);
        assert_eq!(Error::RoomNotFound.status_code(), 404);
        assert_eq!(Error::MessageNotFound.status_code(), 404);
        assert_eq!(Error::InvitationNotFound.status_code(), 404);
    }

    #[test]
    fn test_status_codes_409_conflict() {
        assert_eq!(Error::UsernameTaken.status_code(), 409);
        assert_eq!(Error::EmailTaken.status_code(), 409);
        assert_eq!(Error::AlreadyMember.status_code(), 409);
        assert_eq!(Error::NotInvitee.status_code(), 409);
        assert_eq!(Error::RoomNameTaken.status_code(), 409);
        assert_eq!(Error::AlreadyInvited.status_code(), 409);
        assert_eq!(Error::InvitationUsed.status_code(), 409);
        assert_eq!(Error::RoomFull.status_code(), 409);
        assert_eq!(Error::RoomPrivate.status_code(), 409);
        assert_eq!(Error::LastOwner.status_code(), 409);
        assert_eq!(Error::MessageDeleted.status_code(), 409);
        assert_eq!(Error::InvitationExpired.status_code(), 409);
        assert_eq!(Error::UserBlocked.status_code(), 409);
    }

    #[test]
    fn test_status_codes_429_rate_limited() {
        assert_eq!(Error::RateLimited { retry_after: 60 }.status_code(), 429);
    }

    #[test]
    fn test_status_codes_500_server_error() {
        assert_eq!(Error::Internal("test".to_string()).status_code(), 500);
        assert_eq!(Error::Config("test".to_string()).status_code(), 500);
    }

    // ========================================================================
    // Display Tests
    // ========================================================================

    #[test]
    fn test_error_display() {
        assert_eq!(Error::InvalidCredentials.to_string(), "Invalid credentials");
        assert_eq!(Error::RoomFull.to_string(), "Room is full");
        assert_eq!(Error::RateLimited { retry_after: 60 }.to_string(), "Rate limited, retry after 60 seconds");
        assert_eq!(
            Error::ValidationFailed {
                field: "email".to_string(),
                reason: "invalid format".to_string()
            }.to_string(),
            "Validation failed: email - invalid format"
        );
        assert_eq!(
            Error::InvalidToken { reason: "expired signature".to_string() }.to_string(),
            "Invalid token: expired signature"
        );
        assert_eq!(
            Error::ContentTooLong { max: 5000 }.to_string(),
            "Content too long (max 5000 characters)"
        );
    }
}
