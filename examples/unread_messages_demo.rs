//! Demo script showcasing the unread messages enhancement functionality
//!
//! This demo simulates the unread messages workflow to demonstrate:
//! - Status bar unread count updates
//! - Cross-conversation notifications
//! - Enhanced DM navigation sorting
//! - User interaction flows

use lair_chat::{
    action::Action,
    auth::AuthState,
    chat::DMConversationManager,
    components::{MessageNotification, NotificationOverlay, StatusBar},
    transport::ConnectionStatus,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Lair Chat - Unread Messages Enhancement Demo");
    println!("================================================\n");

    // Initialize components
    let mut dm_manager = DMConversationManager::new("alice".to_string());
    let mut status_bar = StatusBar::new();
    let mut notification_overlay = NotificationOverlay::new();

    // Setup initial state
    status_bar.set_connection_status(ConnectionStatus::CONNECTED);
    status_bar.set_auth_state(AuthState::Unauthenticated);
    status_bar.set_current_room(Some("Lobby".to_string()));

    println!("📊 Initial State:");
    println!("   User: Alice");
    println!("   Active conversation: None");
    println!("   Total unread: {}", dm_manager.get_total_unread_count());
    println!(
        "   Status bar unread: {}\n",
        status_bar.get_unread_dm_count()
    );

    // === Scenario 1: Receiving messages while in lobby ===
    println!("📱 Scenario 1: Receiving DMs while in lobby");
    println!("--------------------------------------------");

    // Receive message from Bob
    println!("📥 Bob sends: 'Hey Alice, how are you?'");
    dm_manager.receive_message("bob".to_string(), "Hey Alice, how are you?".to_string())?;

    // Create notification (would be done by Home component)
    let notification = MessageNotification::new(
        "bob".to_string(),
        "Hey Alice, how are you?".to_string(),
        "bob".to_string(),
        Duration::from_secs(5),
    );
    notification_overlay.add_notification(notification);

    // Update status bar
    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());

    println!("   ✅ DM conversation created with Bob");
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   📊 Status bar shows: 💬 {} (click)",
        status_bar.get_unread_dm_count()
    );
    println!(
        "   🔔 Notification displayed: {} from {}",
        if notification_overlay.is_visible() {
            "Yes"
        } else {
            "No"
        },
        if notification_overlay.notification_count() > 0 {
            "Bob"
        } else {
            "None"
        }
    );

    // Receive another message from Bob
    println!("\n📥 Bob sends: 'Are we still meeting today?'");
    dm_manager.receive_message("bob".to_string(), "Are we still meeting today?".to_string())?;

    let notification2 = MessageNotification::new(
        "bob".to_string(),
        "Are we still meeting today?".to_string(),
        "bob".to_string(),
        Duration::from_secs(5),
    );
    notification_overlay.add_notification(notification2);
    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());

    println!(
        "   📊 Bob's unread count: {}",
        dm_manager.get_unread_count_with_user("bob")?
    );
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   🔔 Notifications: {} (anti-spam: replaced Bob's previous)",
        notification_overlay.notification_count()
    );

    // Receive message from Charlie
    println!("\n📥 Charlie sends: 'Quick question about the project'");
    dm_manager.receive_message(
        "charlie".to_string(),
        "Quick question about the project".to_string(),
    )?;

    let notification3 = MessageNotification::new(
        "charlie".to_string(),
        "Quick question about the project".to_string(),
        "charlie".to_string(),
        Duration::from_secs(5),
    );
    notification_overlay.add_notification(notification3);
    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());

    println!("   ✅ New conversation created with Charlie");
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   🔔 Notifications: {} (Bob + Charlie)",
        notification_overlay.notification_count()
    );

    // === Scenario 2: Opening DM with Bob ===
    println!("\n📱 Scenario 2: Alice opens DM conversation with Bob");
    println!("--------------------------------------------------");

    // Simulate clicking on status bar to open DM navigation
    println!("👆 Alice clicks on status bar unread count");
    let click_action = Action::OpenDMNavigation;
    println!("   🎯 Action generated: {:?}", click_action);

    // Simulate selecting Bob's conversation
    println!("📖 Alice opens conversation with Bob");
    dm_manager.set_active_conversation(Some("bob".to_string()));

    // Update status bar (Bob's messages now read)
    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());

    println!("   ✅ Bob's conversation marked as read");
    println!(
        "   📊 Bob's unread count: {}",
        dm_manager.get_unread_count_with_user("bob")?
    );
    println!(
        "   📊 Charlie's unread count: {}",
        dm_manager.get_unread_count_with_user("charlie")?
    );
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   📊 Status bar shows: 💬 {} (click)",
        status_bar.get_unread_dm_count()
    );

    // === Scenario 3: Cross-conversation notification ===
    println!("\n📱 Scenario 3: Receiving message while in Bob's DM");
    println!("-------------------------------------------------");

    // Receive message from Diana while chatting with Bob
    println!("📥 Diana sends: 'Emergency! Need help with server'");
    dm_manager.receive_message(
        "diana".to_string(),
        "Emergency! Need help with server".to_string(),
    )?;

    // This should create notification since Alice is chatting with Bob, not Diana
    let current_partner = dm_manager.get_active_conversation_partner();
    let should_notify = current_partner.as_ref() != Some(&"diana".to_string());

    if should_notify {
        let notification4 = MessageNotification::new(
            "diana".to_string(),
            "Emergency! Need help with server".to_string(),
            "diana".to_string(),
            Duration::from_secs(5),
        );
        notification_overlay.add_notification(notification4);
        println!("   🔔 Cross-conversation notification created!");
    }

    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());

    println!(
        "   📊 Active conversation: {}",
        current_partner.as_deref().unwrap_or("None")
    );
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   🔔 Should notify: {} (different from active conversation)",
        should_notify
    );

    // === Scenario 4: Enhanced DM navigation sorting ===
    println!("\n📱 Scenario 4: DM Navigation Enhanced Sorting");
    println!("---------------------------------------------");

    // Show conversation summary with sorting priority
    let conversations = vec![
        ("bob", dm_manager.get_unread_count_with_user("bob")?),
        ("charlie", dm_manager.get_unread_count_with_user("charlie")?),
        ("diana", dm_manager.get_unread_count_with_user("diana")?),
    ];

    println!("📋 Conversation sorting (unread first, then by count):");
    let mut sorted_conversations = conversations.clone();
    sorted_conversations.sort_by(|(_, a_unread), (_, b_unread)| {
        // Unread conversations first
        let unread_cmp = (b_unread > &0).cmp(&(a_unread > &0));
        if unread_cmp != std::cmp::Ordering::Equal {
            return unread_cmp;
        }
        // Within unread, sort by count (highest first)
        b_unread.cmp(a_unread)
    });

    for (i, (name, unread)) in sorted_conversations.iter().enumerate() {
        let indicator = if *unread > 10 {
            "●● (RED)"
        } else if *unread > 3 {
            "● (MAGENTA)"
        } else if *unread > 0 {
            "● (GREEN)"
        } else {
            "  (GRAY)"
        };

        let new_badge = if *unread > 0 { "NEW " } else { "" };

        println!(
            "   {}. {} {}{} ({})",
            i + 1,
            indicator,
            new_badge,
            name,
            unread
        );
    }

    // === Scenario 5: Mark all as read ===
    println!("\n📱 Scenario 5: Mark all conversations as read");
    println!("---------------------------------------------");

    println!("🧹 Alice uses 'Mark All Read' functionality");
    dm_manager.mark_all_read();
    status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());
    notification_overlay.dismiss_all();

    let mark_all_action = Action::MarkAllDMsRead;
    println!("   🎯 Action generated: {:?}", mark_all_action);
    println!("   ✅ All conversations marked as read");
    println!(
        "   📊 Total unread: {}",
        dm_manager.get_total_unread_count()
    );
    println!(
        "   📊 Status bar shows: {} unread",
        status_bar.get_unread_dm_count()
    );
    println!(
        "   🔔 Notifications: {} (all dismissed)",
        notification_overlay.notification_count()
    );

    // === Summary ===
    println!("\n🎉 Demo Complete - Enhancement Features Demonstrated:");
    println!("=====================================================");
    println!("✅ Global status bar unread count with click support");
    println!("✅ Cross-conversation notifications with smart logic");
    println!("✅ Enhanced DM navigation sorting and visual indicators");
    println!("✅ Real-time unread count updates");
    println!("✅ Anti-spam notification handling");
    println!("✅ Mark all as read functionality");
    println!("✅ Context-aware notification system");

    println!("\n📈 User Experience Improvements:");
    println!("• Always visible unread count in status bar");
    println!("• No missed messages from other conversations");
    println!("• Priority-based conversation sorting");
    println!("• Interactive status bar elements");
    println!("• Automatic notification cleanup");

    println!("\n🔧 Technical Features:");
    println!("• Action-based communication between components");
    println!("• Event-driven unread count updates");
    println!("• Mouse event handling in status bar");
    println!("• Modular notification overlay system");
    println!("• Efficient sorting algorithms");

    Ok(())
}

#[cfg(test)]
mod demo_tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_scenario_simulation() {
        // Test the core workflow from the demo
        let mut dm_manager = DMConversationManager::new("alice".to_string());
        let mut status_bar = StatusBar::new();

        // Simulate receiving messages
        dm_manager
            .receive_message("bob".to_string(), "Hello".to_string())
            .unwrap();
        dm_manager
            .receive_message("charlie".to_string(), "Hi".to_string())
            .unwrap();

        // Check unread counts
        assert_eq!(dm_manager.get_total_unread_count(), 2);

        // Set active conversation
        dm_manager.set_active_conversation(Some("bob".to_string()));
        assert_eq!(dm_manager.get_total_unread_count(), 1); // Only Charlie's message unread

        // Update status bar
        status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());
        assert_eq!(status_bar.get_unread_dm_count(), 1);

        // Mark all as read
        dm_manager.mark_all_read();
        status_bar.set_unread_dm_count(dm_manager.get_total_unread_count());
        assert_eq!(status_bar.get_unread_dm_count(), 0);
    }

    #[test]
    fn test_sorting_priority() {
        // Test the enhanced sorting logic
        let conversations = vec![
            ("alice", 0),    // No unread
            ("bob", 5),      // Medium unread
            ("charlie", 15), // High unread
            ("diana", 1),    // Low unread
        ];

        let mut sorted = conversations.clone();
        sorted.sort_by(|(_, a_unread), (_, b_unread)| {
            let unread_cmp = (b_unread > &0).cmp(&(a_unread > &0));
            if unread_cmp != std::cmp::Ordering::Equal {
                return unread_cmp;
            }
            b_unread.cmp(a_unread)
        });

        // Should be: charlie(15), bob(5), diana(1), alice(0)
        assert_eq!(sorted[0].0, "charlie");
        assert_eq!(sorted[1].0, "bob");
        assert_eq!(sorted[2].0, "diana");
        assert_eq!(sorted[3].0, "alice");
    }
}
