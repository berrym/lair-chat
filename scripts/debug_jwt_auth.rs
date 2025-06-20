//! JWT Authentication Debug Script
//!
//! This script helps debug JWT authentication issues by:
//! 1. Creating test users with different roles
//! 2. Generating JWT tokens manually
//! 3. Testing token validation
//! 4. Verifying middleware behavior
//!
//! Usage: cargo run --bin debug_jwt_auth

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::collections::HashMap;
use tokio;
use uuid::Uuid;

use lair_chat::server::{
    api::models::auth::{JwtClaims, TokenType, UserRole},
    config::ServerConfig,
    storage::{
        models::{User, UserProfile, UserRole as StorageUserRole, UserSettings},
        DatabaseConfig, StorageManager,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 JWT Authentication Debug Script");
    println!("=================================");

    // Load configuration
    let config = ServerConfig::default();

    // Initialize storage
    let db_config = DatabaseConfig::from(config.database.clone());
    let storage = StorageManager::new(db_config).await?;

    // Test JWT secret
    let jwt_secret = "test_jwt_secret_for_debugging_purposes_make_it_long";
    println!("🔑 Using JWT secret: {}", jwt_secret);

    // Create test users with different roles
    println!("\n📝 Creating test users...");
    let test_users = create_test_users(&storage).await?;

    // Test JWT token generation and validation
    println!("\n🧪 Testing JWT token generation...");
    test_jwt_operations(&test_users, jwt_secret).await?;

    // Test database operations
    println!("\n💾 Testing database operations...");
    test_database_operations(&storage, &test_users).await?;

    // Test role conversion
    println!("\n🔄 Testing role conversion...");
    test_role_conversion();

    // Test middleware simulation
    println!("\n🛡️  Simulating middleware behavior...");
    simulate_middleware_behavior(&storage, jwt_secret).await?;

    println!("\n✅ JWT Authentication debug complete!");
    Ok(())
}

async fn create_test_users(
    storage: &StorageManager,
) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    let mut users = Vec::new();
    let now_timestamp = Utc::now().timestamp() as u64;

    // Test users with different roles
    let test_data = vec![
        ("test_admin", "TestAdmin123!", UserRole::Admin),
        ("test_moderator", "TestMod123!", UserRole::Moderator),
        ("test_user", "TestUser123!", UserRole::User),
        ("test_guest", "TestGuest123!", UserRole::Guest),
    ];

    for (username, password, role) in test_data {
        let user_id = Uuid::new_v4();
        let password_hash = hash_password(password)?;

        let user = User {
            id: user_id.to_string(),
            username: username.to_string(),
            email: Some(format!("{}@test.com", username)),
            password_hash,
            salt: "".to_string(),
            created_at: now_timestamp,
            updated_at: now_timestamp,
            last_seen: None,
            is_active: true,
            role: convert_to_storage_role(&role),
            profile: UserProfile {
                display_name: Some(format!("Test {}", username)),
                avatar: None,
                status_message: Some(format!("Test user with {:?} role", role)),
                bio: None,
                timezone: Some("UTC".to_string()),
                language: Some("en".to_string()),
                custom_fields: HashMap::new(),
            },
            settings: UserSettings::default(),
        };

        // Try to create user, skip if already exists
        match storage.users().create_user(user.clone()).await {
            Ok(created_user) => {
                println!("✅ Created user: {} ({})", username, role);
                users.push(created_user);
            }
            Err(e) => {
                println!("ℹ️  User {} already exists, fetching: {}", username, e);
                if let Ok(Some(existing_user)) =
                    storage.users().get_user_by_username(username).await
                {
                    users.push(existing_user);
                }
            }
        }
    }

    Ok(users)
}

async fn test_jwt_operations(
    users: &[User],
    jwt_secret: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for user in users {
        println!(
            "\n🔐 Testing JWT for user: {} (role: {:?})",
            user.username, user.role
        );

        let user_id = Uuid::parse_str(&user.id)?;
        let session_id = Uuid::new_v4();
        let api_role = convert_from_storage_role(&user.role);

        // Generate access token
        let token = generate_access_token(
            user_id,
            &user.username,
            api_role.clone(),
            session_id,
            jwt_secret,
        )?;
        println!("   📜 Generated token: {}...", &token[..50]);

        // Validate token
        let validation = Validation::new(Algorithm::HS256);
        match decode::<JwtClaims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => {
                println!("   ✅ Token validation successful");
                println!("      User ID: {}", token_data.claims.sub);
                println!("      Role: {:?}", token_data.claims.role);
                println!("      Session ID: {}", token_data.claims.session_id);
                println!(
                    "      Expires: {}",
                    chrono::DateTime::from_timestamp(token_data.claims.exp, 0).unwrap_or_default()
                );

                // Check role matching
                if token_data.claims.role == api_role {
                    println!("   ✅ Role matches expected: {:?}", api_role);
                } else {
                    println!(
                        "   ❌ Role mismatch! Expected: {:?}, Got: {:?}",
                        api_role, token_data.claims.role
                    );
                }
            }
            Err(e) => {
                println!("   ❌ Token validation failed: {}", e);
            }
        }
    }

    Ok(())
}

async fn test_database_operations(
    storage: &StorageManager,
    users: &[User],
) -> Result<(), Box<dyn std::error::Error>> {
    for user in users {
        println!("🗄️  Testing database operations for: {}", user.username);

        // Test user retrieval by username
        match storage.users().get_user_by_username(&user.username).await {
            Ok(Some(db_user)) => {
                println!("   ✅ Retrieved user by username");
                println!("      ID: {}", db_user.id);
                println!("      Role: {:?}", db_user.role);
                println!("      Active: {}", db_user.is_active);
            }
            Ok(None) => {
                println!("   ❌ User not found by username");
            }
            Err(e) => {
                println!("   ❌ Database error: {}", e);
            }
        }

        // Test user retrieval by ID
        match storage.users().get_user_by_id(&user.id).await {
            Ok(Some(db_user)) => {
                println!("   ✅ Retrieved user by ID");
            }
            Ok(None) => {
                println!("   ❌ User not found by ID");
            }
            Err(e) => {
                println!("   ❌ Database error retrieving by ID: {}", e);
            }
        }
    }

    Ok(())
}

fn test_role_conversion() {
    println!("🔄 Testing role conversion functions...");

    let test_cases = vec![
        (StorageUserRole::Admin, UserRole::Admin),
        (StorageUserRole::Moderator, UserRole::Moderator),
        (StorageUserRole::User, UserRole::User),
        (StorageUserRole::Guest, UserRole::Guest),
    ];

    for (storage_role, expected_api_role) in test_cases {
        let converted = convert_from_storage_role(&storage_role);
        if converted == expected_api_role {
            println!("   ✅ {:?} -> {:?}", storage_role, converted);
        } else {
            println!(
                "   ❌ {:?} -> {:?} (expected {:?})",
                storage_role, converted, expected_api_role
            );
        }

        let back_converted = convert_to_storage_role(&converted);
        if back_converted == storage_role {
            println!("   ✅ {:?} -> {:?} (round trip)", converted, back_converted);
        } else {
            println!(
                "   ❌ {:?} -> {:?} (expected {:?}) (round trip)",
                converted, back_converted, storage_role
            );
        }
    }
}

async fn simulate_middleware_behavior(
    storage: &StorageManager,
    jwt_secret: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️  Simulating JWT middleware behavior...");

    // Find an admin user
    let admin_user = if let Some(user) = storage.users().get_user_by_username("test_admin").await? {
        Some(user)
    } else {
        storage.users().get_user_by_username("admin").await?
    };

    if let Some(admin_user) = admin_user {
        println!("   👤 Testing with admin user: {}", admin_user.username);

        let user_id = Uuid::parse_str(&admin_user.id)?;
        let session_id = Uuid::new_v4();
        let api_role = convert_from_storage_role(&admin_user.role);

        // Create a test session
        let session = lair_chat::server::storage::models::Session {
            id: session_id.to_string(),
            user_id: admin_user.id.clone(),
            token: "test_token".to_string(),
            created_at: Utc::now().timestamp() as u64,
            expires_at: (Utc::now() + Duration::hours(24)).timestamp() as u64,
            last_activity: Utc::now().timestamp() as u64,
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test_client".to_string()),
            is_active: true,
            metadata: lair_chat::server::storage::models::SessionMetadata {
                client_type: Some("test".to_string()),
                client_version: None,
                device_info: None,
                location: None,
                custom: HashMap::new(),
            },
        };

        // Store session
        match storage.sessions().create_session(session.clone()).await {
            Ok(_) => {
                println!("   ✅ Created test session: {}", session_id);
            }
            Err(e) => {
                println!(
                    "   ⚠️  Session creation failed (might already exist): {}",
                    e
                );
            }
        }

        // Generate JWT token
        let token = generate_access_token(
            user_id,
            &admin_user.username,
            api_role.clone(),
            session_id,
            jwt_secret,
        )?;
        println!("   🎫 Generated JWT token");

        // Simulate middleware validation
        let validation = Validation::new(Algorithm::HS256);
        match decode::<JwtClaims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => {
                println!("   ✅ JWT token decoded successfully");
                let claims = token_data.claims;

                // Check user ID
                let token_user_id = Uuid::parse_str(&claims.sub)?;
                if token_user_id == user_id {
                    println!("   ✅ User ID matches");
                } else {
                    println!("   ❌ User ID mismatch");
                }

                // Check session ID
                let token_session_id = Uuid::parse_str(&claims.session_id)?;
                if token_session_id == session_id {
                    println!("   ✅ Session ID matches");
                } else {
                    println!("   ❌ Session ID mismatch");
                }

                // Check role
                if claims.role == api_role {
                    println!("   ✅ Role matches: {:?}", claims.role);
                } else {
                    println!(
                        "   ❌ Role mismatch: expected {:?}, got {:?}",
                        api_role, claims.role
                    );
                }

                // Check if token is expired
                let expires_at =
                    chrono::DateTime::from_timestamp(claims.exp, 0).unwrap_or_default();
                if Utc::now() < expires_at {
                    println!("   ✅ Token is not expired (expires: {})", expires_at);
                } else {
                    println!("   ❌ Token is expired");
                }

                // Validate session in database
                match storage
                    .sessions()
                    .get_session(&session_id.to_string())
                    .await
                {
                    Ok(Some(db_session)) => {
                        println!("   ✅ Session found in database");
                        if db_session.is_active {
                            println!("   ✅ Session is active");
                        } else {
                            println!("   ❌ Session is not active");
                        }
                        if db_session.expires_at >= Utc::now().timestamp() as u64 {
                            println!("   ✅ Session is not expired");
                        } else {
                            println!("   ❌ Session is expired");
                        }
                    }
                    Ok(None) => {
                        println!("   ❌ Session not found in database");
                    }
                    Err(e) => {
                        println!("   ❌ Database error checking session: {}", e);
                    }
                }

                // Test admin role check
                let is_admin = matches!(claims.role, UserRole::Admin);
                println!(
                    "   {} Admin check: {}",
                    if is_admin { "✅" } else { "❌" },
                    is_admin
                );

                // Test moderator or higher check
                let is_mod_or_higher = matches!(claims.role, UserRole::Admin | UserRole::Moderator);
                println!(
                    "   {} Moderator+ check: {}",
                    if is_mod_or_higher { "✅" } else { "❌" },
                    is_mod_or_higher
                );
            }
            Err(e) => {
                println!("   ❌ JWT token validation failed: {}", e);
            }
        }
    } else {
        println!("   ⚠️  No admin user found for middleware simulation");
    }

    Ok(())
}

fn generate_access_token(
    user_id: Uuid,
    _username: &str,
    role: UserRole,
    session_id: Uuid,
    jwt_secret: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let now = Utc::now();
    let exp = now + Duration::hours(1);

    let claims = JwtClaims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        iss: "lair-chat-debug".to_string(),
        aud: "lair-chat-api".to_string(),
        jti: Uuid::new_v4().to_string(),
        token_type: TokenType::Access,
        role,
        session_id: session_id.to_string(),
        custom: HashMap::new(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| e.into())
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

fn convert_from_storage_role(role: &StorageUserRole) -> UserRole {
    match role {
        StorageUserRole::Admin => UserRole::Admin,
        StorageUserRole::Moderator => UserRole::Moderator,
        StorageUserRole::User => UserRole::User,
        StorageUserRole::Guest => UserRole::Guest,
    }
}

fn convert_to_storage_role(role: &UserRole) -> StorageUserRole {
    match role {
        UserRole::Admin => StorageUserRole::Admin,
        UserRole::Moderator => StorageUserRole::Moderator,
        UserRole::User => StorageUserRole::User,
        UserRole::Guest => StorageUserRole::Guest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_conversions() {
        // Test all role conversions
        assert_eq!(
            convert_from_storage_role(&StorageUserRole::Admin),
            UserRole::Admin
        );
        assert_eq!(
            convert_from_storage_role(&StorageUserRole::Moderator),
            UserRole::Moderator
        );
        assert_eq!(
            convert_from_storage_role(&StorageUserRole::User),
            UserRole::User
        );
        assert_eq!(
            convert_from_storage_role(&StorageUserRole::Guest),
            UserRole::Guest
        );

        // Test round-trip conversions
        let roles = vec![
            UserRole::Admin,
            UserRole::Moderator,
            UserRole::User,
            UserRole::Guest,
        ];
        for role in roles {
            let storage_role = convert_to_storage_role(&role);
            let back_to_api = convert_from_storage_role(&storage_role);
            assert_eq!(role, back_to_api);
        }
    }

    #[tokio::test]
    async fn test_jwt_token_generation() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let jwt_secret = "test_secret";

        let token =
            generate_access_token(user_id, "testuser", UserRole::Admin, session_id, jwt_secret)
                .unwrap();

        assert!(!token.is_empty());

        // Validate the token
        let validation = Validation::new(Algorithm::HS256);
        let decoded = decode::<JwtClaims>(
            &token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &validation,
        )
        .unwrap();

        assert_eq!(decoded.claims.sub, user_id.to_string());
        assert_eq!(decoded.claims.role, UserRole::Admin);
        assert_eq!(decoded.claims.session_id, session_id.to_string());
    }
}
