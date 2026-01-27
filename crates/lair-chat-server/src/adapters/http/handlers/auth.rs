//! Authentication handlers.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::http::middleware::AuthUser;
use crate::adapters::http::routes::AppState;
use crate::domain::{Session, User};
use crate::storage::Storage;
use crate::Error;

use super::SuccessResponse;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub session: SessionInfo,
    pub token: String,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub expires_at: String,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.to_string(),
            expires_at: session.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new user.
pub async fn register<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), Error> {
    let (user, session, token) = state
        .engine
        .register(&req.username, &req.email, &req.password)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user,
            session: SessionInfo::from(&session),
            token,
        }),
    ))
}

/// Login with username/email and password.
pub async fn login<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, Error> {
    let (user, session, token) = state.engine.login(&req.identifier, &req.password).await?;

    Ok(Json(AuthResponse {
        user,
        session: SessionInfo::from(&session),
        token,
    }))
}

/// Logout (invalidate session).
pub async fn logout<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
) -> Result<Json<SuccessResponse>, Error> {
    state.engine.logout(auth.session_id).await?;
    Ok(Json(SuccessResponse::ok()))
}

/// Refresh the JWT token.
pub async fn refresh_token<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
) -> Result<Json<TokenResponse>, Error> {
    let (session, _user) = state.engine.validate_session(auth.session_id).await?;
    let token = state.engine.refresh_token(auth.session_id).await?;

    Ok(Json(TokenResponse {
        token,
        expires_at: session.expires_at.to_rfc3339(),
    }))
}

/// Change password.
pub async fn change_password<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<SuccessResponse>, Error> {
    state
        .engine
        .change_password(auth.session_id, &req.current_password, &req.new_password)
        .await?;
    Ok(Json(SuccessResponse::ok()))
}
