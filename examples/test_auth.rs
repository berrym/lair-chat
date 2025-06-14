//! Quick authentication test to verify the connection manager fixes
//! This tests the client-server authentication protocol
//! Run with: cargo run --example test_auth

use lair_chat::client::{ConnectionManager, Credentials};
use lair_chat::common::transport::{ConnectionConfig, ConnectionStatus, TcpTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔧 Testing ConnectionManager authentication...");

    // Create connection config
    let config = ConnectionConfig {
        address: "127.0.0.1:8080".parse()?,
        timeout_ms: 5000,
    };

    // Create connection manager
    let mut connection_manager = ConnectionManager::new(config.clone());

    // Add transport
    let transport = Box::new(TcpTransport::new(config));
    connection_manager.with_transport(transport);

    // Add secure AES-GCM encryption
    let encryption = lair_chat::common::crypto::create_aes_gcm_encryption_with_random_key();
    connection_manager.with_encryption(encryption);

    println!("📡 Connecting to server...");

    // Connect to server
    match connection_manager.connect().await {
        Ok(()) => {
            println!("✅ Connected successfully!");

            // Check connection status
            let status = connection_manager.get_status().await;
            println!("📊 Connection status: {:?}", status);

            if status == ConnectionStatus::CONNECTED {
                println!("🔐 Testing authentication...");

                // Test credentials (using default test user)
                let credentials = Credentials {
                    username: "alice".to_string(),
                    password: "password123".to_string(),
                };

                // Attempt login
                match connection_manager.login(credentials).await {
                    Ok(()) => {
                        println!("✅ Authentication successful!");

                        // Check if authenticated
                        if connection_manager.is_authenticated().await {
                            println!("🎉 Login verified - user is authenticated!");

                            // Test sending a message
                            println!("📤 Testing message sending...");
                            match connection_manager
                                .send_message("Hello from ConnectionManager test!".to_string())
                                .await
                            {
                                Ok(()) => println!("✅ Message sent successfully!"),
                                Err(e) => println!("❌ Message send failed: {}", e),
                            }
                        } else {
                            println!(
                                "⚠️ Authentication succeeded but user not marked as authenticated"
                            );
                        }
                    }
                    Err(e) => {
                        println!("❌ Authentication failed: {}", e);
                        return Err(e.into());
                    }
                }
            } else {
                println!("❌ Connection status is not CONNECTED: {:?}", status);
            }
        }
        Err(e) => {
            println!("❌ Connection failed: {}", e);
            return Err(e.into());
        }
    }

    // Test registration with new user
    println!("\n🆕 Testing user registration...");

    let new_credentials = Credentials {
        username: format!(
            "testuser_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        password: "newpassword123".to_string(),
    };

    match connection_manager.register(new_credentials.clone()).await {
        Ok(()) => {
            println!(
                "✅ Registration successful for user: {}",
                new_credentials.username
            );

            if connection_manager.is_authenticated().await {
                println!("🎉 Auto-login after registration successful!");
            } else {
                println!("⚠️ Registration succeeded but auto-login failed");
            }
        }
        Err(e) => {
            println!("❌ Registration failed: {}", e);
        }
    }

    // Disconnect
    println!("\n🔌 Disconnecting...");
    match connection_manager.disconnect().await {
        Ok(()) => println!("✅ Disconnected successfully!"),
        Err(e) => println!("⚠️ Disconnect error: {}", e),
    }

    println!("\n🎯 Test completed!");
    Ok(())
}
