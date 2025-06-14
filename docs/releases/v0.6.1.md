# Lair Chat v0.6.1 Release Notes

**Release Date**: December 2024  
**Version**: 0.6.1  
**Codename**: "Direct Connect"

## 🎉 Major New Feature: Complete Direct Messaging System

Lair Chat v0.6.1 introduces a comprehensive Direct Messaging (DM) system, enabling private encrypted conversations between users alongside the existing public chat functionality.

## 🚀 New Features

### 💬 Direct Messaging
- **Full DM Conversations**: Send and receive private messages with any connected user
- **Encrypted Communication**: All DM messages use the same encryption as public chat
- **Conversation Management**: Maintain separate message histories for each DM conversation
- **Real-time Delivery**: Instant message delivery with server-side routing

### 🎨 Visual Enhancements
- **Distinctive Styling**: Purple bubbles for sent DMs, green bubbles for received DMs
- **Bubble Appearance**: Proper bubble styling with right-alignment for sent messages
- **Mode Headers**: Clear visual indicators showing "LOBBY CHAT" vs "DIRECT MESSAGE with [user]"
- **Context-Aware UI**: Different welcome messages and help text for each mode

### 🔔 Smart Notifications
- **Status Bar Alerts**: "💬 New DM from [username]" notifications appear for 8 seconds
- **Unread Indicators**: Bell icons (🔔) and unread counts in user lists
- **Visual Cues**: Prominent yellow/bold styling for unread message counts
- **Non-Intrusive**: Notifications only appear when not viewing the specific conversation

### ⚡ Easy Navigation
- **Tab-Based Switching**: Press Tab to open chat sidebar showing all conversations
- **Sidebar Navigation**: 
  - Shows "Lobby" + all active DM conversations
  - Current chat highlighted in yellow/bold
  - Unread counts displayed with bell icons
  - Navigate with Up/Down arrows, Enter to switch
- **Quick Returns**: Escape key returns to Lobby from any DM
- **Keyboard Shortcuts**: Intuitive key bindings for all navigation

### 🎯 Improved User Experience
- **User List Integration**: Ctrl+L → N opens user list for starting new DMs
- **Clear Instructions**: Updated help text explains all navigation methods
- **Mode Awareness**: Always know whether you're in Lobby or DM mode
- **Seamless Switching**: Smooth transitions between different conversation types

## 🔧 Technical Improvements

### Architecture
- **DM Conversation Manager**: New `DMConversationManager` handles all DM state
- **Message Routing**: Server-side DM routing with `DM:target:message` protocol
- **Unread Tracking**: Per-conversation unread message counting
- **Memory Efficient**: In-memory conversation storage during session

### UI Components
- **Chat Sidebar**: New collapsible sidebar for conversation navigation
- **Enhanced Status Bar**: DM notifications with timeout management
- **Message Styling**: Extended styling system for DM vs regular messages
- **Responsive Layout**: Dynamic layout adjusts for sidebar visibility

### Protocol Extensions
- **DM Protocol**: `DM:username:content` format for direct messages
- **DM_FROM Messages**: `DM_FROM:sender:content` format for received DMs
- **User List Updates**: Enhanced user list with unread status integration

## 🎮 Controls Summary

### New Keyboard Shortcuts
- **Tab**: Toggle chat sidebar (main navigation method)
- **Up/Down**: Navigate sidebar when open
- **Enter**: Switch to selected chat from sidebar
- **Escape**: Return to Lobby from DM mode or close sidebar

### Existing Controls (Enhanced)
- **Ctrl+L → N**: Open user list for new DM conversations
- **/** (slash): Enter message input mode
- **?**: Toggle help display
- **q**: Quit application

## 🎨 Visual Design

### Color Scheme
- **Regular Chat**: Blue (sent) and light gray (received) bubbles
- **Direct Messages**: Purple (sent) and green (received) bubbles
- **Status Indicators**: Yellow/bold for notifications and unread counts
- **Mode Headers**: Green for Lobby, Magenta for DM mode

### Typography
- **Headers**: Bold, colored section headers for clear mode identification
- **Notifications**: Yellow/bold styling for important alerts
- **Unread Counts**: Prominent bell icons with count display
- **Help Text**: Context-aware instructions for current mode

## 🔒 Security

- **Encryption**: All DM messages use the same X25519 + AES-256-GCM encryption as public chat
- **Server Routing**: Secure server-side message routing without content inspection
- **Session Persistence**: DM conversations maintained during client session
- **Privacy**: No message content stored on server beyond active session

## 💡 User Experience Highlights

### Intuitive Workflow
1. **Start DM**: Press Ctrl+L → N, select user, press Enter
2. **Send Message**: Type message and press Enter (purple bubble appears)
3. **Receive Response**: Green bubble appears for received messages
4. **Switch Chats**: Press Tab to see all conversations and switch
5. **Return to Lobby**: Press Escape or select "Lobby" from sidebar

### Smart Features
- **Automatic Notifications**: Never miss a DM with status bar alerts
- **Unread Tracking**: See at a glance which conversations have new messages
- **Mode Awareness**: Clear headers always show your current context
- **Seamless Navigation**: Switch between any conversation with just a few keystrokes

## 🐛 Bug Fixes

- Fixed stack overflow when sending DM messages
- Resolved user selection navigation issues in DM user list
- Added bounds checking to prevent integer overflow in UI rendering
- Improved empty user list safety with comprehensive bounds checking

## 📈 Performance

- **Efficient Rendering**: Only colored content creates bubbles, not full-width backgrounds
- **Memory Management**: Conversation state properly managed per session
- **UI Responsiveness**: Smooth sidebar animations and navigation
- **Message Display**: Optimized message formatting and display logic

## 🔄 Migration Notes

This release is fully backward compatible with v0.6.0. No migration is required.

### For Developers
- New DM-related APIs available in `DMConversationManager`
- Extended message styling system with `DMSent` and `DMReceived` styles
- Enhanced status bar with notification capabilities

## 🎯 Future Roadmap

Based on v0.6.1 foundation, upcoming features include:
- DM message persistence across server restarts
- Typing indicators for DM conversations
- Message read receipts
- DM history search functionality
- File transfer capabilities
- User blocking and moderation features

## 🏗️ Built On

- **Rust**: Modern systems programming language
- **Tokio**: Async runtime for high-performance networking
- **Ratatui**: Terminal user interface framework
- **AES-256-GCM**: Military-grade encryption for all messages
- **X25519**: Elliptic curve key exchange for perfect forward secrecy

## 🙏 Acknowledgments

Special thanks to all contributors and users who provided feedback during the DM system development. Your input was invaluable in creating an intuitive and powerful messaging experience.

## 📋 Installation

```bash
# Clone the repository
git clone https://github.com/mberry/lair-chat.git
cd lair-chat

# Build the project
cargo build --release

# Start the server
cargo run --bin lair-chat-server

# Start a client (in another terminal)
cargo run --bin lair-chat-client -- --username your_username
```

## 🔗 Links

- **GitHub Repository**: https://github.com/mberry/lair-chat
- **Documentation**: See README.md and docs/ directory
- **Issues**: Report bugs and feature requests on GitHub
- **Previous Release**: [v0.6.0 Release Notes](RELEASE_NOTES_v0.6.0.md)

---

**Enjoy the enhanced messaging experience in Lair Chat v0.6.1!** 🚀