//! Message handlers.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::routes::AppState;
use crate::domain::{Message, MessageId, MessageTarget, Pagination, RoomId, UserId};
use crate::storage::Storage;
use crate::Error;

use super::auth::extract_session_id;
use super::SuccessResponse;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub target: MessageTargetRequest,
    pub content: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageTargetRequest {
    Room { room_id: String },
    DirectMessage { recipient: String },
}

impl TryFrom<MessageTargetRequest> for MessageTarget {
    type Error = Error;

    fn try_from(req: MessageTargetRequest) -> Result<Self, Self::Error> {
        match req {
            MessageTargetRequest::Room { room_id } => {
                let id = RoomId::parse(&room_id).map_err(|_| Error::RoomNotFound)?;
                Ok(MessageTarget::Room { room_id: id })
            }
            MessageTargetRequest::DirectMessage { recipient } => {
                let id = UserId::parse(&recipient).map_err(|_| Error::UserNotFound)?;
                Ok(MessageTarget::DirectMessage { recipient: id })
            }
        }
    }
}

#[derive(Deserialize)]
pub struct GetMessagesQuery {
    pub target_type: String,
    pub target_id: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub before: Option<String>,
    pub after: Option<String>,
}

fn default_limit() -> u32 {
    50
}

#[derive(Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: Message,
}

#[derive(Serialize)]
pub struct MessagesListResponse {
    pub messages: Vec<Message>,
    pub has_more: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// Send a message.
pub async fn send_message<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), Error> {
    let session_id = extract_session_id(&headers)?;
    let target: MessageTarget = req.target.try_into()?;

    let message = state
        .engine
        .send_message(session_id, target, &req.content)
        .await?;

    Ok((StatusCode::CREATED, Json(MessageResponse { message })))
}

/// Get messages for a target.
pub async fn get_messages<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<MessagesListResponse>, Error> {
    let session_id = extract_session_id(&headers)?;

    let target = match query.target_type.as_str() {
        "room" => {
            let id = RoomId::parse(&query.target_id).map_err(|_| Error::RoomNotFound)?;
            MessageTarget::Room { room_id: id }
        }
        "direct_message" => {
            let id = UserId::parse(&query.target_id).map_err(|_| Error::UserNotFound)?;
            MessageTarget::DirectMessage { recipient: id }
        }
        _ => {
            return Err(Error::ValidationFailed {
                field: "target_type".into(),
                reason: "must be 'room' or 'direct_message'".into(),
            })
        }
    };

    let pagination = Pagination {
        limit: query.limit.min(100),
        offset: 0,
    };

    let messages = state
        .engine
        .get_messages(session_id, target, pagination)
        .await?;

    let has_more = messages.len() == query.limit as usize;

    Ok(Json(MessagesListResponse { messages, has_more }))
}

/// Edit a message.
pub async fn edit_message<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Path(message_id): Path<String>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<MessageResponse>, Error> {
    let session_id = extract_session_id(&headers)?;
    let message_id = MessageId::parse(&message_id).map_err(|_| Error::MessageNotFound)?;

    let message = state
        .engine
        .edit_message(session_id, message_id, &req.content)
        .await?;

    Ok(Json(MessageResponse { message }))
}

/// Delete a message.
pub async fn delete_message<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Path(message_id): Path<String>,
) -> Result<Json<SuccessResponse>, Error> {
    let session_id = extract_session_id(&headers)?;
    let message_id = MessageId::parse(&message_id).map_err(|_| Error::MessageNotFound)?;

    state.engine.delete_message(session_id, message_id).await?;
    Ok(Json(SuccessResponse::ok()))
}
