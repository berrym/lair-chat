//! Command history module for Lair-Chat
//! Provides persistent storage and management of command history.

use std::path::PathBuf;
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use tokio::fs;
use thiserror::Error;

/// Maximum number of commands to keep in history
const MAX_HISTORY_SIZE: usize = 1000;

/// Error types for history operations
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Failed to create history directory: {0}")]
    DirectoryCreation(std::io::Error),

    #[error("Failed to read history file: {0}")]
    FileRead(std::io::Error),

    #[error("Failed to write history file: {0}")]
    FileWrite(std::io::Error),

    #[error("Failed to serialize history: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),
}

/// Result type for history operations
pub type HistoryResult<T> = Result<T, HistoryError>;

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The command text
    pub command: String,
    /// When the command was executed
    pub timestamp: u64,
    /// The context (e.g., chat room) where the command was executed
    pub context: Option<String>,
}

/// Command history manager
pub struct CommandHistory {
    /// The command history entries
    entries: VecDeque<HistoryEntry>,
    /// Current position when navigating history
    position: Option<usize>,
    /// Path to the history file
    history_file: PathBuf,
}

impl CommandHistory {
    /// Create a new command history manager
    pub fn new() -> HistoryResult<Self> {
        let project_dirs = ProjectDirs::from("com", "lair-chat", "lair-chat")
            .ok_or_else(|| HistoryError::System("Could not determine project directories".into()))?;

        let data_dir = project_dirs.data_dir();
        std::fs::create_dir_all(data_dir)
            .map_err(HistoryError::DirectoryCreation)?;

        let history_file = data_dir.join("command_history.json");

        Ok(Self {
            entries: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            position: None,
            history_file,
        })
    }

    /// Load history from disk
    pub async fn load(&mut self) -> HistoryResult<()> {
        if !self.history_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.history_file)
            .await
            .map_err(HistoryError::FileRead)?;

        let entries: Vec<HistoryEntry> = serde_json::from_str(&content)?;
        self.entries = VecDeque::from(entries);
        self.position = None;

        Ok(())
    }

    /// Save history to disk
    pub async fn save(&self) -> HistoryResult<()> {
        let json = serde_json::to_string_pretty(&Vec::from(self.entries.clone()))?;
        fs::write(&self.history_file, json)
            .await
            .map_err(HistoryError::FileWrite)?;
        Ok(())
    }

    /// Add a command to history
    pub fn add(&mut self, command: String, context: Option<String>) {
        // Don't add empty commands or duplicates of the last command
        if command.trim().is_empty() || self.is_duplicate(&command) {
            return;
        }

        let entry = HistoryEntry {
            command,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context,
        };

        self.entries.push_back(entry);

        // Maintain maximum size
        while self.entries.len() > MAX_HISTORY_SIZE {
            self.entries.pop_front();
        }

        self.position = None;
    }

    /// Navigate backwards through history
    pub fn previous(&mut self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        self.position = Some(match self.position {
            Some(pos) if pos > 0 => pos - 1,
            Some(_) => 0,
            None => self.entries.len() - 1,
        });

        self.position
            .and_then(|pos| self.entries.get(pos))
            .map(|entry| entry.command.clone())
    }

    /// Navigate forwards through history
    pub fn next(&mut self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        self.position = match self.position {
            Some(pos) if pos < self.entries.len() - 1 => Some(pos + 1),
            _ => None,
        };

        self.position
            .and_then(|pos| self.entries.get(pos))
            .map(|entry| entry.command.clone())
    }

    /// Reset history navigation position
    pub fn reset_position(&mut self) {
        self.position = None;
    }

    /// Get all history entries
    pub fn entries(&self) -> &VecDeque<HistoryEntry> {
        &self.entries
    }

    /// Search history for a command
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.command.contains(query))
            .collect()
    }

    /// Check if command is a duplicate of the last entry
    fn is_duplicate(&self, command: &str) -> bool {
        self.entries
            .back()
            .map_or(false, |last| last.command == command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_history_persistence() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history.json");

        let mut history = CommandHistory {
            entries: VecDeque::new(),
            position: None,
            history_file,
        };

        // Add some commands
        history.add("command1".to_string(), None);
        history.add("command2".to_string(), Some("room1".to_string()));
        history.add("command3".to_string(), None);

        // Save to disk
        history.save().await.unwrap();

        // Create new history instance and load
        let mut new_history = CommandHistory {
            entries: VecDeque::new(),
            position: None,
            history_file: history.history_file.clone(),
        };
        new_history.load().await.unwrap();

        // Verify loaded history matches original
        assert_eq!(history.entries.len(), new_history.entries.len());
        assert_eq!(
            history.entries.back().unwrap().command,
            new_history.entries.back().unwrap().command
        );
    }

    #[test]
    fn test_history_navigation() {
        let mut history = CommandHistory {
            entries: VecDeque::new(),
            position: None,
            history_file: PathBuf::from("test"),
        };

        // Add test commands
        history.add("first".to_string(), None);
        history.add("second".to_string(), None);
        history.add("third".to_string(), None);

        // Test navigation
        assert_eq!(history.previous(), Some("third".to_string()));
        assert_eq!(history.previous(), Some("second".to_string()));
        assert_eq!(history.next(), Some("third".to_string()));
        
        // Reset should clear position
        history.reset_position();
        assert_eq!(history.previous(), Some("third".to_string()));
    }

    #[test]
    fn test_duplicate_prevention() {
        let mut history = CommandHistory {
            entries: VecDeque::new(),
            position: None,
            history_file: PathBuf::from("test"),
        };

        history.add("command".to_string(), None);
        history.add("command".to_string(), None);
        assert_eq!(history.entries.len(), 1);
    }

    #[test]
    fn test_search() {
        let mut history = CommandHistory {
            entries: VecDeque::new(),
            position: None,
            history_file: PathBuf::from("test"),
        };

        history.add("hello world".to_string(), None);
        history.add("goodbye world".to_string(), None);
        history.add("hello there".to_string(), None);

        let results = history.search("hello");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|e| e.command == "hello world"));
        assert!(results.iter().any(|e| e.command == "hello there"));
    }
}