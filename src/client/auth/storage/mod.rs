//! Token storage module for Lair-Chat client
//! Provides secure storage and retrieval of authentication tokens.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use super::types::{AuthError, AuthResult, Session, UserProfile};

/// Stored authentication data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuth {
    /// User profile information
    pub profile: UserProfile,
    /// Authentication session
    pub session: Session,
    /// When this data was stored
    pub stored_at: u64,
}

/// Token storage interface
#[async_trait::async_trait]
pub trait TokenStorage: Send + Sync {
    /// Save authentication data
    async fn save_auth(&self, auth: StoredAuth) -> AuthResult<()>;

    /// Load saved authentication data
    async fn load_auth(&self) -> AuthResult<Option<StoredAuth>>;

    /// Clear saved authentication data
    async fn clear_auth(&self) -> AuthResult<()>;
}

/// File-based token storage implementation
#[derive(Clone)]
pub struct FileTokenStorage {
    auth_file: PathBuf,
}

impl FileTokenStorage {
    /// Create a new file-based token storage
    pub fn new() -> AuthResult<Self> {
        let project_dirs = ProjectDirs::from("com", "lair-chat", "lair-chat").ok_or_else(|| {
            AuthError::InternalError("Could not determine project directories".into())
        })?;

        let config_dir = project_dirs.config_dir();

        // Ensure config directory exists
        std::fs::create_dir_all(config_dir).map_err(|e| {
            AuthError::InternalError(format!("Failed to create config directory: {}", e))
        })?;

        // Create process-specific auth file to avoid conflicts between multiple client instances
        let process_id = std::process::id();
        let auth_file = config_dir.join(format!("auth_{}.json", process_id));

        Ok(Self { auth_file })
    }

    /// Get the path to the auth file
    pub fn auth_file_path(&self) -> &PathBuf {
        &self.auth_file
    }
}

#[async_trait::async_trait]
impl TokenStorage for FileTokenStorage {
    async fn save_auth(&self, auth: StoredAuth) -> AuthResult<()> {
        // Serialize auth data
        let json = serde_json::to_string_pretty(&auth).map_err(|e| {
            AuthError::InternalError(format!("Failed to serialize auth data: {}", e))
        })?;

        // Write to file
        fs::write(&self.auth_file, json)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to write auth file: {}", e)))?;

        Ok(())
    }

    async fn load_auth(&self) -> AuthResult<Option<StoredAuth>> {
        // Check if file exists
        if !self.auth_file.exists() {
            return Ok(None);
        }

        // Read file contents
        let json = fs::read_to_string(&self.auth_file)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to read auth file: {}", e)))?;

        // Deserialize auth data
        let auth: StoredAuth = serde_json::from_str(&json)
            .map_err(|e| AuthError::InternalError(format!("Failed to parse auth data: {}", e)))?;

        // Verify session hasn't expired
        if auth.session.is_expired() {
            self.clear_auth().await?;
            return Ok(None);
        }

        Ok(Some(auth))
    }

    async fn clear_auth(&self) -> AuthResult<()> {
        // Remove the auth file if it exists
        if self.auth_file.exists() {
            fs::remove_file(&self.auth_file).await.map_err(|e| {
                AuthError::InternalError(format!("Failed to remove auth file: {}", e))
            })?;
        }

        Ok(())
    }
}

/// Memory-based token storage implementation (for testing)
#[cfg(test)]
pub struct MemoryTokenStorage {
    auth: tokio::sync::RwLock<Option<StoredAuth>>,
}

#[cfg(test)]
impl MemoryTokenStorage {
    pub fn new() -> Self {
        Self {
            auth: tokio::sync::RwLock::new(None),
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl TokenStorage for MemoryTokenStorage {
    async fn save_auth(&self, auth: StoredAuth) -> AuthResult<()> {
        *self.auth.write().await = Some(auth);
        Ok(())
    }

    async fn load_auth(&self) -> AuthResult<Option<StoredAuth>> {
        Ok(self.auth.read().await.clone())
    }

    async fn clear_auth(&self) -> AuthResult<()> {
        *self.auth.write().await = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    fn create_test_auth() -> StoredAuth {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        StoredAuth {
            profile: UserProfile {
                id: Uuid::new_v4(),
                username: "testuser".to_string(),
                roles: vec!["user".to_string()],
            },
            session: Session {
                id: Uuid::new_v4(),
                token: "test_token".to_string(),
                created_at: now,
                expires_at: now + 3600,
            },
            stored_at: now,
        }
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryTokenStorage::new();

        // Initially empty
        assert!(storage.load_auth().await.unwrap().is_none());

        // Save auth data
        let auth = create_test_auth();
        storage.save_auth(auth.clone()).await.unwrap();

        // Load and verify
        let loaded = storage.load_auth().await.unwrap().unwrap();
        assert_eq!(loaded.profile.username, auth.profile.username);
        assert_eq!(loaded.session.token, auth.session.token);

        // Clear auth
        storage.clear_auth().await.unwrap();
        assert!(storage.load_auth().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let auth_file = temp_dir.path().join("auth.json");

        let storage = FileTokenStorage {
            auth_file: auth_file.clone(),
        };

        // Initially empty
        assert!(storage.load_auth().await.unwrap().is_none());

        // Save auth data
        let auth = create_test_auth();
        storage.save_auth(auth.clone()).await.unwrap();

        // Verify file exists
        assert!(auth_file.exists());

        // Load and verify
        let loaded = storage.load_auth().await.unwrap().unwrap();
        assert_eq!(loaded.profile.username, auth.profile.username);
        assert_eq!(loaded.session.token, auth.session.token);

        // Clear auth
        storage.clear_auth().await.unwrap();
        assert!(!auth_file.exists());
    }
}
