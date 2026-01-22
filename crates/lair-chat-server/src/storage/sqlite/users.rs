//! SQLite user repository implementation.

use async_trait::async_trait;
use sqlx::Row;

use super::SqliteStorage;
use crate::domain::{Email, Pagination, Role, User, UserId, Username};
use crate::storage::UserRepository;
use crate::Result;

#[async_trait]
impl UserRepository for SqliteStorage {
    async fn create(&self, user: &User, password_hash: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id.to_string())
        .bind(user.username.as_str())
        .bind(user.email.as_str())
        .bind(password_hash)
        .bind(user.role.as_str())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, username, email, role, created_at, updated_at, last_seen_at
            FROM users WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, username, email, role, created_at, updated_at, last_seen_at
            FROM users WHERE username = ? COLLATE NOCASE
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, username, email, role, created_at, updated_at, last_seen_at
            FROM users WHERE email = ? COLLATE NOCASE
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_user(row)?)),
            None => Ok(None),
        }
    }

    async fn get_password_hash(&self, user_id: UserId) -> Result<Option<String>> {
        let hash: Option<String> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        Ok(hash)
    }

    async fn update_password_hash(&self, user_id: UserId, password_hash: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(password_hash)
            .bind(now)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::UserNotFound);
        }

        Ok(())
    }

    async fn update(&self, user: &User) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            UPDATE users
            SET username = ?, email = ?, role = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(user.username.as_str())
        .bind(user.email.as_str())
        .bind(user.role.as_str())
        .bind(now)
        .bind(user.id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::UserNotFound);
        }

        Ok(())
    }

    async fn delete(&self, id: UserId) -> Result<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::UserNotFound);
        }

        Ok(())
    }

    async fn list(&self, pagination: Pagination) -> Result<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT id, username, email, role, created_at, updated_at, last_seen_at
            FROM users
            ORDER BY username ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_user).collect()
    }

    async fn count(&self) -> Result<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u64)
    }

    async fn username_exists(&self, username: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ? COLLATE NOCASE)",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    async fn email_exists(&self, email: &str) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = ? COLLATE NOCASE)")
                .bind(email)
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }
}

/// Convert a database row to a User.
fn row_to_user(row: sqlx::sqlite::SqliteRow) -> Result<User> {
    let id: String = row.get("id");
    let username: String = row.get("username");
    let email: String = row.get("email");
    let role: String = row.get("role");
    let created_at: i64 = row.get("created_at");
    let updated_at: i64 = row.get("updated_at");
    let last_seen_at: Option<i64> = row.get("last_seen_at");

    Ok(User {
        id: UserId::parse(&id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        username: Username::new_unchecked(username),
        email: Email::new_unchecked(email),
        role: Role::from_str(&role),
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_default(),
        last_seen_at: last_seen_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::sqlite::SqliteStorage;

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

    #[tokio::test]
    async fn test_create_and_find_user() {
        let storage = setup().await;
        let user = test_user();

        // Create
        UserRepository::create(&storage, &user, "hashed_password")
            .await
            .unwrap();

        // Find by ID
        let found = UserRepository::find_by_id(&storage, user.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.username.as_str(), "testuser");

        // Find by username
        let found = UserRepository::find_by_username(&storage, "testuser")
            .await
            .unwrap();
        assert!(found.is_some());

        // Find by email
        let found = UserRepository::find_by_email(&storage, "test@example.com")
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_username_case_insensitive() {
        let storage = setup().await;
        let user = test_user();

        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Should find with different case
        let found = UserRepository::find_by_username(&storage, "TESTUSER")
            .await
            .unwrap();
        assert!(found.is_some());

        // Should report exists with different case
        let exists = UserRepository::username_exists(&storage, "TestUser")
            .await
            .unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_update_user() {
        let storage = setup().await;
        let mut user = test_user();

        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Update role
        user.role = Role::Admin;
        UserRepository::update(&storage, &user).await.unwrap();

        let found = UserRepository::find_by_id(&storage, user.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.role, Role::Admin);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let storage = setup().await;
        let user = test_user();

        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        // Delete
        UserRepository::delete(&storage, user.id).await.unwrap();

        // Should not find
        let found = UserRepository::find_by_id(&storage, user.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_users() {
        let storage = setup().await;

        // Create multiple users
        for i in 0..5 {
            let user = User::new(
                Username::new(&format!("user{i}")).unwrap(),
                Email::new(&format!("user{i}@example.com")).unwrap(),
                Role::User,
            );
            UserRepository::create(&storage, &user, "password")
                .await
                .unwrap();
        }

        // List with pagination
        let page1 = UserRepository::list(&storage, Pagination::new(0, 3))
            .await
            .unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = UserRepository::list(&storage, Pagination::new(3, 3))
            .await
            .unwrap();
        assert_eq!(page2.len(), 2);

        // Count
        let count = UserRepository::count(&storage).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_password_operations() {
        let storage = setup().await;
        let user = test_user();

        UserRepository::create(&storage, &user, "initial_hash")
            .await
            .unwrap();

        // Get password hash
        let hash = UserRepository::get_password_hash(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(hash, Some("initial_hash".to_string()));

        // Update password hash
        UserRepository::update_password_hash(&storage, user.id, "new_hash")
            .await
            .unwrap();

        let hash = UserRepository::get_password_hash(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(hash, Some("new_hash".to_string()));
    }
}
