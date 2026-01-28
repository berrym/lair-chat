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
