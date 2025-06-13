use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    println!("🔍 DM Test - Identifying the issue with Direct Messaging");
    println!("=========================================================");

    // Start server
    println!("1. 🖥️  Starting server...");
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "lair-chat-server"])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    // Wait for server to start
    thread::sleep(Duration::from_secs(3));

    // Start first client (Alice)
    println!("2. 👤 Starting Alice client...");
    let mut alice = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "lair-chat-client",
            "--",
            "--username",
            "alice",
        ])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start Alice client");

    // Wait for Alice to connect
    thread::sleep(Duration::from_secs(3));

    // Start second client (Bob)
    println!("3. 👤 Starting Bob client...");
    let mut bob = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "lair-chat-client",
            "--",
            "--username",
            "bob",
        ])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start Bob client");

    // Wait for Bob to connect
    thread::sleep(Duration::from_secs(3));

    println!("4. 🔍 Monitoring logs for DM activity...");
    println!("   (In a real test, you would manually:");
    println!("   - Open Alice's client and press Ctrl+L, then 'n'");
    println!("   - Select 'bob' from the user list");
    println!("   - Type a message and press Enter");
    println!("   - Check Bob's client to see if the message appears)");
    println!();

    // Monitor server output
    if let Some(server_stdout) = server.stdout.take() {
        let reader = BufReader::new(server_stdout);
        thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("DM") || line.contains("direct") {
                        println!("📡 SERVER: {}", line);
                    }
                    if line.contains("DEBUG")
                        && (line.contains("send_to_user") || line.contains("DM_FROM"))
                    {
                        println!("🔧 SERVER DEBUG: {}", line);
                    }
                }
            }
        });
    }

    // Monitor Alice output
    if let Some(alice_stdout) = alice.stdout.take() {
        let reader = BufReader::new(alice_stdout);
        thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("🔥")
                        || line.contains("📤")
                        || line.contains("🚀")
                        || line.contains("📡")
                    {
                        println!("👤 ALICE SEND: {}", line);
                    }
                }
            }
        });
    }

    // Monitor Bob output
    if let Some(bob_stdout) = bob.stdout.take() {
        let reader = BufReader::new(bob_stdout);
        thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("📨")
                        || line.contains("📥")
                        || line.contains("💬")
                        || line.contains("✅")
                    {
                        println!("👤 BOB RECV: {}", line);
                    }
                }
            }
        });
    }

    println!("5. ⏱️  Waiting for 30 seconds to observe behavior...");
    println!("   Press Ctrl+C to stop all processes");

    // Wait and then clean up
    thread::sleep(Duration::from_secs(30));

    println!("\n6. 🧹 Cleaning up processes...");
    let _ = server.kill();
    let _ = alice.kill();
    let _ = bob.kill();

    println!("✅ Test complete!");
    println!("\nExpected behavior:");
    println!("- When Alice sends a DM to Bob, you should see:");
    println!("  🔥 ALICE SEND: handle_dm_message_send called");
    println!("  📤 ALICE SEND: Parsed DM - partner: 'bob'");
    println!("  🚀 ALICE SEND: About to send DM to server");
    println!("  📡 SERVER: DEBUG: Received message from alice: 'DM:bob:message'");
    println!("  📡 SERVER: DEBUG: Processing DM message");
    println!("  📡 SERVER: DEBUG: send_to_user called - target: 'bob'");
    println!("  📨 BOB RECV: Received DM_FROM message");
    println!("  💬 BOB RECV: Received direct message from alice");
    println!("\nIf any of these steps are missing, that's where the issue is!");

    println!("\nCommon issues to check:");
    println!("1. 🔍 DM not being sent by Alice client");
    println!("2. 🔍 Server not receiving/processing DM");
    println!("3. 🔍 Server not finding Bob in user list");
    println!("4. 🔍 Server failing to send to Bob");
    println!("5. 🔍 Bob not receiving/processing DM_FROM message");
    println!("6. 🔍 Bob receiving message but not displaying it (if not in DM mode with Alice)");
}
