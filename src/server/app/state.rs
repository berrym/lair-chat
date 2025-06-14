//! Server state management for lair-chat
//!
//! This module contains the shared state structures and management logic
//! for the lair-chat server, including user connections, rooms, and session management.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::server::auth::AuthService;

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
pub type WriteData = (Vec<u8>, Tx<String>);

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
pub struct SharedState {
    pub peers: HashMap<SocketAddr, WriteData>,
    pub auth_service: Arc<AuthService>,
    pub connected_users: HashMap<String, ConnectedUser>,
    pub rooms: HashMap<String, Room>,
}

#[derive(Debug, Clone)]
pub struct ConnectedUser {
    pub username: String,
    pub address: SocketAddr,
    pub connected_at: u64,
    pub current_room: String,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub users: Vec<String>,
    pub created_at: u64,
    pub is_lobby: bool,
}

impl SharedState {
    /// Create a new shared state instance
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        let mut rooms = HashMap::new();

        // Create default lobby room
        rooms.insert(
            "lobby".to_string(),
            Room {
                name: "lobby".to_string(),
                users: Vec::new(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                is_lobby: true,
            },
        );

        Self {
            peers: HashMap::new(),
            auth_service,
            connected_users: HashMap::new(),
            rooms,
        }
    }

    /// Add a new peer connection
    pub fn add_peer(&mut self, addr: SocketAddr, tx: Tx<String>) {
        self.peers.insert(addr, (Vec::new(), tx));
    }

    /// Remove a peer connection
    pub fn remove_peer(&mut self, addr: &SocketAddr) -> Option<WriteData> {
        self.peers.remove(addr)
    }

    /// Get all peer addresses
    pub fn get_peers(&self) -> Vec<SocketAddr> {
        self.peers.keys().cloned().collect()
    }

    /// Add a connected user
    pub fn add_user(&mut self, username: String, addr: SocketAddr) {
        let user = ConnectedUser {
            username: username.clone(),
            address: addr,
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            current_room: "lobby".to_string(),
        };

        self.connected_users.insert(username.clone(), user);

        // Add user to lobby room
        if let Some(lobby) = self.rooms.get_mut("lobby") {
            if !lobby.users.contains(&username) {
                lobby.users.push(username);
            }
        }
    }

    /// Remove a connected user
    pub fn remove_user(&mut self, username: &str) -> Option<ConnectedUser> {
        if let Some(user) = self.connected_users.remove(username) {
            // Remove user from their current room
            if let Some(room) = self.rooms.get_mut(&user.current_room) {
                room.users.retain(|u| u != username);
            }
            Some(user)
        } else {
            None
        }
    }

    /// Get a connected user by username
    pub fn get_user(&self, username: &str) -> Option<&ConnectedUser> {
        self.connected_users.get(username)
    }

    /// Get all connected users
    pub fn get_all_users(&self) -> Vec<&ConnectedUser> {
        self.connected_users.values().collect()
    }

    /// Create a new room
    pub fn create_room(&mut self, name: String) -> bool {
        if self.rooms.contains_key(&name) {
            false
        } else {
            let room = Room {
                name: name.clone(),
                users: Vec::new(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                is_lobby: false,
            };
            self.rooms.insert(name, room);
            true
        }
    }

    /// Get a room by name
    pub fn get_room(&self, name: &str) -> Option<&Room> {
        self.rooms.get(name)
    }

    /// Get all rooms
    pub fn get_all_rooms(&self) -> Vec<&Room> {
        self.rooms.values().collect()
    }

    /// Move user to a different room
    pub fn move_user_to_room(&mut self, username: &str, room_name: &str) -> bool {
        if let Some(user) = self.connected_users.get_mut(username) {
            // Remove user from current room
            if let Some(current_room) = self.rooms.get_mut(&user.current_room) {
                current_room.users.retain(|u| u != username);
            }

            // Add user to new room if it exists
            if let Some(new_room) = self.rooms.get_mut(room_name) {
                new_room.users.push(username.to_string());
                user.current_room = room_name.to_string();
                true
            } else {
                // If room doesn't exist, put user back in lobby
                if let Some(lobby) = self.rooms.get_mut("lobby") {
                    lobby.users.push(username.to_string());
                    user.current_room = "lobby".to_string();
                }
                false
            }
        } else {
            false
        }
    }

    /// Get users in a specific room
    pub fn get_room_users(&self, room_name: &str) -> Vec<String> {
        self.rooms
            .get(room_name)
            .map(|room| room.users.clone())
            .unwrap_or_default()
    }

    /// Broadcast a message to all peers except the sender
    pub fn broadcast_message(&self, sender_addr: &SocketAddr, message: &str) {
        for (addr, (_, tx)) in &self.peers {
            if addr != sender_addr {
                let _ = tx.send(message.to_string());
            }
        }
    }

    /// Send a message to a specific peer
    pub fn send_to_peer(&self, addr: &SocketAddr, message: &str) -> bool {
        if let Some((_, tx)) = self.peers.get(addr) {
            tx.send(message.to_string()).is_ok()
        } else {
            false
        }
    }

    /// Send a message to users in a specific room
    pub fn broadcast_to_room(&self, room_name: &str, message: &str) {
        if let Some(room) = self.rooms.get(room_name) {
            for username in &room.users {
                if let Some(user) = self.connected_users.get(username) {
                    if let Some((_, tx)) = self.peers.get(&user.address) {
                        let _ = tx.send(message.to_string());
                    }
                }
            }
        }
    }

    /// Get statistics about the server state
    pub fn get_stats(&self) -> ServerStats {
        ServerStats {
            total_peers: self.peers.len(),
            total_users: self.connected_users.len(),
            total_rooms: self.rooms.len(),
            rooms_with_users: self.rooms.values().filter(|r| !r.users.is_empty()).count(),
        }
    }
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub total_peers: usize,
    pub total_users: usize,
    pub total_rooms: usize,
    pub rooms_with_users: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::auth::{MemorySessionStorage, MemoryUserStorage};

    fn create_test_state() -> SharedState {
        let user_storage = Arc::new(MemoryUserStorage::new());
        let session_storage = Arc::new(MemorySessionStorage::new());
        let auth_service = Arc::new(AuthService::new(user_storage, session_storage));
        SharedState::new(auth_service)
    }

    #[test]
    fn test_shared_state_creation() {
        let state = create_test_state();
        assert_eq!(state.peers.len(), 0);
        assert_eq!(state.connected_users.len(), 0);
        assert_eq!(state.rooms.len(), 1); // Default lobby room
        assert!(state.rooms.contains_key("lobby"));
    }

    #[test]
    fn test_user_management() {
        let mut state = create_test_state();
        let addr = "127.0.0.1:8080".parse().unwrap();

        // Add user
        state.add_user("alice".to_string(), addr);
        assert_eq!(state.connected_users.len(), 1);
        assert!(state.get_user("alice").is_some());

        // Check user is in lobby
        let lobby_users = state.get_room_users("lobby");
        assert!(lobby_users.contains(&"alice".to_string()));

        // Remove user
        let removed = state.remove_user("alice");
        assert!(removed.is_some());
        assert_eq!(state.connected_users.len(), 0);
        assert!(state.get_room_users("lobby").is_empty());
    }

    #[test]
    fn test_room_management() {
        let mut state = create_test_state();

        // Create room
        assert!(state.create_room("general".to_string()));
        assert_eq!(state.rooms.len(), 2); // lobby + general

        // Can't create duplicate room
        assert!(!state.create_room("general".to_string()));

        // Get room
        assert!(state.get_room("general").is_some());
        assert!(state.get_room("nonexistent").is_none());
    }

    #[test]
    fn test_user_room_movement() {
        let mut state = create_test_state();
        let addr = "127.0.0.1:8080".parse().unwrap();

        // Add user and room
        state.add_user("alice".to_string(), addr);
        state.create_room("general".to_string());

        // Move user to room
        assert!(state.move_user_to_room("alice", "general"));
        assert_eq!(state.get_user("alice").unwrap().current_room, "general");
        assert!(state
            .get_room_users("general")
            .contains(&"alice".to_string()));
        assert!(state.get_room_users("lobby").is_empty());

        // Move to nonexistent room should put user back in lobby
        assert!(!state.move_user_to_room("alice", "nonexistent"));
        assert_eq!(state.get_user("alice").unwrap().current_room, "lobby");
        assert!(state.get_room_users("lobby").contains(&"alice".to_string()));
    }

    #[test]
    fn test_server_stats() {
        let mut state = create_test_state();
        let addr = "127.0.0.1:8080".parse().unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();

        state.add_peer(addr, tx);
        state.add_user("alice".to_string(), addr);
        state.create_room("general".to_string());

        let stats = state.get_stats();
        assert_eq!(stats.total_peers, 1);
        assert_eq!(stats.total_users, 1);
        assert_eq!(stats.total_rooms, 2); // lobby + general
        assert_eq!(stats.rooms_with_users, 1); // lobby has alice
    }
}
