use lair_chat::client::{action::Action, message_router::ClientMessageRouter};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    println!("🧪 Testing invitation message parsing...");

    // Create a channel for actions
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();

    // Create message router
    let mut router = ClientMessageRouter::new(action_tx.clone());
    router.set_current_user(Some("bob".to_string()));
    router.set_debug_enabled(true);

    // Test cases
    let test_cases = vec![
        (
            "SYSTEM_MESSAGE:alice invited you to join room 'testroom'",
            "alice",
            "testroom",
            true,
        ),
        (
            "SYSTEM_MESSAGE:charlie invited you to join room 'gaming'",
            "charlie",
            "gaming",
            true,
        ),
        (
            "SYSTEM_MESSAGE:dave invited you to join room 'work-chat'",
            "dave",
            "work-chat",
            true,
        ),
        (
            "SYSTEM_MESSAGE:eve invited you to join room ''",
            "eve",
            "",
            true, // Edge case - empty room name
        ),
        (
            "SYSTEM_MESSAGE:frank invited you to join room",
            "",
            "",
            false, // Invalid format - missing room quote
        ),
        (
            "SYSTEM_MESSAGE:some other message",
            "",
            "",
            false, // Not an invitation
        ),
    ];

    for (i, (message, expected_from, expected_room, should_work)) in test_cases.iter().enumerate() {
        println!("\n🔍 Test Case {}: {}", i + 1, message);

        // Parse the message
        match router.parse_and_route_protocol_message(message) {
            Ok(()) => {
                println!("   ✅ Message parsed successfully");

                // Check if we received an InvitationReceived action
                if let Ok(action) = action_rx.try_recv() {
                    match action {
                        Action::InvitationReceived(from, room, full_message) => {
                            if *should_work {
                                println!("   🎉 SUCCESS: InvitationReceived action created!");
                                println!("      From: '{}'", from);
                                println!("      Room: '{}'", room);
                                println!("      Full message: '{}'", full_message);

                                assert_eq!(from, *expected_from, "From mismatch in test {}", i + 1);
                                assert_eq!(room, *expected_room, "Room mismatch in test {}", i + 1);
                                println!("   ✅ All assertions passed for test {}", i + 1);
                            } else {
                                println!(
                                    "   ❌ FAIL: Should not have created InvitationReceived action"
                                );
                                panic!("Test {} failed: unexpected invitation action", i + 1);
                            }
                        }
                        other => {
                            if *should_work {
                                println!(
                                    "   ❌ FAIL: Expected InvitationReceived but got: {:?}",
                                    other
                                );
                                panic!("Test {} failed: wrong action type", i + 1);
                            } else {
                                println!("   ✅ Got other action as expected: {:?}", other);
                            }
                        }
                    }
                } else {
                    if *should_work {
                        println!("   ❌ FAIL: No action received");
                        panic!("Test {} failed: no action received", i + 1);
                    } else {
                        println!("   ✅ No action received as expected");
                    }
                }
            }
            Err(e) => {
                if *should_work {
                    println!("   ❌ FAIL: Parse error: {}", e);
                    panic!("Test {} failed: parse error", i + 1);
                } else {
                    println!("   ✅ Parse error as expected: {}", e);
                }
            }
        }
    }

    println!("\n🎉 All invitation parsing tests passed!");

    // Now test the full flow - simulate what happens when invitation is processed
    println!("\n🔄 Testing full invitation flow...");

    let invitation_message = "SYSTEM_MESSAGE:alice invited you to join room 'testroom'";
    router.parse_and_route_protocol_message(invitation_message)?;

    // Should get InvitationReceived action
    if let Ok(Action::InvitationReceived(from, room, message)) = action_rx.try_recv() {
        println!("✅ Step 1: InvitationReceived action created");
        println!("   From: {}", from);
        println!("   Room: {}", room);
        println!("   Message: {}", message);

        // Simulate what the App does when it receives this action
        println!("✅ Step 2: App would now create DisplayMessage actions");
        println!("   - Display: '🔔 INVITATION: {}'", message);
        println!(
            "   - Instructions: 'To respond: /accept {} or /decline {}'",
            room, room
        );
        println!(
            "   - Alternatives: 'You can also use /join {} to accept'",
            room
        );
    } else {
        panic!("❌ Failed to receive InvitationReceived action");
    }

    println!("\n🎊 All tests completed successfully!");
    println!("💡 If invitations aren't showing in the UI, the issue is likely:");
    println!("   1. DisplayMessage actions not being processed correctly");
    println!("   2. Home component not displaying system messages");
    println!("   3. UI rendering issues");

    Ok(())
}
