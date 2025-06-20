use lair_chat::client::message_router::ClientMessageRouter;
use lair_chat::common::messaging::SystemMessage;
use std::sync::mpsc;

#[tokio::test]
async fn test_invitation_message_parsing() {
    // Create a message router for testing
    let (action_sender, action_receiver) = mpsc::channel();
    let mut router = ClientMessageRouter::new(action_sender);
    router.set_current_user("bob".to_string());
    router.set_debug_enabled(true);

    println!("🔬 Testing invitation message parsing...");

    // Test 1: Basic invitation message from server
    let invitation_message = "SYSTEM_MESSAGE:alice invited you to join room 'testroom'";
    println!("🔍 Testing message: '{}'", invitation_message);

    let result = router.parse_and_route_protocol_message(invitation_message);
    println!("🔍 Parse result: {:?}", result);

    // Check if an action was sent
    match action_receiver.try_recv() {
        Ok(action) => {
            println!("✅ Action received: {:?}", action);
            match action {
                lair_chat::client::action::Action::InvitationReceived(from, room, msg) => {
                    assert_eq!(from, "alice");
                    assert_eq!(room, "testroom");
                    println!("✅ Invitation parsed correctly!");
                    println!("   From: {}", from);
                    println!("   Room: {}", room);
                    println!("   Message: {}", msg);
                }
                _ => {
                    panic!("❌ Wrong action type received: {:?}", action);
                }
            }
        }
        Err(e) => {
            panic!("❌ No action received from invitation parsing: {:?}", e);
        }
    }

    // Test 2: Test confirmation message to sender
    let confirmation_message = "SYSTEM_MESSAGE:You invited bob to join room 'testroom'";
    println!(
        "\n🔍 Testing confirmation message: '{}'",
        confirmation_message
    );

    let result2 = router.parse_and_route_protocol_message(confirmation_message);
    println!("🔍 Parse result: {:?}", result2);

    // Check if an action was sent
    match action_receiver.try_recv() {
        Ok(action) => {
            println!("✅ Confirmation action received: {:?}", action);
        }
        Err(e) => {
            println!("⚠️  No confirmation action received: {:?}", e);
        }
    }

    // Test 3: Test malformed invitation message
    let malformed_message = "SYSTEM_MESSAGE:alice invited you to room testroom";
    println!("\n🔍 Testing malformed message: '{}'", malformed_message);

    let result3 = router.parse_and_route_protocol_message(malformed_message);
    println!("🔍 Parse result: {:?}", result3);

    // This should not produce an invitation action
    match action_receiver.try_recv() {
        Ok(action) => {
            println!("📝 Action received for malformed message: {:?}", action);
        }
        Err(_) => {
            println!("✅ No action received for malformed message (expected)");
        }
    }

    println!("\n🔬 Invitation parsing test completed!");
}

#[tokio::test]
async fn test_invitation_pattern_matching() {
    println!("🔬 Testing invitation pattern matching...");

    let test_cases = vec![
        (
            "alice invited you to join room 'testroom'",
            true,
            "alice",
            "testroom",
        ),
        ("bob invited you to join room 'lobby'", true, "bob", "lobby"),
        (
            "user123 invited you to join room 'my-room'",
            true,
            "user123",
            "my-room",
        ),
        ("alice invited you to room 'testroom'", false, "", ""), // Missing "join"
        ("alice invited you to join room testroom", false, "", ""), // Missing quotes
        ("You invited bob to join room 'testroom'", false, "", ""), // Wrong direction
    ];

    for (content, should_match, expected_from, expected_room) in test_cases {
        println!("\n🔍 Testing pattern: '{}'", content);

        let matches_pattern = content.contains(" invited you to join room '");
        println!(
            "   Pattern match: {} (expected: {})",
            matches_pattern, should_match
        );

        if matches_pattern {
            // Test the parsing logic
            if let Some(inviter_end) = content.find(" invited you to join room '") {
                let inviter = &content[..inviter_end];
                let rest = &content[inviter_end + 30..]; // Skip " invited you to join room '"
                if let Some(room_end) = rest.find("'") {
                    let room_name = &rest[..room_end];
                    println!("   Parsed - From: '{}', Room: '{}'", inviter, room_name);

                    if should_match {
                        assert_eq!(inviter, expected_from, "Inviter mismatch");
                        assert_eq!(room_name, expected_room, "Room name mismatch");
                        println!("   ✅ Parsing correct!");
                    }
                } else {
                    println!("   ❌ Failed to find closing quote");
                    assert!(!should_match, "Should have found closing quote");
                }
            }
        } else {
            assert!(!should_match, "Pattern should not have matched");
            println!("   ✅ Correctly rejected");
        }
    }

    println!("\n🔬 Pattern matching test completed!");
}

#[tokio::test]
async fn test_system_message_routing() {
    println!("🔬 Testing system message routing...");

    let (action_sender, action_receiver) = mpsc::channel();
    let mut router = ClientMessageRouter::new(action_sender);
    router.set_current_user("testuser".to_string());

    // Test various system messages
    let test_messages = vec![
        "SYSTEM_MESSAGE:Welcome to the server",
        "SYSTEM_MESSAGE:ERROR: User not found",
        "SYSTEM_MESSAGE:DM sent to bob: hello",
        "SYSTEM_MESSAGE:alice invited you to join room 'testroom'",
        "SYSTEM_MESSAGE:You invited bob to join room 'testroom'",
    ];

    for message in test_messages {
        println!("\n🔍 Processing: '{}'", message);
        let result = router.parse_and_route_protocol_message(message);
        println!("   Result: {:?}", result);

        // Try to receive any actions
        while let Ok(action) = action_receiver.try_recv() {
            println!("   Action: {:?}", action);
        }
    }

    println!("\n🔬 System message routing test completed!");
}

fn main() {
    println!("🚀 Running invitation parsing tests...");

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        test_invitation_message_parsing().await;
        test_invitation_pattern_matching().await;
        test_system_message_routing().await;
    });

    println!("✅ All tests completed!");
}
