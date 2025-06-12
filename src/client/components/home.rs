use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::net::SocketAddr;
use tokio::{sync::mpsc::UnboundedSender, time::Duration};
use tui_input::{backend::crossterm::EventHandler, Input};

/// Enum to distinguish different message types for styling
#[derive(Debug, Clone, PartialEq)]
enum MessageStyle {
    Sent,     // Messages from the current user
    Received, // Messages from other users
    System,   // System messages
}

use super::Component;
use crate::{
    action::Action,
    app::Mode,
    chat::{ChatMessage, MessageType, RoomManager, RoomSettings, RoomUser, UserRole},
    config::Config,
    errors::display::{set_global_error_display_action_sender, show_info, show_validation_error},
    history::CommandHistory,
    transport::ConnectionStatus,
};

/// Get any text in the input box
pub async fn get_user_input(mut input: Input) -> Option<String> {
    let message = input.value().to_string();
    input.reset();
    if message.is_empty() {
        None
    } else {
        Some(message)
    }
}

pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    show_help: bool,
    help_scroll: usize, // Track help popup scroll position
    app_ticker: usize,
    render_ticker: usize,
    mode: Mode,
    prev_mode: Mode,
    input: Input,

    // Connection dialog fields
    dialog_visible: bool,
    dialog_cursor_position: usize,
    dialog_host_input: Input,
    dialog_port_input: Input,

    // Command history
    command_history: CommandHistory,

    // New chat system
    room_manager: RoomManager,
    current_room_id: Option<uuid::Uuid>,
    current_user_id: Option<uuid::Uuid>,

    // Scroll state fields to replace unsafe static variables
    scroll_offset: usize,
    prev_text_len: usize,
    manual_scroll: bool,

    // Connection status for UI display
    connection_status: ConnectionStatus,
}

// Static state variables for scrolling
// Removed unsafe static variables - moved to struct fields

impl Default for Home {
    fn default() -> Self {
        let mut history = CommandHistory::new().unwrap_or_else(|e| {
            eprintln!("Failed to create command history: {}", e);
            // Create a minimal fallback history that won't crash
            CommandHistory::new().unwrap()
        });

        // Load existing history synchronously
        if let Err(e) = history.load_sync() {
            eprintln!("Failed to load command history: {}", e);
        }

        Self {
            command_tx: None,
            config: Config::default(),
            show_help: false,
            help_scroll: 0,
            app_ticker: 0,
            render_ticker: 0,
            mode: Mode::Normal,
            prev_mode: Mode::Normal,
            input: Input::default(),

            dialog_visible: false,
            dialog_cursor_position: 0,
            dialog_host_input: Input::default(),
            dialog_port_input: Input::default(),

            command_history: history,
            room_manager: RoomManager::new(),
            current_room_id: None,
            current_user_id: None,
            scroll_offset: 0,
            prev_text_len: 0,
            manual_scroll: false,

            connection_status: ConnectionStatus::DISCONNECTED,
        }
    }
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize default room and user for chat
    pub fn initialize_chat(&mut self, username: String) -> Result<(), Box<dyn std::error::Error>> {
        // Reset the room manager to clean up any existing state
        self.reset_chat_state();

        // Create a default public room
        let room_settings = RoomSettings::public("General".to_string());
        let room_id = self
            .room_manager
            .create_room(room_settings, uuid::Uuid::nil())?;

        // Create user - ensure username doesn't have a '!' character
        let clean_username = username.replace("!", "");
        let user_id = uuid::Uuid::new_v4();
        let user = RoomUser::new(user_id, clean_username.clone(), UserRole::User);

        // Add user to room
        self.room_manager.join_room(&room_id, user)?;

        // Set current context
        self.current_room_id = Some(room_id);
        self.current_user_id = Some(user_id);

        // Add a system message to indicate successful initialization
        let welcome_msg = format!(
            "Welcome, {}! You are now connected to the chat.",
            clean_username
        );
        if let Some(room) = self.room_manager.get_room_mut(&room_id) {
            let system_msg = ChatMessage::new_system(room_id, welcome_msg);
            let _ = room.add_message(system_msg);
        }

        tracing::info!(
            "DEBUG: Chat initialized with room_id: {:?}, user_id: {:?}, username: {}",
            room_id,
            user_id,
            clean_username
        );

        Ok(())
    }

    /// Reset the chat state when logging out or reinitializing
    pub fn reset_chat_state(&mut self) {
        // Clear room manager
        self.room_manager = RoomManager::new();

        // Clear context
        self.current_room_id = None;
        self.current_user_id = None;

        tracing::info!("DEBUG: Chat state reset");

        // Also reset the scroll position to default
        self.scroll_offset = 0;
        self.prev_text_len = 0;
        self.manual_scroll = false;
    }

    /// Get current room messages for display with styling information
    fn get_display_messages_with_style(&self) -> Vec<(String, MessageStyle)> {
        if let Some(room_id) = self.current_room_id {
            if let Some(room) = self.room_manager.get_room(&room_id) {
                let messages = room
                    .get_messages(Some(50))
                    .iter()
                    .map(|msg| {
                        if msg.message_type == MessageType::System {
                            (msg.content.clone(), MessageStyle::System)
                        } else {
                            // Check if the content already has a username prefix
                            if msg.content.starts_with("You: ") {
                                // Don't add another prefix for outgoing messages
                                (msg.content.clone(), MessageStyle::Sent)
                            } else if msg.content.contains(": ")
                                && !msg
                                    .content
                                    .starts_with(&format!("{}: ", msg.sender_username))
                            {
                                // Message already has some prefix, but not the correct one
                                (msg.content.clone(), MessageStyle::Received)
                            } else {
                                // Add username prefix for normal messages - remove any '!' characters
                                let clean_username = msg.sender_username.replace("!", "");
                                (
                                    format!("{}: {}", clean_username, msg.content),
                                    MessageStyle::Received,
                                )
                            }
                        }
                    })
                    .collect::<Vec<(String, MessageStyle)>>();
                tracing::info!(
                    "DEBUG: get_display_messages_with_style returning {} messages",
                    messages.len()
                );
                return messages;
            } else {
                tracing::warn!(
                    "DEBUG: Room not found in get_display_messages_with_style for room_id: {:?}",
                    room_id
                );
            }
        } else {
            tracing::warn!("DEBUG: No current_room_id in get_display_messages_with_style");
        }

        tracing::info!("DEBUG: get_display_messages_with_style returning empty vector");
        Vec::new()
    }

    /// Get current room messages for display (backwards compatibility)
    fn get_display_messages(&self) -> Vec<String> {
        self.get_display_messages_with_style()
            .into_iter()
            .map(|(content, _)| content)
            .collect()
    }

    /// Check if chat system is initialized (has current room and user)
    pub fn is_chat_initialized(&self) -> bool {
        self.current_room_id.is_some() && self.current_user_id.is_some()
    }

    /// Update connection status for UI display
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    /// Add a message to current room
    pub fn add_message_to_room(&mut self, content: String, is_system: bool) {
        tracing::info!(
            "DEBUG: add_message_to_room called with: '{}', is_system: {}, room_id: {:?}, user_id: {:?}",
            content,
            is_system,
            self.current_room_id,
            self.current_user_id
        );

        // First verify we have a valid room and user - if not, try to recreate them
        if self.current_room_id.is_none() || self.current_user_id.is_none() {
            tracing::warn!("DEBUG: Missing room or user context, attempting to recreate");
            if let Err(e) = self.initialize_chat("ReconnectedUser".to_string()) {
                tracing::error!("DEBUG: Failed to recreate chat context: {}", e);
            }
        }

        tracing::info!(
            "DEBUG: After context check - room_id: {:?}, user_id: {:?}",
            self.current_room_id,
            self.current_user_id
        );

        // Clean up content if it has multiple prefixes or username has '!' character
        let clean_content = if !is_system && content.contains(": ") {
            // Extract the actual message part if it has username prefixes
            let parts: Vec<&str> = content.splitn(2, ": ").collect();
            if parts.len() == 2 && parts[1].contains(": ") {
                // Double prefixed - extract just the original message
                let subparts: Vec<&str> = parts[1].splitn(2, ": ").collect();
                if subparts.len() == 2 {
                    if content.starts_with("You: ") {
                        format!("You: {}", subparts[1])
                    } else {
                        subparts[1].to_string()
                    }
                } else {
                    content.clone()
                }
            } else if parts.len() == 2 && parts[0].contains("!") {
                // Username has '!' - clean it up
                let clean_username = parts[0].replace("!", "");
                format!("{}: {}", clean_username, parts[1])
            } else {
                content.clone()
            }
        } else {
            content.clone()
        };

        // Avoid adding duplicate messages
        // We need to normalize the message format to detect duplicates
        let normalized_content = self.normalize_message_content(&clean_content);

        // Check if this message is already in the room
        if self.is_duplicate_message(&normalized_content) {
            tracing::info!("DEBUG: Skipping duplicate message: '{}'", clean_content);

            // Even for duplicates, still count received messages for status bar
            // if this appears to be a message from another user
            if !is_system && !clean_content.starts_with("You: ") {
                if let Some(tx) = &self.command_tx {
                    let _ = tx.send(Action::RecordReceivedMessage);
                }
            }

            return;
        }

        if let (Some(room_id), Some(user_id)) = (self.current_room_id, self.current_user_id) {
            // First, check if user exists in the room - if not, add them
            let user_in_room = if let Some(room) = self.room_manager.get_room(&room_id) {
                room.get_user(&user_id).is_some()
            } else {
                false
            };

            // If user not in room, add them before proceeding (avoids borrow checker issues)
            if !user_in_room {
                tracing::warn!("DEBUG: User not found in room, adding reconnected user");
                let reconnected_user =
                    RoomUser::new(user_id, "Reconnected User".to_string(), UserRole::User);

                if let Err(e) = self.room_manager.join_room(&room_id, reconnected_user) {
                    tracing::error!("DEBUG: Failed to add user to room: {}", e);
                }
            }

            if let Some(room) = self.room_manager.get_room_mut(&room_id) {
                let message = if is_system {
                    ChatMessage::new_system(room_id, clean_content.clone())
                } else {
                    // Now we can safely get the user's username as we've ensured they're in the room
                    let username = room
                        .get_user(&user_id)
                        .map(|u| u.username.clone().replace("!", ""))
                        .unwrap_or_else(|| "Reconnected User".to_string());

                    // Create the message with username we have
                    ChatMessage::new_text(room_id, user_id, username, clean_content.clone())
                };

                tracing::info!("DEBUG: Adding message to room system: {:?}", message);
                let _ = room.add_message(message);

                // Count messages based on their type:
                // 1. Count system messages as received (important notifications)
                // 2. Don't count outgoing messages (starting with "You: ")
                // 3. Count all other messages as received
                let should_count = is_system || !clean_content.starts_with("You: ");

                if should_count {
                    // Send an action to increment the received message count
                    if let Some(tx) = &self.command_tx {
                        match tx.send(Action::RecordReceivedMessage) {
                            Ok(_) => tracing::info!("DEBUG: Successfully sent RecordReceivedMessage action for message: '{}'", clean_content),
                            Err(e) => tracing::error!("DEBUG: Failed to send RecordReceivedMessage action: {}", e),
                        }
                    } else {
                        tracing::warn!(
                            "DEBUG: Cannot send RecordReceivedMessage - command_tx is None"
                        );
                    }
                }
            } else {
                tracing::warn!(
                    "DEBUG: Room not found in add_message_to_room for room_id: {:?}",
                    room_id
                );
            }
        } else {
            tracing::error!("DEBUG: No room or user context to add message to room - room_id: {:?}, user_id: {:?}",
                self.current_room_id, self.current_user_id);

            // We already tried to recreate the context at the beginning of this method,
            // so if we still don't have a room/user, we should log and return
            tracing::error!("DEBUG: Unable to add message - missing room/user context even after recreation attempt");

            // Display an error to the user via action channel if available
            if let Some(tx) = &self.command_tx {
                let _ = tx.send(Action::ReceiveMessage(
                    "Error: Unable to send message due to connection issues. Please try disconnecting and logging in again.".to_string()
                ));
            }
        }
    }

    /// Check if a message is a system message
    pub fn is_system_message(&self, content: &str) -> bool {
        // System messages typically have special formatting or prefixes
        content.starts_with("STATUS:")
            || content.starts_with("SYSTEM:")
            || content.starts_with("Error:")
            || content.contains("has joined the chat")
            || content.contains("Welcome back")
            || content.contains("Connected to server")
            || content.contains("Disconnected from server")
            || content.contains("Authentication")
            || content.contains("Registration")
    }

    /// Normalize message content to detect duplicates
    fn normalize_message_content(&self, content: &str) -> String {
        // Remove any username prefixes for comparison
        if content.contains(": ") {
            let parts: Vec<&str> = content.splitn(2, ": ").collect();
            if parts.len() == 2 {
                // Check if the second part still has a username prefix (double prefix case)
                if parts[1].contains(": ") {
                    let subparts: Vec<&str> = parts[1].splitn(2, ": ").collect();
                    if subparts.len() == 2 {
                        return subparts[1].to_string();
                    }
                }
                return parts[1].to_string();
            }
        }

        // Remove common system message prefixes for better duplicate detection
        let system_prefixes = [
            "Connected to",
            "Disconnected from",
            "Welcome",
            "joined",
            "left",
            "Error:",
            "ERROR:",
        ];
        for prefix in system_prefixes.iter() {
            if content.contains(prefix) {
                return content.replace("!", "").trim().to_string();
            }
        }

        content.to_string()
    }

    /// Check if this message is already in the current room
    fn is_duplicate_message(&self, content: &str) -> bool {
        if let (Some(room_id), _) = (self.current_room_id, self.current_user_id) {
            if let Some(room) = self.room_manager.get_room(&room_id) {
                // Get the last few messages and check if any match
                let messages = room.get_messages(Some(10)); // Specify how many messages to retrieve
                for msg in messages.iter().rev().take(5) {
                    let normalized = self.normalize_message_content(&msg.content);
                    if normalized == content {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Add a pre-formatted message from server (already contains sender: content format)
    pub fn add_received_message(&mut self, formatted_message: String) {
        if let Some(room_id) = self.current_room_id {
            if let Some(room) = self.room_manager.get_room_mut(&room_id) {
                // Create a system message to avoid double formatting
                let message = ChatMessage::new_system(room_id, formatted_message);
                let _ = room.add_message(message);
            }
        } else {
            // Use modern action system instead of legacy
            if let Some(tx) = &self.command_tx {
                let _ = tx.send(Action::ReceiveMessage(formatted_message));
            }
        }
    }

    // Connection dialog methods

    fn hide_dialog(&mut self) {
        self.dialog_visible = false;
    }

    fn next_dialog_position(&mut self) {
        self.dialog_cursor_position = (self.dialog_cursor_position + 1) % 4;
    }

    fn previous_dialog_position(&mut self) {
        self.dialog_cursor_position = (self.dialog_cursor_position + 3) % 4;
    }

    fn connect_from_dialog(&mut self) -> Result<Option<Action>> {
        if let Some(_tx) = &self.command_tx {
            let host = self.dialog_host_input.value().to_string();
            let port_str = self.dialog_port_input.value().to_string();

            // Validate inputs
            if host.is_empty() {
                show_validation_error("Host", "cannot be empty");
                return Ok(None);
            }

            let port = match port_str.parse::<u16>() {
                Ok(p) => p,
                Err(_) => {
                    show_validation_error("Port", "must be a valid number (1-65535)");
                    return Ok(None);
                }
            };

            // Try to parse the socket address
            let addr_str = format!("{}:{}", host, port);
            match addr_str.parse::<SocketAddr>() {
                Ok(_addr) => {
                    // Schedule connection
                    self.hide_dialog();

                    // Reset the inputs for next time
                    self.dialog_host_input = Input::default();
                    self.dialog_port_input = Input::default();

                    // Use modern action-based connection flow
                    show_info("Please restart the application and use the login screen to connect to this server");

                    return Ok(Some(Action::Update));
                }
                Err(_) => {
                    show_validation_error(
                        "Address",
                        "invalid format - use host:port (e.g., 127.0.0.1:8080)",
                    );
                }
            }
        }
        Ok(None)
    }
    //pub fn new() -> Home {
    //    Home {
    //        command_tx: None,
    //        config: Config::default(),
    //        show_help: false,
    //        app_ticker: 0,
    //        render_ticker: 0,
    //        mode: Mode::Normal,
    //        prev_mode: Mode::Normal,
    //        input: Input::default(),
    //        last_events: Vec::new(),
    //    }
    //}

    pub fn schedule_disconnect_client(&mut self) {
        let tx = self.command_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_millis(250)).await;
            tx.send(Action::DisconnectClient).unwrap();
            tokio::time::sleep(Duration::from_millis(250)).await;
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn tick(&mut self) {
        //log::info!("Tick");
        self.app_ticker = self.app_ticker.saturating_add(1);
    }

    pub fn render_tick(&mut self) {
        //log::debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx.clone());

        // Set up error display system with action sender for modern message handling
        set_global_error_display_action_sender(tx);

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.handle_key_event(key) {
            Ok(action) => action,
            Err(_) => None,
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Handle scrolling with PageUp and PageDown
        if self.mode == Mode::Normal || self.mode == Mode::Processing {
            // Handle scrolling for the help popup if it's visible
            if self.show_help {
                match key.code {
                    KeyCode::PageUp => {
                        if self.help_scroll > 0 {
                            self.help_scroll = self.help_scroll.saturating_sub(5);
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self.help_scroll.saturating_add(5);
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Up => {
                        if self.help_scroll > 0 {
                            self.help_scroll = self.help_scroll.saturating_sub(1);
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Down => {
                        self.help_scroll = self.help_scroll.saturating_add(1);
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Home => {
                        self.help_scroll = 0;
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::End => {
                        // Will be capped in the render code
                        self.help_scroll = 999; // Large number, will be constrained by max scroll
                        return Ok(Some(Action::Render));
                    }
                    _ => {}
                }
            } else {
                // Handle scrolling for the main content
                match key.code {
                    KeyCode::PageUp => {
                        // Enter manual scroll mode
                        self.manual_scroll = true;

                        // Scroll up by decreasing the scroll offset
                        if self.scroll_offset >= 5 {
                            self.scroll_offset -= 5;
                        } else {
                            self.scroll_offset = 0;
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Up => {
                        // Enter manual scroll mode
                        self.manual_scroll = true;

                        // Scroll up by one line
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::PageDown => {
                        // Scroll down by increasing the scroll offset
                        self.scroll_offset += 5;

                        // If we reach the bottom, enable auto-follow again
                        let messages_len = self.get_display_messages().len();
                        if self.scroll_offset >= messages_len {
                            self.manual_scroll = false;
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Down => {
                        // Scroll down by one line
                        self.scroll_offset += 1;

                        // If we reach the bottom, enable auto-follow again
                        let messages_len = self.get_display_messages().len();
                        if self.scroll_offset >= messages_len {
                            self.manual_scroll = false;
                        }
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::End => {
                        // Scroll to the end and re-enable auto-follow
                        let messages_len = self.get_display_messages().len();
                        self.scroll_offset = messages_len;
                        self.manual_scroll = false;
                        return Ok(Some(Action::Render));
                    }
                    // Cancel scroll mode and return to auto-follow on Escape
                    KeyCode::Esc => {
                        if !self.show_help {
                            let messages_len = self.get_display_messages().len();
                            self.scroll_offset = messages_len;
                            self.manual_scroll = false;
                            return Ok(Some(Action::Render));
                        }
                    }
                    KeyCode::Home => {
                        // Scroll to the top
                        self.scroll_offset = 0;
                        self.manual_scroll = true;
                        return Ok(Some(Action::Render));
                    }
                    // Any other key press exits manual scroll mode
                    _ => {
                        // Exit manual scrolling mode on any non-scroll key
                        if self.manual_scroll {
                            self.manual_scroll = false;
                            // When exiting manual scroll, set position to follow most recent messages
                            let messages_len = self.get_display_messages().len();
                            self.scroll_offset = messages_len;
                        }
                    }
                }
            }
        }

        // Exit manual scroll mode and handle dialog keys if dialog is visible
        if self.dialog_visible {
            // Exit manual scroll mode when dialog is opened
            self.manual_scroll = false;
            // Also reset scroll position to follow latest messages
            let messages_len = self.get_display_messages().len();
            self.scroll_offset = messages_len;

            match key.code {
                KeyCode::Esc => {
                    self.hide_dialog();
                    return Ok(Some(Action::Update));
                }
                KeyCode::Tab => {
                    self.next_dialog_position();
                    return Ok(Some(Action::Update));
                }
                KeyCode::BackTab => {
                    self.previous_dialog_position();
                    return Ok(Some(Action::Update));
                }
                KeyCode::Down | KeyCode::Right => {
                    self.next_dialog_position();
                    return Ok(Some(Action::Update));
                }
                KeyCode::Up | KeyCode::Left => {
                    self.previous_dialog_position();
                    return Ok(Some(Action::Update));
                }
                KeyCode::Enter => {
                    match self.dialog_cursor_position {
                        2 => {
                            // Connect button
                            return self.connect_from_dialog();
                        }
                        3 => {
                            // Cancel button
                            self.hide_dialog();
                            return Ok(Some(Action::Update));
                        }
                        _ => {
                            // Move to next field when pressing Enter in input fields
                            self.next_dialog_position();
                            return Ok(Some(Action::Update));
                        }
                    }
                }
                _ => {
                    // Handle input for the active field
                    match self.dialog_cursor_position {
                        0 => {
                            // Host input field
                            self.dialog_host_input
                                .handle_event(&crossterm::event::Event::Key(key));
                            // Force redraw
                            return Ok(Some(Action::Render));
                        }
                        1 => {
                            // Port input field - only allow numbers
                            match key.code {
                                KeyCode::Char(c) if c.is_digit(10) => {
                                    // Directly modify the port input to ensure it's updated
                                    let current = self.dialog_port_input.value().to_string();
                                    let new_value = format!("{}{}", current, c);
                                    self.dialog_port_input = Input::from(new_value);

                                    // Force redraw
                                    return Ok(Some(Action::Render));
                                }
                                KeyCode::Backspace => {
                                    // Directly handle backspace
                                    let current = self.dialog_port_input.value().to_string();
                                    if !current.is_empty() {
                                        let new_value = current[..current.len() - 1].to_string();
                                        self.dialog_port_input = Input::from(new_value);
                                    }

                                    // Force redraw
                                    return Ok(Some(Action::Render));
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            return Ok(Some(Action::Update));
        }

        // Handle regular keys when dialog is not visible
        // Exit manual scroll mode for any action key in normal mode
        if (self.mode == Mode::Normal || self.mode == Mode::Processing) && self.manual_scroll {
            // We're not handling a scroll key at this point, so exit manual scroll
            self.manual_scroll = false;
            // Also reset scroll position to follow latest messages
            let messages_len = self.get_display_messages().len();
            self.scroll_offset = messages_len;
        }

        let action = match self.mode {
            Mode::Normal | Mode::Processing => {
                match key.code {
                    KeyCode::Char('q') => {
                        self.schedule_disconnect_client();
                        Action::Quit
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.schedule_disconnect_client();
                        Action::Quit
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.schedule_disconnect_client();
                        Action::Quit
                    }
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::Suspend
                    }
                    KeyCode::Char('f') => Action::ToggleFps,
                    KeyCode::Char('?') => Action::ToggleShowHelp,
                    KeyCode::Char('/') => {
                        // Use modern connection status instead of legacy CLIENT_STATUS
                        Action::EnterInsert
                    }
                    KeyCode::F(2) => {
                        // F2 key: Connect/Reconnect behavior
                        // - When CONNECTED: Show message that user is already connected, need to disconnect first
                        // - When DISCONNECTED: Go back to login screen (no restart required)
                        if self.connection_status == ConnectionStatus::CONNECTED {
                            show_info("Already connected to server. To start a new connection, you need to disconnect first (press 'd').");
                            return Ok(Some(Action::Update));
                        }
                        // If disconnected, trigger reconnection which goes back to login
                        Action::Reconnect
                    }
                    KeyCode::Char('c') => {
                        // 'c' key: Connect/Reconnect behavior (same as F2)
                        // - When CONNECTED: Show message that user is already connected, need to disconnect first
                        // - When DISCONNECTED: Go back to login screen (no restart required)
                        if self.connection_status == ConnectionStatus::CONNECTED {
                            show_info("Already connected to server. To start a new connection, you need to disconnect first (press 'd').");
                            return Ok(Some(Action::Update));
                        }
                        // If disconnected, trigger reconnection which goes back to login
                        Action::Reconnect
                    }
                    KeyCode::Char('d') => {
                        if self.connection_status == ConnectionStatus::CONNECTED {
                            show_info("Disconnecting from server...");
                            self.schedule_disconnect_client();
                        } else {
                            show_info("Not connected to any server.");
                        }
                        Action::Update
                    }
                    KeyCode::Esc => {
                        if self.show_help {
                            self.show_help = false;
                            self.help_scroll = 0; // Reset help scroll position when closing
                        }
                        Action::Update
                    }
                    _ => Action::Tick,
                }
            }
            Mode::Insert => {
                match key.code {
                    KeyCode::F(2) => {
                        // F2 key: Connect/Reconnect behavior (same in Insert mode)
                        // - When CONNECTED: Show message that user is already connected, need to disconnect first
                        // - When DISCONNECTED: Go back to login screen (no restart required)
                        if self.connection_status == ConnectionStatus::CONNECTED {
                            show_info("Already connected to server. To start a new connection, you need to disconnect first (press 'd').");
                            return Ok(Some(Action::Update));
                        }
                        // If disconnected, trigger reconnection which goes back to login
                        Action::Reconnect
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.schedule_disconnect_client();
                        Action::Quit
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.schedule_disconnect_client();
                        Action::Quit
                    }
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::Suspend
                    }
                    KeyCode::Esc => Action::EnterNormal,
                    KeyCode::Enter => {
                        let message = self.input.value().to_string();
                        if !message.is_empty() {
                            // Add message to command history
                            self.command_history.add(message.clone(), None);

                            // Save history asynchronously
                            let history_clone = self.command_history.clone();
                            tokio::spawn(async move {
                                if let Err(e) = history_clone.save().await {
                                    eprintln!("Failed to save command history: {}", e);
                                }
                            });

                            let action = Action::SendMessage(message);
                            self.input.reset();
                            return Ok(Some(action));
                        } else {
                            show_info("Please enter a message before pressing Enter");
                        }
                        Action::Update
                    }
                    KeyCode::Up => {
                        // Navigate to previous command in history
                        if let Some(prev_command) = self.command_history.previous() {
                            self.input = Input::new(prev_command);
                            // Move cursor to end of input
                            let len = self.input.value().len();
                            for _ in 0..len {
                                self.input.handle_event(&crossterm::event::Event::Key(
                                    KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                                ));
                            }
                        }
                        Action::Render
                    }
                    KeyCode::Down => {
                        // Navigate to next command in history
                        if let Some(next_command) = self.command_history.next() {
                            self.input = Input::new(next_command);
                            // Move cursor to end of input
                            let len = self.input.value().len();
                            for _ in 0..len {
                                self.input.handle_event(&crossterm::event::Event::Key(
                                    KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                                ));
                            }
                        } else {
                            // If no next command, clear input
                            self.input.reset();
                            self.command_history.reset_position();
                        }
                        Action::Render
                    }
                    _ => {
                        // Reset history position when typing new characters
                        if matches!(
                            key.code,
                            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                        ) {
                            self.command_history.reset_position();
                        }
                        self.input.handle_event(&crossterm::event::Event::Key(key));
                        Action::Tick
                    }
                }
            }
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                Action::Tick
            }
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::ToggleShowHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.help_scroll = 0; // Reset scroll position when opening help
                }
            }
            Action::EnterNormal => {
                self.prev_mode = self.mode;
                self.mode = Mode::Normal;
            }
            Action::EnterInsert => {
                self.prev_mode = self.mode;
                self.mode = Mode::Insert;
                // Automatically exit manual scrolling when entering input mode
                self.manual_scroll = false;
                // Also reset scroll position to follow latest messages
                let messages_len = self.get_display_messages().len();
                self.scroll_offset = messages_len;
            }
            Action::EnterProcessing => {
                self.prev_mode = self.mode;
                self.mode = Mode::Processing;
            }
            Action::ExitProcessing => {
                // TODO: Make this go to previous mode instead
                self.mode = self.prev_mode;
            }
            Action::ConnectClient => {
                let user_input = self.input.value().to_string();
                self.input.reset();
                if user_input.is_empty() {
                    show_info(
                        "Enter a server address in the format host:port (e.g., 127.0.0.1:8080)",
                    );
                    return Ok(Some(Action::Update));
                }
                let _address: SocketAddr = match user_input.parse() {
                    Ok(address) => address,
                    Err(_) => {
                        show_validation_error("Server address", "invalid format - use host:port");
                        return Ok(Some(Action::Update));
                    }
                };
                // Use modern action-based connection flow
                show_info("Please restart the application and use the login screen to connect to this server");
            }
            Action::ShowConnectionDialog => {
                show_info(
                    "Please use the authentication system to connect (restart the application)",
                );
            }
            // DisconnectClient is now handled by the main app using modern ConnectionManager
            // No need to handle it here anymore
            _ => {}
        }
        Ok(None)
        // match action {
        //     Action::Tick => {
        //         // add any logic here that should run on every tick
        //     }
        //     Action::Render => {
        //         // add any logic here that should run on every render
        //     }
        //     _ => {a}
        // }
        // Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
            .split(area);

        // Create a horizontal layout for the main content and scrollbar
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),    // Main content area
                Constraint::Length(1), // Scrollbar
            ])
            .split(rects[0]);

        let content_area = content_layout[0];
        let scrollbar_area = content_layout[1];

        // Prepare text content
        let mut text: Vec<Line> = Vec::<Line>::new();
        text.push("".into());
        // Calculate available width for proper alignment (subtract borders)
        let available_width = content_area.width.saturating_sub(4) as usize; // -4 for borders and padding
        let message_max_width = (available_width * 75) / 100; // 75% of available width for messages

        // Helper function to wrap text to a specified width
        let wrap_text = |text: &str, max_width: usize| -> Vec<String> {
            if max_width == 0 {
                return vec![text.to_string()];
            }

            let mut lines = Vec::new();
            let mut current_line = String::new();
            let words: Vec<&str> = text.split_whitespace().collect();

            for word in words {
                // If adding this word would exceed the max width, start a new line
                if !current_line.is_empty() && current_line.len() + 1 + word.len() > max_width {
                    lines.push(current_line.clone());
                    current_line.clear();
                }

                // If the word itself is longer than max_width, we need to break it
                if word.len() > max_width {
                    // First, push any existing content
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }

                    // Break the long word into chunks
                    let mut word_chars: Vec<char> = word.chars().collect();
                    while !word_chars.is_empty() {
                        let chunk_size = max_width.min(word_chars.len());
                        let chunk: String = word_chars.drain(..chunk_size).collect();
                        lines.push(chunk);
                    }
                } else {
                    // Add word to current line
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }
            }

            // Don't forget the last line
            if !current_line.is_empty() {
                lines.push(current_line);
            }

            // If no lines were created, return the original text
            if lines.is_empty() {
                lines.push(text.to_string());
            }

            lines
        };

        let messages: Vec<Line> = self
            .get_display_messages_with_style()
            .iter()
            .enumerate()
            .flat_map(|(index, (content, style))| {
                let mut lines = Vec::new();

                // Add spacing between messages (except for the first one)
                if index > 0 {
                    lines.push(Line::from(""));
                }

                match style {
                    MessageStyle::Sent => {
                        // Sent messages: Clean blue bubble, right-aligned with width limit
                        let style = Style::default()
                            .fg(Color::Rgb(255, 255, 255)) // Pure white text
                            .bg(Color::Rgb(59, 130, 246)) // Vibrant blue (#3B82F6)
                            .add_modifier(Modifier::BOLD);

                        // Wrap text to fit within message_max_width minus padding
                        let content_width = message_max_width.saturating_sub(4); // -4 for padding
                        let wrapped_lines = wrap_text(content, content_width.max(10));

                        for (_line_index, wrapped_line) in wrapped_lines.iter().enumerate() {
                            let bubble_content = format!("  {}  ", wrapped_line);
                            let content_len = bubble_content.len();

                            // Right-align the bubble
                            let padding = if content_len < available_width {
                                available_width.saturating_sub(content_len)
                            } else {
                                0
                            };

                            // Create line with mixed styling: transparent padding + colored content
                            let line = Line::from(vec![
                                Span::raw(" ".repeat(padding)),      // Transparent padding
                                Span::styled(bubble_content, style), // Colored content only
                            ]);
                            lines.push(line);
                        }
                    }
                    MessageStyle::Received => {
                        // Received messages: Clean light bubble with width limit
                        let style = Style::default()
                            .fg(Color::Rgb(55, 65, 81)) // Slate-700 (#374151)
                            .bg(Color::Rgb(249, 250, 251)); // Gray-50 (#F9FAFB)

                        // Wrap text to fit within message_max_width minus padding
                        let content_width = message_max_width.saturating_sub(4); // -4 for padding
                        let wrapped_lines = wrap_text(content, content_width.max(10));

                        for wrapped_line in wrapped_lines.iter() {
                            let bubble_content = format!("  {}  ", wrapped_line);
                            // Create line with colored content only (no full-width background)
                            let line = Line::from(vec![
                                Span::styled(bubble_content, style), // Colored content only
                            ]);
                            lines.push(line);
                        }
                    }
                    MessageStyle::System => {
                        // System messages: Subtle centered notification style
                        let style = Style::default()
                            .fg(Color::Rgb(156, 163, 175)) // Gray-400 (#9CA3AF)
                            .add_modifier(Modifier::ITALIC);

                        let system_content = format!("• {} •", content);
                        let content_len = system_content.len();
                        let padding = if content_len < available_width {
                            (available_width.saturating_sub(content_len)) / 2
                        } else {
                            0
                        };
                        let centered_content = format!("{}{}", " ".repeat(padding), system_content);
                        lines.push(Line::from(centered_content).style(style));
                    }
                };

                lines
            })
            .collect();
        if messages.is_empty() {
            text.push("".into());
            text.push("Waiting for messages...".dim().into());
            text.push("".into());
            text.push("Controls:".white().bold().into());
            text.push("   ? - Show/hide help".cyan().into());
            text.push("   f - Toggle FPS counter".cyan().into());
            text.push("   q - Quit application".cyan().into());
            text.push("".into());
        } else {
            for l in messages {
                text.push(l.into());
            }
        }
        text.push("".into());

        // Calculate available height for text (accounting for borders)
        let available_height = content_area.height.saturating_sub(2) as usize; // -2 for top/bottom borders

        // Debug: Ensure we have a minimum height
        if available_height == 0 {
            return Ok(());
        }

        // Calculate scroll position - start from the bottom of the text
        let text_len = text.len();

        // Always auto-scroll to show latest messages unless in manual scroll mode
        let scroll_position = {
            let old_text_len = self.prev_text_len;

            // Update scroll state when new content is added or not in manual mode
            if text_len > old_text_len || !self.manual_scroll {
                self.manual_scroll = false; // Ensure we're in auto mode
            }

            self.prev_text_len = text_len;

            // Always show the bottom-most content
            if !self.manual_scroll {
                if text_len > available_height {
                    // Show the latest messages at the bottom by scrolling to show the last available_height lines
                    // Add 1 to ensure we see the very latest message
                    text_len.saturating_sub(available_height.saturating_sub(1))
                } else {
                    0
                }
            } else {
                // In manual scroll mode, use stored scroll position
                if text_len > available_height {
                    // Clamp scroll_offset to valid range
                    self.scroll_offset
                        .min(text_len.saturating_sub(available_height))
                } else {
                    0
                }
            }
        };

        // Simplified line counting - let ratatui handle wrapping
        let total_lines = text_len;

        // Calculate visible percentage for scrollbar
        let _visible_percentage = if total_lines > 0 {
            (available_height as f64 / total_lines as f64).min(1.0)
        } else {
            1.0
        };

        // Render scrollbar if there's enough content to scroll
        if total_lines > available_height {
            // Calculate scrollbar parameters
            let scrollbar_height = scrollbar_area.height.saturating_sub(2) as usize;
            let content_height = total_lines;

            // Calculate scrollbar thumb position and size
            let thumb_height = ((scrollbar_height as f64 * available_height as f64)
                / content_height as f64)
                .max(1.0) as usize;
            let thumb_position = ((scroll_position as f64 * scrollbar_height as f64)
                / content_height as f64) as usize;

            // Create the scrollbar string
            let mut scrollbar = vec![String::from("│"); scrollbar_height];

            // Draw the thumb
            for i in thumb_position..thumb_position + thumb_height {
                if i < scrollbar_height {
                    scrollbar[i] = String::from("█");
                }
            }

            // Add up/down indicators at the ends of the scrollbar when scrollable
            if scroll_position > 0 {
                scrollbar[0] = String::from("▲");
            }
            if scroll_position + available_height < total_lines {
                if scrollbar_height > 0 {
                    scrollbar[scrollbar_height - 1] = String::from("▼");
                }
            }

            // Render scrollbar
            let scrollbar_block = Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::default().fg(Color::DarkGray));

            frame.render_widget(scrollbar_block, scrollbar_area);

            // Render scrollbar thumb
            for (i, symbol) in scrollbar.iter().enumerate() {
                if i < scrollbar_height {
                    // Use brighter color for the indicators
                    let color = if symbol == "▲" || symbol == "▼" {
                        Color::Yellow
                    } else if symbol == "█" {
                        Color::White
                    } else {
                        Color::Gray
                    };

                    let scrollbar_piece =
                        Paragraph::new(symbol.clone()).style(Style::default().fg(color));
                    frame.render_widget(
                        scrollbar_piece,
                        Rect::new(
                            scrollbar_area.x,
                            scrollbar_area.y + 1 + i as u16, // +1 for top border
                            1,
                            1,
                        ),
                    );
                }
            }

            // No longer displaying scroll position in title
        }

        // Render main content with appropriate scroll
        let content_borders = if total_lines > available_height {
            Borders::ALL & !Borders::RIGHT // Remove right border when scrollbar is present
        } else {
            Borders::ALL
        };

        frame.render_widget(
            Paragraph::new(text.clone())
                .scroll((scroll_position as u16, 0))
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .title_top(Line::from("v0.6.0".white()).left_aligned())
                        .title_top(
                            Line::from(vec![Span::styled(
                                "THE LAIR",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )])
                            .centered(),
                        )
                        .title_top(Line::from("(C) 2025".white()).right_aligned())
                        .borders(content_borders)
                        .border_style(match self.mode {
                            Mode::Processing => Style::default().bg(Color::Black).fg(Color::Yellow),
                            _ => Style::default().bg(Color::Black).fg(Color::Cyan),
                        })
                        .border_type(BorderType::Rounded),
                )
                .style(
                    Style::default()
                        .bg(Color::Rgb(17, 24, 39)) // Gray-900 (#111827)
                        .fg(Color::Rgb(229, 231, 235)), // Gray-200 (#E5E7EB)
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false }),
            content_area,
        );

        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let input_scroll = self.input.visual_scroll(width as usize);
        let input_box = Paragraph::new(self.input.value())
            .style(match self.mode {
                Mode::Insert => Style::default().bg(Color::Black).fg(Color::Yellow),
                _ => Style::default().bg(Color::Black).fg(Color::White),
            })
            .scroll((0, input_scroll as u16)) // Fixed input box scrolling
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        Span::raw("Insert Text Here"),
                        Span::styled("(Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "/",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to start, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "ESC",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to stop, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "?",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" for help)", Style::default().fg(Color::DarkGray)),
                    ])),
            );
        frame.render_widget(input_box, rects[1]);
        if self.mode == Mode::Insert {
            frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                (rects[1].x + 1 + self.input.cursor() as u16).min(rects[1].x + rects[1].width - 2),
                // Move one line down, from the border to the input line
                rects[1].y + 1,
            ))
        }

        // Draw connection dialog if visible - this appears on top of everything else
        if self.dialog_visible {
            // Calculate dialog dimensions
            let dialog_width = 60; // Wider dialog for better field display
            let dialog_height = 14; // Taller for better spacing

            let dialog_area = Rect::new(
                (area.width.saturating_sub(dialog_width)) / 2,
                (area.height.saturating_sub(dialog_height)) / 2,
                dialog_width.min(area.width),
                dialog_height.min(area.height),
            );

            // Draw a clear background behind the dialog to create a modal effect
            frame.render_widget(Clear, dialog_area);

            // Dialog border
            let dialog_block = Block::default()
                .title("Connect to Server")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));

            frame.render_widget(dialog_block.clone(), dialog_area);

            // Create inner area for the dialog content
            let inner_area = dialog_block.inner(dialog_area);

            // Create layout for the dialog content
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Padding
                    Constraint::Length(3), // Host input
                    Constraint::Length(3), // Port input
                    Constraint::Length(1), // Buttons
                    Constraint::Length(1), // Padding
                ])
                .split(inner_area);

            // Host input field
            let host_input_style = if self.dialog_cursor_position == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let host_block = Block::default()
                .title("Host")
                .borders(Borders::ALL)
                .style(host_input_style);

            // Get the host value for display
            let host_value = self.dialog_host_input.value().to_string();

            // Render just the block without text content
            let host_input = Paragraph::new("").block(host_block).style(host_input_style);

            frame.render_widget(host_input, layout[1]);

            // Create inner area for text with better padding
            let host_inner_area = layout[1].inner(Margin {
                vertical: 1,   // Avoid overwriting the title
                horizontal: 2, // Add horizontal padding for better appearance
            });

            // Render the host value in the inner area only
            let host_text = Paragraph::new(host_value)
                .style(
                    Style::default()
                        .fg(Color::White) // White is more readable on dark gray background
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Left);

            frame.render_widget(host_text, host_inner_area);

            // Port input field
            let port_input_style = if self.dialog_cursor_position == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let port_block = Block::default()
                .title("Port")
                .borders(Borders::ALL)
                .style(port_input_style);

            // Get the port value for display
            let port_value = self.dialog_port_input.value().to_string();

            // Port value for display

            // Render just the block without text content
            let port_input = Paragraph::new("").block(port_block).style(port_input_style);

            frame.render_widget(port_input, layout[2]);

            // Create larger inner area for port text to ensure visibility
            let port_inner_area = layout[2].inner(Margin {
                vertical: 1,   // Avoid overwriting the title
                horizontal: 2, // Add more horizontal padding for better appearance
            });

            // Render the port value with enhanced visibility - make it stand out more
            let display_text = port_value;

            // Use a bold, bright text to ensure visibility
            let value_text = Paragraph::new(display_text)
                .style(
                    Style::default()
                        .fg(Color::White) // White is more readable on dark gray background
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Left);

            frame.render_widget(value_text, port_inner_area);

            // Buttons
            let button_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[3]);

            // Connect button
            let connect_style = if self.dialog_cursor_position == 2 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let connect_button = Paragraph::new("[ Connect ]")
                .alignment(Alignment::Center)
                .style(connect_style);

            frame.render_widget(connect_button, button_layout[0]);

            // Cancel button
            let cancel_style = if self.dialog_cursor_position == 3 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let cancel_button = Paragraph::new("[ Cancel ]")
                .alignment(Alignment::Center)
                .style(cancel_style);

            frame.render_widget(cancel_button, button_layout[1]);
        }

        if self.show_help {
            let rect = area.inner(Margin {
                horizontal: 4,
                vertical: 2,
            });
            frame.render_widget(Clear, rect);

            // Create layout with content area and scrollbar
            let help_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(10),   // Content area
                    Constraint::Length(1), // Scrollbar
                ])
                .split(rect);

            let content_area = help_layout[0];
            let scrollbar_area = help_layout[1];

            // Create the block for the help dialog
            let block = Block::default()
                .title(Line::from(vec![Span::styled(
                    "Key Bindings",
                    Style::default().add_modifier(Modifier::BOLD),
                )]))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().bg(Color::Blue).fg(Color::Yellow));

            frame.render_widget(block.clone(), content_area);

            // Create rows for the help table
            let rows = vec![
                Row::new(vec!["/", "Enter Message Input Mode"]),
                Row::new(vec!["enter", "Send Message"]),
                Row::new(vec!["↑/↓", "Navigate Command History"]),
                Row::new(vec!["esc", "Exit Message Input Mode"]),
                Row::new(vec!["Ctrl+C", "Quit Application"]),
                Row::new(vec!["q", "Quit"]),
                Row::new(vec!["ctrl-z", "Suspend Program"]),
                Row::new(vec!["?", "Open/Close Help"]),
                Row::new(vec!["↑", "Scroll Up One Line"]),
                Row::new(vec!["↓", "Scroll Down One Line"]),
                Row::new(vec!["PageUp", "Scroll Up One Page"]),
                Row::new(vec!["PageDown", "Scroll Down One Page"]),
                Row::new(vec!["Home", "Scroll to Top"]),
                Row::new(vec!["End", "Scroll to Bottom"]),
                Row::new(vec!["f", "Toggle FPS counter"]),
            ];

            // Calculate available height for the table content
            let inner_area = content_area.inner(Margin {
                vertical: 4,
                horizontal: 4,
            });

            let available_height = inner_area.height as usize;

            // Calculate maximum scroll position
            let max_scroll = if rows.len() > available_height {
                rows.len() - available_height
            } else {
                0
            };

            // Constrain scroll position
            self.help_scroll = self.help_scroll.min(max_scroll);

            // Create a scrollable table
            let table = Table::new(
                // Take a slice of rows based on scroll position
                rows.iter()
                    .skip(self.help_scroll)
                    .take(available_height)
                    .cloned()
                    .collect::<Vec<_>>(),
                [Constraint::Percentage(20), Constraint::Percentage(80)],
            )
            .header(
                Row::new(vec!["Key", "Action"]).bottom_margin(1).style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Blue)
                        .fg(Color::White),
                ),
            )
            .column_spacing(5)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

            // Render the table
            frame.render_widget(table, inner_area);

            // Render scrollbar if needed
            if max_scroll > 0 {
                // Calculate scrollbar thumb parameters
                let scrollbar_height = scrollbar_area.height.saturating_sub(2) as usize;
                let thumb_height = ((available_height as f64 / rows.len() as f64)
                    * scrollbar_height as f64)
                    .max(1.0) as usize;
                let thumb_position = ((self.help_scroll as f64 / max_scroll as f64)
                    * (scrollbar_height - thumb_height) as f64)
                    as usize;

                // Create scrollbar block
                let scrollbar_block = Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .style(Style::default().fg(Color::DarkGray));

                frame.render_widget(scrollbar_block, scrollbar_area);

                // Create scrollbar elements
                let mut scrollbar = vec![String::from("│"); scrollbar_height];

                // Draw the thumb
                for i in thumb_position..thumb_position + thumb_height {
                    if i < scrollbar_height {
                        scrollbar[i] = String::from("█");
                    }
                }

                // Render scrollbar thumb
                for (i, symbol) in scrollbar.iter().enumerate() {
                    if i < scrollbar_height {
                        let scrollbar_piece =
                            Paragraph::new(symbol.clone()).style(Style::default().fg(Color::Gray));
                        frame.render_widget(
                            scrollbar_piece,
                            Rect::new(
                                scrollbar_area.x,
                                scrollbar_area.y + 1 + i as u16, // +1 for top border
                                1,
                                1,
                            ),
                        );
                    }
                }

                // Add small scroll indicator in title if scrollable
                let scroll_indicator = format!(" ↕ ");
                let scroll_text = Paragraph::new(scroll_indicator)
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(Color::DarkGray));

                // Render subtle scroll indicator
                frame.render_widget(
                    scroll_text,
                    Rect::new(
                        content_area.x + 2,
                        content_area.y,
                        content_area.width - 4,
                        1,
                    ),
                );
            }
        };

        Ok(())
    }
}
