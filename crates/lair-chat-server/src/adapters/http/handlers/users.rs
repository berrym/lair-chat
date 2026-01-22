//! User handlers.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::adapters::http::routes::AppState;
use crate::domain::{Pagination, User, UserId};
use crate::storage::Storage;
use crate::Error;

use super::auth::extract_session_id;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct ListUsersQuery {
    pub search: Option<String>,
    #[serde(default)]
    pub online_only: bool,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    50
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub user: User,
}

#[derive(Serialize)]
pub struct UsersListResponse {
    pub users: Vec<UserWithStatus>,
    pub has_more: bool,
    pub total_count: u32,
}

#[derive(Serialize)]
pub struct UserWithStatus {
    pub user: User,
    pub online: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get the current authenticated user.
pub async fn get_current_user<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, Error> {
    let session_id = extract_session_id(&headers)?;
    let user = state.engine.get_current_user(session_id).await?;
    Ok(Json(UserResponse { user }))
}

/// Update the current user's profile.
pub async fn update_profile<S: Storage + Clone + 'static>(
    State(_state): State<AppState<S>>,
    headers: HeaderMap,
    Json(_req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, Error> {
    let _session_id = extract_session_id(&headers)?;
    // TODO: Implement profile update
    Err(Error::Internal("Not implemented".into()))
}

/// Get a user by ID.
pub async fn get_user<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<UserWithStatus>, Error> {
    let _session_id = extract_session_id(&headers)?;
    let user_id = UserId::parse(&user_id).map_err(|_| Error::UserNotFound)?;

    let user = state
        .engine
        .get_user(user_id)
        .await?
        .ok_or(Error::UserNotFound)?;

    Ok(Json(UserWithStatus {
        user,
        online: false, // TODO: Check actual online status
    }))
}

/// List users with filtering.
pub async fn list_users<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<UsersListResponse>, Error> {
    let _session_id = extract_session_id(&headers)?;

    let pagination = Pagination {
        limit: query.limit.min(100),
        offset: query.offset,
    };

    let users = state.engine.list_users(pagination).await?;
    let has_more = users.len() == query.limit as usize;
    let total_count = users.len() as u32;

    let users: Vec<UserWithStatus> = users
        .into_iter()
        .map(|user| UserWithStatus {
            user,
            online: false,
        })
        .collect();

    Ok(Json(UsersListResponse {
        users,
        has_more,
        total_count,
    }))
}
