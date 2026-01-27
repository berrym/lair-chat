//! HTTP route definitions.

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};

use crate::core::engine::ChatEngine;
use crate::storage::Storage;

use super::handlers;
use super::middleware::{jwt_service_layer, rate_limit_middleware, RateLimiter};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState<S: Storage + Clone> {
    pub engine: Arc<ChatEngine<S>>,
}

/// Create the main router with all routes.
pub fn create_router<S: Storage + Clone + 'static>(engine: Arc<ChatEngine<S>>) -> Router {
    let state = AppState {
        engine: engine.clone(),
    };

    // Get JWT service from engine for middleware
    let jwt_service = Arc::new(engine.jwt_service().clone());

    // Create rate limiters
    let auth_limiter = Arc::new(RateLimiter::auth());
    let general_limiter = Arc::new(RateLimiter::general());

    Router::new()
        // Health endpoints (no auth required, no rate limit)
        .route("/health", get(handlers::health::health_check))
        .route("/ready", get(handlers::health::readiness_check))
        // Auth endpoints (auth rate limit, no JWT required for login/register)
        .nest(
            "/api/v1/auth",
            auth_routes().layer(middleware::from_fn({
                let limiter = auth_limiter.clone();
                move |req, next| rate_limit_middleware(limiter.clone(), req, next)
            })),
        )
        // Protected endpoints (general rate limit, JWT required)
        .nest("/api/v1/users", user_routes())
        .nest("/api/v1/rooms", room_routes())
        .nest("/api/v1/messages", message_routes())
        .nest("/api/v1/invitations", invitation_routes())
        .nest("/api/v1/admin", admin_routes())
        // Apply JWT service layer to all routes (extractors use it)
        .layer(middleware::from_fn({
            let jwt = jwt_service.clone();
            move |req, next| jwt_service_layer(jwt.clone(), req, next)
        }))
        // Apply general rate limit to protected endpoints
        .layer(middleware::from_fn({
            let limiter = general_limiter.clone();
            move |req, next| rate_limit_middleware(limiter.clone(), req, next)
        }))
        .with_state(state)
}

/// Authentication routes.
fn auth_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/logout", post(handlers::auth::logout))
        .route("/refresh", post(handlers::auth::refresh_token))
        .route("/change-password", post(handlers::auth::change_password))
}

/// User routes.
fn user_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/me", get(handlers::users::get_current_user))
        .route("/me", patch(handlers::users::update_profile))
        .route("/", get(handlers::users::list_users))
        .route("/{user_id}", get(handlers::users::get_user))
}

/// Room routes.
fn room_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/", post(handlers::rooms::create_room))
        .route("/", get(handlers::rooms::list_rooms))
        .route("/{room_id}", get(handlers::rooms::get_room))
        .route("/{room_id}", patch(handlers::rooms::update_room))
        .route("/{room_id}", delete(handlers::rooms::delete_room))
        .route("/{room_id}/join", post(handlers::rooms::join_room))
        .route("/{room_id}/leave", post(handlers::rooms::leave_room))
        .route("/{room_id}/members", get(handlers::rooms::get_members))
}

/// Message routes.
fn message_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/", post(handlers::messages::send_message))
        .route("/", get(handlers::messages::get_messages))
        .route("/{message_id}", patch(handlers::messages::edit_message))
        .route("/{message_id}", delete(handlers::messages::delete_message))
}

/// Invitation routes.
fn invitation_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/", post(handlers::invitations::create_invitation))
        .route("/", get(handlers::invitations::list_invitations))
        .route(
            "/{invitation_id}/accept",
            post(handlers::invitations::accept_invitation),
        )
        .route(
            "/{invitation_id}/decline",
            post(handlers::invitations::decline_invitation),
        )
}

/// Admin routes.
fn admin_routes<S: Storage + Clone + 'static>() -> Router<AppState<S>> {
    Router::new().route("/stats", get(handlers::admin::get_stats))
}
