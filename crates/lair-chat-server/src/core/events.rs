//! Event dispatching - broadcasting events to connected clients.
//!
//! The event dispatcher:
//! - Receives events from services
//! - Routes events to appropriate recipients
//! - Manages online user tracking
//! - Supports typing indicators

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use crate::domain::{
    events::{
        Event, EventPayload, EventTarget, UserOfflineEvent, UserOnlineEvent, UserTypingEvent,
    },
    MessageTarget, UserId,
};

// ============================================================================
// EventDispatcher
// ============================================================================

/// Event dispatcher for broadcasting events to connected clients.
///
/// Protocol adapters register their connections with the dispatcher and
/// receive events targeted at their users.
#[derive(Clone)]
pub struct EventDispatcher {
    inner: Arc<EventDispatcherInner>,
}

struct EventDispatcherInner {
    /// Broadcast channel for events.
    sender: broadcast::Sender<Event>,

    /// Set of online users.
    online_users: RwLock<HashSet<UserId>>,

    /// User connection counts (for multi-device support).
    connection_counts: RwLock<HashMap<UserId, u32>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(EventDispatcherInner {
                sender,
                online_users: RwLock::new(HashSet::new()),
                connection_counts: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Subscribe to events.
    ///
    /// Returns a receiver that will receive all dispatched events.
    /// The caller is responsible for filtering events based on their user.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.sender.subscribe()
    }

    /// Dispatch an event to all subscribers.
    pub async fn dispatch(&self, event: Event) {
        // Ignore send errors (no subscribers)
        let _ = self.inner.sender.send(event);
    }

    /// Mark a user as online.
    ///
    /// Called when a user establishes a connection.
    /// Emits `UserOnline` event if this is their first connection.
    pub async fn user_online(&self, user_id: UserId, username: String) {
        let mut counts = self.inner.connection_counts.write().await;
        let count = counts.entry(user_id).or_insert(0);
        *count += 1;

        // If this is the first connection, mark as online and emit event
        if *count == 1 {
            let mut online = self.inner.online_users.write().await;
            online.insert(user_id);
            drop(online);

            // Emit online event
            let event = Event::new(EventPayload::UserOnline(UserOnlineEvent::new(
                user_id, username,
            )));
            self.dispatch(event).await;
        }
    }

    /// Mark a user as offline.
    ///
    /// Called when a user's connection closes.
    /// Emits `UserOffline` event if this was their last connection.
    pub async fn user_offline(&self, user_id: UserId, username: String) {
        let mut counts = self.inner.connection_counts.write().await;
        if let Some(count) = counts.get_mut(&user_id) {
            *count = count.saturating_sub(1);

            // If this was the last connection, mark as offline and emit event
            if *count == 0 {
                counts.remove(&user_id);
                drop(counts);

                let mut online = self.inner.online_users.write().await;
                online.remove(&user_id);
                drop(online);

                // Emit offline event
                let event = Event::new(EventPayload::UserOffline(UserOfflineEvent::new(
                    user_id, username,
                )));
                self.dispatch(event).await;
            }
        }
    }

    /// Check if a user is online.
    pub async fn is_online(&self, user_id: UserId) -> bool {
        let online = self.inner.online_users.read().await;
        online.contains(&user_id)
    }

    /// Get the count of online users.
    pub async fn online_user_count(&self) -> usize {
        let online = self.inner.online_users.read().await;
        online.len()
    }

    /// Get all online users.
    pub async fn online_users(&self) -> Vec<UserId> {
        let online = self.inner.online_users.read().await;
        online.iter().copied().collect()
    }

    /// Send a typing indicator.
    pub async fn user_typing(&self, user_id: UserId, target: MessageTarget) {
        let event = Event::new(EventPayload::UserTyping(UserTypingEvent::new(
            user_id, target,
        )));
        self.dispatch(event).await;
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Event Filtering
// ============================================================================

/// Check if a user should receive an event.
///
/// This is used by protocol adapters to filter events for their connections.
pub fn should_receive_event(
    event: &Event,
    user_id: UserId,
    user_rooms: &[crate::domain::RoomId],
) -> bool {
    match event.target() {
        EventTarget::User(target_user) => target_user == user_id,
        EventTarget::Room(room_id) => user_rooms.contains(&room_id),
        EventTarget::DirectMessage { user1, user2 } => user_id == user1 || user_id == user2,
        EventTarget::UserConnections(_target_user) => {
            // User receives events about people they might know
            // In practice, this should check shared rooms or DM history
            // For now, broadcast to everyone (will be filtered by client interest)
            true
        }
        EventTarget::Session(_session_id) => {
            // Session-specific events are handled separately
            false
        }
        EventTarget::Broadcast => true,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_online_offline() {
        let dispatcher = EventDispatcher::new();
        let user_id = UserId::new();

        assert!(!dispatcher.is_online(user_id).await);

        dispatcher.user_online(user_id, "testuser".to_string()).await;
        assert!(dispatcher.is_online(user_id).await);
        assert_eq!(dispatcher.online_user_count().await, 1);

        dispatcher.user_offline(user_id, "testuser".to_string()).await;
        assert!(!dispatcher.is_online(user_id).await);
        assert_eq!(dispatcher.online_user_count().await, 0);
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let dispatcher = EventDispatcher::new();
        let user_id = UserId::new();

        // First connection
        dispatcher.user_online(user_id, "testuser".to_string()).await;
        assert!(dispatcher.is_online(user_id).await);

        // Second connection
        dispatcher.user_online(user_id, "testuser".to_string()).await;
        assert!(dispatcher.is_online(user_id).await);

        // First connection closes - still online
        dispatcher.user_offline(user_id, "testuser".to_string()).await;
        assert!(dispatcher.is_online(user_id).await);

        // Second connection closes - now offline
        dispatcher.user_offline(user_id, "testuser".to_string()).await;
        assert!(!dispatcher.is_online(user_id).await);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let dispatcher = EventDispatcher::new();
        let mut receiver = dispatcher.subscribe();

        let user_id = UserId::new();
        let event = Event::new(EventPayload::UserOnline(UserOnlineEvent::new(
            user_id,
            "test".to_string(),
        )));

        dispatcher.dispatch(event.clone()).await;

        let received = receiver.try_recv().unwrap();
        assert_eq!(received.id, event.id);
    }
}
