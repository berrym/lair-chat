# Unread Messages Enhancement Plan

## Overview
Enhance the Lair Chat user experience by improving unread message indicators and notifications when messages arrive from conversations other than the currently viewed chat.

## Current State Analysis

### ✅ Already Implemented
1. **ConversationSummary unread tracking**: Each conversation tracks `unread_count`
2. **DM Navigation Panel indicators**: Shows "●" for unread messages and count badges
3. **Title badges**: Navigation panel title shows total unread count like "Direct Messages (5)"
4. **Visual styling**: Unread conversations are bold and highlighted in red
5. **Mark as read functionality**: `mark_messages_read` API exists
6. **Total unread count tracking**: `get_total_unread_count` available

### ❌ Missing Features
1. **Global status bar indicator**: No system-wide unread count visible outside DM panel
2. **Cross-conversation notifications**: No alerts when viewing one chat and receiving messages in another
3. **Real-time updates**: Unread counts may not update in real-time across UI components
4. **Audio/visual notifications**: No sound or visual flash for new messages
5. **Desktop notifications**: No system notifications for background messages

## Enhancement Goals

### 1. Global Unread Message Indicator
**Problem**: Users can't see if they have unread DMs unless the DM navigation panel is open.
**Solution**: Add persistent unread count badge to main UI status bar.

### 2. Cross-Conversation Message Alerts  
**Problem**: When viewing a specific chat, users miss messages from other conversations.
**Solution**: Show temporary notification overlay when messages arrive from other chats.

### 3. Enhanced Tab/Chat View Indicators
**Problem**: Hard to see which conversations need attention in the navigation list.
**Solution**: Improve visual indicators and sorting by unread status.

## Implementation Plan

### Phase 1: Global Status Bar Enhancement (2-3 hours)

#### 1.1 Status Bar Unread Count
- Add unread message count to main status bar
- Show format: "💬 3" or "DMs (3)" when unread messages exist
- Hide indicator when no unread messages
- Make clickable to open DM navigation panel

**Files to modify:**
- `src/client/components/status/mod.rs` (if exists) or status bar component
- `src/client/components/home.rs` (status bar rendering)
- `src/client/app.rs` (global state management)

#### 1.2 Real-time Updates
- Ensure status bar updates immediately when messages arrive
- Connect to DirectMessageManager events
- Update count when conversations are marked as read

### Phase 2: Cross-Conversation Notifications (3-4 hours)

#### 2.1 Notification System
- Create notification overlay component
- Show temporary notifications for messages from other conversations
- Include sender name and message preview
- Auto-dismiss after 3-5 seconds
- Allow manual dismiss with Esc or click

**New files:**
- `src/client/components/notifications/message_notification.rs`
- `src/client/components/notifications/mod.rs`

#### 2.2 Smart Notification Logic
- Only show notifications when:
  - Currently viewing a different conversation OR
  - Currently in lobby/room chat mode
- Don't show notifications for currently active conversation
- Respect "muted" conversation settings

### Phase 3: Enhanced Visual Indicators (2-3 hours)

#### 3.1 Improved DM Navigation Panel
- Sort conversations by unread status (unread first)
- Add visual pulse/glow animation for new messages
- Show "NEW" badge for very recent messages (< 5 minutes)
- Improve color coding and contrast

**Files to modify:**
- `src/client/components/dm_navigation.rs`

#### 3.2 Chat Tab Indicators
- Add unread count next to conversation names
- Use different colors for different urgency levels:
  - Red: > 10 unread messages
  - Orange: 3-10 unread messages  
  - Blue: 1-2 unread messages
- Show last message timestamp

### Phase 4: Advanced Features (Optional, 2-3 hours)

#### 4.1 Audio Notifications
- Add optional sound effects for new messages
- Different sounds for DMs vs room messages
- User preference to enable/disable sounds

#### 4.2 Desktop Notifications
- System tray notifications (if supported)
- Brief message preview in notification
- Click to focus application

## Technical Architecture

### State Management
```rust
// Global unread state in App
pub struct UnreadMessageState {
    pub total_dm_count: u32,
    pub conversation_counts: HashMap<ConversationId, u32>,
    pub last_notification_time: HashMap<ConversationId, u64>,
    pub notifications_enabled: bool,
    pub sound_enabled: bool,
}
```

### Event System  
```rust
// New events for unread message updates
pub enum UnreadMessageEvent {
    CountUpdated { conversation_id: ConversationId, new_count: u32 },
    NewMessageReceived { conversation_id: ConversationId, sender: String, preview: String },
    ConversationMarkedRead { conversation_id: ConversationId },
    GlobalCountChanged { total_count: u32 },
}
```

### Notification Component
```rust
// Temporary notification overlay
pub struct MessageNotification {
    pub id: uuid::Uuid,
    pub sender_name: String,
    pub message_preview: String,
    pub conversation_id: ConversationId,
    pub created_at: SystemTime,
    pub auto_dismiss_time: SystemTime,
}
```

## Implementation Steps

### Step 1: Status Bar Integration
1. Create `UnreadMessageState` in main App
2. Connect to DirectMessageManager events
3. Add unread count display to status bar
4. Test real-time updates

### Step 2: Notification Overlay
1. Create notification component
2. Add notification queue management
3. Implement auto-dismiss logic
4. Style notifications for good UX

### Step 3: Enhanced Navigation
1. Update DM navigation sorting
2. Add visual indicators and animations
3. Improve color scheme
4. Test with multiple conversations

### Step 4: Integration Testing
1. Test across all chat modes (lobby, DM, room)
2. Verify performance with many conversations
3. Test edge cases (offline, reconnection)
4. User acceptance testing

## User Experience Flow

### Scenario 1: User in Lobby, receives DM
1. Status bar shows "💬 1"
2. Notification overlay appears: "Alice: Hey there!" (3 seconds)
3. DM navigation panel (if open) shows Alice's conversation with unread indicator

### Scenario 2: User in DM with Alice, receives message from Bob
1. Status bar updates to "💬 1" 
2. Notification overlay: "Bob: Quick question..." (3 seconds)
3. DM navigation shows Bob's conversation moved to top with unread indicator
4. Alice's conversation remains active/focused

### Scenario 3: User opens DM navigation panel
1. Conversations sorted by: unread first, then by recent activity
2. Unread conversations show bold text + count badges
3. Visual pulse on newly arrived messages
4. Clear visual hierarchy

## Success Criteria

### Must Have
- [x] Users can see unread DM count without opening navigation panel
- [x] Users get notified of messages from other conversations
- [x] Unread indicators update in real-time
- [x] No performance impact on chat functionality

### Nice to Have
- [x] Audio notifications (user preference)
- [x] Desktop system notifications
- [x] Advanced visual effects (pulse, glow)
- [x] Keyboard shortcuts for navigation

## Risk Mitigation

### Performance Risks
- **Risk**: Too many real-time updates impacting performance
- **Mitigation**: Debounce updates, batch notifications, limit notification queue

### UX Risks  
- **Risk**: Notification overload, too distracting
- **Mitigation**: Smart filtering, user preferences, respectful timing

### Technical Risks
- **Risk**: State synchronization issues between components
- **Mitigation**: Centralized state management, proper event handling

## Testing Strategy

### Unit Tests
- Unread count calculations
- Notification queue management  
- Event handling logic

### Integration Tests
- Cross-component state updates
- Real-time message flow
- Navigation panel updates

### User Testing
- Usability of notification system
- Discoverability of unread indicators
- Performance under load

## Future Enhancements

### Advanced Features
1. **Smart Notifications**: Learn user patterns, adjust timing
2. **Message Categories**: Different indicators for urgent vs casual messages
3. **Conversation Grouping**: Group related conversations, show aggregate counts
4. **Mobile-style Indicators**: Badge overlays on conversation avatars
5. **Accessibility**: Screen reader support, high contrast modes

### Integration Opportunities
1. **Email Notifications**: For offline users
2. **Webhook Support**: External integrations
3. **Analytics**: Track unread message patterns
4. **AI Summaries**: Smart preview generation for long messages

## Implementation Timeline

- **Week 1**: Phase 1 (Status bar enhancement)
- **Week 2**: Phase 2 (Cross-conversation notifications) 
- **Week 3**: Phase 3 (Enhanced visual indicators)
- **Week 4**: Phase 4 (Optional advanced features) + Testing

**Total Effort**: 9-13 hours over 4 weeks
**Priority**: High (significantly improves user experience)
**Complexity**: Medium (builds on existing infrastructure)

## Conclusion

This enhancement plan builds upon the solid foundation already in place while addressing key user experience gaps. The phased approach allows for incremental delivery and testing, ensuring stability while adding valuable functionality.

The focus on real-time updates and cross-conversation awareness will make Lair Chat much more user-friendly for active conversations and prevent users from missing important messages.