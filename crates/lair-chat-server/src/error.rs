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
