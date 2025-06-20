//! Client-side Message Router Implementation
//!
//! This module provides the client-side implementation of the unified message routing system.
//! It handles displaying messages in the UI, managing message state, and coordinating with
//! the home component for proper message presentation.

use crate::action::Action;
use crate::common::messaging::{ChatMessage, Message, MessageRoute, MessageTarget, SystemMessage};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Client-side message router that handles displaying messages in the UI
pub struct ClientMessageRouter {
    /// Channel to send actions to the main app loop
    action_sender: mpsc::UnboundedSender<Action>,
    /// Current username (if authenticated)
    current_user: Option<String>,
    /// Whether debug logging is enabled
    debug_enabled: bool,
}

/// Trait for routing messages in the client
pub trait MessageRouter {
    /// Route a message to its destination(s)
    fn route_message(&mut self, route: MessageRoute) -> Result<(), String>;

    /// Route a system message to a specific target
    fn route_system_message(
        &mut self,
        target: MessageTarget,
        message: SystemMessage,
    ) -> Result<(), String>;

    /// Route a chat message to a specific target
    fn route_chat_message(
        &mut self,
        target: MessageTarget,
        message: ChatMessage,
    ) -> Result<(), String>;

    /// Handle display of a message in the UI
    fn display_message(
        &mut self,
        message: &Message,
        is_for_current_user: bool,
    ) -> Result<(), String>;

    /// Check if a message should be displayed for the current user
    fn should_display_for_user(&self, target: &MessageTarget) -> bool;

    /// Parse and route a raw protocol message from the server
    fn parse_and_route_protocol_message(&mut self, raw_message: &str) -> Result<(), String>;
}

impl ClientMessageRouter {
    /// Create a new client message router
    pub fn new(action_sender: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            action_sender,
            current_user: None,
            debug_enabled: true,
        }
    }

    /// Set the current authenticated user
    pub fn set_current_user(&mut self, username: Option<String>) {
        self.current_user = username.clone();
        debug!("ClientMessageRouter: Current user set to {:?}", username);
    }

    /// Enable or disable debug logging
    pub fn set_debug_enabled(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
    }

    /// Get the current user
    pub fn current_user(&self) -> Option<&String> {
        self.current_user.as_ref()
    }

    /// Log debug message if debug is enabled
    fn debug_log(&self, message: &str) {
        if self.debug_enabled {
            debug!("🔀 MessageRouter: {}", message);
        }
    }

    /// Send an action to the main app loop
    fn send_action(&self, action: Action) -> Result<(), String> {
        debug!("Attempting to send action: {:?}", action);
        match self.action_sender.send(action) {
            Ok(()) => {
                debug!("Action sent successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to send action: {}", e);
                Err(format!("Failed to send action: {}", e))
            }
        }
    }

    /// Handle direct message display
    fn handle_direct_message(&mut self, from: &str, to: &str, content: &str) -> Result<(), String> {
        let current_user = self.current_user.as_ref().ok_or("No current user set")?;

        debug!(
            "handle_direct_message - from: '{}', to: '{}', content: '{}', current_user: '{}'",
            from, to, content, current_user
        );

        // Only display if this DM is for the current user
        if to == current_user {
            debug!("DM is for current user, adding to DM conversation (without auto-entering)");
            self.debug_log(&format!(
                "Adding DM message without auto-entering DM mode: {} -> {}",
                from, content
            ));

            // DON'T automatically start DM conversation - just add the message
            // The user can choose to enter DM mode by clicking the sidebar

            // Display the DM message - it will be routed to DM conversation properly
            let display_message = format!("💬 {}: {}", from, content);
            debug!(
                "Sending DisplayMessage action for DM: '{}'",
                display_message
            );
            self.send_action(Action::DisplayMessage {
                content: display_message,
                is_system: false, // DMs are not system messages
            })?;

            // Update unread DM count for notification
            debug!("Updating unread DM count");
            self.send_action(Action::UpdateUnreadDMCount(1))?;
        } else {
            debug!(
                "DM not for current user (to: '{}', current: '{}'), ignoring",
                to, current_user
            );
            self.debug_log(&format!(
                "Ignoring DM from {} to {} (not for current user)",
                from, to
            ));
        }

        Ok(())
    }

    /// Handle DM confirmation display
    fn handle_dm_confirmation(&mut self, target: &str, content: &str) -> Result<(), String> {
        self.debug_log(&format!(
            "Handling DM confirmation for target: {}, content: {}",
            target, content
        ));
        debug!(
            "🔍 MESSAGE_ROUTER: handle_dm_confirmation called - target: '{}', content: '{}'",
            target, content
        );

        // Just show a brief system confirmation - the sent message will be added
        // when the user actually sends it via the DM system
        let confirmation_message = format!("✅ Sent to {}", target);
        debug!(
            "🔍 MESSAGE_ROUTER: Sending confirmation message: '{}'",
            confirmation_message
        );
        self.send_action(Action::DisplayMessage {
            content: confirmation_message,
            is_system: true,
        })?;

        debug!("🔍 MESSAGE_ROUTER: DM confirmation processing completed successfully");
        Ok(())
    }

    /// Handle user join/leave notifications
    fn handle_user_presence(
        &mut self,
        username: &str,
        room: &str,
        joined: bool,
    ) -> Result<(), String> {
        let current_user = self.current_user.as_ref().ok_or("No current user set")?;

        // Don't show notifications for the current user's own actions
        if username == current_user {
            return Ok(());
        }

        let icon = if joined { "👋" } else { "🚪" };
        let action = if joined { "joined" } else { "left" };
        let display_message = format!("{} {} {} {}", icon, username, action, room);

        self.debug_log(&format!("User presence update: {}", display_message));

        self.send_action(Action::DisplayMessage {
            content: display_message,
            is_system: true,
        })?;

        Ok(())
    }

    /// Handle room-related messages
    fn handle_room_message(&mut self, message: &SystemMessage) -> Result<(), String> {
        match message {
            SystemMessage::RoomCreated { room_name, creator } => {
                // Send the RoomCreated action for proper tab creation
                self.send_action(Action::RoomCreated(room_name.clone()))?;

                // Also send a display message for user feedback
                let display_message = format!("🏠 Room '{}' created", room_name);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
            }
            SystemMessage::RoomJoined {
                room_name,
                username,
            } => {
                self.send_action(Action::RoomJoined(room_name.clone()))?;
                if let Some(current_user) = &self.current_user {
                    if username == current_user {
                        // Update status bar for current user
                        self.send_action(Action::UpdateCurrentRoom(room_name.clone()))?;
                    }
                }
            }
            SystemMessage::RoomLeft {
                room_name,
                username,
            } => {
                self.send_action(Action::RoomLeft(room_name.clone()))?;
                if let Some(current_user) = &self.current_user {
                    if username == current_user {
                        // Return to lobby
                        self.send_action(Action::ReturnToLobby)?;
                    }
                }
            }
            SystemMessage::RoomListResponse { rooms } => {
                self.send_action(Action::RoomListReceived(rooms.clone()))?;
            }
            SystemMessage::RoomNotFound { room_name } => {
                let display_message = format!("❌ Room '{}' not found", room_name);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
            }
            SystemMessage::RoomAlreadyExists { room_name } => {
                let display_message = format!("⚠️ Room '{}' already exists", room_name);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
            }
            _ => {
                // Generic room message display
                let display_message = format!("{}", message);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
            }
        }
        Ok(())
    }

    /// Handle invitation messages
    fn handle_invitation_message(&mut self, message: &SystemMessage) -> Result<(), String> {
        match message {
            SystemMessage::InvitationReceived {
                from,
                room_name,
                message: invite_msg,
            } => {
                // Send specific invitation action
                let result = self.send_action(Action::InvitationReceived(
                    from.clone(),
                    room_name.clone(),
                    invite_msg.clone(),
                ));
                result?;
            }
            SystemMessage::InvitationSent { to, room_name } => {
                let display_message = format!("Invitation sent to {} for room '{}'", to, room_name);
                let result = self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                });
                result?;
            }
            SystemMessage::InvitationAccepted { from, room_name } => {
                self.send_action(Action::InvitationAccepted(room_name.clone()))?;
            }
            SystemMessage::InvitationDeclined { from, room_name } => {
                self.send_action(Action::InvitationDeclined(room_name.clone()))?;
            }
            SystemMessage::InvitationError { reason } => {
                self.send_action(Action::InviteError(reason.clone()))?;
            }
            _ => {
                let display_message = format!("{}", message);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
            }
        }
        Ok(())
    }

    /// Handle authentication messages
    fn handle_auth_message(&mut self, message: &SystemMessage) -> Result<(), String> {
        match message {
            SystemMessage::Welcome { username } => {
                self.set_current_user(Some(username.clone()));
                // Let the existing auth system handle this
                Ok(())
            }
            SystemMessage::AuthenticationFailed { reason } => {
                self.send_action(Action::AuthenticationFailure(reason.clone()))?;
                Ok(())
            }
            SystemMessage::RegistrationSuccess { username } => {
                self.send_action(Action::RegistrationSuccess(username.clone()))?;
                Ok(())
            }
            _ => {
                let display_message = format!("{}", message);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                Ok(())
            }
        }
    }
}

impl ClientMessageRouter {
    /// Parse raw protocol messages and convert them to the unified system
    pub fn parse_and_route_protocol_message(&mut self, raw_message: &str) -> Result<(), String> {
        self.debug_log(&format!("Parsing protocol message: '{}'", raw_message));

        // Handle PRIVATE_MESSAGE format: "PRIVATE_MESSAGE:from:content"
        if let Some(dm_data) = raw_message.strip_prefix("PRIVATE_MESSAGE:") {
            debug!("Found PRIVATE_MESSAGE, parsing: '{}'", dm_data);
            let parts: Vec<&str> = dm_data.splitn(2, ':').collect();
            if parts.len() == 2 {
                let from = parts[0].to_string();
                let content = parts[1].to_string();
                debug!("Parsed DM - from: '{}', content: '{}'", from, content);

                // Get current user to determine target
                if let Some(current_user) = &self.current_user {
                    debug!("Current user is '{}', creating DM route", current_user);
                    let message = SystemMessage::DirectMessage {
                        from: from.clone(),
                        to: current_user.clone(),
                        content: content.clone(),
                    };
                    let route = MessageRoute::new(
                        MessageTarget::User(current_user.clone()),
                        Message::System(message),
                    );
                    debug!("Routing DM message to user '{}'", current_user);
                    return self.route_message(route);
                } else {
                    warn!("No current user set, cannot route DM");
                    return Err("No current user set for PRIVATE_MESSAGE".to_string());
                }
            } else {
                warn!(
                    "Invalid PRIVATE_MESSAGE format - expected 2 parts, got {}",
                    parts.len()
                );
            }
            return Err("Invalid PRIVATE_MESSAGE format".to_string());
        }

        // Handle SYSTEM_MESSAGE format: "SYSTEM_MESSAGE:content"
        if let Some(system_content) = raw_message.strip_prefix("SYSTEM_MESSAGE:") {
            let content = system_content.trim();

            // Parse specific system message patterns
            if content.starts_with("DM sent to ") {
                // Extract target and content from "DM sent to target: content" or "DM sent to target:content"
                if let Some(rest) = content.strip_prefix("DM sent to ") {
                    // Try parsing with ": " first, then fall back to ":"
                    let parts: Vec<&str> = if rest.contains(": ") {
                        rest.splitn(2, ": ").collect()
                    } else {
                        rest.splitn(2, ":").collect()
                    };

                    if parts.len() == 2 {
                        let target = parts[0].to_string();
                        let dm_content = parts[1].to_string();
                        let message = SystemMessage::DirectMessageConfirmation {
                            target,
                            content: dm_content,
                        };
                        let route =
                            MessageRoute::new(MessageTarget::Sender, Message::System(message));
                        return self.route_message(route);
                    }
                }
            }

            // Handle invitation messages
            if content.contains(" invited you to join room '") {
                // Parse "user invited you to join room 'roomname'"
                if let Some(inviter_end) = content.find(" invited you to join room '") {
                    let inviter = &content[..inviter_end];
                    let pattern = " invited you to join room '";
                    let rest = &content[inviter_end + pattern.len()..]; // Skip the pattern
                    if let Some(room_end) = rest.find("'") {
                        let room_name = &rest[..room_end];

                        // Direct display approach - bypass action routing for immediate visibility
                        let invitation_display = format!("🔔 INVITATION: {}", content);
                        self.send_action(Action::DisplayMessage {
                            content: invitation_display,
                            is_system: true,
                        })?;

                        let instructions = format!(
                            "💡 To respond: '/accept {}' or '/decline {}' or just '/accept' for latest",
                            room_name, room_name
                        );
                        self.send_action(Action::DisplayMessage {
                            content: instructions,
                            is_system: true,
                        })?;

                        let alternatives = format!(
                            "   You can also use '/join {}' to accept or '/invites' to see all pending",
                            room_name
                        );
                        self.send_action(Action::DisplayMessage {
                            content: alternatives,
                            is_system: true,
                        })?;

                        // Also send the traditional action for any other handlers
                        self.send_action(Action::InvitationReceived(
                            inviter.to_string(),
                            room_name.to_string(),
                            content.to_string(),
                        ))?;

                        return Ok(());
                    }
                }
            }

            if content.starts_with("You invited ") && content.contains(" to join room '") {
                // Parse "You invited user to join room 'roomname'"
                if let Some(user_start) = content.find("You invited ") {
                    let rest = &content[user_start + 12..]; // Skip "You invited "
                    if let Some(user_end) = rest.find(" to join room '") {
                        let target_user = &rest[..user_end];
                        let room_part = &rest[user_end + 15..]; // Skip " to join room '"
                        if let Some(room_end) = room_part.find("'") {
                            let room_name = &room_part[..room_end];
                            let message = SystemMessage::InvitationSent {
                                to: target_user.to_string(),
                                room_name: room_name.to_string(),
                            };
                            let route =
                                MessageRoute::new(MessageTarget::Sender, Message::System(message));
                            return self.route_message(route);
                        }
                    }
                }
            }

            // Handle error messages
            if content.starts_with("ERROR: ") {
                let error_content = &content[7..]; // Skip "ERROR: "
                let display_message = format!("❌ {}", error_content);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            // Handle authentication messages
            if content.contains("Welcome to The Lair")
                || content.contains("Please login or register")
            {
                let display_message = format!("🎉 {}", content);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            if content.contains("Authentication successful")
                || content.contains("Welcome to The Lair!")
            {
                let display_message = format!("✅ {}", content);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            if content.starts_with("Authentication failed") || content.contains("Login failed") {
                let display_message = format!("❌ {}", content);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            // Handle invitation-related error messages
            if content.starts_with("ERROR: Room ") && content.contains(" does not exist") {
                let display_message = format!("Invitation failed: {}", &content[7..]); // Remove "ERROR: " prefix
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            if content.starts_with("ERROR: You must be in room ")
                && content.contains(" to invite others")
            {
                let display_message = format!("Invitation failed: {}", &content[7..]); // Remove "ERROR: " prefix
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            if content.starts_with("ERROR: User ")
                && (content.contains(" is not online or not found")
                    || content.contains(" is not online"))
            {
                let display_message = format!("Invitation failed: {}", &content[7..]); // Remove "ERROR: " prefix
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                return Ok(());
            }

            // Handle other system messages generically
            let message = SystemMessage::StatusUpdate {
                message: content.to_string(),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle ROOM_STATUS format: "ROOM_STATUS:room_name,username"
        if let Some(status_data) = raw_message.strip_prefix("ROOM_STATUS:") {
            let parts: Vec<&str> = status_data.splitn(2, ',').collect();
            if parts.len() == 2 {
                let room_name = parts[0];
                let username = parts[1];

                // Send user presence update
                let display_message = format!("👤 {} joined {}", username, room_name);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;

                // Also update connected users if needed
                // This could trigger a user list refresh
                return Ok(());
            }
        }

        // Handle USER_LIST format: "USER_LIST:user1,user2,user3"
        if let Some(user_list_str) = raw_message.strip_prefix("USER_LIST:") {
            let users: Vec<String> = if user_list_str.is_empty() {
                Vec::new()
            } else {
                user_list_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            };
            let message = SystemMessage::UserListUpdate { users };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle ROOM_CREATED format: "ROOM_CREATED:room_name"
        if let Some(room_name) = raw_message.strip_prefix("ROOM_CREATED:") {
            let message = SystemMessage::RoomCreated {
                room_name: room_name.to_string(),
                creator: self.current_user.clone().unwrap_or_default(),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle ROOM_JOINED format: "ROOM_JOINED:room_name"
        if let Some(room_name) = raw_message.strip_prefix("ROOM_JOINED:") {
            let message = SystemMessage::RoomJoined {
                room_name: room_name.to_string(),
                username: self.current_user.clone().unwrap_or_default(),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle ROOM_LEFT format: "ROOM_LEFT:room_name"
        if let Some(room_name) = raw_message.strip_prefix("ROOM_LEFT:") {
            let display_message = format!("🚪 Left room: {}", room_name);
            self.send_action(Action::DisplayMessage {
                content: display_message,
                is_system: true,
            })?;

            // Also update current room back to Lobby
            self.send_action(Action::UpdateCurrentRoom("Lobby".to_string()))?;
            return Ok(());
        }

        // Handle CURRENT_ROOM format: "CURRENT_ROOM:room_name"
        if let Some(room_name) = raw_message.strip_prefix("CURRENT_ROOM:") {
            // Send action to update current room in status bar and UI
            self.send_action(Action::UpdateCurrentRoom(room_name.to_string()))?;
            return Ok(());
        }

        // Handle ROOM_LIST format: "ROOM_LIST:room1,room2,room3"
        if let Some(room_list_str) = raw_message.strip_prefix("ROOM_LIST:") {
            let rooms: Vec<String> = if room_list_str.is_empty() {
                Vec::new()
            } else {
                room_list_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            };

            let message = SystemMessage::RoomListResponse { rooms };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle ROOM_ERROR format: "ROOM_ERROR:error_message"
        if let Some(error_msg) = raw_message.strip_prefix("ROOM_ERROR:") {
            let message = SystemMessage::Error {
                message: format!("Room error: {}", error_msg),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle INVITE_ERROR format: "INVITE_ERROR:error_message"
        if let Some(error_msg) = raw_message.strip_prefix("INVITE_ERROR:") {
            let message = SystemMessage::InvitationError {
                reason: error_msg.to_string(),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle welcome and authentication messages
        if raw_message.contains("Welcome back") || raw_message.contains("has joined the chat") {
            // Extract username for welcome messages
            let username = if raw_message.contains("Welcome back") {
                raw_message
                    .split("Welcome back, ")
                    .nth(1)
                    .and_then(|s| s.split(',').next())
                    .unwrap_or("User")
                    .to_string()
            } else if raw_message.contains("has joined") {
                raw_message
                    .split(" has joined")
                    .next()
                    .unwrap_or("User")
                    .to_string()
            } else {
                "User".to_string()
            };

            let message = SystemMessage::Welcome { username };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        // Handle USER_LIST format: "USER_LIST:user1,user2,user3"
        if let Some(user_list_str) = raw_message.strip_prefix("USER_LIST:") {
            // Just update the user list, don't display it as a message
            let users: Vec<String> = if user_list_str.is_empty() {
                Vec::new()
            } else {
                user_list_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            };

            self.send_action(Action::UpdateConnectedUsers(users))?;
            return Ok(());
        }

        // STEP 0: Filter ONLY display-harmful protocol messages, allow legitimate processing
        // Filter Reconnected User protocol messages (these should never be displayed)
        if raw_message.starts_with("Reconnected User: ")
            && !raw_message.contains(" joined the room")
        {
            debug!("🚫 FILTERED Reconnected User protocol: '{}'", raw_message);
            tracing::warn!(
                "FILTERING DEBUG: Reconnected User message caught in STEP 0: '{}'",
                raw_message
            );
            return Ok(());
        }

        // Filter username-prefixed protocol messages that should never be displayed
        if raw_message.contains(": ") && !raw_message.starts_with("ERROR:") {
            let parts: Vec<&str> = raw_message.splitn(2, ": ").collect();
            if parts.len() == 2 {
                let from = parts[0];
                let content = parts[1];

                // Filter only username: PROTOCOL patterns that should never be chat messages
                if (content.starts_with("USER_LIST:") || content.starts_with("USER_LIST"))
                    || (content.starts_with("ROOM_LIST:") || content.starts_with("ROOM_LIST"))
                    || (content.starts_with("CURRENT_ROOM:") || content.starts_with("CURRENT_ROOM"))
                    || (content.starts_with("ROOM_STATUS:") || content.starts_with("ROOM_STATUS"))
                    || (content == "true" || content.trim() == "true")
                    || content.starts_with("REQUEST_USER_LIST")
                {
                    debug!(
                        "🚫 FILTERED username protocol: '{}' from '{}'",
                        content, from
                    );
                    tracing::warn!(
                        "FILTERING DEBUG: Username protocol message caught in STEP 0: from='{}', content='{}'",
                        from, content
                    );
                    return Ok(());
                }
            }
        }

        // STEP 1: Check if this is a pure protocol message that should be silently processed
        if self.is_pure_protocol_message(raw_message) {
            debug!("🚫 FILTERED protocol message: '{}'", raw_message);
            if raw_message.contains("Reconnected User") {
                tracing::warn!(
                    "FILTERING DEBUG: Reconnected User message caught in STEP 1: '{}'",
                    raw_message
                );
            }
            return Ok(());
        }

        // STEP 2: Handle messages that might be displayable (format system messages properly)
        if !raw_message.is_empty() {
            // Handle username-prefixed messages
            if raw_message.contains(": ") && !raw_message.starts_with("ERROR:") {
                let parts: Vec<&str> = raw_message.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    let from = parts[0].to_string();
                    let content = parts[1].to_string();

                    // STEP 2A: EXPLICIT check for Reconnected User protocol messages
                    if from == "Reconnected User" && !content.contains(" joined the room") {
                        debug!(
                            "🚫 FILTERED Reconnected User protocol content: '{}'",
                            content
                        );
                        tracing::warn!("FILTERING DEBUG: Reconnected User message caught in STEP 2A: from='{}', content='{}'", from, content);
                        return Ok(());
                    }

                    // STEP 2B: Check if this is STILL a protocol message (username: PROTOCOL)
                    if self.is_protocol_content(&content) {
                        debug!(
                            "🚫 FILTERED username-prefixed protocol: from='{}', content='{}'",
                            from, content
                        );
                        return Ok(());
                    }

                    // STEP 2C: Format SYSTEM_MESSAGE content properly
                    if content.starts_with("SYSTEM_MESSAGE:") {
                        return self.format_system_message(&content[15..]);
                    }

                    // STEP 2D: Handle special "Reconnected User" messages
                    if from == "Reconnected User" {
                        // Only allow room join messages, filter everything else
                        if content.contains(" joined the room") {
                            return self.handle_reconnected_user_message(&content);
                        } else {
                            debug!("🚫 FILTERED Reconnected User protocol: '{}'", content);
                            tracing::warn!("FILTERING DEBUG: Reconnected User message caught in STEP 2D: from='{}', content='{}'", from, content);
                            return Ok(());
                        }
                    }

                    // STEP 2E: This is a legitimate chat message
                    debug!(
                        "💬 LEGITIMATE chat message: from='{}', content='{}'",
                        from, content
                    );
                    let chat_msg = ChatMessage::new(from, content, "lobby".to_string());
                    let route = MessageRoute::new(
                        MessageTarget::Room("lobby".to_string()),
                        Message::Chat(chat_msg),
                    );
                    return self.route_message(route);
                }
            }

            // STEP 3: Handle direct system messages (no username prefix)
            if raw_message.starts_with("SYSTEM_MESSAGE:") {
                return self.format_system_message(&raw_message[15..]);
            }

            // STEP 4: Final check - is this still a protocol message?
            if self.is_protocol_content(raw_message) {
                debug!("🚫 FILTERED direct protocol: '{}'", raw_message);
                return Ok(());
            }

            // STEP 5: Treat as generic system message
            let message = SystemMessage::StatusUpdate {
                message: raw_message.to_string(),
            };
            let route = MessageRoute::new(MessageTarget::Sender, Message::System(message));
            return self.route_message(route);
        }

        Ok(())
    }

    /// Check if a message is pure protocol data that should never be displayed
    fn is_pure_protocol_message(&self, message: &str) -> bool {
        // Direct protocol commands
        message == "REQUEST_USER_LIST"
            || message.starts_with("DM:")
            || message.starts_with("INVITATION_LIST:")
            || message == "true"
            || message.trim() == "true"
            || message.ends_with(": true")
            || message.contains(": true")
            // Protocol with colons
            || message.starts_with("USER_LIST:")
            || message.starts_with("ROOM_LIST:")
            || message.starts_with("CURRENT_ROOM:")
            || message.starts_with("ROOM_STATUS:")
            || message.starts_with("ROOM_CREATED:")
            || message.starts_with("ROOM_JOINED:")
            || message.starts_with("ROOM_LEFT:")
            || message.starts_with("ACCEPT_INVITATION:")
            || message.starts_with("DECLINE_INVITATION:")
            || message.starts_with("JOIN_ROOM:")
            || message.starts_with("LEAVE_ROOM:")
            // Protocol without colons
            || message.starts_with("USER_LIST")
            || message.starts_with("ROOM_LIST")
            || message.starts_with("CURRENT_ROOM")
            || message.starts_with("ROOM_STATUS")
            || message.starts_with("ROOM_CREATED")
            || message.starts_with("ROOM_JOINED")
            || message.starts_with("ROOM_LEFT")
            || message.starts_with("ACCEPT_INVITATION")
            || message.starts_with("DECLINE_INVITATION")
            || message.starts_with("JOIN_ROOM")
            || message.starts_with("LEAVE_ROOM")
            // Reconnected User protocol messages (EXCEPT room joins)
            || (message.starts_with("Reconnected User: ") && !message.contains(" joined the room"))
            // Additional Reconnected User patterns
            || message.starts_with("Reconnected User:")
            // Username-prefixed protocol patterns
            || (message.contains(": ") && {
                let parts: Vec<&str> = message.splitn(2, ": ").collect();
                parts.len() == 2 && (
                    parts[1].starts_with("USER_LIST:") ||
                    parts[1].starts_with("USER_LIST") ||
                    parts[1].starts_with("ROOM_LIST:") ||
                    parts[1].starts_with("ROOM_LIST") ||
                    parts[1].starts_with("CURRENT_ROOM:") ||
                    parts[1].starts_with("CURRENT_ROOM") ||
                    parts[1].starts_with("ROOM_STATUS:") ||
                    parts[1].starts_with("ROOM_STATUS") ||
                    parts[1].starts_with("ROOM_CREATED:") ||
                    parts[1].starts_with("ROOM_CREATED") ||
                    parts[1].starts_with("ROOM_JOINED:") ||
                    parts[1].starts_with("ROOM_JOINED") ||
                    parts[1].starts_with("ROOM_LEFT:") ||
                    parts[1].starts_with("ROOM_LEFT") ||
                    parts[1] == "true" ||
                    parts[1].trim() == "true" ||
                    parts[1].starts_with("REQUEST_USER_LIST")
                )
            })
            // Heuristic patterns
            || (message.len() < 50 && message.matches(',').count() > 1)
            || (message.len() < 3)
    }

    /// Check if content (after username:) is protocol data
    fn is_protocol_content(&self, content: &str) -> bool {
        content == "true"
            || content.trim() == "true"
            || content.starts_with("USER_LIST:")
            || content.starts_with("USER_LIST")
            || content.starts_with("ROOM_LIST:")
            || content.starts_with("ROOM_LIST")
            || content.starts_with("CURRENT_ROOM:")
            || content.starts_with("CURRENT_ROOM")
            || content.starts_with("ROOM_STATUS:")
            || content.starts_with("ROOM_STATUS")
            || content.starts_with("ROOM_CREATED")
            || content.starts_with("ROOM_JOINED")
            || content.starts_with("ROOM_LEFT")
            || content.starts_with("ACCEPT_INVITATION")
            || content.starts_with("DECLINE_INVITATION")
            || content.starts_with("JOIN_ROOM")
            || content.starts_with("LEAVE_ROOM")
            || content.starts_with("REQUEST_USER_LIST")
            || (content.len() < 100 && content.matches(',').count() > 2)
    }

    /// Format SYSTEM_MESSAGE content into clean user-friendly messages
    fn format_system_message(&mut self, content: &str) -> Result<(), String> {
        // Invitation confirmations: "You invited user to join room 'name'"
        if content.starts_with("You invited ") && content.contains(" to join room '") {
            if let Some(user_start) = content.find("You invited ") {
                let rest = &content[user_start + 12..]; // Skip "You invited "
                if let Some(user_end) = rest.find(" to join room '") {
                    let target_user = &rest[..user_end];
                    let room_part = &rest[user_end + 15..]; // Skip " to join room '"
                    if let Some(room_end) = room_part.find("'") {
                        let room_name = &room_part[..room_end];
                        let formatted_message = format!(
                            "ℹ️  You invited {} to join room '{}'",
                            target_user, room_name
                        );
                        self.send_action(Action::DisplayMessage {
                            content: formatted_message,
                            is_system: true,
                        })?;
                        return Ok(());
                    }
                }
            }
        }

        // Incoming invitations: "user invited you to join room 'name'"
        if content.contains(" invited you to join room '") {
            if let Some(inviter_end) = content.find(" invited you to join room '") {
                let inviter = &content[..inviter_end];
                let pattern = " invited you to join room '";
                let rest = &content[inviter_end + pattern.len()..];
                if let Some(room_end) = rest.find("'") {
                    let room_name = &rest[..room_end];

                    // Clean invitation message
                    let invitation_msg =
                        format!("🔔 {} invited you to join room '{}'", inviter, room_name);
                    self.send_action(Action::DisplayMessage {
                        content: invitation_msg,
                        is_system: true,
                    })?;

                    // Instructions
                    let instructions = format!(
                        "ℹ️  You can '/accept {}' or '/decline {}' to respond",
                        room_name, room_name
                    );
                    self.send_action(Action::DisplayMessage {
                        content: instructions,
                        is_system: true,
                    })?;

                    return Ok(());
                }
            }
        }

        // Room joined confirmations: "Joined room 'name'"
        if content.starts_with("Joined room '") {
            if let Some(room_end) = content[13..].find("'") {
                let room_name = &content[13..13 + room_end];
                let formatted_message = format!("ℹ️  Joined room '{}'", room_name);
                self.send_action(Action::DisplayMessage {
                    content: formatted_message,
                    is_system: true,
                })?;
                return Ok(());
            }
        }

        // Error messages
        if content.starts_with("ERROR: ") {
            let error_content = &content[7..]; // Skip "ERROR: "
            let display_message = format!("❌ {}", error_content);
            self.send_action(Action::DisplayMessage {
                content: display_message,
                is_system: true,
            })?;
            return Ok(());
        }

        // Generic system message
        let display_message = format!("ℹ️  {}", content);
        self.send_action(Action::DisplayMessage {
            content: display_message,
            is_system: true,
        })?;
        Ok(())
    }

    /// Handle messages from "Reconnected User"
    fn handle_reconnected_user_message(&mut self, content: &str) -> Result<(), String> {
        // Room join events: "username joined the room"
        if content.contains(" joined the room") {
            if let Some(username_end) = content.find(" joined the room") {
                let username = &content[..username_end];
                let formatted_message = format!("ℹ️  {} joined the room", username);
                self.send_action(Action::DisplayMessage {
                    content: formatted_message,
                    is_system: true,
                })?;
                return Ok(());
            }
        }

        // Filter out other Reconnected User protocol messages
        if self.is_protocol_content(content) {
            debug!("🚫 FILTERED Reconnected User protocol: '{}'", content);
            return Ok(());
        }

        // Allow other Reconnected User messages (shouldn't happen, but safety)
        let display_message = format!("ℹ️  {}", content);
        self.send_action(Action::DisplayMessage {
            content: display_message,
            is_system: true,
        })?;
        Ok(())
    }
}

impl MessageRouter for ClientMessageRouter {
    fn route_message(&mut self, route: MessageRoute) -> Result<(), String> {
        let MessageRoute {
            target,
            message,
            priority: _,
            sender_id: _,
        } = route;

        self.debug_log(&format!("Routing message to {:?}: {}", target, message));

        // Check if this message should be displayed for the current user
        if !self.should_display_for_user(&target) {
            self.debug_log("Message not for current user, skipping display");

            return Ok(());
        }

        // Route based on message type
        match &message {
            Message::System(sys_msg) => self.route_system_message(target, sys_msg.clone()),
            Message::Chat(chat_msg) => self.route_chat_message(target, chat_msg.clone()),
        }
    }

    fn route_system_message(
        &mut self,
        target: MessageTarget,
        message: SystemMessage,
    ) -> Result<(), String> {
        self.debug_log(&format!("Routing system message: {:?}", message));

        // Handle specific system message types
        match &message {
            SystemMessage::DirectMessage { from, to, content } => {
                self.handle_direct_message(from, to, content)
            }
            SystemMessage::DirectMessageConfirmation { target, content } => {
                self.handle_dm_confirmation(target, content)
            }
            SystemMessage::UserJoined { username, room } => {
                self.handle_user_presence(username, room, true)
            }
            SystemMessage::UserLeft { username, room } => {
                self.handle_user_presence(username, room, false)
            }
            SystemMessage::UserListUpdate { users } => {
                // Send action to update connected users
                self.send_action(Action::UpdateConnectedUsers(users.clone()))?;
                Ok(())
            }
            SystemMessage::Error { message: error_msg } => {
                self.send_action(Action::Error(error_msg.clone()))?;
                Ok(())
            }

            // Room-related messages
            SystemMessage::RoomCreated { .. }
            | SystemMessage::RoomJoined { .. }
            | SystemMessage::RoomLeft { .. }
            | SystemMessage::RoomListResponse { .. }
            | SystemMessage::RoomNotFound { .. }
            | SystemMessage::RoomAlreadyExists { .. } => self.handle_room_message(&message),

            // Invitation-related messages
            SystemMessage::InvitationReceived { .. }
            | SystemMessage::InvitationSent { .. }
            | SystemMessage::InvitationAccepted { .. }
            | SystemMessage::InvitationDeclined { .. }
            | SystemMessage::InvitationError { .. } => self.handle_invitation_message(&message),

            // Authentication-related messages
            SystemMessage::Welcome { .. }
            | SystemMessage::AuthenticationFailed { .. }
            | SystemMessage::RegistrationSuccess { .. }
            | SystemMessage::Logout { .. } => self.handle_auth_message(&message),

            // Generic system messages
            _ => {
                let display_message = format!("{}", message);
                self.send_action(Action::DisplayMessage {
                    content: display_message,
                    is_system: true,
                })?;
                Ok(())
            }
        }
    }

    fn route_chat_message(
        &mut self,
        _target: MessageTarget,
        message: ChatMessage,
    ) -> Result<(), String> {
        self.debug_log(&format!(
            "Routing chat message from {}: {}",
            message.from, message.content
        ));

        // For chat messages, display with the username prefix
        let display_message = format!("{}: {}", message.from, message.content);
        self.send_action(Action::DisplayMessage {
            content: display_message,
            is_system: false,
        })?;

        Ok(())
    }

    fn display_message(
        &mut self,
        message: &Message,
        is_for_current_user: bool,
    ) -> Result<(), String> {
        if !is_for_current_user {
            return Ok(());
        }

        let display_content = format!("{}", message);
        let is_system = matches!(message, Message::System(_));

        self.send_action(Action::DisplayMessage {
            content: display_content,
            is_system,
        })?;

        Ok(())
    }

    fn should_display_for_user(&self, target: &MessageTarget) -> bool {
        match target {
            MessageTarget::User(username) => {
                // Only display if this message is for the current user
                self.current_user
                    .as_ref()
                    .map_or(false, |current| current == username)
            }
            MessageTarget::UserList(users) => {
                // Display if current user is in the list
                self.current_user
                    .as_ref()
                    .map_or(false, |current| users.contains(current))
            }
            MessageTarget::Room(_) => {
                // Always display room messages (the UI will filter by current room)
                true
            }
            MessageTarget::Broadcast => {
                // Always display broadcast messages
                true
            }
            MessageTarget::Sender => {
                // Always display messages targeted to the sender (current user)
                true
            }
            MessageTarget::Others => {
                // Don't display "others" messages for the current user
                false
            }
        }
    }

    fn parse_and_route_protocol_message(&mut self, raw_message: &str) -> Result<(), String> {
        self.parse_and_route_protocol_message(raw_message)
    }
}

/// Helper trait to add display actions
pub trait DisplayMessage {
    fn content(&self) -> String;
    fn is_system(&self) -> bool;
}

/// New action type for displaying messages through the router
#[derive(Debug, Clone)]
pub struct DisplayMessageAction {
    pub content: String,
    pub is_system: bool,
}

impl DisplayMessage for DisplayMessageAction {
    fn content(&self) -> String {
        self.content.clone()
    }

    fn is_system(&self) -> bool {
        self.is_system
    }
}

/// Convenience functions for creating common message routes
impl MessageRoute {
    /// Create a route for a direct message
    pub fn direct_message(from: String, to: String, content: String) -> Self {
        let message = SystemMessage::direct_message(from, to.clone(), content);
        MessageRoute::new(MessageTarget::User(to), Message::System(message))
    }

    /// Create a route for a DM confirmation
    pub fn dm_confirmation(sender: String, target: String, content: String) -> Self {
        let message = SystemMessage::dm_confirmation(target, content);
        MessageRoute::new(MessageTarget::User(sender), Message::System(message))
    }

    /// Create a route for a user joined notification
    pub fn user_joined(username: String, room: String) -> Self {
        let message = SystemMessage::user_joined(username, room.clone());
        MessageRoute::new(MessageTarget::Room(room), Message::System(message))
    }

    /// Create a route for an error message to a specific user
    pub fn error_to_user(username: String, error: String) -> Self {
        let message = SystemMessage::error(error);
        MessageRoute::new(MessageTarget::User(username), Message::System(message))
    }

    /// Create a route for broadcasting a message to all users
    pub fn broadcast_system(message: SystemMessage) -> Self {
        MessageRoute::new(MessageTarget::Broadcast, Message::System(message))
    }

    /// Create a route for a chat message in a room
    pub fn chat_to_room(from: String, content: String, room: String) -> Self {
        let message = ChatMessage::new(from, content, room.clone());
        MessageRoute::new(MessageTarget::Room(room), Message::Chat(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_client_message_router_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let router = ClientMessageRouter::new(tx);

        assert!(router.current_user().is_none());
        assert!(router.debug_enabled);
    }

    #[test]
    fn test_should_display_for_user() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut router = ClientMessageRouter::new(tx);

        // No current user set
        assert!(!router.should_display_for_user(&MessageTarget::User("alice".to_string())));

        // Set current user
        router.set_current_user(Some("alice".to_string()));

        // Should display for current user
        assert!(router.should_display_for_user(&MessageTarget::User("alice".to_string())));
        assert!(!router.should_display_for_user(&MessageTarget::User("bob".to_string())));
        assert!(router.should_display_for_user(&MessageTarget::Broadcast));
        assert!(router.should_display_for_user(&MessageTarget::Room("lobby".to_string())));
    }

    #[test]
    fn test_message_route_helpers() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let route = MessageRoute::direct_message(
            "alice".to_string(),
            "bob".to_string(),
            "hello".to_string(),
        );

        assert_eq!(route.target, MessageTarget::User("bob".to_string()));
        match route.message {
            Message::System(SystemMessage::DirectMessage { from, to, content }) => {
                assert_eq!(from, "alice");
                assert_eq!(to, "bob");
                assert_eq!(content, "hello");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_invitation_message_parsing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut router = ClientMessageRouter::new(tx);
        router.set_current_user(Some("bob".to_string()));
        router.set_debug_enabled(true);

        // Test invitation message parsing
        let invitation_message = "SYSTEM_MESSAGE:alice invited you to join room 'testroom'";
        let result = router.parse_and_route_protocol_message(invitation_message);

        assert!(
            result.is_ok(),
            "Should parse invitation message successfully"
        );

        // Should receive multiple actions: DisplayMessage actions and InvitationReceived
        let mut actions_received = Vec::new();

        // Collect all actions (should be 4: invitation display, instructions, alternatives, and InvitationReceived)
        for _ in 0..4 {
            if let Ok(action) = rx.try_recv() {
                actions_received.push(action);
            }
        }

        assert!(
            !actions_received.is_empty(),
            "Should receive actions from invitation parsing"
        );

        // Check that we got the InvitationReceived action
        let invitation_action = actions_received
            .iter()
            .find(|action| matches!(action, Action::InvitationReceived(_, _, _)));

        assert!(
            invitation_action.is_some(),
            "Should receive InvitationReceived action"
        );

        if let Some(Action::InvitationReceived(from, room, _)) = invitation_action {
            assert_eq!(from, "alice");
            assert_eq!(room, "testroom");
        }

        // Check that we got DisplayMessage actions
        let display_actions: Vec<_> = actions_received
            .iter()
            .filter(|action| matches!(action, Action::DisplayMessage { .. }))
            .collect();

        assert_eq!(
            display_actions.len(),
            3,
            "Should receive 3 DisplayMessage actions"
        );
    }

    #[tokio::test]
    async fn test_invitation_confirmation_parsing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut router = ClientMessageRouter::new(tx);
        router.set_current_user(Some("alice".to_string()));

        // Test invitation confirmation message parsing
        let confirmation_message = "SYSTEM_MESSAGE:You invited bob to join room 'testroom'";
        let result = router.parse_and_route_protocol_message(confirmation_message);

        assert!(
            result.is_ok(),
            "Should parse confirmation message successfully"
        );

        // Should receive an action
        if let Ok(action) = rx.try_recv() {
            // The action should be routed through the message system
            // This tests that the "You invited" pattern is handled correctly
            println!("Received action: {:?}", action);
        }
    }

    #[tokio::test]
    async fn test_protocol_spam_filtering() {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();
        let mut router = ClientMessageRouter::new(action_tx);
        router.set_current_user(Some("testuser".to_string()));

        // Test the exact messages that should be BLOCKED
        let protocol_spam = vec![
            "fox: true",
            "mberry: true",
            "lusus: true",
            "alice: USER_LIST:fox,bob",
            "mberry: ROOM_LIST:Lobby,",
            "bob: CURRENT_ROOM:general",
            "charlie: ROOM_STATUS:lobby,alice",
            "Reconnected User: CURRENT_ROOM:hope",
            "Reconnected User: USER_LIST:mberry,lusus",
            "Reconnected User: ROOM_STATUS:Lobby,lusus",
            "alice: ACCEPT_INVITATION:testroom",
            "bob: DECLINE_INVITATION:general",
            "USER_LIST:alice,bob,charlie",
            "ROOM_LIST:lobby,general,test",
            "CURRENT_ROOM:general",
            "true",
        ];

        for message in protocol_spam {
            println!("Testing protocol block: '{}'", message);

            // Process the message
            let result = router.parse_and_route_protocol_message(message);
            assert!(result.is_ok(), "Message processing failed: {}", message);

            // Check that NO DisplayMessage actions were generated
            let mut actions_generated = 0;
            while let Ok(_) = action_rx.try_recv() {
                actions_generated += 1;
            }

            assert_eq!(
                actions_generated, 0,
                "FAILED: Protocol message '{}' generated {} display actions when it should be blocked!",
                message, actions_generated
            );
        }
    }

    #[tokio::test]
    async fn test_specific_user_complaints() {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();
        let mut router = ClientMessageRouter::new(action_tx);
        router.set_current_user(Some("testuser".to_string()));

        // The EXACT messages the user complained about
        let complaint_messages = vec![
            "Reconnected User: CURRENT_ROOM:hope",
            "Reconnected User: USER_LIST:mberry,lusus",
            "Reconnected User: ROOM_STATUS:Lobby,lusus",
        ];

        for message in complaint_messages {
            println!("Testing user complaint: '{}'", message);

            let result = router.parse_and_route_protocol_message(message);
            assert!(result.is_ok(), "Message processing failed: {}", message);

            // Ensure NO display actions are generated
            let mut actions_generated = 0;
            while let Ok(_) = action_rx.try_recv() {
                actions_generated += 1;
            }

            assert_eq!(
                actions_generated, 0,
                "CRITICAL FAILURE: User complaint message '{}' is STILL getting through! Generated {} actions",
                message, actions_generated
            );
        }
    }

    #[test]
    fn test_protocol_detection_functions() {
        let (action_tx, _) = mpsc::unbounded_channel();
        let router = ClientMessageRouter::new(action_tx);

        // Test pure protocol message detection
        assert!(router.is_pure_protocol_message("USER_LIST:alice,bob"));
        assert!(router.is_pure_protocol_message("CURRENT_ROOM:general"));
        assert!(router.is_pure_protocol_message("true"));
        assert!(router.is_pure_protocol_message("Reconnected User: CURRENT_ROOM:test"));
        assert!(!router.is_pure_protocol_message("alice: Hello world"));
        assert!(!router.is_pure_protocol_message("Reconnected User: bob joined the room"));

        // Test protocol content detection
        assert!(router.is_protocol_content("USER_LIST:alice,bob"));
        assert!(router.is_protocol_content("CURRENT_ROOM:general"));
        assert!(router.is_protocol_content("true"));
        assert!(!router.is_protocol_content("Hello world"));
        assert!(!router.is_protocol_content("bob joined the room"));
    }
}
