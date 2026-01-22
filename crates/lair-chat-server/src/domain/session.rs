//! Session domain types.
//!
//! See [DOMAIN_MODEL.md](../../../../docs/architecture/DOMAIN_MODEL.md) for full specification.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

use super::{UserId, ValidationError};

// ============================================================================
// SessionId
// ============================================================================

/// Unique identifier for an active session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random SessionId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a SessionId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a SessionId from a string.
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

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SessionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ============================================================================
// Protocol
// ============================================================================

/// The protocol used to connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// TCP socket connection.
    Tcp,
    /// HTTP REST API.
    Http,
    /// WebSocket connection.
    WebSocket,
}

impl Protocol {
    /// Check if this protocol supports real-time push notifications.
    pub fn supports_push(&self) -> bool {
        matches!(self, Protocol::Tcp | Protocol::WebSocket)
    }

    /// Check if this protocol is stateful (maintains connection).
    pub fn is_stateful(&self) -> bool {
        matches!(self, Protocol::Tcp | Protocol::WebSocket)
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Http => write!(f, "http"),
            Protocol::WebSocket => write!(f, "websocket"),
        }
    }
}

// ============================================================================
// Session
// ============================================================================

/// An authenticated session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier.
    pub id: SessionId,
    /// The authenticated user.
    pub user_id: UserId,
    /// Which protocol created this session.
    pub protocol: Protocol,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session expires.
    pub expires_at: DateTime<Utc>,
    /// Last activity timestamp.
    pub last_active: DateTime<Utc>,
}

impl Session {
    /// Default session duration (24 hours).
    pub const DEFAULT_DURATION: Duration = Duration::hours(24);

    /// Extended session duration for "remember me" (30 days).
    pub const EXTENDED_DURATION: Duration = Duration::days(30);

    /// Create a new session with default expiration.
    pub fn new(user_id: UserId, protocol: Protocol) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id,
            protocol,
            created_at: now,
            expires_at: now + Self::DEFAULT_DURATION,
            last_active: now,
        }
    }

    /// Create a new session with extended expiration.
    pub fn new_extended(user_id: UserId, protocol: Protocol) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id,
            protocol,
            created_at: now,
            expires_at: now + Self::EXTENDED_DURATION,
            last_active: now,
        }
    }

    /// Create a new session with custom duration.
    pub fn with_duration(user_id: UserId, protocol: Protocol, duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id,
            protocol,
            created_at: now,
            expires_at: now + duration,
            last_active: now,
        }
    }

    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the session is still valid.
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Update the last activity timestamp.
    pub fn touch(&mut self) {
        self.last_active = Utc::now();
    }

    /// Extend the session expiration.
    pub fn extend(&mut self, duration: Duration) {
        self.expires_at = self.expires_at + duration;
    }

    /// Extend the session to the default duration from now.
    pub fn refresh(&mut self) {
        let now = Utc::now();
        self.last_active = now;
        self.expires_at = now + Self::DEFAULT_DURATION;
    }

    /// Get the time remaining until expiration.
    pub fn time_remaining(&self) -> Duration {
        let remaining = self.expires_at - Utc::now();
        if remaining < Duration::zero() {
            Duration::zero()
        } else {
            remaining
        }
    }

    /// Check if the session supports push notifications.
    pub fn supports_push(&self) -> bool {
        self.protocol.supports_push()
    }

    /// Check if the session protocol is stateful.
    pub fn is_stateful(&self) -> bool {
        self.protocol.is_stateful()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);

        let parsed = SessionId::parse(&id1.to_string()).unwrap();
        assert_eq!(id1, parsed);

        assert!(SessionId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn test_protocol_properties() {
        assert!(Protocol::Tcp.supports_push());
        assert!(Protocol::WebSocket.supports_push());
        assert!(!Protocol::Http.supports_push());

        assert!(Protocol::Tcp.is_stateful());
        assert!(Protocol::WebSocket.is_stateful());
        assert!(!Protocol::Http.is_stateful());
    }

    #[test]
    fn test_session_creation() {
        let user_id = UserId::new();
        let session = Session::new(user_id, Protocol::Tcp);

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.protocol, Protocol::Tcp);
        assert!(session.is_valid());
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_extended() {
        let user_id = UserId::new();
        let normal = Session::new(user_id, Protocol::Http);
        let extended = Session::new_extended(user_id, Protocol::Http);

        assert!(extended.expires_at > normal.expires_at);
    }

    #[test]
    fn test_session_custom_duration() {
        let user_id = UserId::new();
        let duration = Duration::hours(1);
        let session = Session::with_duration(user_id, Protocol::Http, duration);

        let remaining = session.time_remaining();
        // Should be close to 1 hour (allowing for test execution time)
        assert!(remaining > Duration::minutes(59));
        assert!(remaining <= Duration::hours(1));
    }

    #[test]
    fn test_session_touch() {
        let user_id = UserId::new();
        let mut session = Session::new(user_id, Protocol::Tcp);
        let original_last_active = session.last_active;

        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_active > original_last_active);
    }

    #[test]
    fn test_session_refresh() {
        let user_id = UserId::new();
        let mut session = Session::with_duration(user_id, Protocol::Tcp, Duration::hours(1));
        let original_expires = session.expires_at;

        session.refresh();

        // After refresh, should expire in ~24 hours (default), not 1 hour
        assert!(session.expires_at > original_expires);
    }

    #[test]
    fn test_session_expired() {
        let user_id = UserId::new();
        // Create session that expires immediately
        let session = Session::with_duration(user_id, Protocol::Http, Duration::seconds(-1));

        assert!(session.is_expired());
        assert!(!session.is_valid());
        assert_eq!(session.time_remaining(), Duration::zero());
    }

    #[test]
    fn test_protocol_serialization() {
        let tcp = Protocol::Tcp;
        let json = serde_json::to_string(&tcp).unwrap();
        assert_eq!(json, "\"tcp\"");

        let http = Protocol::Http;
        let json = serde_json::to_string(&http).unwrap();
        assert_eq!(json, "\"http\"");

        let ws = Protocol::WebSocket;
        let json = serde_json::to_string(&ws).unwrap();
        assert_eq!(json, "\"websocket\"");
    }
}
