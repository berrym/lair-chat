//! SQLite invitation repository implementation.

use async_trait::async_trait;
use sqlx::Row;

use super::SqliteStorage;
use crate::domain::{Invitation, InvitationId, InvitationStatus, RoomId, UserId};
use crate::storage::InvitationRepository;
use crate::Result;

#[async_trait]
impl InvitationRepository for SqliteStorage {
    async fn create(&self, invitation: &Invitation) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO invitations (id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(invitation.id.to_string())
        .bind(invitation.room_id.to_string())
        .bind(invitation.inviter.to_string())
        .bind(invitation.invitee.to_string())
        .bind(invitation.status.as_str())
        .bind(&invitation.message)
        .bind(invitation.created_at.timestamp())
        .bind(invitation.responded_at.map(|t| t.timestamp()))
        .bind(invitation.expires_at.timestamp())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: InvitationId) -> Result<Option<Invitation>> {
        let row = sqlx::query(
            r#"
            SELECT id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at
            FROM invitations WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_invitation(row)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, invitation: &Invitation) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE invitations
            SET status = ?, responded_at = ?
            WHERE id = ?
            "#,
        )
        .bind(invitation.status.as_str())
        .bind(invitation.responded_at.map(|t| t.timestamp()))
        .bind(invitation.id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::InvitationNotFound);
        }

        Ok(())
    }

    async fn delete(&self, id: InvitationId) -> Result<()> {
        let result = sqlx::query("DELETE FROM invitations WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::InvitationNotFound);
        }

        Ok(())
    }

    async fn list_pending_for_user(&self, user_id: UserId) -> Result<Vec<Invitation>> {
        let now = chrono::Utc::now().timestamp();

        let rows = sqlx::query(
            r#"
            SELECT id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at
            FROM invitations
            WHERE invitee_id = ? AND status = 'pending' AND expires_at > ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_invitation).collect()
    }

    async fn list_sent_by_user(&self, user_id: UserId) -> Result<Vec<Invitation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at
            FROM invitations
            WHERE inviter_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_invitation).collect()
    }

    async fn list_for_room(&self, room_id: RoomId) -> Result<Vec<Invitation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at
            FROM invitations
            WHERE room_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(room_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_invitation).collect()
    }

    async fn find_pending(&self, room_id: RoomId, invitee: UserId) -> Result<Option<Invitation>> {
        let now = chrono::Utc::now().timestamp();

        let row = sqlx::query(
            r#"
            SELECT id, room_id, inviter_id, invitee_id, status, message, created_at, responded_at, expires_at
            FROM invitations
            WHERE room_id = ? AND invitee_id = ? AND status = 'pending' AND expires_at > ?
            "#,
        )
        .bind(room_id.to_string())
        .bind(invitee.to_string())
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_invitation(row)?)),
            None => Ok(None),
        }
    }

    async fn update_status(&self, id: InvitationId, status: InvitationStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let responded_at = if status != InvitationStatus::Pending {
            Some(now)
        } else {
            None
        };

        let result =
            sqlx::query("UPDATE invitations SET status = ?, responded_at = ? WHERE id = ?")
                .bind(status.as_str())
                .bind(responded_at)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::InvitationNotFound);
        }

        Ok(())
    }

    async fn delete_by_room(&self, room_id: RoomId) -> Result<u64> {
        let result = sqlx::query("DELETE FROM invitations WHERE room_id = ?")
            .bind(room_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn expire_old(&self) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            "UPDATE invitations SET status = 'expired' WHERE status = 'pending' AND expires_at < ?",
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Convert a database row to an Invitation.
fn row_to_invitation(row: sqlx::sqlite::SqliteRow) -> Result<Invitation> {
    let id: String = row.get("id");
    let room_id: String = row.get("room_id");
    let inviter_id: String = row.get("inviter_id");
    let invitee_id: String = row.get("invitee_id");
    let status: String = row.get("status");
    let message: Option<String> = row.get("message");
    let created_at: i64 = row.get("created_at");
    let responded_at: Option<i64> = row.get("responded_at");
    let expires_at: i64 = row.get("expires_at");

    Ok(Invitation {
        id: InvitationId::parse(&id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        room_id: RoomId::parse(&room_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        inviter: UserId::parse(&inviter_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        invitee: UserId::parse(&invitee_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        status: InvitationStatus::from_str(&status),
        message,
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
        responded_at: responded_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
        expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, Role, Room, RoomName, RoomSettings, User, Username};
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::{RoomRepository, UserRepository};
    use chrono::Duration;

    async fn setup() -> SqliteStorage {
        SqliteStorage::in_memory().await.unwrap()
    }

    fn test_user(name: &str) -> User {
        User::new(
            Username::new(name).unwrap(),
            Email::new(&format!("{name}@example.com")).unwrap(),
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

    fn test_invitation(room: &Room, inviter: &User, invitee: &User) -> Invitation {
        Invitation::new(room.id, inviter.id, invitee.id)
    }

    #[tokio::test]
    async fn test_create_and_find_invitation() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        let invitation = test_invitation(&room, &inviter, &invitee);
        InvitationRepository::create(&storage, &invitation)
            .await
            .unwrap();

        // Find by ID
        let found = InvitationRepository::find_by_id(&storage, invitation.id)
            .await
            .unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.status, InvitationStatus::Pending);
    }

    #[tokio::test]
    async fn test_find_pending() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        let invitation = test_invitation(&room, &inviter, &invitee);
        InvitationRepository::create(&storage, &invitation)
            .await
            .unwrap();

        // Find pending
        let found = InvitationRepository::find_pending(&storage, room.id, invitee.id)
            .await
            .unwrap();
        assert!(found.is_some());

        // Non-existent user
        let other_user = test_user("other");
        UserRepository::create(&storage, &other_user, "password")
            .await
            .unwrap();

        let found = InvitationRepository::find_pending(&storage, room.id, other_user.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_pending_for_user() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        // Create multiple rooms and invitations
        for i in 0..3 {
            let room = test_room(&format!("room{i}"), &inviter);
            RoomRepository::create(&storage, &room).await.unwrap();

            let invitation = test_invitation(&room, &inviter, &invitee);
            InvitationRepository::create(&storage, &invitation)
                .await
                .unwrap();
        }

        let pending = InvitationRepository::list_pending_for_user(&storage, invitee.id)
            .await
            .unwrap();
        assert_eq!(pending.len(), 3);
    }

    #[tokio::test]
    async fn test_list_sent_by_user() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Invite multiple users
        for i in 0..3 {
            let invitee = test_user(&format!("invitee{i}"));
            UserRepository::create(&storage, &invitee, "password")
                .await
                .unwrap();

            let invitation = test_invitation(&room, &inviter, &invitee);
            InvitationRepository::create(&storage, &invitation)
                .await
                .unwrap();
        }

        let sent = InvitationRepository::list_sent_by_user(&storage, inviter.id)
            .await
            .unwrap();
        assert_eq!(sent.len(), 3);
    }

    #[tokio::test]
    async fn test_update_status() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        let invitation = test_invitation(&room, &inviter, &invitee);
        InvitationRepository::create(&storage, &invitation)
            .await
            .unwrap();

        // Accept
        InvitationRepository::update_status(&storage, invitation.id, InvitationStatus::Accepted)
            .await
            .unwrap();

        let found = InvitationRepository::find_by_id(&storage, invitation.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.status, InvitationStatus::Accepted);
        assert!(found.responded_at.is_some());

        // Should no longer be in pending list
        let pending = InvitationRepository::list_pending_for_user(&storage, invitee.id)
            .await
            .unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_delete_invitation() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        let invitation = test_invitation(&room, &inviter, &invitee);
        InvitationRepository::create(&storage, &invitation)
            .await
            .unwrap();

        // Delete
        InvitationRepository::delete(&storage, invitation.id)
            .await
            .unwrap();

        let found = InvitationRepository::find_by_id(&storage, invitation.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_room() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Create multiple invitations
        for i in 0..3 {
            let invitee = test_user(&format!("invitee{i}"));
            UserRepository::create(&storage, &invitee, "password")
                .await
                .unwrap();

            let invitation = test_invitation(&room, &inviter, &invitee);
            InvitationRepository::create(&storage, &invitation)
                .await
                .unwrap();
        }

        // Delete all for room
        let deleted = InvitationRepository::delete_by_room(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(deleted, 3);

        let invitations = InvitationRepository::list_for_room(&storage, room.id)
            .await
            .unwrap();
        assert!(invitations.is_empty());
    }

    #[tokio::test]
    async fn test_expire_old() {
        let storage = setup().await;

        let inviter = test_user("inviter");
        let invitee = test_user("invitee");
        UserRepository::create(&storage, &inviter, "password")
            .await
            .unwrap();
        UserRepository::create(&storage, &invitee, "password")
            .await
            .unwrap();

        let room = test_room("private", &inviter);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Create a valid invitation
        let valid = test_invitation(&room, &inviter, &invitee);
        InvitationRepository::create(&storage, &valid)
            .await
            .unwrap();

        // Create an expired invitation
        let invitee2 = test_user("invitee2");
        UserRepository::create(&storage, &invitee2, "password")
            .await
            .unwrap();

        let mut expired = Invitation::new(room.id, inviter.id, invitee2.id);
        expired.expires_at = chrono::Utc::now() - Duration::hours(1);
        InvitationRepository::create(&storage, &expired)
            .await
            .unwrap();

        // Expire old
        let count = InvitationRepository::expire_old(&storage).await.unwrap();
        assert_eq!(count, 1);

        // Check statuses
        let found_valid = InvitationRepository::find_by_id(&storage, valid.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_valid.status, InvitationStatus::Pending);

        let found_expired = InvitationRepository::find_by_id(&storage, expired.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_expired.status, InvitationStatus::Expired);
    }
}
