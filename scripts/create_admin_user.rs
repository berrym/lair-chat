//! Admin User Creation Utility
//!
//! This script creates admin users for testing the admin dashboard
//! and verifying JWT authentication with admin privileges.
//!
//! Usage: cargo run --bin create_admin_user

use std::env;
use tokio;
use uuid::Uuid;

use lair_chat::server::{
    config::ServerConfig,
    storage::{
        models::{User, UserProfile, UserRole, UserSettings},
        DatabaseConfig, StorageManager,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔧 Lair Chat Admin User Creation Utility");
    println!("========================================");

    // Get arguments
    let args: Vec<String> = env::args().collect();

    let (username, password, email) = if args.len() >= 4 {
        (args[1].clone(), args[2].clone(), Some(args[3].clone()))
    } else {
        // Default admin user for testing
        println!("ℹ️  No arguments provided, creating default admin user");
        (
            "admin".to_string(),
            "AdminPassword123!".to_string(),
            Some("admin@example.com".to_string()),
        )
    };

    println!("📝 Creating admin user:");
    println!("   Username: {}", username);
    println!("   Email: {}", email.as_ref().unwrap_or(&"N/A".to_string()));
    println!("   Role: Admin");

    // Load server configuration
    let config = ServerConfig::default();

    // Initialize storage
    let db_config = DatabaseConfig::from(config.database.clone());
    let storage = StorageManager::new(db_config).await?;

    // Check if user already exists
    if let Ok(Some(_)) = storage.users().get_user_by_username(&username).await {
        println!("❌ User '{}' already exists!", username);

        // Ask if user wants to update to admin role
        println!("🔄 Would you like to update this user to admin role? (y/N)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            update_user_to_admin(&storage, &username).await?;
            return Ok(());
        } else {
            println!("⏹️  Operation cancelled");
            return Ok(());
        }
    }

    // Hash password
    let password_hash = hash_password(&password)?;

    // Create admin user
    let user_id = Uuid::new_v4();
    let now_timestamp = chrono::Utc::now().timestamp() as u64;

    let admin_user = User {
        id: user_id.to_string(),
        username: username.clone(),
        email,
        password_hash,
        salt: "".to_string(), // Salt is included in the hash
        created_at: now_timestamp,
        updated_at: now_timestamp,
        last_seen: None,
        is_active: true,
        role: UserRole::Admin,
        profile: UserProfile {
            display_name: Some(format!("Administrator {}", username)),
            avatar: None,
            status_message: Some("System Administrator".to_string()),
            bio: Some("Administrative account for system management".to_string()),
            timezone: Some("UTC".to_string()),
            language: Some("en".to_string()),
            custom_fields: std::collections::HashMap::new(),
        },
        settings: UserSettings::default(),
    };

    // Store user in database
    match storage.users().create_user(admin_user.clone()).await {
        Ok(created_user) => {
            println!("✅ Admin user created successfully!");
            println!("   User ID: {}", created_user.id);
            println!("   Username: {}", created_user.username);
            println!(
                "   Email: {}",
                created_user.email.unwrap_or_else(|| "N/A".to_string())
            );
            println!("   Role: {:?}", created_user.role);
            println!(
                "   Created: {}",
                chrono::DateTime::from_timestamp(created_user.created_at as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
        Err(e) => {
            println!("❌ Failed to create admin user: {}", e);
            return Err(e.into());
        }
    }

    println!("\n🎉 Admin user setup complete!");
    println!("📋 You can now use these credentials to log into the admin dashboard:");
    println!("   URL: http://127.0.0.1:8082/admin-dashboard/");
    println!("   Username: {}", username);
    println!("   Password: {}", password);

    println!("\n🚀 Next steps:");
    println!("   1. Start the server: cargo run --bin server");
    println!("   2. Open the admin dashboard in your browser");
    println!("   3. Login with the credentials above");

    Ok(())
}

async fn update_user_to_admin(
    storage: &StorageManager,
    username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Updating user '{}' to admin role...", username);

    // Get existing user
    let mut user = storage
        .users()
        .get_user_by_username(username)
        .await?
        .ok_or("User not found")?;

    // Update role to admin
    user.role = UserRole::Admin;
    user.updated_at = chrono::Utc::now().timestamp() as u64;

    // Update profile
    if user.profile.display_name.is_none() {
        user.profile.display_name = Some(format!("Administrator {}", username));
    }
    if user.profile.status_message.is_none() {
        user.profile.status_message = Some("System Administrator".to_string());
    }

    // Save updated user
    match storage.users().update_user(user.clone()).await {
        Ok(_) => {
            println!("✅ User '{}' successfully updated to admin role!", username);
            println!("   User ID: {}", user.id);
            println!("   Role: {:?}", user.role);
        }
        Err(e) => {
            println!("❌ Failed to update user role: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

fn hash_password(password: &str) -> Result<String, Box<dyn std::error::Error>> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))?;

    Ok(password_hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "TestPassword123!";
        let hash = hash_password(password).unwrap();

        assert_ne!(hash, password);
        assert!(hash.len() > 50); // Argon2 hashes are quite long
    }
}
