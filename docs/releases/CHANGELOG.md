# LAIR-CHAT Release Notes

*Last Updated: June 14, 2025*

## Current Version: 0.6.3 - Active Development

## Version 0.6.3 - June 14, 2025

### Overview
Critical bug fix release that resolves authentication and encryption handshake issues.

### 🔧 Bug Fixes
- **Fixed Authentication Transport Issue**: Resolved critical bug where AuthManager was using raw TCP transport instead of encrypted transport for authentication
- **Fixed Encryption Handshake Sequence**: Corrected the timing of authentication setup to occur after encryption handshake completion
- **Resolved Server Panic**: Fixed `InvalidByte(0, 123)` error caused by server receiving unencrypted JSON when expecting encrypted data
- **Fixed Client "No Response" Error**: Resolved authentication timeout issues caused by transport layer mismatch

### 🏗️ Technical Changes
- Removed premature `with_auth()` call from `App::new()` initialization
- Authentication now properly uses `EncryptedTransport` after X25519 + AES-GCM handshake
- Improved connection sequence: TCP → Encryption Handshake → Authentication Setup

### 📊 Previous Version: 0.6.2 - Active Development

### Upcoming Releases (Projected)

## Version 0.7.0 - September 2025 (Planned)

### Overview
Enhanced user experience and stability release focusing on polish and reliability improvements.

### Planned Major Features
- **Theme System**: Configurable color schemes and visual customization
- **Enhanced Navigation**: Improved keyboard shortcuts and context-sensitive help
- **Error Recovery**: Automatic reconnection with exponential backoff
- **Message Management**: Basic message editing and deletion capabilities

## Version 0.8.0 - December 2025 (Planned)

### Overview
Major release introducing secure file sharing capabilities and rich content support. This release represents a significant step toward the v1.0 production-ready milestone.

### Planned Major Features
- **Secure File Transfer**: End-to-end encrypted file sharing with progress indicators
- **Rich Content Support**: Image preview in compatible terminals (Sixel, Kitty)
- **Drag-and-Drop**: File sharing support in modern terminal emulators
- **Content Management**: File size limits, bandwidth throttling, and storage management

## Version 0.6.2

### Overview
The v0.6.2 release introduces a comprehensive unread messages enhancement system and a complete project reorganization. This release significantly improves user experience by providing better visibility and management of unread messages across conversations, while establishing a modern, scalable architecture foundation for future development.

### Major New Features
- **Global Status Bar Unread Indicator**: Always-visible unread DM count with clickable functionality
- **Cross-Conversation Notification System**: Smart temporary overlay notifications for messages from other conversations
- **Enhanced DM Navigation Visual Indicators**: Priority-based sorting with color-coded unread indicators
- **Interactive Status Bar Elements**: Click-to-open functionality for instant DM navigation
- **Mark All as Read**: Single-action functionality to clear all unread messages

### Architecture Improvements
- **🏗️ Complete Project Reorganization**: Systematic 5-phase migration to modern architecture
- **📁 Modular Structure**: Clean separation with `common/`, `client/`, and `server/` directories
- **🔗 Shared Modules**: Common functionality extracted for better code reuse
- **🚀 Enhanced Maintainability**: Logical grouping improves developer navigation and understanding
- **📦 Binary Reorganization**: Entry points moved to `src/bin/` for cleaner structure
- **🧪 Improved Testing**: Test organization now matches source structure

### Key Improvements
- **Real-time Updates**: Unread counts update automatically at 4Hz via tick events
- **Smart Notification Logic**: Context-aware notifications only when not viewing active conversation
- **Anti-spam Protection**: Intelligent notification deduplication from same sender
- **Color-coded Indicators**: Green (1-3), Magenta (4-10), Red (>10) unread messages
- **NEW Badges**: Visual indicators for messages received within last 5 minutes
- **Performance Optimized**: Minimal CPU overhead with efficient rendering

### Developer Experience
- **📚 Updated Documentation**: All docs reflect new project structure
- **🔄 Migration Guide**: Comprehensive guide for adapting to new module paths
- **🛠️ Enhanced Development**: Improved code organization for easier contribution
- **📈 Future-Ready**: Foundation prepared for plugin system and advanced features

### Technical Enhancements
- **3 New Actions**: UpdateUnreadDMCount, OpenDMNavigation, MarkAllDMsRead
- **NotificationOverlay Component**: Modular temporary notification system
- **Mouse Event Support**: StatusBar component handles interactive clicks
- **Enhanced Sorting Algorithm**: Priority-based conversation ordering
- **Comprehensive Test Coverage**: 8 test cases covering all new functionality

For detailed implementation information, see `RELEASE_NOTES_v0.6.2.md` and `UNREAD_MESSAGES_IMPLEMENTATION_SUMMARY.md`.

## Version 0.6.1

### Overview
The v0.6.1 release introduces complete Direct Messaging functionality, enabling private encrypted conversations between users alongside the existing public chat. This release builds on the solid v0.6.0 modern architecture foundation to deliver a comprehensive messaging platform.

### Major New Features
- **Complete Direct Messaging System**: Full DM functionality with encrypted private conversations
- **Visual Distinction**: Purple/green bubble styling for DM messages vs regular chat bubbles
- **Smart Notifications**: Status bar alerts and unread tracking for new DMs
- **Chat Switching Interface**: Tab-based sidebar for seamless navigation between conversations
- **Intuitive UX**: Clear mode headers, context-aware help, and visual indicators

### Key Improvements
- **Status Bar Notifications**: "💬 New DM from [username]" alerts for 8 seconds
- **Unread Tracking**: Bell icons (🔔) and counts for conversations with unread messages
- **Bubble Styling**: Proper bubble appearance with purple (sent) and green (received) DM colors
- **Navigation Enhancement**: Tab key toggles chat sidebar, Escape returns to Lobby
- **User List Integration**: Ctrl+L → N opens user list for starting new DMs

### Technical Enhancements
- **DM Conversation Manager**: New system for managing multiple DM conversations
- **Message Routing**: Server-side DM routing with `DM:target:message` protocol
- **Enhanced UI Layout**: Dynamic layout with collapsible chat sidebar
- **Protocol Extensions**: `DM_FROM:sender:content` format for received DMs
- **Memory Management**: Efficient in-memory conversation storage during session

### Controls Summary
- **Tab**: Toggle chat sidebar (main navigation method)
- **Up/Down**: Navigate sidebar when open
- **Enter**: Switch to selected chat from sidebar
- **Escape**: Return to Lobby from DM mode or close sidebar
- **Ctrl+L → N**: Open user list for new DM conversations

### Bug Fixes
- Fixed stack overflow when sending DM messages
- Resolved user selection navigation issues in DM user list
- Added bounds checking to prevent integer overflow in UI rendering
- Improved empty user list safety with comprehensive bounds checking

### What's Next (v0.7.0)
1. **DM Message Persistence**: Save DM history across server restarts
2. **File Transfer System**: Implement secure file sharing capabilities
3. **Advanced DM Features**: Typing indicators and read receipts for DMs
4. **Performance Optimization**: Enhanced performance for large conversation histories
5. **DM Search**: Search functionality within DM conversation history

## Version 0.6.0

### Overview
Complete architectural modernization with async/await patterns, clean abstractions, and enhanced security. This release represents a complete rewrite of the core systems.

### Major Architectural Changes
- **Modern Async Architecture**: Complete rewrite with async/await throughout
- **ConnectionManager**: New abstraction layer for all network operations
- **Type-Safe Error Handling**: Comprehensive error types replace string-based errors
- **Observer Pattern**: Event-driven communication between components
- **Legacy Code Removal**: All legacy global state and compatibility layers removed

### Performance Improvements
- **60% CPU Usage Reduction**: Optimized async operations and connection management
- **40% Memory Usage Reduction**: Efficient resource management and cleanup
- **Enhanced Responsiveness**: Non-blocking operations throughout the application

### Security Enhancements
- **X25519 + AES-256-GCM**: Enhanced encryption with elliptic curve key exchange
- **Server-Compatible Encryption**: Standardized encryption across client-server communication
- **Perfect Forward Secrecy**: Advanced cryptographic protection

### Testing & Quality
- **85% Test Coverage**: Comprehensive test suite with mock implementations
- **Integration Testing**: End-to-end testing infrastructure
- **Performance Benchmarking**: Established performance baselines and monitoring

## Version 0.5.1

### Overview
This release focuses on improving the message display formatting and fixing several key UI issues that emerged during the async integration refactoring process. It represents the final phase of the refactoring work to modernize the chat application's architecture.

### Key Improvements
- **Fixed Message Formatting**: Resolved issues with duplicate username prefixes and inconsistent formatting in messages
- **Status Bar Enhancement**: Fixed received message counting in the status bar
- **Username Formatting**: Eliminated unwanted "!" characters from username displays
- **Message Deduplication**: Improved algorithm for detecting and preventing duplicate messages
- **System Message Handling**: Enhanced detection and formatting of system messages
- **Logging**: Added additional debug logging to help diagnose messaging issues

### Technical Enhancements
- Modified `get_display_messages` to handle username formatting consistently
- Enhanced `add_message_to_room` to properly clean message content
- Updated message normalization to better detect duplicates
- Improved handling of the `RecordReceivedMessage` action
- Added robust error handling for status bar updates

### What's Next
1. **Complete Migration to ConnectionManager**: Finalize deprecation of legacy transport system
2. **Documentation**: Update system architecture documentation to reflect the new design
3. **Enhanced Typing Indicators**: Implement more responsive typing indicators between clients
4. **Notification System**: Improve notification handling for unread messages
5. **Encrypted File Sharing**: Begin implementation of secure file sharing features

### Notes for Developers
- The compatibility layer will be removed in v0.6.0
- Warnings about deprecated functions can be safely ignored for now as they relate to the ongoing migration
- The test suite has been updated to accommodate the new message handling architecture