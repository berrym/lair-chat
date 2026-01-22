//! SQLite room and membership repository implementation.

use async_trait::async_trait;
use sqlx::Row;

use super::SqliteStorage;
use crate::domain::{
    Email, Pagination, Role, Room, RoomId, RoomMembership, RoomName, RoomRole, RoomSettings, User,
    UserId, Username,
};
use crate::storage::{MembershipRepository, RoomRepository};
use crate::Result;

#[async_trait]
impl RoomRepository for SqliteStorage {
    async fn create(&self, room: &Room) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO rooms (id, name, description, owner_id, is_private, max_members, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(room.id.to_string())
        .bind(room.name.as_str())
        .bind(&room.settings.description)
        .bind(room.owner.to_string())
        .bind(room.settings.is_private)
        .bind(room.settings.max_members.map(|m| m as i64))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: RoomId) -> Result<Option<Room>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, owner_id, is_private, max_members, created_at, updated_at
            FROM rooms WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_room(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, owner_id, is_private, max_members, created_at, updated_at
            FROM rooms WHERE name = ? COLLATE NOCASE
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_room(row)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, room: &Room) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            UPDATE rooms
            SET name = ?, description = ?, owner_id = ?, is_private = ?, max_members = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(room.name.as_str())
        .bind(&room.settings.description)
        .bind(room.owner.to_string())
        .bind(room.settings.is_private)
        .bind(room.settings.max_members.map(|m| m as i64))
        .bind(now)
        .bind(room.id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::RoomNotFound);
        }

        Ok(())
    }

    async fn delete(&self, id: RoomId) -> Result<()> {
        let result = sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::RoomNotFound);
        }

        Ok(())
    }

    async fn list_public(&self, pagination: Pagination) -> Result<Vec<Room>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, owner_id, is_private, max_members, created_at, updated_at
            FROM rooms
            WHERE is_private = 0
            ORDER BY name ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_room).collect()
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Room>> {
        let rows = sqlx::query(
            r#"
            SELECT r.id, r.name, r.description, r.owner_id, r.is_private, r.max_members, r.created_at, r.updated_at
            FROM rooms r
            INNER JOIN room_memberships m ON r.id = m.room_id
            WHERE m.user_id = ?
            ORDER BY r.name ASC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_room).collect()
    }

    async fn count(&self) -> Result<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u64)
    }

    async fn name_exists(&self, name: &str) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rooms WHERE name = ? COLLATE NOCASE)")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }
}

#[async_trait]
impl MembershipRepository for SqliteStorage {
    async fn add_member(&self, membership: &RoomMembership) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO room_memberships (room_id, user_id, role, joined_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(membership.room_id.to_string())
        .bind(membership.user_id.to_string())
        .bind(membership.role.as_str())
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        let result = sqlx::query("DELETE FROM room_memberships WHERE room_id = ? AND user_id = ?")
            .bind(room_id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotRoomMember);
        }

        Ok(())
    }

    async fn get_membership(
        &self,
        room_id: RoomId,
        user_id: UserId,
    ) -> Result<Option<RoomMembership>> {
        let row = sqlx::query(
            r#"
            SELECT room_id, user_id, role, joined_at
            FROM room_memberships
            WHERE room_id = ? AND user_id = ?
            "#,
        )
        .bind(room_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_membership(row)?)),
            None => Ok(None),
        }
    }

    async fn update_role(&self, room_id: RoomId, user_id: UserId, role: RoomRole) -> Result<()> {
        let result =
            sqlx::query("UPDATE room_memberships SET role = ? WHERE room_id = ? AND user_id = ?")
                .bind(role.as_str())
                .bind(room_id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotRoomMember);
        }

        Ok(())
    }

    async fn list_members(&self, room_id: RoomId) -> Result<Vec<RoomMembership>> {
        let rows = sqlx::query(
            r#"
            SELECT room_id, user_id, role, joined_at
            FROM room_memberships
            WHERE room_id = ?
            ORDER BY joined_at ASC
            "#,
        )
        .bind(room_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_membership).collect()
    }

    async fn list_members_with_users(
        &self,
        room_id: RoomId,
    ) -> Result<Vec<(User, RoomMembership)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                u.id, u.username, u.email, u.role, u.created_at, u.updated_at, u.last_seen_at,
                m.room_id, m.user_id, m.role as member_role, m.joined_at
            FROM room_memberships m
            INNER JOIN users u ON m.user_id = u.id
            WHERE m.room_id = ?
            ORDER BY m.joined_at ASC
            "#,
        )
        .bind(room_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let user = row_to_user_from_join(&row)?;
                let membership = row_to_membership_from_join(&row)?;
                Ok((user, membership))
            })
            .collect()
    }

    async fn count_members(&self, room_id: RoomId) -> Result<u32> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM room_memberships WHERE room_id = ?")
                .bind(room_id.to_string())
                .fetch_one(&self.pool)
                .await?;

        Ok(count as u32)
    }

    async fn is_member(&self, room_id: RoomId, user_id: UserId) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM room_memberships WHERE room_id = ? AND user_id = ?)",
        )
        .bind(room_id.to_string())
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
}

/// Convert a database row to a Room.
fn row_to_room(row: sqlx::sqlite::SqliteRow) -> Result<Room> {
    let id: String = row.get("id");
    let name: String = row.get("name");
    let description: Option<String> = row.get("description");
    let owner_id: String = row.get("owner_id");
    let is_private: bool = row.get("is_private");
    let max_members: Option<i64> = row.get("max_members");
    let created_at: i64 = row.get("created_at");

    Ok(Room {
        id: RoomId::parse(&id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        name: RoomName::new_unchecked(name),
        owner: UserId::parse(&owner_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        settings: RoomSettings {
            description,
            is_private,
            max_members: max_members.map(|m| m as u32),
        },
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
    })
}

/// Convert a database row to a RoomMembership.
fn row_to_membership(row: sqlx::sqlite::SqliteRow) -> Result<RoomMembership> {
    let room_id: String = row.get("room_id");
    let user_id: String = row.get("user_id");
    let role: String = row.get("role");
    let joined_at: i64 = row.get("joined_at");

    Ok(RoomMembership {
        room_id: RoomId::parse(&room_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        user_id: UserId::parse(&user_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        role: RoomRole::parse(&role),
        joined_at: chrono::DateTime::from_timestamp(joined_at, 0).unwrap_or_default(),
    })
}

/// Convert a joined row to a User (when joined with memberships).
fn row_to_user_from_join(row: &sqlx::sqlite::SqliteRow) -> Result<User> {
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
        role: Role::parse(&role),
        created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_default(),
        last_seen_at: last_seen_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
    })
}

/// Convert a joined row to a RoomMembership (when joined with users).
fn row_to_membership_from_join(row: &sqlx::sqlite::SqliteRow) -> Result<RoomMembership> {
    let room_id: String = row.get("room_id");
    let user_id: String = row.get("user_id");
    let role: String = row.get("member_role");
    let joined_at: i64 = row.get("joined_at");

    Ok(RoomMembership {
        room_id: RoomId::parse(&room_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        user_id: UserId::parse(&user_id).map_err(|e| crate::Error::Internal(e.to_string()))?,
        role: RoomRole::parse(&role),
        joined_at: chrono::DateTime::from_timestamp(joined_at, 0).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::UserRepository;

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

    #[tokio::test]
    async fn test_create_and_find_room() {
        let storage = setup().await;
        let owner = test_user("owner");
        UserRepository::create(&storage, &owner, "password")
            .await
            .unwrap();

        let room = test_room("general", &owner);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Find by ID
        let found = RoomRepository::find_by_id(&storage, room.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name.as_str(), "general");

        // Find by name
        let found = RoomRepository::find_by_name(&storage, "general")
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_room_membership() {
        let storage = setup().await;

        // Create owner and room
        let owner = test_user("owner");
        UserRepository::create(&storage, &owner, "password")
            .await
            .unwrap();

        let room = test_room("general", &owner);
        RoomRepository::create(&storage, &room).await.unwrap();

        // Add owner as member
        let membership = RoomMembership::new(room.id, owner.id, RoomRole::Owner);
        MembershipRepository::add_member(&storage, &membership)
            .await
            .unwrap();

        // Create and add another user
        let user = test_user("user1");
        UserRepository::create(&storage, &user, "password")
            .await
            .unwrap();

        let user_membership = RoomMembership::new(room.id, user.id, RoomRole::Member);
        MembershipRepository::add_member(&storage, &user_membership)
            .await
            .unwrap();

        // Check membership
        assert!(MembershipRepository::is_member(&storage, room.id, owner.id)
            .await
            .unwrap());
        assert!(MembershipRepository::is_member(&storage, room.id, user.id)
            .await
            .unwrap());

        // Count members
        let count = MembershipRepository::count_members(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(count, 2);

        // List members
        let members = MembershipRepository::list_members(&storage, room.id)
            .await
            .unwrap();
        assert_eq!(members.len(), 2);

        // List rooms for user
        let rooms = RoomRepository::list_for_user(&storage, user.id)
            .await
            .unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name.as_str(), "general");
    }

    #[tokio::test]
    async fn test_update_member_role() {
        let storage = setup().await;

        let owner = test_user("owner");
        UserRepository::create(&storage, &owner, "password")
            .await
            .unwrap();

        let room = test_room("general", &owner);
        RoomRepository::create(&storage, &room).await.unwrap();

        let membership = RoomMembership::new(room.id, owner.id, RoomRole::Member);
        MembershipRepository::add_member(&storage, &membership)
            .await
            .unwrap();

        // Update role
        MembershipRepository::update_role(&storage, room.id, owner.id, RoomRole::Moderator)
            .await
            .unwrap();

        let found = MembershipRepository::get_membership(&storage, room.id, owner.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.role, RoomRole::Moderator);
    }

    #[tokio::test]
    async fn test_remove_member() {
        let storage = setup().await;

        let owner = test_user("owner");
        UserRepository::create(&storage, &owner, "password")
            .await
            .unwrap();

        let room = test_room("general", &owner);
        RoomRepository::create(&storage, &room).await.unwrap();

        let membership = RoomMembership::new(room.id, owner.id, RoomRole::Owner);
        MembershipRepository::add_member(&storage, &membership)
            .await
            .unwrap();

        // Remove
        MembershipRepository::remove_member(&storage, room.id, owner.id)
            .await
            .unwrap();

        assert!(
            !MembershipRepository::is_member(&storage, room.id, owner.id)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_list_members_with_users() {
        let storage = setup().await;

        let owner = test_user("owner");
        UserRepository::create(&storage, &owner, "password")
            .await
            .unwrap();

        let room = test_room("general", &owner);
        RoomRepository::create(&storage, &room).await.unwrap();

        let membership = RoomMembership::new(room.id, owner.id, RoomRole::Owner);
        MembershipRepository::add_member(&storage, &membership)
            .await
            .unwrap();

        let members = MembershipRepository::list_members_with_users(&storage, room.id)
            .await
            .unwrap();

        assert_eq!(members.len(), 1);
        let (user, mem) = &members[0];
        assert_eq!(user.username.as_str(), "owner");
        assert_eq!(mem.role, RoomRole::Owner);
    }
}
