//! Shared types between TCP and REST servers for integrated monitoring
//!
//! This module contains types that need to be shared between the TCP chat server
//! and the REST API server to enable integrated monitoring and administration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
/// Write data tuple containing message bytes and response channel
pub type WriteData = (Vec<u8>, Tx<String>);

/// Data that is shared between all peers in the TCP chat server.
#[derive(Debug)]
pub struct TcpServerState {
    /// Connected TCP peers by their socket address
    pub peers: HashMap<SocketAddr, WriteData>,
    /// Map of authenticated usernames to their connection info
    pub connected_users: HashMap<String, ConnectedUser>,
    /// Active chat rooms
    pub rooms: HashMap<String, Room>,
    /// Pending room invitations
    pub pending_invitations: HashMap<String, Vec<PendingInvitation>>,
}

impl TcpServerState {
    /// Create new empty TCP server state
    pub fn new() -> Self {
        let mut state = Self {
            peers: HashMap::new(),
            connected_users: HashMap::new(),
            rooms: HashMap::new(),
            pending_invitations: HashMap::new(),
        };

        // Create default lobby room
        state
            .rooms
            .insert("lobby".to_string(), Room::new("lobby".to_string(), true));

        state
    }

    /// Get count of connected TCP peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get count of authenticated TCP users
    pub fn authenticated_user_count(&self) -> usize {
        self.connected_users.len()
    }

    /// Get count of active rooms
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Get count of pending invitations
    pub fn pending_invitation_count(&self) -> usize {
        self.pending_invitations.values().map(|v| v.len()).sum()
    }

    /// Get statistics for monitoring
    pub fn get_stats(&self) -> TcpServerStats {
        let mut room_user_counts = HashMap::new();
        for (room_name, room) in &self.rooms {
            room_user_counts.insert(room_name.clone(), room.users.len());
        }

        TcpServerStats {
            connected_peers: self.peer_count(),
            authenticated_users: self.authenticated_user_count(),
            active_rooms: self.room_count(),
            pending_invitations: self.pending_invitation_count(),
            room_user_counts,
            uptime_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Add a new peer connection
    pub fn add_peer(&mut self, addr: SocketAddr, write_data: WriteData) {
        self.peers.insert(addr, write_data);
    }

    /// Remove a peer connection
    pub fn remove_peer(&mut self, addr: &SocketAddr) -> Option<WriteData> {
        self.peers.remove(addr)
    }

    /// Add an authenticated user
    pub fn add_user(&mut self, username: String, user: ConnectedUser) {
        self.connected_users.insert(username, user);
    }

    /// Remove an authenticated user
    pub fn remove_user(&mut self, username: &str) -> Option<ConnectedUser> {
        self.connected_users.remove(username)
    }

    /// Get a room by name
    pub fn get_room(&self, name: &str) -> Option<&Room> {
        self.rooms.get(name)
    }

    /// Get a mutable room by name
    pub fn get_room_mut(&mut self, name: &str) -> Option<&mut Room> {
        self.rooms.get_mut(name)
    }

    /// Create a new room
    pub fn create_room(&mut self, name: String, is_lobby: bool) -> &mut Room {
        let room = Room::new(name.clone(), is_lobby);
        self.rooms.insert(name.clone(), room);
        self.rooms.get_mut(&name).unwrap()
    }

    /// Remove a room
    pub fn remove_room(&mut self, name: &str) -> Option<Room> {
        self.rooms.remove(name)
    }
}

/// Statistics about the TCP server for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpServerStats {
    /// Number of connected TCP peers (raw connections)
    pub connected_peers: usize,
    /// Number of authenticated users
    pub authenticated_users: usize,
    /// Number of active rooms
    pub active_rooms: usize,
    /// Number of pending invitations
    pub pending_invitations: usize,
    /// User count per room
    pub room_user_counts: HashMap<String, usize>,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

/// Information about a connected TCP user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    /// Username of the connected user
    pub username: String,
    /// Socket address of the user's connection
    pub address: SocketAddr,
    /// Timestamp when user connected (Unix epoch)
    pub connected_at: u64,
    /// Current room the user is in
    pub current_room: String,
}

impl ConnectedUser {
    /// Create new connected user
    pub fn new(username: String, address: SocketAddr, current_room: String) -> Self {
        let connected_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            username,
            address,
            connected_at,
            current_room,
        }
    }

    /// Get connection duration in seconds
    pub fn connection_duration(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.connected_at)
    }
}

/// A chat room in the TCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Name of the room
    pub name: String,
    /// List of usernames in the room
    pub users: Vec<String>,
    /// Timestamp when room was created (Unix epoch)
    pub created_at: u64,
    /// Whether this is the default lobby room
    pub is_lobby: bool,
}

impl Room {
    /// Create a new room
    pub fn new(name: String, is_lobby: bool) -> Self {
        Self {
            name,
            users: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_lobby,
        }
    }

    /// Add a user to the room
    pub fn add_user(&mut self, username: String) {
        if !self.users.contains(&username) {
            self.users.push(username);
        }
    }

    /// Remove a user from the room
    pub fn remove_user(&mut self, username: &str) {
        self.users.retain(|u| u != username);
    }

    /// Check if user is in the room
    pub fn has_user(&self, username: &str) -> bool {
        self.users.contains(&username.to_string())
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Get room age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.created_at)
    }
}

/// A pending room invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInvitation {
    /// Username who sent the invitation
    pub inviter: String,
    /// Name of the room being invited to
    pub room_name: String,
    /// Timestamp when invitation was sent (Unix epoch)
    pub invited_at: u64,
}

impl PendingInvitation {
    /// Create a new pending invitation
    pub fn new(inviter: String, room_name: String) -> Self {
        Self {
            inviter,
            room_name,
            invited_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if invitation has expired (older than 1 hour)
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.invited_at) > 3600 // 1 hour
    }

    /// Get invitation age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.invited_at)
    }
}

/// Type alias for shared TCP server state
pub type SharedTcpState = Arc<Mutex<TcpServerState>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_server_state_creation() {
        let state = TcpServerState::new();
        assert_eq!(state.peer_count(), 0);
        assert_eq!(state.authenticated_user_count(), 0);
        assert_eq!(state.room_count(), 1); // lobby room
        assert!(state.get_room("lobby").is_some());
    }

    #[test]
    fn test_room_operations() {
        let mut room = Room::new("test".to_string(), false);
        assert_eq!(room.user_count(), 0);

        room.add_user("alice".to_string());
        assert_eq!(room.user_count(), 1);
        assert!(room.has_user("alice"));

        room.add_user("alice".to_string()); // duplicate
        assert_eq!(room.user_count(), 1);

        room.add_user("bob".to_string());
        assert_eq!(room.user_count(), 2);

        room.remove_user("alice");
        assert_eq!(room.user_count(), 1);
        assert!(!room.has_user("alice"));
        assert!(room.has_user("bob"));
    }

    #[test]
    fn test_connected_user() {
        use std::net::{IpAddr, Ipv4Addr};
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let user = ConnectedUser::new("alice".to_string(), addr, "lobby".to_string());

        assert_eq!(user.username, "alice");
        assert_eq!(user.address, addr);
        assert_eq!(user.current_room, "lobby");
        assert!(user.connection_duration() < 1); // Should be very small
    }

    #[test]
    fn test_pending_invitation() {
        let invitation = PendingInvitation::new("alice".to_string(), "test-room".to_string());
        assert_eq!(invitation.inviter, "alice");
        assert_eq!(invitation.room_name, "test-room");
        assert!(!invitation.is_expired());
        assert!(invitation.age_seconds() < 1);
    }

    #[test]
    fn test_tcp_server_stats() {
        let mut state = TcpServerState::new();

        // Add some test data
        use std::net::{IpAddr, Ipv4Addr};
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let user = ConnectedUser::new("alice".to_string(), addr, "lobby".to_string());
        state.add_user("alice".to_string(), user);

        let stats = state.get_stats();
        assert_eq!(stats.authenticated_users, 1);
        assert_eq!(stats.active_rooms, 1);
        assert_eq!(stats.connected_peers, 0);
    }
}
