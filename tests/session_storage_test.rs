//! Comprehensive tests for SessionStorage implementation
//! Tests all functionality including CRUD operations, statistics, and edge cases

use std::collections::HashMap;
use tokio;

use lair_chat::server::storage::{
    models::{
        ChatSettings, NotificationSettings, PrivacySettings, Session, SessionMetadata, User,
        UserProfile, UserRole, UserSettings,
    },
    sqlite::SqliteStorage,
    traits::{SessionStorage, UserStorage},
    DatabaseConfig, Pagination, StorageResult,
};

/// Helper function to create a test database configuration
fn create_test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        url: ":memory:".to_string(),
        max_connections: 10,
        min_connections: 1,
        connection_timeout: std::time::Duration::from_secs(30),
        idle_timeout: std::time::Duration::from_secs(600),
        auto_migrate: true,
    }
}

/// Helper function to create a test user
fn create_test_user(user_id: &str, username: &str) -> User {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    User {
        id: user_id.to_string(),
        username: username.to_string(),
        email: Some(format!("{}@test.com", username)),
        password_hash: "test_hash".to_string(),
        salt: "test_salt".to_string(),
        created_at: now,
        updated_at: now,
        last_seen: Some(now),
        is_active: true,
        role: UserRole::User,
        profile: UserProfile {
            display_name: Some(username.to_string()),
            bio: Some("Test user".to_string()),
            avatar: None,
            status_message: None,
            timezone: Some("UTC".to_string()),
            language: Some("en".to_string()),
            custom_fields: HashMap::new(),
        },
        settings: UserSettings {
            theme: Some("dark".to_string()),
            notifications: NotificationSettings::default(),
            privacy: PrivacySettings::default(),
            chat: ChatSettings::default(),
        },
    }
}

/// Helper function to create a test session
fn create_test_session(user_id: &str, token: &str) -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Session {
        id: format!("session_{}", user_id),
        user_id: user_id.to_string(),
        token: token.to_string(),
        created_at: now,
        expires_at: now + 3600, // 1 hour from now
        last_activity: now,
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: Some("Test Client/1.0".to_string()),
        is_active: true,
        metadata: SessionMetadata {
            client_type: Some("desktop".to_string()),
            client_version: Some("1.0.0".to_string()),
            device_info: Some("Test Desktop".to_string()),
            location: Some("Test Location".to_string()),
            custom: HashMap::from([
                ("test_key".to_string(), "test_value".to_string()),
                ("environment".to_string(), "test".to_string()),
            ]),
        },
    }
}

/// Helper function to create an expired session
fn create_expired_session(user_id: &str, token: &str) -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Session {
        id: format!("expired_session_{}", user_id),
        user_id: user_id.to_string(),
        token: token.to_string(),
        created_at: now - 7200, // 2 hours ago
        expires_at: now - 3600, // 1 hour ago (expired)
        last_activity: now - 3600,
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("Mobile Client/2.0".to_string()),
        is_active: false,
        metadata: SessionMetadata {
            client_type: Some("mobile".to_string()),
            client_version: Some("2.0.0".to_string()),
            device_info: Some("Test Mobile".to_string()),
            location: Some("Mobile Location".to_string()),
            custom: HashMap::new(),
        },
    }
}

#[tokio::test]
async fn test_create_and_get_session() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first (required for foreign key constraint)
    let user = create_test_user("user123", "testuser123");
    storage.create_user(user).await?;

    let session = create_test_session("user123", "token123");

    // Create session
    let created_session = storage.create_session(session.clone()).await?;
    assert_eq!(created_session.id, session.id);
    assert_eq!(created_session.user_id, session.user_id);
    assert_eq!(created_session.token, session.token);

    // Get session by ID
    let retrieved_session = storage.get_session_by_id(&session.id).await?;
    assert!(retrieved_session.is_some());
    let retrieved = retrieved_session.unwrap();
    assert_eq!(retrieved.id, session.id);
    assert_eq!(retrieved.user_id, session.user_id);
    assert_eq!(retrieved.token, session.token);
    assert_eq!(retrieved.is_active, session.is_active);

    // Get session by token
    let retrieved_by_token = storage.get_session_by_token(&session.token).await?;
    assert!(retrieved_by_token.is_some());
    let retrieved = retrieved_by_token.unwrap();
    assert_eq!(retrieved.id, session.id);
    assert_eq!(retrieved.token, session.token);

    Ok(())
}

#[tokio::test]
async fn test_session_not_found() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Try to get non-existent session
    let result = storage.get_session_by_id("nonexistent").await?;
    assert!(result.is_none());

    let result = storage.get_session_by_token("nonexistent_token").await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_update_session_activity() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first
    let user = create_test_user("user456", "testuser456");
    storage.create_user(user).await?;

    let session = create_test_session("user456", "token456");

    // Create session
    storage.create_session(session.clone()).await?;

    // Update activity
    let new_timestamp = session.last_activity + 3600;
    storage
        .update_session_activity(&session.id, new_timestamp)
        .await?;

    // Verify update
    let updated_session = storage.get_session_by_id(&session.id).await?;
    assert!(updated_session.is_some());
    assert_eq!(updated_session.unwrap().last_activity, new_timestamp);

    Ok(())
}

#[tokio::test]
async fn test_update_session_metadata() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first
    let user = create_test_user("user789", "testuser789");
    storage.create_user(user).await?;

    let session = create_test_session("user789", "token789");

    // Create session
    storage.create_session(session.clone()).await?;

    // Update metadata
    let new_metadata = SessionMetadata {
        client_type: Some("web".to_string()),
        client_version: Some("2.1.0".to_string()),
        device_info: Some("Updated Browser".to_string()),
        location: Some("New Location".to_string()),
        custom: HashMap::from([
            ("updated_key".to_string(), "updated_value".to_string()),
            ("feature_flag".to_string(), "enabled".to_string()),
        ]),
    };

    storage
        .update_session_metadata(&session.id, new_metadata.clone())
        .await?;

    // Verify update
    let updated_session = storage.get_session_by_id(&session.id).await?;
    assert!(updated_session.is_some());
    let retrieved = updated_session.unwrap();
    assert_eq!(retrieved.metadata.client_type, new_metadata.client_type);
    assert_eq!(
        retrieved.metadata.client_version,
        new_metadata.client_version
    );
    assert_eq!(retrieved.metadata.device_info, new_metadata.device_info);
    assert_eq!(retrieved.metadata.custom, new_metadata.custom);

    Ok(())
}

#[tokio::test]
async fn test_deactivate_session() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first
    let user = create_test_user("user_deactivate", "testuser_deactivate");
    storage.create_user(user).await?;

    let session = create_test_session("user_deactivate", "token_deactivate");

    // Create session
    storage.create_session(session.clone()).await?;

    // Verify it's active
    let active_session = storage.get_session_by_id(&session.id).await?;
    assert!(active_session.is_some());
    assert!(active_session.unwrap().is_active);

    // Deactivate session
    storage.deactivate_session(&session.id).await?;

    // Verify it's deactivated
    let deactivated_session = storage.get_session_by_id(&session.id).await?;
    assert!(deactivated_session.is_some());
    assert!(!deactivated_session.unwrap().is_active);

    Ok(())
}

#[tokio::test]
async fn test_delete_session() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first
    let user = create_test_user("user_delete", "testuser_delete");
    storage.create_user(user).await?;

    let session = create_test_session("user_delete", "token_delete");

    // Create session
    storage.create_session(session.clone()).await?;

    // Verify it exists
    let existing_session = storage.get_session_by_id(&session.id).await?;
    assert!(existing_session.is_some());

    // Delete session
    storage.delete_session(&session.id).await?;

    // Verify it's deleted
    let deleted_session = storage.get_session_by_id(&session.id).await?;
    assert!(deleted_session.is_none());

    Ok(())
}

#[tokio::test]
async fn test_user_sessions() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;
    let user_id = "user_multiple";

    // Create user first
    let user = create_test_user(user_id, "testuser_multiple");
    storage.create_user(user).await?;

    // Create multiple sessions for the same user
    let session1 = create_test_session(user_id, "token1");
    let session2 = create_test_session(&format!("{}2", user_id), "token2");
    let session3 = create_test_session(&format!("{}3", user_id), "token3");

    // Make session2 and session3 belong to the same user
    let mut session2_same_user = session2.clone();
    session2_same_user.id = "session_user_multiple2".to_string();
    session2_same_user.user_id = user_id.to_string();

    let mut session3_same_user = session3.clone();
    session3_same_user.id = "session_user_multiple3".to_string();
    session3_same_user.user_id = user_id.to_string();

    storage.create_session(session1.clone()).await?;
    storage.create_session(session2_same_user.clone()).await?;
    storage.create_session(session3_same_user.clone()).await?;

    // Get user sessions with pagination
    let pagination = Pagination {
        limit: 10,
        offset: 0,
    };
    let user_sessions = storage.get_user_sessions(user_id, pagination).await?;
    assert_eq!(user_sessions.len(), 3);

    // Get active user sessions
    let active_sessions = storage.get_active_user_sessions(user_id).await?;
    assert_eq!(active_sessions.len(), 3);

    // Deactivate one session
    storage.deactivate_session(&session1.id).await?;

    // Check active sessions again
    let active_sessions_after = storage.get_active_user_sessions(user_id).await?;
    assert_eq!(active_sessions_after.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_deactivate_all_user_sessions() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;
    let user_id = "user_deactivate_all";

    // Create user first
    let user = create_test_user(user_id, "testuser_deactivate_all");
    storage.create_user(user).await?;

    // Create multiple sessions for the user
    let session1 = create_test_session(user_id, "token_all_1");
    let mut session2 = create_test_session(user_id, "token_all_2");
    session2.id = "session_user_deactivate_all2".to_string();

    storage.create_session(session1.clone()).await?;
    storage.create_session(session2.clone()).await?;

    // Verify both are active
    let active_sessions_before = storage.get_active_user_sessions(user_id).await?;
    assert_eq!(active_sessions_before.len(), 2);

    // Deactivate all user sessions
    storage.deactivate_user_sessions(user_id).await?;

    // Verify all are deactivated
    let active_sessions_after = storage.get_active_user_sessions(user_id).await?;
    assert_eq!(active_sessions_after.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_cleanup_expired_sessions() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create users first
    let active_user = create_test_user("user_active", "testuser_active");
    let expired_user = create_test_user("user_expired", "testuser_expired");
    storage.create_user(active_user).await?;
    storage.create_user(expired_user).await?;

    // Create active and expired sessions
    let active_session = create_test_session("user_active", "token_active");
    let expired_session = create_expired_session("user_expired", "token_expired");

    storage.create_session(active_session.clone()).await?;
    storage.create_session(expired_session.clone()).await?;

    // Verify both exist
    assert!(storage
        .get_session_by_id(&active_session.id)
        .await?
        .is_some());
    assert!(storage
        .get_session_by_id(&expired_session.id)
        .await?
        .is_some());

    // Cleanup expired sessions
    let cleaned_count = storage.cleanup_expired_sessions().await?;
    assert_eq!(cleaned_count, 1);

    // Verify only active session remains
    assert!(storage
        .get_session_by_id(&active_session.id)
        .await?
        .is_some());
    assert!(storage
        .get_session_by_id(&expired_session.id)
        .await?
        .is_none());

    Ok(())
}

#[tokio::test]
async fn test_session_counts() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;
    let user_id = "user_counts";

    // Create user first
    let user = create_test_user(user_id, "testuser_counts");
    storage.create_user(user).await?;

    // Create sessions
    let session1 = create_test_session(user_id, "token_count1");
    let mut session2 = create_test_session(user_id, "token_count2");
    session2.id = "session_user_counts2".to_string();
    let expired_session = create_expired_session(user_id, "token_expired_count");

    storage.create_session(session1.clone()).await?;
    storage.create_session(session2.clone()).await?;
    storage.create_session(expired_session.clone()).await?;

    // Count active sessions
    let active_count = storage.count_active_sessions().await?;
    assert_eq!(active_count, 2); // Only non-expired sessions

    // Count user sessions
    let user_count = storage.count_user_sessions(user_id).await?;
    assert_eq!(user_count, 3); // All sessions for the user

    Ok(())
}

#[tokio::test]
async fn test_session_statistics() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create users first
    let desktop_user = create_test_user("user_desktop", "testuser_desktop");
    let mobile_user = create_test_user("user_mobile", "testuser_mobile");
    let web_user = create_test_user("user_web", "testuser_web");
    let expired_user = create_test_user("user_expired_stats", "testuser_expired_stats");
    storage.create_user(desktop_user).await?;
    storage.create_user(mobile_user).await?;
    storage.create_user(web_user).await?;
    storage.create_user(expired_user).await?;

    // Create various sessions with different client types
    let desktop_session = create_test_session("user_desktop", "token_desktop");

    let mut mobile_session = create_test_session("user_mobile", "token_mobile");
    mobile_session.id = "session_mobile".to_string();
    mobile_session.metadata.client_type = Some("mobile".to_string());

    let mut web_session = create_test_session("user_web", "token_web");
    web_session.id = "session_web".to_string();
    web_session.metadata.client_type = Some("web".to_string());

    let expired_session = create_expired_session("user_expired_stats", "token_expired_stats");

    storage.create_session(desktop_session).await?;
    storage.create_session(mobile_session).await?;
    storage.create_session(web_session).await?;
    storage.create_session(expired_session).await?;

    // Get statistics
    let stats = storage.get_session_stats().await?;

    assert_eq!(stats.total_sessions, 4);
    assert_eq!(stats.active_sessions, 3); // Expired session is not active
    assert!(stats.sessions_today >= 4); // All sessions created today
    assert!(stats.sessions_this_week >= 4); // All sessions created this week

    // Check client type distribution
    assert!(stats.sessions_by_client.contains_key("desktop"));
    assert!(stats.sessions_by_client.contains_key("mobile"));
    assert!(stats.sessions_by_client.contains_key("web"));

    // Average duration should be available (might be 0 if no completed sessions)
    assert!(stats.average_session_duration >= 0.0);

    Ok(())
}

#[tokio::test]
async fn test_pagination() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;
    let user_id = "user_pagination";

    // Create user first
    let user = create_test_user(user_id, "testuser_pagination");
    storage.create_user(user).await?;

    // Create multiple sessions
    for i in 0..5 {
        let mut session = create_test_session(user_id, &format!("token_page_{}", i));
        session.id = format!("session_pagination_{}", i);
        storage.create_session(session).await?;
    }

    // Test pagination
    let page1 = storage
        .get_user_sessions(
            user_id,
            Pagination {
                limit: 2,
                offset: 0,
            },
        )
        .await?;
    assert_eq!(page1.len(), 2);

    let page2 = storage
        .get_user_sessions(
            user_id,
            Pagination {
                limit: 2,
                offset: 2,
            },
        )
        .await?;
    assert_eq!(page2.len(), 2);

    let page3 = storage
        .get_user_sessions(
            user_id,
            Pagination {
                limit: 2,
                offset: 4,
            },
        )
        .await?;
    assert_eq!(page3.len(), 1);

    // Ensure no overlap between pages
    assert_ne!(page1[0].id, page2[0].id);
    assert_ne!(page2[0].id, page3[0].id);

    Ok(())
}

#[tokio::test]
async fn test_session_metadata_serialization() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create user first
    let user = create_test_user("user_metadata", "testuser_metadata");
    storage.create_user(user).await?;

    // Create session with complex metadata
    let mut session = create_test_session("user_metadata", "token_metadata");
    session.metadata.custom.insert(
        "nested_json".to_string(),
        r#"{"key": "value", "number": 42}"#.to_string(),
    );
    session.metadata.custom.insert(
        "special_chars".to_string(),
        "Special: åäö, 中文, 🚀".to_string(),
    );

    storage.create_session(session.clone()).await?;

    // Retrieve and verify metadata is preserved
    let retrieved = storage.get_session_by_id(&session.id).await?;
    assert!(retrieved.is_some());
    let retrieved_session = retrieved.unwrap();

    assert_eq!(
        retrieved_session.metadata.custom.get("nested_json"),
        session.metadata.custom.get("nested_json")
    );
    assert_eq!(
        retrieved_session.metadata.custom.get("special_chars"),
        session.metadata.custom.get("special_chars")
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_session_operations() -> StorageResult<()> {
    let storage = SqliteStorage::new(create_test_db_config()).await?;

    // Create users first
    for i in 0..10 {
        let user = create_test_user(&format!("user_{}", i), &format!("testuser_{}", i));
        storage.create_user(user).await?;
    }

    // Create multiple sessions concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let storage_clone = storage.clone();
        let handle = tokio::spawn(async move {
            let session = create_test_session(&format!("user_{}", i), &format!("token_{}", i));
            storage_clone.create_session(session).await
        });
        handles.push(handle);
    }

    // Wait for all sessions to be created
    for handle in handles {
        handle.await.unwrap()?;
    }

    // Verify all sessions were created
    let total_sessions = storage.count_active_sessions().await?;
    assert_eq!(total_sessions, 10);

    Ok(())
}
