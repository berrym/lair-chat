//! Tests for unread messages enhancement functionality
//!
//! This module tests the new unread message features including:
//! - Status bar unread count display
//! - Cross-conversation notifications
//! - Enhanced visual indicators in DM navigation

use std::time::Duration;

use lair_chat::{
    action::Action,
    auth::AuthState,
    chat::DMConversationManager,
    components::{MessageNotification, NotificationOverlay, StatusBar},
    transport::ConnectionStatus,
};

#[tokio::test]
async fn test_status_bar_unread_count() {
    let mut status_bar = StatusBar::new();

    // Initially no unread messages
    assert_eq!(status_bar.get_unread_dm_count(), 0);

    // Set unread count
    status_bar.set_unread_dm_count(3);
    assert_eq!(status_bar.get_unread_dm_count(), 3);

    // Reset to zero
    status_bar.set_unread_dm_count(0);
    assert_eq!(status_bar.get_unread_dm_count(), 0);
}

#[tokio::test]
async fn test_dm_conversation_manager_unread_tracking() {
    let mut dm_manager = DMConversationManager::new("alice".to_string());

    // Initially no unread messages
    assert_eq!(dm_manager.get_total_unread_count(), 0);

    // Receive a message from Bob (should be unread)
    dm_manager
        .receive_message("bob".to_string(), "Hello Alice!".to_string())
        .expect("Failed to receive message");

    assert_eq!(dm_manager.get_total_unread_count(), 1);
    assert_eq!(dm_manager.get_unread_count_with_user("bob").unwrap(), 1);

    // Receive another message from Bob
    dm_manager
        .receive_message("bob".to_string(), "How are you?".to_string())
        .expect("Failed to receive message");

    assert_eq!(dm_manager.get_total_unread_count(), 2);
    assert_eq!(dm_manager.get_unread_count_with_user("bob").unwrap(), 2);

    // Receive a message from Charlie
    dm_manager
        .receive_message("charlie".to_string(), "Hi there!".to_string())
        .expect("Failed to receive message");

    assert_eq!(dm_manager.get_total_unread_count(), 3);
    assert_eq!(dm_manager.get_unread_count_with_user("charlie").unwrap(), 1);

    // Set Bob's conversation as active (should mark as read)
    dm_manager.set_active_conversation(Some("bob".to_string()));

    assert_eq!(dm_manager.get_total_unread_count(), 1); // Only Charlie's message
    assert_eq!(dm_manager.get_unread_count_with_user("bob").unwrap(), 0);
    assert_eq!(dm_manager.get_unread_count_with_user("charlie").unwrap(), 1);

    // Mark all as read
    dm_manager.mark_all_read();
    assert_eq!(dm_manager.get_total_unread_count(), 0);
    assert_eq!(dm_manager.get_unread_count_with_user("charlie").unwrap(), 0);
}

#[tokio::test]
async fn test_notification_overlay() {
    let mut overlay = NotificationOverlay::new();

    // Initially not visible
    assert!(!overlay.is_visible());
    assert_eq!(overlay.notification_count(), 0);

    // Add a notification
    let notification = MessageNotification::new(
        "Alice".to_string(),
        "Hey there!".to_string(),
        "alice_123".to_string(),
        Duration::from_secs(5),
    );

    overlay.add_notification(notification);

    assert!(overlay.is_visible());
    assert_eq!(overlay.notification_count(), 1);

    // Add another notification from the same sender (should replace)
    let notification2 = MessageNotification::new(
        "Alice".to_string(),
        "Another message".to_string(),
        "alice_123".to_string(),
        Duration::from_secs(5),
    );

    overlay.add_notification(notification2);

    assert_eq!(overlay.notification_count(), 1); // Should still be 1 (replaced)

    // Add notification from different sender
    let notification3 = MessageNotification::new(
        "Bob".to_string(),
        "Hello from Bob".to_string(),
        "bob_456".to_string(),
        Duration::from_secs(5),
    );

    overlay.add_notification(notification3);

    assert_eq!(overlay.notification_count(), 2);

    // Dismiss all
    overlay.dismiss_all();

    assert!(!overlay.is_visible());
    assert_eq!(overlay.notification_count(), 0);
}

#[tokio::test]
async fn test_notification_expiry() {
    let mut overlay = NotificationOverlay::new();

    // Create an already-expired notification
    let expired_notification = MessageNotification {
        id: uuid::Uuid::new_v4(),
        sender_name: "TestUser".to_string(),
        message_preview: "Test message".to_string(),
        conversation_id: "test_conv".to_string(),
        created_at: std::time::SystemTime::now() - Duration::from_secs(10),
        auto_dismiss_time: std::time::SystemTime::now() - Duration::from_secs(5),
    };

    // Manually add expired notification (bypassing the normal add_notification method)
    overlay.add_notification(expired_notification);

    assert_eq!(overlay.notification_count(), 1);

    // Clean up expired notifications
    overlay.cleanup_expired();

    assert_eq!(overlay.notification_count(), 0);
    assert!(!overlay.is_visible());
}

#[tokio::test]
async fn test_message_notification_creation() {
    let notification = MessageNotification::new(
        "TestUser".to_string(),
        "This is a test message".to_string(),
        "conversation_123".to_string(),
        Duration::from_secs(10),
    );

    assert_eq!(notification.sender_name, "TestUser");
    assert_eq!(notification.message_preview, "This is a test message");
    assert_eq!(notification.conversation_id, "conversation_123");

    // Should not be dismissed immediately
    assert!(!notification.should_dismiss());

    // Time remaining should be close to 10 seconds
    let time_remaining = notification.time_remaining().unwrap();
    assert!(time_remaining.as_secs() <= 10);
    assert!(time_remaining.as_secs() >= 9); // Allow for small timing differences
}

#[tokio::test]
async fn test_status_bar_dm_notification() {
    let mut status_bar = StatusBar::new();

    // Show a DM notification
    status_bar.show_dm_notification("Alice".to_string(), Duration::from_secs(3));

    // The notification should be visible initially
    // Note: We can't directly test the rendering, but we can verify the method doesn't panic

    // Set connection status and auth state for completeness
    status_bar.set_connection_status(ConnectionStatus::CONNECTED);
    status_bar.set_auth_state(AuthState::Unauthenticated);
    status_bar.set_current_room(Some("Test Room".to_string()));

    // Record some messages
    status_bar.record_sent_message();
    status_bar.record_received_message();

    assert_eq!(status_bar.get_sent_count(), 1);
    assert_eq!(status_bar.get_received_count(), 1);
}

#[test]
fn test_actions_for_unread_messages() {
    // Test that our new actions can be created and compared
    let update_action = Action::UpdateUnreadDMCount(5);
    let open_dm_action = Action::OpenDMNavigation;
    let mark_read_action = Action::MarkAllDMsRead;

    // Test action formatting (Debug trait)
    let update_str = format!("{:?}", update_action);
    assert!(update_str.contains("UpdateUnreadDMCount"));
    assert!(update_str.contains("5"));

    let open_str = format!("{:?}", open_dm_action);
    assert!(open_str.contains("OpenDMNavigation"));

    let mark_read_str = format!("{:?}", mark_read_action);
    assert!(mark_read_str.contains("MarkAllDMsRead"));

    // Test action equality
    assert_eq!(
        Action::UpdateUnreadDMCount(5),
        Action::UpdateUnreadDMCount(5)
    );
    assert_ne!(
        Action::UpdateUnreadDMCount(5),
        Action::UpdateUnreadDMCount(3)
    );
    assert_eq!(Action::OpenDMNavigation, Action::OpenDMNavigation);
    assert_eq!(Action::MarkAllDMsRead, Action::MarkAllDMsRead);
}

#[tokio::test]
async fn test_cross_conversation_workflow() {
    // This test simulates the cross-conversation notification workflow
    let mut dm_manager = DMConversationManager::new("alice".to_string());
    let mut notification_overlay = NotificationOverlay::new();
    let mut status_bar = StatusBar::new();

    // Alice is currently in DM mode with Bob
    dm_manager.set_active_conversation(Some("bob".to_string()));

    // Send a message to Bob (should not create notification)
    dm_manager
        .send_message("bob".to_string(), "Hi Bob!".to_string())
        .expect("Failed to send message");

    // Receive a message from Charlie (should create notification since not active conversation)
    dm_manager
        .receive_message("charlie".to_string(), "Hello Alice!".to_string())
        .expect("Failed to receive message");

    // Simulate adding notification (this would be done in the Home component)
    let should_notify = true; // In real code: current_dm_partner != "charlie"
    if should_notify {
        let notification = MessageNotification::new(
            "charlie".to_string(),
            "Hello Alice!".to_string(),
            "charlie".to_string(),
            Duration::from_secs(5),
        );
        notification_overlay.add_notification(notification);
    }

    // Update status bar with unread count
    let total_unread = dm_manager.get_total_unread_count();
    status_bar.set_unread_dm_count(total_unread);

    // Verify the state
    assert_eq!(dm_manager.get_total_unread_count(), 1);
    assert_eq!(status_bar.get_unread_dm_count(), 1);
    assert!(notification_overlay.is_visible());
    assert_eq!(notification_overlay.notification_count(), 1);

    // Switch to Charlie's conversation
    dm_manager.set_active_conversation(Some("charlie".to_string()));

    // Update status bar and dismiss notifications
    let total_unread_after = dm_manager.get_total_unread_count();
    status_bar.set_unread_dm_count(total_unread_after);
    notification_overlay.dismiss_all();

    // Verify cleanup
    assert_eq!(dm_manager.get_total_unread_count(), 0);
    assert_eq!(status_bar.get_unread_dm_count(), 0);
    assert!(!notification_overlay.is_visible());
}
