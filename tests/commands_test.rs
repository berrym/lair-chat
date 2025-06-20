//! Tests for command processing functionality

use lair_chat::action::Action;
use lair_chat::commands::{CommandProcessor, CommandResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_detection() {
        assert!(CommandProcessor::is_command("/help"));
        assert!(CommandProcessor::is_command("  /dm user message"));
        assert!(CommandProcessor::is_command("/quit"));
        assert!(!CommandProcessor::is_command("regular message"));
        assert!(!CommandProcessor::is_command("not a command"));
        assert!(!CommandProcessor::is_command(""));
    }

    #[test]
    fn test_help_command() {
        let processor = CommandProcessor::new();

        // Test general help
        match processor.process_command("/help") {
            CommandResult::Messages(messages) => {
                assert!(!messages.is_empty());
                assert!(messages[0].contains("Available commands"));
            }
            _ => panic!("Expected Messages result for /help"),
        }

        // Test help for specific command
        match processor.process_command("/help dm") {
            CommandResult::Messages(messages) => {
                assert!(!messages.is_empty());
                assert!(messages.iter().any(|m| m.contains("dm")));
            }
            _ => panic!("Expected Messages result for /help dm"),
        }

        // Test help for unknown command
        match processor.process_command("/help nonexistent") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Unknown command"));
            }
            _ => panic!("Expected Error result for unknown command help"),
        }
    }

    #[test]
    fn test_dm_command() {
        let processor = CommandProcessor::new();

        // Test valid DM command
        match processor.process_command("/dm alice hello world") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "DM:alice:hello world");
            }
            _ => panic!("Expected Action result for valid DM command"),
        }

        // Test DM with insufficient arguments
        match processor.process_command("/dm alice") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /dm"));
            }
            _ => panic!("Expected Error result for insufficient DM args"),
        }

        // Test DM with no arguments
        match processor.process_command("/dm") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /dm"));
            }
            _ => panic!("Expected Error result for no DM args"),
        }

        // Test DM with empty message
        match processor.process_command("/dm alice ") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Message cannot be empty"));
            }
            _ => panic!("Expected Error result for empty DM message"),
        }
    }

    #[test]
    fn test_command_aliases() {
        let processor = CommandProcessor::new();

        // Test help aliases
        for alias in &["h", "?"] {
            match processor.process_command(&format!("/{}", alias)) {
                CommandResult::Messages(_) => {} // Should work
                _ => panic!("Alias /{} should work for help", alias),
            }
        }

        // Test DM aliases
        for alias in &["msg", "whisper", "w"] {
            match processor.process_command(&format!("/{} alice test", alias)) {
                CommandResult::Action(Action::SendMessage(msg)) => {
                    assert_eq!(msg, "DM:alice:test");
                }
                _ => panic!("Alias /{} should work for DM", alias),
            }
        }

        // Test quit aliases
        for alias in &["exit", "q"] {
            match processor.process_command(&format!("/{}", alias)) {
                CommandResult::Action(Action::Quit) => {} // Should work
                _ => panic!("Alias /{} should work for quit", alias),
            }
        }
    }

    #[test]
    fn test_clear_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/clear") {
            CommandResult::Action(Action::ClearScreen) => {} // Expected
            _ => panic!("Expected ClearScreen action for /clear"),
        }

        // Test clear alias
        match processor.process_command("/cls") {
            CommandResult::Action(Action::ClearScreen) => {} // Expected
            _ => panic!("Expected ClearScreen action for /cls"),
        }
    }

    #[test]
    fn test_quit_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/quit") {
            CommandResult::Action(Action::Quit) => {} // Expected
            _ => panic!("Expected Quit action for /quit"),
        }
    }

    #[test]
    fn test_me_command() {
        let processor = CommandProcessor::new();

        // Test valid action message
        match processor.process_command("/me waves at everyone") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACTION:waves at everyone");
            }
            _ => panic!("Expected Action result for /me command"),
        }

        // Test empty action
        match processor.process_command("/me") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /me"));
            }
            _ => panic!("Expected Error result for empty /me command"),
        }

        // Test action alias
        match processor.process_command("/action does something") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "ACTION:does something");
            }
            _ => panic!("Expected Action result for /action command"),
        }
    }

    #[test]
    fn test_join_command() {
        let processor = CommandProcessor::new();

        // Test join command (placeholder implementation)
        match processor.process_command("/join testroom") {
            CommandResult::Message(msg) => {
                assert!(msg.contains("testroom"));
                assert!(msg.contains("not yet implemented"));
            }
            _ => panic!("Expected Message result for /join command"),
        }

        // Test join without room name
        match processor.process_command("/join") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage: /join"));
            }
            _ => panic!("Expected Error result for /join without args"),
        }
    }

    #[test]
    fn test_leave_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/leave") {
            CommandResult::Action(Action::ReturnToLobby) => {} // Expected
            _ => panic!("Expected ReturnToLobby action for /leave"),
        }

        // Test leave alias
        match processor.process_command("/part") {
            CommandResult::Action(Action::ReturnToLobby) => {} // Expected
            _ => panic!("Expected ReturnToLobby action for /part"),
        }
    }

    #[test]
    fn test_status_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/status") {
            CommandResult::Message(msg) => {
                assert!(msg.contains("Status information"));
            }
            _ => panic!("Expected Message result for /status command"),
        }

        // Test status alias
        match processor.process_command("/stat") {
            CommandResult::Message(msg) => {
                assert!(msg.contains("Status information"));
            }
            _ => panic!("Expected Message result for /stat command"),
        }
    }

    #[test]
    fn test_users_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/users") {
            CommandResult::Message(msg) => {
                assert!(msg.contains("Connected users"));
            }
            _ => panic!("Expected Message result for /users command"),
        }

        // Test users aliases
        for alias in &["who", "list"] {
            match processor.process_command(&format!("/{}", alias)) {
                CommandResult::Message(_) => {} // Should work
                _ => panic!("Alias /{} should work for users", alias),
            }
        }
    }

    #[test]
    fn test_unknown_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/nonexistent") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Unknown command"));
                assert!(msg.contains("nonexistent"));
            }
            _ => panic!("Expected Error result for unknown command"),
        }
    }

    #[test]
    fn test_empty_command() {
        let processor = CommandProcessor::new();

        match processor.process_command("/") {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Empty command"));
            }
            _ => panic!("Expected Error result for empty command"),
        }
    }

    #[test]
    fn test_command_suggestions() {
        let processor = CommandProcessor::new();

        // Test partial matching
        let suggestions = processor.get_command_suggestions("he");
        assert!(suggestions.contains(&"/help".to_string()));

        let suggestions = processor.get_command_suggestions("d");
        assert!(suggestions.contains(&"/dm".to_string()));

        let suggestions = processor.get_command_suggestions("q");
        assert!(suggestions.contains(&"/quit".to_string()));

        // Test no matches
        let suggestions = processor.get_command_suggestions("xyz");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_case_insensitive_commands() {
        let processor = CommandProcessor::new();

        // Test mixed case commands
        match processor.process_command("/HELP") {
            CommandResult::Messages(_) => {} // Should work
            _ => panic!("Uppercase /HELP should work"),
        }

        match processor.process_command("/DM alice test") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "DM:alice:test");
            }
            _ => panic!("Uppercase /DM should work"),
        }

        match processor.process_command("/QuIt") {
            CommandResult::Action(Action::Quit) => {} // Should work
            _ => panic!("Mixed case /QuIt should work"),
        }
    }

    #[test]
    fn test_whitespace_handling() {
        let processor = CommandProcessor::new();

        // Test commands with extra whitespace
        match processor.process_command("  /help  ") {
            CommandResult::Messages(_) => {} // Should work
            _ => panic!("Command with extra whitespace should work"),
        }

        match processor.process_command("/dm    alice    hello   world") {
            CommandResult::Action(Action::SendMessage(msg)) => {
                assert_eq!(msg, "DM:alice:hello world");
            }
            _ => panic!("DM with extra whitespace should work"),
        }
    }

    #[test]
    fn test_non_command_messages() {
        let processor = CommandProcessor::new();

        // Test regular messages that aren't commands
        match processor.process_command("hello world") {
            CommandResult::NotFound => {} // Expected
            _ => panic!("Regular message should return NotFound"),
        }

        match processor.process_command("this is not a command") {
            CommandResult::NotFound => {} // Expected
            _ => panic!("Regular message should return NotFound"),
        }

        // Test message that starts with / but in the middle of text
        match processor.process_command("check out /help for commands") {
            CommandResult::NotFound => {} // Expected - doesn't start with /
            _ => panic!("Message not starting with / should return NotFound"),
        }
    }
}
