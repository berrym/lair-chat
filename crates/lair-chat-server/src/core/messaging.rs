//! Messaging service - sending, editing, and deleting messages.
//!
//! This service handles:
//! - Sending messages to rooms and users (DMs)
//! - Editing messages
//! - Deleting messages
//! - Retrieving message history

use std::sync::Arc;

use crate::domain::{
    events::{Event, EventPayload, MessageDeletedEvent, MessageEditedEvent, MessageReceivedEvent},
    Message, MessageContent, MessageId, MessageTarget, Pagination, RoomId, UserId,
};
use crate::storage::{MembershipRepository, MessageRepository, Storage, UserRepository};
use crate::{Error, Result};

use super::events::EventDispatcher;

// ============================================================================
// MessagingService
// ============================================================================

/// Service for message operations.
pub struct MessagingService<S: Storage> {
    storage: Arc<S>,
    events: EventDispatcher,
}

impl<S: Storage + 'static> MessagingService<S> {
    /// Create a new messaging service.
    pub fn new(storage: Arc<S>, events: EventDispatcher) -> Self {
        Self { storage, events }
    }

    /// Send a message.
    ///
    /// # Errors
    ///
    /// - `ContentInvalid` - Message content is empty or invalid
    /// - `RoomNotFound` - Target room doesn't exist
    /// - `NotRoomMember` - User is not a member of the room
    /// - `UserNotFound` - DM recipient doesn't exist
    pub async fn send(
        &self,
        author_id: UserId,
        target: MessageTarget,
        content: &str,
    ) -> Result<Message> {
        // Validate content
        let content = MessageContent::new(content).map_err(|e| Error::ContentInvalid {
            reason: e.to_string(),
        })?;

        // Validate permissions based on target
        match &target {
            MessageTarget::Room { room_id } => {
                // Verify user is a member of the room
                if !MembershipRepository::is_member(&*self.storage, *room_id, author_id).await? {
                    return Err(Error::NotRoomMember);
                }
            }
            MessageTarget::DirectMessage { recipient } => {
                // Verify recipient exists
                if UserRepository::find_by_id(&*self.storage, *recipient)
                    .await?
                    .is_none()
                {
                    return Err(Error::UserNotFound);
                }
            }
        }

        // Create message
        let message = Message::new(author_id, target, content);
        MessageRepository::create(&*self.storage, &message).await?;

        // Emit event
        let event = Event::new(EventPayload::MessageReceived(MessageReceivedEvent::new(
            message.clone(),
        )));
        self.events.dispatch(event).await;

        Ok(message)
    }

    /// Edit a message.
    ///
    /// Only the message author can edit their messages.
    ///
    /// # Errors
    ///
    /// - `MessageNotFound` - Message doesn't exist
    /// - `NotMessageAuthor` - User didn't write this message
    /// - `ContentInvalid` - New content is empty or invalid
    pub async fn edit(
        &self,
        user_id: UserId,
        message_id: MessageId,
        content: &str,
    ) -> Result<Message> {
        // Find message
        let mut message = MessageRepository::find_by_id(&*self.storage, message_id)
            .await?
            .ok_or(Error::MessageNotFound)?;

        // Verify ownership
        if message.author != user_id {
            return Err(Error::NotMessageAuthor);
        }

        // Validate new content
        let new_content = MessageContent::new(content).map_err(|e| Error::ContentInvalid {
            reason: e.to_string(),
        })?;

        // Store previous content for event
        let previous_content = message.content.as_str().to_string();

        // Edit message
        message.edit(new_content);
        MessageRepository::update(&*self.storage, &message).await?;

        // Emit event
        let event = Event::new(EventPayload::MessageEdited(MessageEditedEvent::new(
            message.clone(),
            previous_content,
        )));
        self.events.dispatch(event).await;

        Ok(message)
    }

    /// Delete a message.
    ///
    /// The message author, room moderators, and admins can delete messages.
    ///
    /// # Errors
    ///
    /// - `MessageNotFound` - Message doesn't exist
    /// - `PermissionDenied` - User can't delete this message
    pub async fn delete(&self, user_id: UserId, message_id: MessageId) -> Result<()> {
        // Find message
        let message = MessageRepository::find_by_id(&*self.storage, message_id)
            .await?
            .ok_or(Error::MessageNotFound)?;

        // Check permissions
        let can_delete = if message.author == user_id {
            // Author can always delete their own messages
            true
        } else {
            // For room messages, check if user is moderator
            match &message.target {
                MessageTarget::Room { room_id } => {
                    if let Some(membership) =
                        MembershipRepository::get_membership(&*self.storage, *room_id, user_id)
                            .await?
                    {
                        membership.is_moderator()
                    } else {
                        false
                    }
                }
                MessageTarget::DirectMessage { .. } => {
                    // Only the author can delete DMs
                    false
                }
            }
        };

        if !can_delete {
            return Err(Error::PermissionDenied);
        }

        // Delete message
        MessageRepository::delete(&*self.storage, message_id).await?;

        // Emit event
        let event = Event::new(EventPayload::MessageDeleted(MessageDeletedEvent::new(
            message_id,
            message.target,
            user_id,
        )));
        self.events.dispatch(event).await;

        Ok(())
    }

    /// Get messages for a target (room or DM conversation).
    ///
    /// # Errors
    ///
    /// - `RoomNotFound` - Room doesn't exist
    /// - `NotRoomMember` - User is not a member of the room
    pub async fn get_messages(
        &self,
        user_id: UserId,
        target: MessageTarget,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        // Validate permissions and get messages based on target
        match &target {
            MessageTarget::Room { room_id } => {
                // Verify user is a member of the room
                if !MembershipRepository::is_member(&*self.storage, *room_id, user_id).await? {
                    return Err(Error::NotRoomMember);
                }
                // Get room messages
                MessageRepository::find_by_target(&*self.storage, &target, pagination).await
            }
            MessageTarget::DirectMessage { recipient } => {
                // User is always allowed to see their own DMs
                // Just verify the other user exists
                if UserRepository::find_by_id(&*self.storage, *recipient)
                    .await?
                    .is_none()
                {
                    return Err(Error::UserNotFound);
                }
                // Get DM messages in both directions (user_id <-> recipient)
                MessageRepository::find_direct_messages(
                    &*self.storage,
                    user_id,
                    *recipient,
                    pagination,
                )
                .await
            }
        }
    }

    /// Get messages for a room.
    pub async fn get_room_messages(
        &self,
        room_id: RoomId,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        MessageRepository::find_by_room(&*self.storage, room_id, pagination).await
    }

    /// Get direct messages between two users.
    pub async fn get_direct_messages(
        &self,
        user1: UserId,
        user2: UserId,
        pagination: Pagination,
    ) -> Result<Vec<Message>> {
        MessageRepository::find_direct_messages(&*self.storage, user1, user2, pagination).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::room::{RoomMembership, RoomName};
    use crate::domain::user::{Email, Role, Username};
    use crate::domain::{Room, RoomSettings, User};
    use crate::storage::sqlite::SqliteStorage;
    use crate::storage::{MembershipRepository, RoomRepository, UserRepository};

    async fn create_test_storage() -> Arc<SqliteStorage> {
        Arc::new(SqliteStorage::in_memory().await.unwrap())
    }

    async fn create_test_user(storage: &SqliteStorage, username: &str, email: &str) -> User {
        let username = Username::new(username).unwrap();
        let email = Email::new(email).unwrap();
        let user = User::new(username, email, Role::User);
        UserRepository::create(storage, &user, "hashed_password")
            .await
            .unwrap();
        user
    }

    async fn create_test_room(storage: &SqliteStorage, owner_id: UserId, name: &str) -> Room {
        let room_name = RoomName::new(name).unwrap();
        let room = Room::new(room_name, owner_id, RoomSettings::default());
        RoomRepository::create(storage, &room).await.unwrap();

        // Add owner as member
        let membership = RoomMembership::as_owner(room.id, owner_id);
        MembershipRepository::add_member(storage, &membership)
            .await
            .unwrap();

        room
    }

    fn create_messaging_service(storage: Arc<SqliteStorage>) -> MessagingService<SqliteStorage> {
        let events = EventDispatcher::new();
        MessagingService::new(storage, events)
    }

    // ========================================================================
    // Send message tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_room_message_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "sender", "sender@test.com").await;
        let room = create_test_room(&storage, user.id, "testroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let result = service.send(user.id, target, "Hello, world!").await;

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.content.as_str(), "Hello, world!");
        assert_eq!(message.author, user.id);
    }

    #[tokio::test]
    async fn test_send_dm_success() {
        let storage = create_test_storage().await;
        let sender = create_test_user(&storage, "dmsender", "dmsender@test.com").await;
        let recipient = create_test_user(&storage, "dmrecipient", "dmrecipient@test.com").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::DirectMessage {
            recipient: recipient.id,
        };
        let result = service.send(sender.id, target, "Hello DM!").await;

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.content.as_str(), "Hello DM!");
    }

    #[tokio::test]
    async fn test_send_message_empty_content() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "emptyuser", "empty@test.com").await;
        let room = create_test_room(&storage, user.id, "emptyroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let result = service.send(user.id, target, "").await;

        assert!(matches!(result, Err(Error::ContentInvalid { .. })));
    }

    #[tokio::test]
    async fn test_send_room_message_not_member() {
        let storage = create_test_storage().await;
        let owner = create_test_user(&storage, "owner", "owner@test.com").await;
        let non_member = create_test_user(&storage, "nonmember", "nonmember@test.com").await;
        let room = create_test_room(&storage, owner.id, "privateroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let result = service.send(non_member.id, target, "Hello").await;

        assert!(matches!(result, Err(Error::NotRoomMember)));
    }

    #[tokio::test]
    async fn test_send_dm_recipient_not_found() {
        let storage = create_test_storage().await;
        let sender = create_test_user(&storage, "lonelysender", "lonely@test.com").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::DirectMessage {
            recipient: UserId::new(),
        };
        let result = service.send(sender.id, target, "Hello?").await;

        assert!(matches!(result, Err(Error::UserNotFound)));
    }

    // ========================================================================
    // Edit message tests
    // ========================================================================

    #[tokio::test]
    async fn test_edit_message_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "editor", "editor@test.com").await;
        let room = create_test_room(&storage, user.id, "editroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let message = service
            .send(user.id, target, "Original content")
            .await
            .unwrap();

        let result = service.edit(user.id, message.id, "Edited content").await;

        assert!(result.is_ok());
        let edited = result.unwrap();
        assert_eq!(edited.content.as_str(), "Edited content");
        assert!(edited.is_edited);
    }

    #[tokio::test]
    async fn test_edit_message_not_author() {
        let storage = create_test_storage().await;
        let author = create_test_user(&storage, "msgauthor", "author@test.com").await;
        let other = create_test_user(&storage, "otheruser", "other@test.com").await;
        let room = create_test_room(&storage, author.id, "authroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let message = service.send(author.id, target, "My message").await.unwrap();

        let result = service.edit(other.id, message.id, "Hacked!").await;

        assert!(matches!(result, Err(Error::NotMessageAuthor)));
    }

    #[tokio::test]
    async fn test_edit_message_not_found() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "editfail", "editfail@test.com").await;
        let service = create_messaging_service(storage);

        let result = service.edit(user.id, MessageId::new(), "New content").await;

        assert!(matches!(result, Err(Error::MessageNotFound)));
    }

    #[tokio::test]
    async fn test_edit_message_empty_content() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "emptyeditor", "emptyeditor@test.com").await;
        let room = create_test_room(&storage, user.id, "emptyeditroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let message = service.send(user.id, target, "Original").await.unwrap();

        let result = service.edit(user.id, message.id, "").await;

        assert!(matches!(result, Err(Error::ContentInvalid { .. })));
    }

    // ========================================================================
    // Delete message tests
    // ========================================================================

    #[tokio::test]
    async fn test_delete_message_as_author() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "deleter", "deleter@test.com").await;
        let room = create_test_room(&storage, user.id, "delroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let message = service.send(user.id, target, "To delete").await.unwrap();

        let result = service.delete(user.id, message.id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_message_not_author_not_moderator() {
        let storage = create_test_storage().await;
        let author = create_test_user(&storage, "delauthor", "delauthor@test.com").await;
        let other = create_test_user(&storage, "delother", "delother@test.com").await;
        let room = create_test_room(&storage, author.id, "delroom2").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let message = service.send(author.id, target, "Protected").await.unwrap();

        let result = service.delete(other.id, message.id).await;

        assert!(matches!(result, Err(Error::PermissionDenied)));
    }

    #[tokio::test]
    async fn test_delete_message_not_found() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "delfail", "delfail@test.com").await;
        let service = create_messaging_service(storage);

        let result = service.delete(user.id, MessageId::new()).await;

        assert!(matches!(result, Err(Error::MessageNotFound)));
    }

    #[tokio::test]
    async fn test_delete_dm_not_author() {
        let storage = create_test_storage().await;
        let sender = create_test_user(&storage, "dmdelsender", "dmdel@test.com").await;
        let recipient = create_test_user(&storage, "dmdelrecip", "dmdelrecip@test.com").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::DirectMessage {
            recipient: recipient.id,
        };
        let message = service.send(sender.id, target, "Private").await.unwrap();

        // Recipient should not be able to delete sender's message
        let result = service.delete(recipient.id, message.id).await;

        assert!(matches!(result, Err(Error::PermissionDenied)));
    }

    // ========================================================================
    // Get messages tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_room_messages_success() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "getuser", "getuser@test.com").await;
        let room = create_test_room(&storage, user.id, "getroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        service
            .send(user.id, target.clone(), "Message 1")
            .await
            .unwrap();
        service
            .send(user.id, target.clone(), "Message 2")
            .await
            .unwrap();

        let result = service
            .get_messages(user.id, target, Pagination::default())
            .await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_room_messages_not_member() {
        let storage = create_test_storage().await;
        let owner = create_test_user(&storage, "roomowner", "roomowner@test.com").await;
        let outsider = create_test_user(&storage, "outsider", "outsider@test.com").await;
        let room = create_test_room(&storage, owner.id, "secretroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        let result = service
            .get_messages(outsider.id, target, Pagination::default())
            .await;

        assert!(matches!(result, Err(Error::NotRoomMember)));
    }

    #[tokio::test]
    async fn test_get_direct_messages_success() {
        let storage = create_test_storage().await;
        let user1 = create_test_user(&storage, "dmuser1", "dmuser1@test.com").await;
        let user2 = create_test_user(&storage, "dmuser2", "dmuser2@test.com").await;
        let service = create_messaging_service(storage);

        // User1 sends to User2
        let target1 = MessageTarget::DirectMessage {
            recipient: user2.id,
        };
        service.send(user1.id, target1, "Hi user2!").await.unwrap();

        // User2 sends to User1
        let target2 = MessageTarget::DirectMessage {
            recipient: user1.id,
        };
        service.send(user2.id, target2, "Hi user1!").await.unwrap();

        // Get messages from user1's perspective
        let target = MessageTarget::DirectMessage {
            recipient: user2.id,
        };
        let result = service
            .get_messages(user1.id, target, Pagination::default())
            .await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_dm_recipient_not_found() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "lonelyget", "lonelyget@test.com").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::DirectMessage {
            recipient: UserId::new(),
        };
        let result = service
            .get_messages(user.id, target, Pagination::default())
            .await;

        assert!(matches!(result, Err(Error::UserNotFound)));
    }

    // ========================================================================
    // Get room messages (by room_id) tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_room_messages_by_room_id() {
        let storage = create_test_storage().await;
        let user = create_test_user(&storage, "roommsguser", "roommsg@test.com").await;
        let room = create_test_room(&storage, user.id, "roommsgroom").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::Room { room_id: room.id };
        service.send(user.id, target, "Test message").await.unwrap();

        let result = service
            .get_room_messages(room.id, Pagination::default())
            .await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
    }

    // ========================================================================
    // Get direct messages (between users) tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_direct_messages_between_users() {
        let storage = create_test_storage().await;
        let user1 = create_test_user(&storage, "direct1", "direct1@test.com").await;
        let user2 = create_test_user(&storage, "direct2", "direct2@test.com").await;
        let service = create_messaging_service(storage);

        let target = MessageTarget::DirectMessage {
            recipient: user2.id,
        };
        service
            .send(user1.id, target, "Direct message")
            .await
            .unwrap();

        let result = service
            .get_direct_messages(user1.id, user2.id, Pagination::default())
            .await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
    }
}
