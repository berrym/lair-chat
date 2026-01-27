//! Admin handlers.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::adapters::http::middleware::AuthUser;
use crate::adapters::http::routes::AppState;
use crate::storage::Storage;
use crate::Error;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct StatsResponse {
    pub stats: SystemStats,
}

#[derive(Serialize)]
pub struct SystemStats {
    pub total_users: u64,
    pub total_rooms: u64,
    pub online_users: u64,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get system statistics (admin only).
pub async fn get_stats<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
    auth: AuthUser,
) -> Result<Json<StatsResponse>, Error> {
    let stats = state.engine.get_stats(auth.session_id).await?;

    Ok(Json(StatsResponse {
        stats: SystemStats {
            total_users: stats.total_users,
            total_rooms: stats.total_rooms,
            online_users: stats.online_users,
        },
    }))
}
