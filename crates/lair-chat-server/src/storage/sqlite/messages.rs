//! SQLite message repository implementation.

use async_trait::async_trait;
use sqlx::Row;

use super::SqliteStorage;
use crate::domain::{
    Message, MessageContent, MessageId, MessageTarget, Pagination, RoomId, UserId,
};
use crate::storage::MessageRepository;
use crate::Result;

#[async_trait]
impl MessageRepository for SqliteStorage {
    async fn create(&self, message: &Message) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let (target_type, target_id) = target_to_db(&message.target);

        sqlx::query(
            r#"
            INSERT INTO messages (id, author_id, target_type, target_id, content, is_edited, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.author.to_string())
        .bind(target_type)
        .bind(&target_id)
        .bind(message.content.as_str())
        .bind(message.is_edited)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: MessageId) -> Result<Option<Message>> {
        let row = sqlx::query(
            r#"
            SELECT id, author_id, target_type, target_id, content, is_edited, created_at, updated_at
            FROM messages WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_message(row)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, message: &Message) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            UPDATE messages
            SET content = ?, is_edited = 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(message.content.as_str())
        .bind(now)
        .bind(message.id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::MessageNotFound);
        }

        Ok(())
    }

    async fn delete(&self, id: MessageId) -> Result<()> {
        let result = sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::MessageNotFound);
        }

        Ok(())
    }

    async fn find_by_room(&self, room_id: RoomId, pagination: Pagination) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, author_id, target_type, target_id, content, is_edited, created_at, updated_at
            FROM messages
            WHERE target_type = 'room' AND target_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(room_id.to_string())
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_message).collect()
    }

    async fn find_direct_messages(
        &self,
        user1: UserId,
        user2: UserId,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        // DM target_id is the recipient, so we need to look for messages where
        // either user sent to the other
        let rows = sqlx::query(
            r#"
            SELECT id, author_id, target_type, target_id, content, is_edited, created_at, updated_at
            FROM messages
            WHERE target_type = 'dm' AND (
                (author_id = ? AND target_id = ?) OR
                (author_id = ? AND target_id = ?)
            )
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user1.to_string())
        .bind(user2.to_string())
        .bind(user2.to_string())
        .bind(user1.to_string())
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_message).collect()
    }

    async fn find_by_target(
        &self,
        target: &MessageTarget,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        let (target_type, target_id) = target_to_db(target);

        let rows = sqlx::query(
            r#"
            SELECT id, author_id, target_type, target_id, content, is_edited, created_at, updated_at
            FROM messages
            WHERE target_type = ? AND target_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(target_type)
        .bind(&target_id)
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_message).collect()
    }

    async fn count_by_room(&self, room_id: RoomId) -> Result<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE target_type = 'room' AND target_id = ?",
        )
        .bind(room_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    async fn count_direct_messages(&self, user1: UserId, user2: UserId) -> Result<u64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM messages
            WHERE target_type = 'dm' AND (
                (author_id = ? AND target_id = ?) OR
                (author_id = ? AND target_id = ?)
            )
            "#,
        )
        .bind(user1.to_string())
        .bind(user2.to_string())
        .bind(user2.to_string())
        .bind(user1.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    async fn get_latest_in_room(&self, room_id: RoomId) -> Result<Option<Message>> {
        let row = sqlx::query(
            r#"
            SELECT id, author_id, target_type, target_id, content, is_edited, created_at, updated_at
            FROM messages
            WHERE target_type = 'room' AND target_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(room_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_message(row)?)),
            None => Ok(None),
        }
    }

    async fn delete_by_room(&self, room_id: RoomId) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM messages WHERE target_type = 'room' AND target_id = ?")
                .bind(room_id.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_author(&self, author_id: UserId) -> Result<u64> {
        let result = sqlx::query("DELETE FROM messages WHERE author_id = ?")
            .bind(author_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Convert MessageTarget to database representation.
fn target_to_db(target: &MessageTarget) -> (&'static str, String) {
    match target {
        MessageTarget::Room { room_id } => ("room", room_id.to_string()),
        MessageTarget::DirectMessage { recipient } => ("dm", recipient.to_string()),
    }
}

/// Convert a database row to a Message.
fn row_to_message(row: sqlx::sqlite::SqliteRow) -> Result<Message> {
    let id: String = row.get("id");
    let author_id: String = row.get("author_id");
    let target_type: String = row.get("target_type");
    let target_id: String = row.get("target_id");
    let content: String = row.get("content");
    let is_edited: bool = row.get("is_edited");
    let created_at: i64 = row.get("created_at");

    let target = match target_type.as_str() {
        "room" => MessageTarget::Room {
            room_id: RoomId::parse(&target_id)
                .map_err(|e| crate::Error::Internal(e.to_string()))?,
        },
        "dm" => MessageTarget::DirectMessage {
            recipient: UserId::parse(&target_id)
                .map_err(|e| crate::Error::Internal(e.to_string()))?,
        },
        _ => {
            return Err(crate::Error::Internal(format!(
                "Unknown message target type: {target_type}"
            )))
        }
    };

    Ok(Message {
        id: MessageId::parse(&id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        author: UserId::parse(&author_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        target,
        content: MessageContent::new_unchecked(content),
        is_edited,
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, Role, Room, RoomName, RoomSettings, User, Username};
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::{RoomRepository, UserRepository};

    async fn setup() -> SqliteStorage {
        SqliteStorage::in_memory().await.unwrap()
    }

    fn test_user(name: &str) -> User {
        User::new(
            Username::new(name).unwrap(),
            Email::new(format!("{name}@example.com")).unwrap(),
            Role::User,
        )
    }

    fn test_room(name: &str, owner: &User) -> Room {
        Room::new(
            RoomName::new(name).unwrap(),
            owner.id,
            RoomSettings::default(),
        )
    }

    fn test_message(author: &User, room: &Room, content: &str) -> Message {
        Message::new(
            author.id,
            MessageTarget::Room { room_id: room.id },
            MessageContent::new(content).unwrap(),
        )
    }

    #[tokio::test]
    async fn test_create_and_find_message() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        let message = test_message(&user, &room, "Hello, world!");
        MessageRepository::create(&storage, &message).await.unwrap();

        // Find by ID
        let found = MessageRepository::find_by_id(&storage, message.id)
            .await
            .unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.content.as_str(), "Hello, world!");
        assert!(!found.is_edited);
    }

    #[tokio::test]
    async fn test_find_by_room() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Create multiple messages
        for i in 0..5 {
            let message = test_message(&user, &room, &format!("Message {i}"));
            MessageRepository::create(&storage, &message).await.unwrap();
        }

        // Find by room
        let messages = MessageRepository::find_by_room(&storage, room.id, Pagination::default())
            .await
            .unwrap();
        assert_eq!(messages.len(), 5);

        // Count
        let count = MessageRepository::count_by_room(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(count, 5);

        // Latest
        let latest = MessageRepository::get_latest_in_room(&storage, room.id)
            .await
            .unwrap();
        assert!(latest.is_some());
    }

    #[tokio::test]
    async fn test_update_message() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        let mut message = test_message(&user, &room, "Original");
        MessageRepository::create(&storage, &message).await.unwrap();

        // Update
        message.content = MessageContent::new("Edited").unwrap();
        message.is_edited = true;
        MessageRepository::update(&storage, &message).await.unwrap();

        let found = MessageRepository::find_by_id(&storage, message.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.content.as_str(), "Edited");
        assert!(found.is_edited);
    }

    #[tokio::test]
    async fn test_delete_message() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        let message = test_message(&user, &room, "To be deleted");
        MessageRepository::create(&storage, &message).await.unwrap();

        // Delete
        MessageRepository::delete(&storage, message.id)
            .await
            .unwrap();

        let found = MessageRepository::find_by_id(&storage, message.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_room() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Create messages
        for i in 0..3 {
            let message = test_message(&user, &room, &format!("Message {i}"));
            MessageRepository::create(&storage, &message).await.unwrap();
        }

        // Delete all messages in room
        let deleted = MessageRepository::delete_by_room(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(deleted, 3);

        let count = MessageRepository::count_by_room(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_direct_messages() {
        let storage = setup().await;

        let user1 = test_user("alice");
        let user2 = test_user("bob");
        UserRepository::create(&storage, &user1, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &user2, "password")
            .await
            .unwrap();

        // Send DM from user1 to user2
        let dm1 = Message::new(
            user1.id,
            MessageTarget::DirectMessage {
                recipient: user2.id,
            },
            MessageContent::new("Hi Bob!").unwrap(),
        );
        MessageRepository::create(&storage, &dm1).await.unwrap();

        // Send DM from user2 to user1
        let dm2 = Message::new(
            user2.id,
            MessageTarget::DirectMessage {
                recipient: user1.id,
            },
            MessageContent::new("Hi Alice!").unwrap(),
        );
        MessageRepository::create(&storage, &dm2).await.unwrap();

        // Find DMs between them (order shouldn't matter)
        let dms = MessageRepository::find_direct_messages(
            &storage,
            user1.id,
            user2.id,
            Pagination::default(),
        )
        .await
        .unwrap();
        assert_eq!(dms.len(), 2);

        // Count
        let count = MessageRepository::count_direct_messages(&storage, user1.id, user2.id)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_pagination() {
        let storage = setup().await;

        let user = test_user("sender");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let room = test_room("general", &user);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Create 10 messages
        for i in 0..10 {
            let message = test_message(&user, &room, &format!("Message {i}"));
            MessageRepository::create(&storage, &message).await.unwrap();
        }

        // Page 1
        let page1 = MessageRepository::find_by_room(&storage, room.id, Pagination::new(0, 3))
            .await
            .unwrap();
        assert_eq!(page1.len(), 3);

        // Page 2
        let page2 = MessageRepository::find_by_room(&storage, room.id, Pagination::new(3, 3))
            .await
            .unwrap();
        assert_eq!(page2.len(), 3);

        // Last page
        let page4 = MessageRepository::find_by_room(&storage, room.id, Pagination::new(9, 3))
            .await
            .unwrap();
        assert_eq!(page4.len(), 1);
    }
}
