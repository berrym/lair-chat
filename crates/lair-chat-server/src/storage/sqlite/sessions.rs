//! SQLite session repository implementation.

use async_trait::async_trait;
use sqlx::Row;

use super::SqliteStorage;
use crate::domain::{Protocol, Session, SessionId, UserId};
use crate::storage::SessionRepository;
use crate::Result;

#[async_trait]
impl SessionRepository for SqliteStorage {
    async fn create(&self, session: &Session) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, protocol, ip_address, user_agent, created_at, expires_at, last_active_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.user_id.to_string())
        .bind(session.protocol.as_str())
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.created_at.timestamp())
        .bind(session.expires_at.timestamp())
        .bind(session.last_active_at.timestamp())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: SessionId) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, protocol, ip_address, user_agent, created_at, expires_at, last_active_at
            FROM sessions WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_session(row)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, session: &Session) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET expires_at = ?, last_active_at = ?
            WHERE id = ?
            "#,
        )
        .bind(session.expires_at.timestamp())
        .bind(session.last_active_at.timestamp())
        .bind(session.id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::SessionNotFound);
        }

        Ok(())
    }

    async fn delete(&self, id: SessionId) -> Result<()> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::SessionNotFound);
        }

        Ok(())
    }

    async fn delete_by_user(&self, user_id: UserId) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn list_by_user(&self, user_id: UserId) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, protocol, ip_address, user_agent, created_at, expires_at, last_active_at
            FROM sessions
            WHERE user_id = ?
            ORDER BY last_active_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_session).collect()
    }

    async fn count_by_user(&self, user_id: UserId) -> Result<u32> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
            .bind(user_id.to_string())
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u32)
    }

    async fn delete_expired(&self) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn is_valid(&self, id: SessionId) -> Result<bool> {
        let now = chrono::Utc::now().timestamp();

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ? AND expires_at > ?)",
        )
        .bind(id.to_string())
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
}

/// Convert a database row to a Session.
fn row_to_session(row: sqlx::sqlite::SqliteRow) -> Result<Session> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let protocol: String = row.get("protocol");
    let ip_address: Option<String> = row.get("ip_address");
    let user_agent: Option<String> = row.get("user_agent");
    let created_at: i64 = row.get("created_at");
    let expires_at: i64 = row.get("expires_at");
    let last_active_at: i64 = row.get("last_active_at");

    Ok(Session {
        id: SessionId::parse(&id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        user_id: UserId::parse(&user_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        protocol: Protocol::from_str(&protocol),
        ip_address,
        user_agent,
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
        expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_default(),
        last_active_at: chrono::DateTime::from_timestamp(last_active_at, 0).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, Role, User, Username};
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::UserRepository;
    use chrono::Duration;

    async fn setup() -> SqliteStorage {
        SqliteStorage::in_memory().await.unwrap()
    }

    fn test_user() -> User {
        User::new(
            Username::new("testuser").unwrap(),
            Email::new("test@example.com").unwrap(),
            Role::User,
        )
    }

    fn test_session(user: &User) -> Session {
        Session::new(user.id, Protocol::Tcp)
    }

    #[tokio::test]
    async fn test_create_and_find_session() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let session = test_session(&user);
        SessionRepository::create(&storage, &session).await.unwrap();

        // Find by ID
        let found = SessionRepository::find_by_id(&storage, session.id)
            .await
            .unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.user_id, user.id);
        assert_eq!(found.protocol, Protocol::Tcp);
    }

    #[tokio::test]
    async fn test_session_validity() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let session = test_session(&user);
        SessionRepository::create(&storage, &session).await.unwrap();

        // Should be valid (just created)
        let is_valid = SessionRepository::is_valid(&storage, session.id)
            .await
            .unwrap();
        assert!(is_valid);

        // Create an expired session
        let mut expired = Session::new(user.id, Protocol::Http);
        expired.expires_at = chrono::Utc::now() - Duration::hours(1);
        SessionRepository::create(&storage, &expired).await.unwrap();

        let is_valid = SessionRepository::is_valid(&storage, expired.id)
            .await
            .unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_update_session() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let mut session = test_session(&user);
        SessionRepository::create(&storage, &session).await.unwrap();

        // Touch session
        session.touch();
        SessionRepository::update(&storage, &session).await.unwrap();

        let found = SessionRepository::find_by_id(&storage, session.id)
            .await
            .unwrap()
            .unwrap();
        // Compare DB values (same precision) - last_active_at should be >= created_at
        assert!(found.last_active_at >= found.created_at);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let session = test_session(&user);
        SessionRepository::create(&storage, &session).await.unwrap();

        // Delete
        SessionRepository::delete(&storage, session.id)
            .await
            .unwrap();

        let found = SessionRepository::find_by_id(&storage, session.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions_by_user() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Create multiple sessions
        for _ in 0..3 {
            let session = test_session(&user);
            SessionRepository::create(&storage, &session).await.unwrap();
        }

        let sessions = SessionRepository::list_by_user(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 3);

        let count = SessionRepository::count_by_user(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_delete_by_user() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Create multiple sessions
        for _ in 0..3 {
            let session = test_session(&user);
            SessionRepository::create(&storage, &session).await.unwrap();
        }

        // Delete all
        let deleted = SessionRepository::delete_by_user(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(deleted, 3);

        let count = SessionRepository::count_by_user(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let storage = setup().await;

        let user = test_user();
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Create a valid session
        let valid = test_session(&user);
        SessionRepository::create(&storage, &valid).await.unwrap();

        // Create an expired session
        let mut expired = Session::new(user.id, Protocol::Http);
        expired.expires_at = chrono::Utc::now() - Duration::hours(1);
        SessionRepository::create(&storage, &expired).await.unwrap();

        // Delete expired
        let deleted = SessionRepository::delete_expired(&storage).await.unwrap();
        assert_eq!(deleted, 1);

        // Valid should still exist
        let found = SessionRepository::find_by_id(&storage, valid.id)
            .await
            .unwrap();
        assert!(found.is_some());

        // Expired should be gone
        let found = SessionRepository::find_by_id(&storage, expired.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
