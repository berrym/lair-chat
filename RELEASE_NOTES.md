# LAIR-CHAT Release Notes

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