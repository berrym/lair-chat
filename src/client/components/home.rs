use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::net::SocketAddr;
use tokio::{sync::mpsc::UnboundedSender, time::Duration};
use tui_input::{backend::crossterm::EventHandler, Input};

use super::Component;
use crate::{
    action::Action,
    app::Mode,
    config::Config,
    history::CommandHistory,
    migration_facade,
    transport::*,
    errors::display::{show_validation_error, show_disconnection, show_info, show_warning},
    chat::{RoomManager, Room, ChatMessage, MessageType, RoomUser, UserRole, RoomSettings, RoomType},
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
    help_scroll: usize,      // Track help popup scroll position
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
        }
    }
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize default room and user for chat
    pub fn initialize_chat(&mut self, username: String) -> Result<(), Box<dyn std::error::Error>> {
        // Create a default public room
        let room_settings = RoomSettings::public("General".to_string());
        let room_id = self.room_manager.create_room(room_settings, uuid::Uuid::nil())?;
        
        // Create user
        let user_id = uuid::Uuid::new_v4();
        let user = RoomUser::new(user_id, username, UserRole::User);
        
        // Add user to room
        self.room_manager.join_room(&room_id, user)?;
        
        // Set current context
        self.current_room_id = Some(room_id);
        self.current_user_id = Some(user_id);
        
        Ok(())
    }

    /// Get current room messages for display
    fn get_display_messages(&self) -> Vec<String> {
        if let Some(room_id) = self.current_room_id {
            if let Some(room) = self.room_manager.get_room(&room_id) {
                return room.get_messages(Some(50))
                    .iter()
                    .map(|msg| {
                        if msg.message_type == MessageType::System {
                            msg.content.clone()
                        } else {
                            format!("{}: {}", msg.sender_username, msg.content)
                        }
                    })
                    .collect();
            }
        }
        
        // Fallback to legacy system if no room is available
        MESSAGES.lock().unwrap().text.clone()
    }

    /// Check if chat system is initialized (has current room and user)
    pub fn is_chat_initialized(&self) -> bool {
        self.current_room_id.is_some() && self.current_user_id.is_some()
    }

    /// Add a message to current room
    pub fn add_message_to_room(&mut self, content: String, is_system: bool) {
        if let (Some(room_id), Some(user_id)) = (self.current_room_id, self.current_user_id) {
            if let Some(room) = self.room_manager.get_room_mut(&room_id) {
                let message = if is_system {
                    ChatMessage::new_system(room_id, content)
                } else {
                    let username = room.get_user(&user_id)
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    ChatMessage::new_text(room_id, user_id, username, content)
                };
                
                let _ = room.add_message(message);
            }
        } else {
            // Fallback to legacy system
            add_text_message(content);
        }
    }

    // Connection dialog methods
    fn show_dialog(&mut self) {
        self.dialog_visible = true;
        self.dialog_cursor_position = 0;
    }

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
                Ok(addr) => {
                    // Schedule connection
                    self.hide_dialog();
                    
                    // Reset the inputs for next time
                    self.dialog_host_input = Input::default();
                    self.dialog_port_input = Input::default();
                    
                    let input = self.input.clone();
                    tokio::spawn(async move {
                        let _ = migration_facade::connect_client(input, addr).await;
                    });
                    
                    return Ok(Some(Action::Update));
                }
                Err(_) => {
                    show_validation_error("Address", "invalid format - use host:port (e.g., 127.0.0.1:8080)");
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
        self.command_tx = Some(tx);
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
                    },
                    KeyCode::PageDown => {
                        self.help_scroll = self.help_scroll.saturating_add(5);
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::Up => {
                        if self.help_scroll > 0 {
                            self.help_scroll = self.help_scroll.saturating_sub(1);
                        }
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::Down => {
                        self.help_scroll = self.help_scroll.saturating_add(1);
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::Home => {
                        self.help_scroll = 0;
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::End => {
                        // Will be capped in the render code
                        self.help_scroll = 999; // Large number, will be constrained by max scroll
                        return Ok(Some(Action::Render));
                    },
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
                    },
                    KeyCode::Up => {
                        // Enter manual scroll mode
                        self.manual_scroll = true;
                        
                        // Scroll up by one line
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::PageDown => {
                        // Scroll down by increasing the scroll offset
                        self.scroll_offset += 5;
                        
                        // If we reach the bottom, enable auto-follow again
                        let messages_len = self.get_display_messages().len();
                        if self.scroll_offset >= messages_len {
                            self.manual_scroll = false;
                        }
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::Down => {
                        // Scroll down by one line
                        self.scroll_offset += 1;
                        
                        // If we reach the bottom, enable auto-follow again
                        let messages_len = self.get_display_messages().len();
                        if self.scroll_offset >= messages_len {
                            self.manual_scroll = false;
                        }
                        return Ok(Some(Action::Render));
                    },
                    KeyCode::End => {
                        // Scroll to the end and re-enable auto-follow
                        let messages_len = self.get_display_messages().len();
                        self.scroll_offset = messages_len;
                        self.manual_scroll = false;
                        return Ok(Some(Action::Render));
                    },
                    // Cancel scroll mode and return to auto-follow on Escape
                    KeyCode::Esc => {
                        if !self.show_help {
                            let messages_len = self.get_display_messages().len();
                            self.scroll_offset = messages_len;
                            self.manual_scroll = false;
                            return Ok(Some(Action::Render));
                        }
                    },
                    KeyCode::Home => {
                        // Scroll to the top
                        self.scroll_offset = 0;
                        self.manual_scroll = true;
                        return Ok(Some(Action::Render));
                    },
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
                            self.dialog_host_input.handle_event(&crossterm::event::Event::Key(key));
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
                                        let new_value = current[..current.len()-1].to_string();
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
            Mode::Normal | Mode::Processing => match key.code {
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
                    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::DISCONNECTED {
                        show_disconnection();
                        return Ok(Some(Action::EnterInsert));
                    }
                    Action::EnterInsert
                }
                KeyCode::F(2) => {
                    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
                        show_warning("Already connected to a server");
                        return Ok(Some(Action::Update));
                    }
                    show_info("Please use the authentication system to connect (restart the application)");
                    Action::Update
                }
                KeyCode::Char('c') => {
                    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
                        show_warning("Already connected to a server");
                        return Ok(Some(Action::Update));
                    }
                    show_info("Please use the authentication system to connect (restart the application)");
                    Action::Update
                }
                KeyCode::Char('d') => {
                    self.schedule_disconnect_client();
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
            },
            Mode::Insert => match key.code {
                KeyCode::F(2) => {
                    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
                        show_warning("Already connected to a server");
                        return Ok(Some(Action::Update));
                    }
                    show_info("Please use the authentication system to connect (restart the application)");
                    Action::Update
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
                    let client_status = CLIENT_STATUS.lock().unwrap();
                    if client_status.status == ConnectionStatus::DISCONNECTED {
                        show_warning("Not connected - please restart the application to reconnect and authenticate");
                        return Ok(Some(Action::Update));
                    }
                    let message = self.input.value().to_string();
                    if !message.is_empty() && client_status.status == ConnectionStatus::CONNECTED {
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
                        if !message.is_empty() {
                            show_warning("Cannot send message - connection lost. Please restart the application");
                        }
                        if client_status.status == ConnectionStatus::CONNECTED {
                            show_info("Please enter a message before pressing Enter");
                        }
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
                                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
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
                                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
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
                    if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete) {
                        self.command_history.reset_position();
                    }
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Tick
                }
            },
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
            },
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
                    show_info("Enter a server address in the format host:port (e.g., 127.0.0.1:8080)");
                    return Ok(Some(Action::Update));
                }
                let address: SocketAddr = match user_input.parse() {
                    Ok(address) => address,
                    Err(_) => {
                        show_validation_error("Server address", "invalid format - use host:port");
                        return Ok(Some(Action::Update));
                    }
                };
                let input = self.input.clone();
                tokio::spawn(async move {
                    let _ = migration_facade::connect_client(input, address).await;
                });
            }
            Action::ShowConnectionDialog => {
                show_info("Please use the authentication system to connect (restart the application)");
            }
            Action::DisconnectClient => {
                tokio::spawn(async move {
                    let _ = migration_facade::disconnect_client().await;
                });
            }
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
                Constraint::Min(1),     // Main content area
                Constraint::Length(1),  // Scrollbar
            ])
            .split(rects[0]);
            
        let content_area = content_layout[0];
        let scrollbar_area = content_layout[1];
        
        // Prepare text content
        let mut text: Vec<Line> = Vec::<Line>::new();
        text.push("".into());
        let messages: Vec<Line> = self.get_display_messages()
            .iter()
            .map(|l| Line::from(l.clone()))
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
                    self.scroll_offset.min(text_len.saturating_sub(available_height))
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
            let thumb_height = ((scrollbar_height as f64 * available_height as f64) / content_height as f64).max(1.0) as usize;
            let thumb_position = ((scroll_position as f64 * scrollbar_height as f64) / content_height as f64) as usize;
            
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
                    
                    let scrollbar_piece = Paragraph::new(symbol.clone())
                        .style(Style::default().fg(color));
                    frame.render_widget(
                        scrollbar_piece,
                        Rect::new(
                            scrollbar_area.x,
                            scrollbar_area.y + 1 + i as u16, // +1 for top border
                            1,
                            1
                        )
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
                        .title_top(Line::from("v0.5.0".white()).left_aligned())
                        .title_top(Line::from(vec![
                            Span::styled("THE LAIR", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        ]).centered())
                        .title_top(Line::from("(C) 2025".white()).right_aligned())
                        .borders(content_borders)
                        .border_style(match self.mode {
                            Mode::Processing => Style::default().bg(Color::Black).fg(Color::Yellow),
                            _ => Style::default().bg(Color::Black).fg(Color::Cyan),
                        })
                        .border_type(BorderType::Rounded),
                )
                .style(Style::default().bg(Color::Black).fg(Color::Green))
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
                    Constraint::Length(1),  // Padding
                    Constraint::Length(3),  // Host input
                    Constraint::Length(3),  // Port input 
                    Constraint::Length(1),  // Buttons
                    Constraint::Length(1),  // Padding
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
            let host_input = Paragraph::new("")
                .block(host_block)
                .style(host_input_style);
            
            frame.render_widget(host_input, layout[1]);
            
            // Create inner area for text with better padding
            let host_inner_area = layout[1].inner(Margin {
                vertical: 1,    // Avoid overwriting the title
                horizontal: 2,  // Add horizontal padding for better appearance
            });
            
            // Render the host value in the inner area only
            let host_text = Paragraph::new(host_value)
            .style(Style::default()
                .fg(Color::White)  // White is more readable on dark gray background
                .add_modifier(Modifier::BOLD))
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
            let port_input = Paragraph::new("")
                .block(port_block)
                .style(port_input_style);
            
            frame.render_widget(port_input, layout[2]);
            
            // Create larger inner area for port text to ensure visibility
            let port_inner_area = layout[2].inner(Margin { 
                vertical: 1,    // Avoid overwriting the title
                horizontal: 2,  // Add more horizontal padding for better appearance
            });
            
            // Render the port value with enhanced visibility - make it stand out more
            let display_text = port_value;
            
            // Use a bold, bright text to ensure visibility
            let value_text = Paragraph::new(display_text)
                .style(Style::default()
                    .fg(Color::White)  // White is more readable on dark gray background
                    .add_modifier(Modifier::BOLD))
                .alignment(Alignment::Left);
            
            frame.render_widget(value_text, port_inner_area);

            // Buttons
            let button_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
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
                    Constraint::Min(10),      // Content area
                    Constraint::Length(1),    // Scrollbar
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
                let thumb_height = ((available_height as f64 / rows.len() as f64) * scrollbar_height as f64).max(1.0) as usize;
                let thumb_position = ((self.help_scroll as f64 / max_scroll as f64) * (scrollbar_height - thumb_height) as f64) as usize;
                
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
                        let scrollbar_piece = Paragraph::new(symbol.clone())
                            .style(Style::default().fg(Color::Gray));
                        frame.render_widget(
                            scrollbar_piece,
                            Rect::new(
                                scrollbar_area.x,
                                scrollbar_area.y + 1 + i as u16, // +1 for top border
                                1,
                                1
                            )
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
                        1
                    )
                );
            }
        };



        Ok(())
    }
}
