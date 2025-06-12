# Message Styling Test Guide

This document describes the new visual message styling implemented in Lair Chat v0.5.2.

## New Message Styling Features

### Visual Distinctions

1. **Sent Messages (Your messages)**
   - Color: Bright cyan text (`RGB(100, 255, 255)`)
   - Background: Blue (`RGB(0, 50, 100)`)
   - Style: Bold text
   - Format: Right-aligned with decorative borders
   - Example: `                     ┌─ You: Hello everyone! ─┐`

2. **Received Messages (From other users)**
   - Color: Light gray text (`RGB(220, 220, 220)`)
   - Background: Dark gray (`RGB(40, 40, 50)`)
   - Style: Normal weight
   - Format: Left-aligned with decorative borders
   - Example: `└─ Alice: Hi there! How are you? ─┘`

3. **System Messages**
   - Color: Golden yellow (`RGB(255, 215, 0)`)
   - Background: Transparent (inherits from main area)
   - Style: Italic text
   - Format: Centered with special decoration
   - Example: `          ═══ User Alice joined the chat ═══`

## Testing Instructions

### Method 1: Run the Application
1. Build the application: `cargo build --release`
2. Start the client: `./target/release/lair-chat`
3. Connect to a server
4. Send some messages and observe the styling differences

### Method 2: View in Different Terminal Environments
Test the styling in various terminal environments to ensure compatibility:
- Standard terminal (gnome-terminal, Terminal.app, etc.)
- Terminal with different color schemes
- Terminal with limited color support

### Expected Behavior

When you send a message:
- Your message should appear with cyan text on blue background
- It should be right-aligned with the `┌─ ─┐` border style
- Text should appear bold

When you receive a message:
- Other users' messages should appear with light gray text on dark gray background
- They should be left-aligned with the `└─ ─┘` border style
- Text should appear in normal weight

System messages:
- Should appear in golden yellow
- Should be centered with `═══ ═══` borders
- Should appear in italic style

## Implementation Details

The styling is implemented in `src/client/components/home.rs` using:
- `MessageStyle` enum to categorize message types
- `get_display_messages_with_style()` method to provide styling information
- Ratatui's `Style` system with RGB color support
- Unicode box-drawing characters for decorative borders
- Dynamic width calculation for proper right-alignment of sent messages
- Centered alignment for system messages based on available terminal width

## Backwards Compatibility

The implementation maintains backwards compatibility:
- The original `get_display_messages()` method still works
- Existing message handling logic is preserved
- No breaking changes to the API

## Future Enhancements

Potential improvements for future versions:
- User-configurable color schemes
- Emoji reactions display
- Message timestamps
- Thread indicators
- File attachment indicators
- User avatar placeholders