# Migration Examples: Legacy to Modern Architecture

## Overview

This document provides concrete examples of migrating from legacy global state patterns to the modern ConnectionManager architecture in Lair Chat.

## Quick Reference

### Legacy → Modern API Mapping

| Legacy API | Modern Replacement | Notes |
|------------|-------------------|-------|
| `CLIENT_STATUS.lock().unwrap()` | `ConnectionManager::get_status()` | Async, proper error handling |
| `add_text_message(msg)` | `Observer::on_message(msg)` | Observer pattern |
| `add_outgoing_message(msg)` | `ConnectionManager::send_message(msg)` | Async, returns Result |
| `connect_client_compat()` | `ConnectionManager::connect()` | Direct usage, no compatibility layer |
| `authenticate_compat()` | `AuthManager::login()` | Proper authentication flow |

## Migration Examples

### Example 1: Connection Management

#### Legacy Code (DEPRECATED)
```rust
// ❌ OLD: Direct global state access
use crate::transport::{CLIENT_STATUS, ConnectionStatus};

fn check_connection() -> bool {
    let status = CLIENT_STATUS.lock().unwrap();
    status.status == ConnectionStatus::CONNECTED
}

fn send_message(msg: String) {
    let status = CLIENT_STATUS.lock().unwrap();
    if status.status == ConnectionStatus::CONNECTED {
        add_outgoing_message(msg);
    }
}
```

#### Modern Code (RECOMMENDED)
```rust
// ✅ NEW: ConnectionManager with proper encapsulation
use crate::connection_manager::ConnectionManager;
use crate::transport::{ConnectionConfig, ConnectionStatus};

pub struct ChatService {
    connection_manager: ConnectionManager,
}

impl ChatService {
    pub fn new(server_address: &str) -> Result<Self, ConnectionError> {
        let config = ConnectionConfig::new(server_address.parse()?);
        let connection_manager = ConnectionManager::new(config);
        
        Ok(Self { connection_manager })
    }
    
    pub async fn check_connection(&self) -> ConnectionStatus {
        self.connection_manager.get_status().await
    }
    
    pub async fn send_message(&mut self, msg: String) -> Result<(), MessageError> {
        match self.connection_manager.get_status().await {
            ConnectionStatus::CONNECTED => {
                self.connection_manager.send_message(&msg).await?;
                Ok(())
            }
            ConnectionStatus::DISCONNECTED => {
                Err(MessageError::NotConnected)
            }
        }
    }
}
```

### Example 2: Authentication Flow

#### Legacy Code (DEPRECATED)
```rust
// ❌ OLD: Compatibility layer usage
use crate::compatibility_layer::{connect_client_compat, authenticate_compat};
use crate::transport::CLIENT_STATUS;

async fn authenticate_user(username: String, password: String) -> Result<(), String> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let input = tui_input::Input::default();
    
    connect_client_compat(input, addr).await
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
    
    authenticate_compat(username, password).await
        .map_err(|e| format!("Auth failed: {}", e))?;
    
    Ok(())
}
```

#### Modern Code (RECOMMENDED)
```rust
// ✅ NEW: Direct AuthManager and ConnectionManager usage
use crate::connection_manager::ConnectionManager;
use crate::auth::{AuthManager, Credentials, AuthError};
use crate::transport::ConnectionConfig;

pub struct AuthenticatedSession {
    connection_manager: ConnectionManager,
    auth_manager: Arc<AuthManager>,
    auth_state: Option<AuthState>,
}

impl AuthenticatedSession {
    pub async fn new(server_address: &str) -> Result<Self, SessionError> {
        let config = ConnectionConfig::new(server_address.parse()?);
        let mut connection_manager = ConnectionManager::new(config.clone());
        
        // Set up transport
        let transport = Box::new(TcpTransport::new(config));
        connection_manager.with_transport(transport);
        
        // Set up authentication
        let token_storage = Box::new(FileTokenStorage::new()?);
        let auth_manager = Arc::new(AuthManager::new(
            Arc::new(Mutex::new(Box::new(connection_manager.clone()) as Box<dyn Transport + Send + Sync>)),
            token_storage
        ));
        
        Ok(Self {
            connection_manager,
            auth_manager,
            auth_state: None,
        })
    }
    
    pub async fn authenticate(&mut self, username: String, password: String) -> Result<AuthState, AuthError> {
        // Connect first
        self.connection_manager.connect().await?;
        
        // Then authenticate
        let credentials = Credentials::new(username, password);
        let auth_state = self.auth_manager.login(credentials).await?;
        
        self.auth_state = Some(auth_state.clone());
        Ok(auth_state)
    }
    
    pub async fn is_authenticated(&self) -> bool {
        match &self.auth_state {
            Some(AuthState::Authenticated { .. }) => true,
            _ => false,
        }
    }
}
```

### Example 3: Message Handling with Observer Pattern

#### Legacy Code (DEPRECATED)
```rust
// ❌ OLD: Direct message injection
use crate::transport::add_text_message;

fn handle_server_message(message: String) {
    add_text_message(message);
}

fn handle_error(error: String) {
    add_text_message(format!("Error: {}", error));
}
```

#### Modern Code (RECOMMENDED)
```rust
// ✅ NEW: Observer pattern with proper typing
use crate::transport::{ConnectionObserver, MessageStore};
use crate::action::Action;
use tokio::sync::mpsc;

pub struct ChatMessageObserver {
    action_sender: mpsc::UnboundedSender<Action>,
    message_store: Arc<Mutex<MessageStore>>,
}

impl ChatMessageObserver {
    pub fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            action_sender,
            message_store: Arc::new(Mutex::new(MessageStore::new())),
        }
    }
}

impl ConnectionObserver for ChatMessageObserver {
    fn on_message(&self, message: String) {
        // Store message
        {
            let mut store = self.message_store.lock().unwrap();
            store.add_message(Message::received_message(message.clone()));
        }
        
        // Notify UI via action system
        let _ = self.action_sender.send(Action::ReceiveMessage(message));
    }
    
    fn on_error(&self, error: String) {
        // Store error as system message
        {
            let mut store = self.message_store.lock().unwrap();
            store.add_message(Message::error_message(error.clone()));
        }
        
        // Notify UI via action system
        let _ = self.action_sender.send(Action::ConnectionError(error));
    }
    
    fn on_status_change(&self, connected: bool) {
        let status = if connected {
            ConnectionStatus::CONNECTED
        } else {
            ConnectionStatus::DISCONNECTED
        };
        
        let _ = self.action_sender.send(Action::ConnectionStatusChanged(status));
    }
}

// Usage in main application
impl App {
    fn setup_connection_observer(&mut self) -> Result<(), AppError> {
        let observer = Arc::new(ChatMessageObserver::new(self.action_tx.clone()));
        self.connection_manager.register_observer(observer);
        Ok(())
    }
}
```

### Example 4: Error Handling Migration

#### Legacy Code (DEPRECATED)
```rust
// ❌ OLD: String-based error handling
use crate::transport::add_text_message;

fn handle_connection_error(error: &str) {
    add_text_message(format!("Connection failed: {}", error));
    add_text_message("Please check your network connection.".to_string());
}
```

#### Modern Code (RECOMMENDED)
```rust
// ✅ NEW: Typed error handling with context
use crate::errors::{ChatError, ConnectionError, UserFriendlyError};

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthError),
    
    #[error("Message error: {0}")]
    Message(#[from] MessageError),
}

impl ChatError {
    pub fn to_user_message(&self) -> String {
        match self {
            ChatError::Connection(ConnectionError::NetworkFailure(_)) => {
                "Network connection failed. Please check your internet connection and try again.".to_string()
            }
            ChatError::Connection(ConnectionError::ServerUnavailable) => {
                "Chat server is currently unavailable. Please try again later.".to_string()
            }
            ChatError::Authentication(AuthError::InvalidCredentials) => {
                "Invalid username or password. Please check your credentials.".to_string()
            }
            _ => format!("An error occurred: {}", self),
        }
    }
}

pub struct ErrorHandler {
    action_sender: mpsc::UnboundedSender<Action>,
}

impl ErrorHandler {
    pub fn handle_error(&self, error: ChatError) {
        let user_message = error.to_user_message();
        let _ = self.action_sender.send(Action::ShowError(user_message));
        
        // Log the full error for debugging
        log::error!("Chat error occurred: {:?}", error);
    }
}
```

### Example 5: Complete Application Structure

#### Legacy Code (DEPRECATED)
```rust
// ❌ OLD: Mixed patterns with global state
use crate::transport::{CLIENT_STATUS, MESSAGES, add_text_message};
use crate::compatibility_layer::connect_client_compat;

struct OldApp {
    // Minimal state, relies on globals
    should_quit: bool,
}

impl OldApp {
    fn send_message(&self, msg: String) {
        if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
            add_outgoing_message(msg);
        } else {
            add_text_message("Not connected!".to_string());
        }
    }
}
```

#### Modern Code (RECOMMENDED)
```rust
// ✅ NEW: Clean architecture with dependency injection
use crate::connection_manager::ConnectionManager;
use crate::auth::AuthManager;
use crate::transport::{ConnectionConfig, MessageStore};

pub struct ModernApp {
    // Core services
    connection_manager: ConnectionManager,
    auth_manager: Arc<AuthManager>,
    error_handler: ErrorHandler,
    
    // Application state
    auth_state: AuthState,
    ui_state: UiState,
    
    // Communication
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl ModernApp {
    pub async fn new(config: AppConfig) -> Result<Self, AppError> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        
        // Set up connection management
        let conn_config = ConnectionConfig::new(config.server_address);
        let mut connection_manager = ConnectionManager::new(conn_config.clone());
        
        // Configure transport and encryption
        let transport = Box::new(TcpTransport::new(conn_config));
        let encryption = Box::new(AesGcmEncryption::new(&config.encryption_key));
        
        connection_manager
            .with_transport(transport)
            .with_encryption(encryption);
        
        // Set up authentication
        let token_storage = Box::new(FileTokenStorage::new()?);
        let auth_manager = Arc::new(AuthManager::new(
            Arc::new(Mutex::new(Box::new(connection_manager.clone()) as Box<dyn Transport + Send + Sync>)),
            token_storage
        ));
        
        // Set up error handling
        let error_handler = ErrorHandler::new(action_tx.clone());
        
        // Set up message observer
        let message_observer = Arc::new(ChatMessageObserver::new(action_tx.clone()));
        connection_manager.register_observer(message_observer);
        
        Ok(Self {
            connection_manager,
            auth_manager,
            error_handler,
            auth_state: AuthState::Unauthenticated,
            ui_state: UiState::new(),
            action_tx,
            action_rx,
        })
    }
    
    pub async fn send_message(&mut self, msg: String) -> Result<(), ChatError> {
        match self.connection_manager.get_status().await {
            ConnectionStatus::CONNECTED => {
                self.connection_manager.send_message(&msg).await?;
                Ok(())
            }
            ConnectionStatus::DISCONNECTED => {
                Err(ChatError::Connection(ConnectionError::NotConnected))
            }
        }
    }
    
    pub async fn authenticate(&mut self, credentials: Credentials) -> Result<(), ChatError> {
        // Connect if not already connected
        if self.connection_manager.get_status().await == ConnectionStatus::DISCONNECTED {
            self.connection_manager.connect().await?;
        }
        
        // Authenticate
        let auth_state = self.auth_manager.login(credentials).await?;
        self.auth_state = auth_state;
        
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<(), AppError> {
        // Main application loop with proper error handling
        while !self.ui_state.should_quit {
            // Handle actions
            while let Ok(action) = self.action_rx.try_recv() {
                match self.handle_action(action).await {
                    Ok(()) => {}
                    Err(e) => self.error_handler.handle_error(e),
                }
            }
            
            // Update UI
            self.ui_state.render()?;
            
            // Small delay to prevent busy loop
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
        
        Ok(())
    }
    
    async fn handle_action(&mut self, action: Action) -> Result<(), ChatError> {
        match action {
            Action::SendMessage(msg) => self.send_message(msg).await?,
            Action::Authenticate(creds) => self.authenticate(creds).await?,
            Action::Disconnect => self.connection_manager.disconnect().await?,
            Action::Quit => self.ui_state.should_quit = true,
            _ => {} // Handle other actions
        }
        Ok(())
    }
}
```

## Migration Checklist

### Phase 1: Preparation
- [ ] Review legacy code usage with `grep -r "CLIENT_STATUS\|add_text_message\|connect_client_compat" src/`
- [ ] Identify all deprecated API usage in your code
- [ ] Plan migration strategy for each component
- [ ] Set up ConnectionManager configuration

### Phase 2: Core Migration
- [ ] Replace global state access with ConnectionManager methods
- [ ] Implement observer pattern for message handling
- [ ] Migrate authentication to AuthManager
- [ ] Add proper error typing and handling
- [ ] Update tests to use modern patterns

### Phase 3: Cleanup
- [ ] Remove `#[allow(deprecated)]` annotations
- [ ] Verify no deprecated API usage remains
- [ ] Run comprehensive tests
- [ ] Update documentation

## Testing Modern Code

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockTransport;
    
    #[tokio::test]
    async fn test_modern_message_sending() {
        let mut app = ModernApp::new(AppConfig::test()).await.unwrap();
        
        // Connect and authenticate
        app.connection_manager.connect().await.unwrap();
        app.authenticate(Credentials::test()).await.unwrap();
        
        // Send message
        let result = app.send_message("Hello, World!".to_string()).await;
        assert!(result.is_ok());
        
        // Verify message was sent
        let messages = app.connection_manager.get_message_store();
        let store = messages.lock().await;
        assert_eq!(store.messages.len(), 1);
    }
}
```

## Resources

- **LEGACY_CODE_AUDIT_AND_DEPRECATION_PLAN.md**: Complete deprecation timeline
- **NEXT_STEPS.md**: Strategic roadmap for modernization
- **ConnectionManager API**: `src/client/connection_manager.rs`
- **AuthManager API**: `src/client/auth/manager.rs`
- **Observer Pattern**: `src/client/transport.rs` (ConnectionObserver trait)

## Getting Help

1. Check deprecation warnings for specific migration guidance
2. Review test files for modern usage examples
3. Consult the deprecation plan for timeline and strategy
4. Use compiler warnings to identify remaining legacy usage

Remember: The goal is to eliminate global mutable state and adopt proper encapsulation, error handling, and observer patterns for a maintainable, testable codebase.