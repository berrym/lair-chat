# Direct Messaging User Guide

This guide explains how to use the direct messaging system in Lair Chat.

## Overview

The direct messaging (DM) system allows you to have private conversations with other users currently in the Lobby. All users automatically join the shared "Lobby" room when they connect, making them available for direct messaging. You can send text messages, share files, and manage your conversations efficiently.

## Getting Started

### Opening the DM System

- **Keyboard Shortcut**: Press `Ctrl+L` to open the DM navigation panel
- The DM panel will appear on the left side of your screen
- Use `Ctrl+L` again to close the DM panel

### Help System

- Press `?` to open the help popup, which includes all DM keybindings
- The help system shows both general chat controls and DM-specific commands

## DM Navigation Panel

When the DM navigation panel is open, you can:

### Basic Navigation
- `↑/↓` or `k/j` - Navigate up/down through conversations
- `Enter` - Open the selected conversation
- `Esc` - Close the DM navigation panel

## Starting New Conversations
- `n` - Start a new DM (opens user list showing other lobby users)
- Select a user from the list and press `Enter` to start chatting
- **Note**: Only users currently in the Lobby room will appear in the list

### How Users Appear in the List
The user list shows other users currently in the Lobby room:
- **Lobby System**: All users automatically join a shared "Lobby" room when they connect
- **Real Users**: Shows actual usernames of people connected to the same server
- **No Placeholder Data**: The list is now populated from real room participants
- **Status Indicators**: ● Online (users are online if they're in the lobby)
- **User Info**: Shows usernames and "In lobby" status for connected users

### Managing Conversations
- `a` - Archive/unarchive the selected conversation
- `m` - Mute/unmute the selected conversation
- `r` - Mark the selected conversation as read
- `R` - Mark ALL conversations as read
- `F5` - Refresh the conversation list

### Searching and Filtering
- `/` - Search through conversations (type to filter)
- `Tab` - Switch between view modes:
  - **Active** - Show only active conversations
  - **Archived** - Show only archived conversations  
  - **All** - Show all conversations
- `Ctrl+/` - Advanced search within conversations

## In-Conversation Features

Once you open a conversation:

### Sending Messages
- Type your message and press `Enter` to send
- Messages are delivered in real-time to the recipient

### Message Management
- `Ctrl+R` - Mark conversation as read
- View message timestamps and delivery status
- See when the other user was last active

### File Attachments
- The system supports file attachments (implementation varies by client)
- Supported file types include images, documents, and other common formats

## Conversation Views

### Active Conversations
- Shows conversations with recent activity
- Unread message counts are displayed
- Sorted by most recent activity

### Archived Conversations  
- View older or archived conversations
- Archive conversations to declutter your active list
- Archived conversations can be easily restored

### Search Results
- Real-time filtering as you type
- Searches conversation names and recent messages
- Clear search with `Esc`

## Status Indicators

### User Presence
- Online/offline status of conversation participants
- Last seen timestamps for offline users
- Real-time activity indicators

### Message Status
- Sent - Message has been sent
- Delivered - Message reached the recipient
- Read - Recipient has seen the message (if enabled)

### Conversation Status
- 🔔 Unmuted conversations (receive notifications)
- 🔕 Muted conversations (no notifications)
- 📁 Archived conversations
- 💬 Unread message count badges

## Tips and Best Practices

### Keyboard Efficiency
- Learn the keyboard shortcuts for faster navigation
- Use `j/k` for vim-style navigation if you prefer
- `Tab` quickly switches between conversation views

### Organization
- Archive old conversations to keep your active list clean
- Use mute for group conversations or busy contacts
- Mark conversations as read to maintain accurate unread counts

### Privacy and Security
- Direct messages are encrypted end-to-end
- Conversations are private between participants
- Message history is preserved locally
- Users must be in the same Lobby to initiate new conversations

### Lobby System
- **Automatic Join**: All users join the "Lobby" room automatically when connecting
- **User Discovery**: The DM user list shows other users currently in the Lobby
- **Shared Space**: The Lobby serves as both a public chat room and user discovery mechanism
- **Real-time Updates**: User list updates as people join/leave the Lobby

## Troubleshooting

### Common Issues
- **DM panel won't open**: Make sure you're connected to the server
- **Can't see conversations**: Try refreshing with `F5`
- **Search not working**: Clear search with `Esc` and try again

### Key Combinations Not Working
- Ensure the DM panel has focus (cursor should be visible in the panel)
- Some keys only work when not in search mode
- Exit search mode with `Esc` if commands aren't responding

### Performance
- Large conversation lists may take a moment to load
- Use search/filtering to navigate quickly through many conversations
- Archive old conversations to improve performance

## Complete Keybinding Reference

### Global Commands (Available Anywhere)
| Key | Action |
|-----|--------|
| `Ctrl+L` | Toggle DM navigation panel |
| `?` | Show/hide help popup |
| `Ctrl+C` | Quit application |

### DM Navigation Panel (When Panel is Open)
| Key | Action |
|-----|--------|
| `↑/↓` or `k/j` | Navigate conversations |
| `Enter` | Open selected conversation |
| `n` | Start new DM |
| `a` | Archive/unarchive conversation |
| `m` | Mute/unmute conversation |
| `r` | Mark conversation as read |
| `R` | Mark all conversations as read |
| `/` | Search conversations |
| `Tab` | Switch view mode (Active/Archived/All) |
| `F5` | Refresh conversation list |
| `Esc` | Close panel or exit search |

### In-Conversation View
| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Ctrl+R` | Mark as read |
| `Ctrl+/` | Search within conversation |
| `Esc` | Return to conversation list |

---

For additional help or to report issues with the DM system, please refer to the main documentation or contact support.