//! Password Verification Test
//!
//! This script tests password verification against the stored hash
//! to debug authentication issues.

use tokio;

use lair_chat::server::{
    config::ServerConfig,
    storage::{DatabaseConfig, StorageManager},
};

use argon2::{password_hash::PasswordVerifier, Argon2, PasswordHash};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Password Verification Test");
    println!("============================");

    // Load server configuration
    let config = ServerConfig::default();
    let db_config = DatabaseConfig::from(config.database.clone());

    // Initialize storage manager
    let storage = StorageManager::new(db_config).await?;

    // Get admin user
    match storage.users().get_user_by_username("admin").await {
        Ok(Some(user)) => {
            println!("👤 Found user: {}", user.username);
            println!("🔑 Stored hash: {}", user.password_hash);
            println!("👥 User role: {:?}", user.role);
            println!("✅ User active: {}", user.is_active);

            // Test password verification
            let test_password = "AdminPassword123!";
            println!("🧪 Testing password: {}", test_password);

            match verify_password(test_password, &user.password_hash) {
                Ok(true) => {
                    println!("✅ Password verification: SUCCESS");
                }
                Ok(false) => {
                    println!("❌ Password verification: FAILED (incorrect password)");
                }
                Err(e) => {
                    println!("💥 Password verification: ERROR - {}", e);
                }
            }

            // Also test a wrong password
            let wrong_password = "wrongpassword";
            println!("🧪 Testing wrong password: {}", wrong_password);
            match verify_password(wrong_password, &user.password_hash) {
                Ok(true) => {
                    println!("⚠️  Wrong password verification: SUCCESS (this is bad!)");
                }
                Ok(false) => {
                    println!("✅ Wrong password verification: FAILED (this is correct)");
                }
                Err(e) => {
                    println!("💥 Wrong password verification: ERROR - {}", e);
                }
            }

            // Test the hash format
            println!("🔍 Hash analysis:");
            if user.password_hash.starts_with("$argon2") {
                println!("  ✅ Hash format: Argon2 (correct)");
            } else if user.password_hash.starts_with("$2b$") {
                println!("  ❌ Hash format: bcrypt (incorrect - should be Argon2)");
            } else {
                println!("  ❓ Hash format: Unknown");
            }
        }
        Ok(None) => {
            println!("❌ Admin user not found in database");
        }
        Err(e) => {
            println!("💥 Database error: {}", e);
        }
    }

    Ok(())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let parsed_hash = PasswordHash::new(hash)?;

    let result = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(result)
}
