//! SQLite storage implementation
pub mod users;
pub mod rooms;
pub mod messages;
pub mod sessions;
pub mod invitations;

/// SQLite-based storage implementation
pub struct SqliteStorage;

impl SqliteStorage {
    pub async fn new(_url: &str) -> crate::Result<Self> {
        todo!("Implement SQLite connection")
    }
}
