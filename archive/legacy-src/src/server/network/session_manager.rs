//! Session manager for lair-chat server
//!
//! This module contains session management functionality for tracking active
//! user sessions, handling session lifecycle, and managing session state.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Session identifier
pub type SessionId = Uuid;

/// Active session information
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Username associated with this session
    pub username: String,
    /// Socket address of the connection
    pub address: SocketAddr,
    /// Session creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Whether the session is authenticated
    pub authenticated: bool,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

impl Session {
    /// Create a new session
    pub fn new(username: String, address: SocketAddr) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4(),
            username,
            address,
            created_at: now,
            last_activity: now,
            authenticated: false,
            metadata: HashMap::new(),
        }
    }

    /// Update the session's last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Mark the session as authenticated
    pub fn authenticate(&mut self) {
        self.authenticated = true;
        self.update_activity();
    }

    /// Check if the session has expired
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_activity > timeout_seconds
    }

    /// Get session duration in seconds
    pub fn duration(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.created_at
    }
}

/// Session manager for tracking active sessions
pub struct SessionManager {
    /// Sessions indexed by session ID
    sessions: HashMap<SessionId, Session>,
    /// Sessions indexed by socket address
    sessions_by_addr: HashMap<SocketAddr, SessionId>,
    /// Sessions indexed by username
    sessions_by_user: HashMap<String, SessionId>,
    /// Session timeout in seconds
    timeout_seconds: u64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            sessions_by_addr: HashMap::new(),
            sessions_by_user: HashMap::new(),
            timeout_seconds: 3600, // 1 hour default
        }
    }

    /// Set the session timeout
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Create a new session
    pub fn create_session(&mut self, username: String, address: SocketAddr) -> SessionId {
        let session = Session::new(username.clone(), address);
        let session_id = session.id;

        // Remove any existing session for this user or address
        self.remove_session_by_user(&username);
        self.remove_session_by_addr(&address);

        // Add the new session
        self.sessions_by_addr.insert(address, session_id);
        self.sessions_by_user.insert(username, session_id);
        self.sessions.insert(session_id, session);

        session_id
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &SessionId) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Get a mutable reference to a session by ID
    pub fn get_session_mut(&mut self, session_id: &SessionId) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    /// Get a session by socket address
    pub fn get_session_by_addr(&self, address: &SocketAddr) -> Option<&Session> {
        if let Some(session_id) = self.sessions_by_addr.get(address) {
            self.sessions.get(session_id)
        } else {
            None
        }
    }

    /// Get a session by username
    pub fn get_session_by_user(&self, username: &str) -> Option<&Session> {
        if let Some(session_id) = self.sessions_by_user.get(username) {
            self.sessions.get(session_id)
        } else {
            None
        }
    }

    /// Remove a session by ID
    pub fn remove_session(&mut self, session_id: &SessionId) -> Option<Session> {
        if let Some(session) = self.sessions.remove(session_id) {
            self.sessions_by_addr.remove(&session.address);
            self.sessions_by_user.remove(&session.username);
            Some(session)
        } else {
            None
        }
    }

    /// Remove a session by socket address
    pub fn remove_session_by_addr(&mut self, address: &SocketAddr) -> Option<Session> {
        if let Some(session_id) = self.sessions_by_addr.remove(address) {
            self.remove_session(&session_id)
        } else {
            None
        }
    }

    /// Remove a session by username
    pub fn remove_session_by_user(&mut self, username: &str) -> Option<Session> {
        if let Some(session_id) = self.sessions_by_user.remove(username) {
            self.remove_session(&session_id)
        } else {
            None
        }
    }

    /// Update session activity
    pub fn update_activity(&mut self, session_id: &SessionId) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.update_activity();
            true
        } else {
            false
        }
    }

    /// Update session activity by address
    pub fn update_activity_by_addr(&mut self, address: &SocketAddr) -> bool {
        if let Some(session_id) = self.sessions_by_addr.get(address) {
            let session_id = *session_id;
            self.update_activity(&session_id)
        } else {
            false
        }
    }

    /// Authenticate a session
    pub fn authenticate_session(&mut self, session_id: &SessionId) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.authenticate();
            true
        } else {
            false
        }
    }

    /// Get all active sessions
    pub fn get_all_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// Get expired sessions
    pub fn get_expired_sessions(&self) -> Vec<SessionId> {
        self.sessions
            .iter()
            .filter(|(_, session)| session.is_expired(self.timeout_seconds))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&mut self) -> Vec<Session> {
        let expired_ids = self.get_expired_sessions();
        let mut removed_sessions = Vec::new();

        for session_id in expired_ids {
            if let Some(session) = self.remove_session(&session_id) {
                removed_sessions.push(session);
            }
        }

        removed_sessions
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        let total = self.sessions.len();
        let authenticated = self.sessions.values().filter(|s| s.authenticated).count();
        let expired = self
            .sessions
            .values()
            .filter(|s| s.is_expired(self.timeout_seconds))
            .count();

        SessionStats {
            total_sessions: total,
            authenticated_sessions: authenticated,
            expired_sessions: expired,
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub authenticated_sessions: usize,
    pub expired_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let session = Session::new("alice".to_string(), addr);

        assert_eq!(session.username, "alice");
        assert_eq!(session.address, addr);
        assert!(!session.authenticated);
    }

    #[test]
    fn test_session_activity() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let mut session = Session::new("alice".to_string(), addr);

        let initial_activity = session.last_activity;
        std::thread::sleep(std::time::Duration::from_millis(10));

        session.update_activity();
        assert!(session.last_activity > initial_activity);
    }

    #[test]
    fn test_session_manager_basic_operations() {
        let mut manager = SessionManager::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        // Create session
        let session_id = manager.create_session("alice".to_string(), addr);
        assert_eq!(manager.session_count(), 1);

        // Get session
        assert!(manager.get_session(&session_id).is_some());
        assert!(manager.get_session_by_addr(&addr).is_some());
        assert!(manager.get_session_by_user("alice").is_some());

        // Remove session
        assert!(manager.remove_session(&session_id).is_some());
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_authentication() {
        let mut manager = SessionManager::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        let session_id = manager.create_session("alice".to_string(), addr);
        assert!(!manager.get_session(&session_id).unwrap().authenticated);

        assert!(manager.authenticate_session(&session_id));
        assert!(manager.get_session(&session_id).unwrap().authenticated);
    }

    #[test]
    fn test_session_expiration() {
        let mut manager = SessionManager::new();
        manager.set_timeout(1); // 1 second timeout

        let addr = "127.0.0.1:8080".parse().unwrap();
        let session_id = manager.create_session("alice".to_string(), addr);

        // Should not be expired immediately
        assert!(manager.get_expired_sessions().is_empty());

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should now be expired
        let expired = manager.get_expired_sessions();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], session_id);
    }
}
