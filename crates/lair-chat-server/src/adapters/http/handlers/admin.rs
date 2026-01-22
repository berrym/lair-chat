//! Admin handlers.

use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;

use crate::adapters::http::routes::AppState;
use crate::storage::Storage;
use crate::Error;

use super::auth::extract_session_id;

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
    headers: HeaderMap,
) -> Result<Json<StatsResponse>, Error> {
    let session_id = extract_session_id(&headers)?;

    let stats = state.engine.get_stats(session_id).await?;

    Ok(Json(StatsResponse {
        stats: SystemStats {
            total_users: stats.total_users,
            total_rooms: stats.total_rooms,
            online_users: stats.online_users,
        },
    }))
}
