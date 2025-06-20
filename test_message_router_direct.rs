use lair_chat::action::Action;
use lair_chat::client::message_router::ClientMessageRouter;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 DIRECT MESSAGE ROUTER TEST");
    println!("============================");

    // Create message router
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let mut router = ClientMessageRouter::new(action_tx);
    router.set_current_user(Some("bob".to_string()));
    router.set_debug_enabled(true);

    println!("✅ Message router created for user 'bob'");

    // Test messages that Bob should receive
    let test_messages = vec![
        ("ROOM_LIST:testroom", "Room list with testroom"),
        (
            "SYSTEM_MESSAGE:alice invited you to join room 'testroom'",
            "Invitation from alice",
        ),
        ("SYSTEM_MESSAGE:Welcome to the server", "Welcome message"),
        ("PRIVATE_MESSAGE:alice:Hello Bob!", "DM from alice"),
        ("alice: hello world", "Regular chat from alice"),
    ];

    for (message, description) in test_messages {
        println!("\n🔍 Testing: {}", description);
        println!("   Message: '{}'", message);

        // Process the message
        match router.parse_and_route_protocol_message(message) {
            Ok(()) => {
                println!("   ✅ Message processed successfully");

                // Check for actions
                let mut action_count = 0;
                while let Ok(action) = action_rx.try_recv() {
                    action_count += 1;
                    match action {
                        Action::DisplayMessage { content, is_system } => {
                            println!(
                                "   📄 DisplayMessage: '{}' (system: {})",
                                content, is_system
                            );
                        }
                        Action::InvitationReceived(from, room, _) => {
                            println!(
                                "   🔔 InvitationReceived: from '{}' to room '{}'",
                                from, room
                            );
                        }
                        _ => {
                            println!("   📦 Other action: {:?}", action);
                        }
                    }
                }

                if action_count == 0 {
                    println!("   ⚠️  No actions generated");
                }
            }
            Err(e) => {
                println!("   ❌ Error processing message: {}", e);
            }
        }
    }

    println!("\n🎯 SUMMARY:");
    println!("============");
    println!("If you see:");
    println!("- ✅ DisplayMessage actions for invitations → Message router works");
    println!("- ⚠️ No actions for invitations → Message router bug");
    println!("- ❌ Error processing messages → Parsing bug");

    Ok(())
}
