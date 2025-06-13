# Direct Messaging (DM) Implementation Action Plan

## Overview
This document outlines the remaining tasks to complete the direct messaging system implementation. The core infrastructure (server room management, user tracking, and basic client-server communication) has been established. This plan focuses on the UI/UX integration and complete DM workflow.

## Current Status ✅
- [x] Server creates Lobby room at startup
- [x] All users automatically join Lobby when connecting
- [x] Server tracks connected users and broadcasts user lists
- [x] Client shows "Lobby" in status bar instead of "No Room"
- [x] User DM list populated with all connected users (Ctrl+L + n)
- [x] Encrypted communication for all server messages
- [x] Support for DM protocol messages (DM:target:message format)
- [x] Client handles DM_FROM messages from server

## Phase 1: User Selection and DM Initiation (Priority: High)

### Task 1.1: Implement User Selection Events
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/components/home.rs`
- `src/client/components/user_list.rs`

**Steps:**
1. [ ] Modify `Home::new()` to create UserListPanel with event sender
2. [ ] Create event receiver channel in Home component
3. [ ] Add `handle_user_list_events()` method to process UserListEvent::UserSelected
4. [ ] Test user selection triggers correct event

**Acceptance Criteria:**
- User can select someone from user list with Enter key
- Selection event is properly received by Home component
- User list closes after selection

### Task 1.2: Create DM Conversation State Management
**Estimate: 3-4 hours**
**Files to create/modify:**
- `src/client/chat/dm_conversations.rs` (new)
- `src/client/chat/mod.rs`

**Steps:**
1. [ ] Create `DMConversation` struct to track individual DM threads
2. [ ] Create `DMConversationManager` to manage multiple DM conversations
3. [ ] Implement conversation creation, message storage, and retrieval
4. [ ] Add conversation state persistence (in-memory for now)
5. [ ] Integrate with existing chat message system

**Acceptance Criteria:**
- Can create new DM conversation with selected user
- Each DM conversation maintains separate message history
- Conversations persist during client session

### Task 1.3: DM Mode Toggle and UI State
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/components/home.rs`
- `src/client/action.rs`

**Steps:**
1. [ ] Add `current_dm_partner: Option<String>` to Home state
2. [ ] Add `dm_mode: bool` flag to track when in DM conversation
3. [ ] Create `Action::StartDMConversation(username: String)` action
4. [ ] Implement DM mode toggle logic in `handle_user_list_events()`
5. [ ] Update status bar to show DM partner when in DM mode

**Acceptance Criteria:**
- Selecting user from list switches to DM mode
- Status bar shows "DM with [username]" instead of "Lobby"
- Can return to Lobby mode (Escape key)

## Phase 2: DM Message Input and Display (Priority: High)

### Task 2.1: DM Message Input Handling
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/components/home.rs`

**Steps:**
1. [ ] Modify input handling to detect DM mode vs Lobby mode
2. [ ] Format outgoing messages as "DM:target_user:message_content"
3. [ ] Send DM messages through existing `Action::SendMessage` system
4. [ ] Add visual indicator when typing in DM mode
5. [ ] Test message sending to server with DM format

**Acceptance Criteria:**
- Messages typed in DM mode are sent with DM: prefix
- Server receives and routes DM messages correctly
- Input area shows DM context (e.g., "To: username")

### Task 2.2: DM Message Display and Formatting
**Estimate: 3-4 hours**
**Files to modify:**
- `src/client/components/home.rs`

**Steps:**
1. [ ] Modify `get_display_messages_with_style()` to handle DM mode
2. [ ] Filter messages to show only DM conversation when in DM mode
3. [ ] Update message formatting for DM display (remove "DM from" prefix in DM view)
4. [ ] Add sent DM message tracking ("You: message" format)
5. [ ] Implement conversation switching UI

**Acceptance Criteria:**
- DM mode shows only messages from current DM conversation
- Sent messages show as "You: message"
- Received messages show as "Partner: message"
- Clean, chat-like appearance without protocol prefixes

### Task 2.3: DM Conversation History
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/chat/dm_conversations.rs`
- `src/client/components/home.rs`

**Steps:**
1. [ ] Implement message history storage per DM conversation
2. [ ] Add conversation retrieval by partner username
3. [ ] Integrate DM history with display system
4. [ ] Add conversation creation timestamp
5. [ ] Test history persistence during client session

**Acceptance Criteria:**
- Each DM conversation maintains independent message history
- Switching between DM conversations shows correct history
- Message history survives mode switches (DM -> Lobby -> DM)

## Phase 3: DM Navigation and Management (Priority: Medium)

### Task 3.1: Active DM Conversations List
**Estimate: 3-4 hours**
**Files to modify:**
- `src/client/components/dm_navigation.rs`
- `src/client/components/home.rs`

**Steps:**
1. [ ] Modify DM navigation panel to show active conversations
2. [ ] Add conversation list with last message preview
3. [ ] Implement conversation selection from navigation panel
4. [ ] Add unread message indicators
5. [ ] Add conversation close/delete functionality

**Acceptance Criteria:**
- DM navigation shows list of active conversations
- Can switch between conversations from navigation panel
- Visual indicators for new/unread messages
- Can close conversations when done

### Task 3.2: Enhanced User Status and Presence
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/components/user_list.rs`
- `src/server/main.rs`

**Steps:**
1. [ ] Add user status tracking on server (online/away/busy)
2. [ ] Broadcast user status changes to all clients
3. [ ] Update user list to show status indicators
4. [ ] Add "last seen" information for offline users
5. [ ] Implement status filtering in user list

**Acceptance Criteria:**
- User list shows online/offline status clearly
- Status updates in real-time when users connect/disconnect
- Can filter user list by online status

### Task 3.3: DM Notifications and Indicators
**Estimate: 2-3 hours**
**Files to modify:**
- `src/client/components/home.rs`
- `src/client/components/status/mod.rs`

**Steps:**
1. [ ] Add notification system for new DM messages
2. [ ] Update status bar to show DM notification count
3. [ ] Add visual/audio notification when not in DM mode
4. [ ] Implement message read/unread tracking
5. [ ] Add notification clearing when entering DM conversation

**Acceptance Criteria:**
- New DM messages trigger notifications when not in that conversation
- Status bar shows unread DM count
- Notifications clear when viewing conversation
- Non-intrusive notification system

## Phase 4: Polish and Robustness (Priority: Low)

### Task 4.1: Error Handling and Edge Cases
**Estimate: 3-4 hours**
**Files to modify:**
- Multiple files across client and server

**Steps:**
1. [ ] Handle DM to offline/disconnected users gracefully
2. [ ] Add error messages for failed DM delivery
3. [ ] Implement DM retry mechanism for network issues
4. [ ] Handle conversation cleanup when users disconnect
5. [ ] Add validation for DM message limits/content

**Acceptance Criteria:**
- Clear error messages for DM failures
- Graceful degradation when network issues occur
- Proper cleanup when users disconnect
- Input validation prevents malformed messages

### Task 4.2: DM Message Persistence (Optional)
**Estimate: 4-6 hours**
**Files to create/modify:**
- `src/server/dm_storage.rs` (new)
- `src/server/main.rs`

**Steps:**
1. [ ] Design DM message storage schema
2. [ ] Implement file-based or SQLite DM storage
3. [ ] Add DM history retrieval on client connect
4. [ ] Implement DM history limits and cleanup
5. [ ] Add configuration for DM retention policies

**Acceptance Criteria:**
- DM conversations persist across server restarts
- Configurable message retention policies
- Efficient storage and retrieval of DM history

### Task 4.3: Advanced DM Features
**Estimate: 5-8 hours**
**Files to modify:**
- Multiple files across client and server

**Steps:**
1. [ ] Add typing indicators for DM conversations
2. [ ] Implement message read receipts
3. [ ] Add DM message search functionality
4. [ ] Implement DM conversation export
5. [ ] Add DM blocking/muting functionality

**Acceptance Criteria:**
- Typing indicators work in DM conversations
- Read receipts show message delivery status
- Can search within DM conversation history
- Can export DM conversations to text files

## Testing Plan

### Unit Tests Required:
- [ ] DM conversation creation and management
- [ ] Message routing (DM vs public messages)
- [ ] User selection event handling
- [ ] DM message formatting and display

### Integration Tests Required:
- [ ] End-to-end DM workflow (select user -> send message -> receive reply)
- [ ] Multiple concurrent DM conversations
- [ ] DM with user disconnect/reconnect scenarios
- [ ] DM message persistence (if implemented)

### Manual Testing Scenarios:
1. [ ] User A selects User B from list and starts DM conversation
2. [ ] User A sends message to User B, User B receives and replies
3. [ ] User A has DM with User B, then starts new DM with User C
4. [ ] User switches between Lobby and DM modes repeatedly
5. [ ] User disconnects/reconnects during active DM conversation
6. [ ] Multiple users in Lobby + multiple DM conversations simultaneously

## Success Metrics

### Phase 1 Complete:
- ✅ Users can select someone from user list and enter DM mode
- ✅ Status bar shows DM partner information
- ✅ Basic DM conversation state management works

### Phase 2 Complete:
- ✅ Users can send and receive DM messages
- ✅ DM conversations display correctly with proper formatting
- ✅ Message history works for each DM conversation

### Phase 3 Complete:
- ✅ Users can manage multiple DM conversations
- ✅ DM navigation panel shows active conversations
- ✅ Notification system works for new DM messages

### Phase 4 Complete:
- ✅ Robust error handling and edge case management
- ✅ Optional: DM persistence across server restarts
- ✅ Optional: Advanced features like typing indicators

## Technical Notes

### Key Design Decisions:
1. **DM Routing**: Using "DM:target:message" format for server-side routing
2. **State Management**: Each DM conversation maintains independent state
3. **UI Mode**: Toggle between Lobby mode and DM mode rather than tabs
4. **Storage**: Start with in-memory, optionally add persistence later

### Performance Considerations:
1. Limit number of concurrent DM conversations per user
2. Implement message history limits to prevent memory bloat
3. Efficient conversation switching without full reloads
4. Minimal server-side storage requirements

### Security Considerations:
1. Validate DM target usernames server-side
2. Rate limiting for DM messages to prevent spam
3. Input sanitization for DM message content
4. User blocking/reporting mechanisms (future enhancement)

## Timeline Estimate

- **Phase 1**: 1-2 days (7-10 hours)
- **Phase 2**: 1-2 days (7-10 hours)  
- **Phase 3**: 2-3 days (7-12 hours)
- **Phase 4**: 2-4 days (12-18 hours)

**Total Estimated Time**: 1-2 weeks for full implementation with all phases
**Minimum Viable Product**: Phases 1-2 (2-4 days for basic DM functionality)