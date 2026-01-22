use crate::action::Action;
use std::collections::HashMap;
use tracing::{debug, info};

/// Represents a chat command that can be executed
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub aliases: Vec<String>,
}

/// Result of processing a command
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command was processed successfully, return an action
    Action(Action),
    /// Command was processed successfully, return multiple actions
    Actions(Vec<Action>),
    /// Command was processed successfully, show a message to user
    Message(String),
    /// Command was processed successfully, show multiple messages
    Messages(Vec<String>),
    /// Command was processed but no action needed
    Success,
    /// Command failed with an error message
    Error(String),
    /// Command was not found
    NotFound,
}

/// Command processor for handling chat commands
pub struct CommandProcessor {
    commands: HashMap<String, Command>,
}

impl CommandProcessor {
    /// Create a new command processor with default commands
    pub fn new() -> Self {
        let mut processor = Self {
            commands: HashMap::new(),
        };

        processor.register_default_commands();
        processor
    }

    /// Register all default commands
    fn register_default_commands(&mut self) {
        // Help command
        self.register_command(Command {
            name: "help".to_string(),
            description: "Show available commands".to_string(),
            usage: "/help [command]".to_string(),
            aliases: vec!["h".to_string(), "?".to_string()],
        });

        // Direct message command
        self.register_command(Command {
            name: "dm".to_string(),
            description: "Send a direct message to a user".to_string(),
            usage: "/dm <username> <message>".to_string(),
            aliases: vec!["msg".to_string(), "whisper".to_string(), "w".to_string()],
        });

        // Join command (for future room functionality)
        self.register_command(Command {
            name: "join".to_string(),
            description: "Join a chat room".to_string(),
            usage: "/join <room_name>".to_string(),
            aliases: vec!["j".to_string()],
        });

        // Leave command
        self.register_command(Command {
            name: "leave".to_string(),
            description: "Leave current chat room".to_string(),
            usage: "/leave".to_string(),
            aliases: vec!["part".to_string()],
        });

        // List users command
        self.register_command(Command {
            name: "users".to_string(),
            description: "List all connected users".to_string(),
            usage: "/users".to_string(),
            aliases: vec!["who".to_string(), "list".to_string()],
        });

        // Clear screen command
        self.register_command(Command {
            name: "clear".to_string(),
            description: "Clear the chat screen".to_string(),
            usage: "/clear".to_string(),
            aliases: vec!["cls".to_string()],
        });

        // Quit command
        self.register_command(Command {
            name: "quit".to_string(),
            description: "Quit the application".to_string(),
            usage: "/quit".to_string(),
            aliases: vec!["exit".to_string(), "q".to_string()],
        });

        // Status command
        self.register_command(Command {
            name: "status".to_string(),
            description: "Show connection status".to_string(),
            usage: "/status".to_string(),
            aliases: vec!["stat".to_string()],
        });

        // Me command (for actions)
        self.register_command(Command {
            name: "me".to_string(),
            description: "Send an action message".to_string(),
            usage: "/me <action>".to_string(),
            aliases: vec!["action".to_string()],
        });

        // Create room command
        self.register_command(Command {
            name: "create-room".to_string(),
            description: "Create a new chat room".to_string(),
            usage: "/create-room <room_name>".to_string(),
            aliases: vec!["create".to_string(), "cr".to_string()],
        });

        // List rooms command
        self.register_command(Command {
            name: "rooms".to_string(),
            description: "List all available rooms".to_string(),
            usage: "/rooms".to_string(),
            aliases: vec!["list-rooms".to_string(), "lr".to_string()],
        });

        // Invite user to room command
        self.register_command(Command {
            name: "invite".to_string(),
            description: "Invite a user to join a room".to_string(),
            usage: "/invite <username> <room_name>".to_string(),
            aliases: vec!["inv".to_string()],
        });

        // Accept invitation command
        self.register_command(Command {
            name: "accept".to_string(),
            description: "Accept a room invitation".to_string(),
            usage: "/accept [room_name]".to_string(),
            aliases: vec!["acc".to_string()],
        });

        // Decline invitation command
        self.register_command(Command {
            name: "decline".to_string(),
            description: "Decline a room invitation".to_string(),
            usage: "/decline [room_name]".to_string(),
            aliases: vec!["dec".to_string()],
        });

        // List invitations command
        self.register_command(Command {
            name: "invites".to_string(),
            description: "List all pending room invitations".to_string(),
            usage: "/invites".to_string(),
            aliases: vec!["invitations".to_string(), "pending".to_string()],
        });

        // Accept all invitations command
        self.register_command(Command {
            name: "accept-all".to_string(),
            description: "Accept all pending room invitations".to_string(),
            usage: "/accept-all".to_string(),
            aliases: vec!["acc-all".to_string(), "acceptall".to_string()],
        });
    }

    /// Register a new command
    pub fn register_command(&mut self, command: Command) {
        // Register the main command name
        self.commands.insert(command.name.clone(), command.clone());

        // Register all aliases
        for alias in &command.aliases {
            self.commands.insert(alias.clone(), command.clone());
        }
    }

    /// Check if a message is a command (starts with /)
    pub fn is_command(message: &str) -> bool {
        message.trim_start().starts_with('/')
    }

    /// Process a command and return the result
    pub fn process_command(&self, message: &str) -> CommandResult {
        let trimmed = message.trim();

        if !trimmed.starts_with('/') {
            return CommandResult::NotFound;
        }

        // Remove the leading '/' and split into parts
        let command_text = &trimmed[1..];
        let parts: Vec<&str> = command_text.split_whitespace().collect();

        if parts.is_empty() {
            return CommandResult::Error("Empty command".to_string());
        }

        let command_name = parts[0].to_lowercase();
        let args = &parts[1..];

        debug!("Processing command: {} with args: {:?}", command_name, args);

        // Check if command exists
        if !self.commands.contains_key(&command_name) {
            return CommandResult::Error(format!("Unknown command: /{}", command_name));
        }

        // Process the command
        match command_name.as_str() {
            "help" | "h" | "?" => self.handle_help_command(args),
            "dm" | "msg" | "whisper" | "w" => self.handle_dm_command(args),
            "join" | "j" => self.handle_join_command(args),
            "leave" | "part" => self.handle_leave_command(args),
            "users" | "who" | "list" => self.handle_users_command(args),
            "clear" | "cls" => self.handle_clear_command(args),
            "quit" | "exit" | "q" => self.handle_quit_command(args),
            "status" | "stat" => self.handle_status_command(args),
            "me" | "action" => self.handle_me_command(args),
            "create-room" | "create" | "cr" => self.handle_create_room_command(args),
            "rooms" | "list-rooms" | "lr" => self.handle_rooms_command(args),
            "invite" | "inv" => self.handle_invite_command(args),
            "accept" | "acc" => self.handle_accept_command(args),
            "decline" | "dec" => self.handle_decline_command(args),
            "invites" | "invitations" | "pending" => self.handle_invites_command(args),
            "accept-all" | "acc-all" | "acceptall" => self.handle_accept_all_command(args),
            _ => CommandResult::Error(format!("Command not implemented: /{}", command_name)),
        }
    }

    /// Handle help command
    fn handle_help_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            // Show all commands
            let mut help_text = vec!["Available commands:".to_string(), "".to_string()];

            // Collect unique commands (avoiding duplicates from aliases)
            let mut unique_commands: Vec<&Command> = self
                .commands
                .values()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            // Sort by name for consistent output
            unique_commands.sort_by(|a, b| a.name.cmp(&b.name));

            for command in unique_commands {
                help_text.push(format!("  {} - {}", command.usage, command.description));
                if !command.aliases.is_empty() {
                    help_text.push(format!("    Aliases: /{}", command.aliases.join(", /")));
                }
                help_text.push("".to_string());
            }

            help_text.push(
                "Tip: Use /help <command> for detailed help on a specific command".to_string(),
            );

            CommandResult::Messages(help_text)
        } else {
            // Show help for specific command
            let command_name = args[0].to_lowercase();
            if let Some(command) = self.commands.get(&command_name) {
                let mut help_text = vec![
                    format!("Command: /{}", command.name),
                    format!("Usage: {}", command.usage),
                    format!("Description: {}", command.description),
                ];

                if !command.aliases.is_empty() {
                    help_text.push(format!("Aliases: /{}", command.aliases.join(", /")));
                }

                CommandResult::Messages(help_text)
            } else {
                CommandResult::Error(format!("Unknown command: /{}", command_name))
            }
        }
    }

    /// Handle direct message command
    fn handle_dm_command(&self, args: &[&str]) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::Error("Usage: /dm <username> <message>".to_string());
        }

        let username = args[0];
        let message = args[1..].join(" ");

        if message.trim().is_empty() {
            return CommandResult::Error("Message cannot be empty".to_string());
        }

        info!(
            "Processing DM command: to={}, message={}",
            username, message
        );

        // Format the message for the existing DM system
        let dm_message = format!("DM:{}:{}", username, message);

        // Return multiple actions: start DM conversation AND send message
        CommandResult::Actions(vec![
            Action::StartDMConversation(username.to_string()),
            Action::SendMessage(dm_message),
        ])
    }

    /// Handle join command
    fn handle_join_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error("Usage: /join <room_name>".to_string());
        }

        let room_name = args[0].to_string();

        // Validate room name
        if room_name.is_empty() || room_name.contains(',') || room_name.len() > 50 {
            return CommandResult::Error("Invalid room name. Room names cannot be empty, contain commas, or exceed 50 characters.".to_string());
        }

        info!("Processing join command: room={}", room_name);

        CommandResult::Action(Action::JoinRoom(room_name))
    }

    /// Handle leave command
    fn handle_leave_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing leave command");

        // This will leave the current room and return to Lobby
        CommandResult::Action(Action::LeaveRoom)
    }

    /// Handle users command
    fn handle_users_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing users command");

        // Send request for user list
        CommandResult::Action(Action::SendMessage("REQUEST_USER_LIST".to_string()))
    }

    /// Handle clear command
    fn handle_clear_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing clear command");

        CommandResult::Action(Action::ClearScreen)
    }

    /// Handle quit command
    fn handle_quit_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing quit command");

        CommandResult::Action(Action::Quit)
    }

    /// Handle status command
    fn handle_status_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing status command");

        // This would show connection status, current room, etc.
        CommandResult::Message("Status information would be displayed here".to_string())
    }

    /// Handle me command (action messages)
    fn handle_me_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error("Usage: /me <action>".to_string());
        }

        let action_text = args.join(" ");
        info!("Processing me command: action={}", action_text);

        // Format as an action message (typically shown as "* username does something")
        let action_message = format!("ACTION:{}", action_text);

        CommandResult::Action(Action::SendMessage(action_message))
    }

    /// Get list of all available commands
    pub fn get_commands(&self) -> Vec<&Command> {
        // Return unique commands (no duplicates from aliases)
        self.commands
            .values()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Handle create room command
    fn handle_create_room_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error("Usage: /create-room <room_name>".to_string());
        }

        let room_name = args[0].to_string();

        // Validate room name
        if room_name.is_empty() {
            return CommandResult::Error("Room name cannot be empty".to_string());
        }

        if room_name.contains(',') {
            return CommandResult::Error("Room names cannot contain commas".to_string());
        }

        if room_name.len() > 50 {
            return CommandResult::Error("Room name cannot exceed 50 characters".to_string());
        }

        if room_name.to_lowercase() == "lobby" {
            return CommandResult::Error(
                "Cannot create a room named 'Lobby' (reserved)".to_string(),
            );
        }

        // Check for reserved characters and patterns
        if room_name.contains([':', ';', '|', '\n', '\r', '\t']) {
            return CommandResult::Error(
                "Room names cannot contain special characters (:;|\\n\\r\\t)".to_string(),
            );
        }

        info!("Processing create-room command: room={}", room_name);

        CommandResult::Action(Action::CreateRoom(room_name))
    }

    /// Handle rooms command (list available rooms)
    fn handle_rooms_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing rooms command");

        CommandResult::Action(Action::ListRooms)
    }

    /// Handle invite command
    fn handle_invite_command(&self, args: &[&str]) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::Error("Usage: /invite <username> <room_name>".to_string());
        }

        let username = args[0].to_string();
        let room_name = args[1].to_string();

        // Validate username
        if username.is_empty() {
            return CommandResult::Error("Username cannot be empty".to_string());
        }

        if username.len() > 50 {
            return CommandResult::Error("Username cannot exceed 50 characters".to_string());
        }

        // Validate room name
        if room_name.is_empty() {
            return CommandResult::Error("Room name cannot be empty".to_string());
        }

        if room_name.contains(',') {
            return CommandResult::Error("Room names cannot contain commas".to_string());
        }

        if room_name.len() > 50 {
            return CommandResult::Error("Room name cannot exceed 50 characters".to_string());
        }

        // Check for reserved characters and patterns
        if room_name.contains([':', ';', '|', '\n', '\r', '\t']) {
            return CommandResult::Error(
                "Room names cannot contain special characters (:;|\\n\\r\\t)".to_string(),
            );
        }

        info!(
            "Processing invite command: user={}, room={}",
            username, room_name
        );

        // Create invite message format for server
        let invite_message = format!("INVITE_USER:{}:{}", username, room_name);

        CommandResult::Action(Action::SendMessage(invite_message))
    }

    /// Handle accept command
    fn handle_accept_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            // Accept the most recent invitation
            let accept_message = "ACCEPT_INVITATION:LATEST".to_string();
            return CommandResult::Action(Action::SendMessage(accept_message));
        }

        let room_name = args[0].to_string();

        // Validate room name
        if room_name.is_empty() {
            return CommandResult::Error("Room name cannot be empty".to_string());
        }

        if room_name.contains(',') {
            return CommandResult::Error("Room names cannot contain commas".to_string());
        }

        if room_name.len() > 50 {
            return CommandResult::Error("Room name cannot exceed 50 characters".to_string());
        }

        // Check for reserved characters and patterns
        if room_name.contains([':', ';', '|', '\n', '\r', '\t']) {
            return CommandResult::Error(
                "Room names cannot contain special characters (:;|\\n\\r\\t)".to_string(),
            );
        }

        info!("Processing accept command for room: {}", room_name);

        // Create accept message format for server
        let accept_message = format!("ACCEPT_INVITATION:{}", room_name);

        CommandResult::Action(Action::SendMessage(accept_message))
    }

    /// Handle decline command
    fn handle_decline_command(&self, args: &[&str]) -> CommandResult {
        if args.is_empty() {
            // Decline the most recent invitation
            let decline_message = "DECLINE_INVITATION:LATEST".to_string();
            return CommandResult::Action(Action::SendMessage(decline_message));
        }

        let room_name = args[0].to_string();

        // Validate room name
        if room_name.is_empty() {
            return CommandResult::Error("Room name cannot be empty".to_string());
        }

        if room_name.contains(',') {
            return CommandResult::Error("Room names cannot contain commas".to_string());
        }

        if room_name.len() > 50 {
            return CommandResult::Error("Room name cannot exceed 50 characters".to_string());
        }

        // Check for reserved characters and patterns
        if room_name.contains([':', ';', '|', '\n', '\r', '\t']) {
            return CommandResult::Error(
                "Room names cannot contain special characters (:;|\\n\\r\\t)".to_string(),
            );
        }

        info!("Processing decline command for room: {}", room_name);

        // Create decline message format for server
        let decline_message = format!("DECLINE_INVITATION:{}", room_name);

        CommandResult::Action(Action::SendMessage(decline_message))
    }

    /// Handle invites command (list pending invitations)
    fn handle_invites_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing invites command");

        // Create request to list pending invitations
        let invites_message = "LIST_INVITATIONS".to_string();

        CommandResult::Action(Action::SendMessage(invites_message))
    }

    /// Handle accept-all command
    fn handle_accept_all_command(&self, _args: &[&str]) -> CommandResult {
        info!("Processing accept-all command");

        // Create request to accept all pending invitations
        let accept_all_message = "ACCEPT_ALL_INVITATIONS".to_string();

        CommandResult::Action(Action::SendMessage(accept_all_message))
    }

    /// Get command suggestions for autocomplete
    pub fn get_command_suggestions(&self, partial: &str) -> Vec<String> {
        let partial_lower = partial.to_lowercase();
        let mut suggestions: Vec<String> = self
            .commands
            .keys()
            .filter(|name| name.starts_with(&partial_lower))
            .map(|name| format!("/{}", name))
            .collect();

        suggestions.sort();
        suggestions.dedup(); // Remove duplicates
        suggestions
    }
}

impl Default for CommandProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_command() {
        assert!(CommandProcessor::is_command("/help"));
        assert!(CommandProcessor::is_command("  /dm user message"));
        assert!(!CommandProcessor::is_command("regular message"));
        assert!(!CommandProcessor::is_command(""));
    }

    #[test]
    fn test_help_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/help") {
            CommandResult::Messages(messages) => {
                assert!(!messages.is_empty());
                assert!(messages[0].contains("Available commands"));
            }
            _ => panic!("Expected Messages result"),
        }
    }

    #[test]
    fn test_dm_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/dm alice hello world") {
            CommandResult::Actions(actions) => {
                assert_eq!(actions.len(), 2);
                assert_eq!(actions[0], Action::StartDMConversation("alice".to_string()));
                assert_eq!(
                    actions[1],
                    Action::SendMessage("DM:alice:hello world".to_string())
                );
            }
            _ => panic!("Expected Actions result"),
        }
    }

    #[test]
    fn test_dm_command_insufficient_args() {
        let processor = CommandProcessor::new();

        match processor.process_command("/dm alice") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /dm"));
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn test_unknown_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/unknown") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Unknown command"));
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn test_command_aliases() {
        let processor = CommandProcessor::new();

        // Test that aliases work
        match processor.process_command("/h") {
            CommandResult::Messages(_) => {} // Help command should work
            _ => panic!("Alias /h should work for help"),
        }

        match processor.process_command("/q") {
            CommandResult::Action(Action::Quit) => {}
            _ => panic!("Alias /q should work for quit"),
        }
    }

    #[test]
    fn test_command_suggestions() {
        let processor = CommandProcessor::new();

        let suggestions = processor.get_command_suggestions("he");
        assert!(suggestions.contains(&"/help".to_string()));

        let suggestions = processor.get_command_suggestions("d");
        assert!(suggestions.contains(&"/dm".to_string()));
    }

    #[test]
    fn test_me_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/me waves at everyone") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACTION:waves at everyone");
            }
            _ => panic!("Expected Action result"),
        }
    }

    #[test]
    fn test_create_room_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/create-room general") {
            CommandResult::Action(Action::CreateRoom(room_name)) => {
                assert_eq!(room_name, "general");
            }
            _ => panic!("Expected CreateRoom action"),
        }
    }

    #[test]
    fn test_create_room_validation() {
        let processor = CommandProcessor::new();

        // Empty room name
        match processor.process_command("/create-room") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /create-room"));
            }
            _ => panic!("Expected Error result for empty room name"),
        }

        // Reserved room name
        match processor.process_command("/create-room Lobby") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("reserved"));
            }
            _ => panic!("Expected Error result for reserved room name"),
        }

        // Invalid characters
        match processor.process_command("/create-room test:room") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("special characters"));
            }
            _ => panic!("Expected Error result for invalid characters"),
        }
    }

    #[test]
    fn test_join_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/join general") {
            CommandResult::Action(Action::JoinRoom(room_name)) => {
                assert_eq!(room_name, "general");
            }
            _ => panic!("Expected JoinRoom action"),
        }
    }

    #[test]
    fn test_rooms_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/rooms") {
            CommandResult::Action(Action::ListRooms) => {}
            _ => panic!("Expected ListRooms action"),
        }
    }

    #[test]
    fn test_invite_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/invite alice general") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "INVITE_USER:alice:general");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_invite_command_insufficient_args() {
        let processor = CommandProcessor::new();

        match processor.process_command("/invite alice") {
            CommandResult::Error(err) => {
                assert!(err.contains("Usage: /invite <username> <room_name>"));
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn test_accept_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/accept general") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACCEPT_INVITATION:general");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_accept_command_no_args() {
        let processor = CommandProcessor::new();

        match processor.process_command("/accept") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACCEPT_INVITATION:LATEST");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_decline_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/decline general") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "DECLINE_INVITATION:general");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_decline_command_no_args() {
        let processor = CommandProcessor::new();

        match processor.process_command("/decline") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "DECLINE_INVITATION:LATEST");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_invites_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/invites") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "LIST_INVITATIONS");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }

    #[test]
    fn test_accept_all_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/accept-all") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACCEPT_ALL_INVITATIONS");
            }
            _ => panic!("Expected SendMessage action"),
        }
    }
}
