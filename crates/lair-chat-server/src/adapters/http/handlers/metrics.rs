//! Prometheus metrics endpoint.
//!
//! Exposes application metrics in Prometheus format at `/metrics`.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusHandle;

use crate::adapters::http::routes::AppState;
use crate::storage::Storage;

/// Shared state for the metrics endpoint.
#[derive(Clone)]
pub struct MetricsState {
    pub handle: Arc<PrometheusHandle>,
}

/// Initialize metrics recorder and return handle for rendering.
///
/// Call this once at application startup. Returns a handle that can be
/// used to render metrics in Prometheus format.
///
/// Returns `None` if a recorder has already been installed (e.g., in tests).
pub fn init_metrics() -> Option<PrometheusHandle> {
    // Initialize metric descriptions
    describe_gauge!("lair_chat_online_users", "Number of currently online users");
    describe_gauge!("lair_chat_total_users", "Total number of registered users");
    describe_gauge!("lair_chat_total_rooms", "Total number of rooms");

    // Create and install the Prometheus recorder
    // Returns None if a recorder is already installed (e.g., in another test)
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    builder.install_recorder().ok()
}

/// Handler for GET /metrics.
///
/// Returns metrics in Prometheus exposition format.
pub async fn get_metrics<S: Storage + Clone + 'static>(
    State(state): State<AppState<S>>,
) -> Response {
    // Update gauges with current values
    let online_count = state.engine.online_user_count().await as f64;
    gauge!("lair_chat_online_users").set(online_count);

    // Update total users count
    if let Ok(total_users) = state.engine.total_user_count().await {
        gauge!("lair_chat_total_users").set(total_users as f64);
    }

    // Update total rooms count
    if let Ok(total_rooms) = state.engine.total_room_count().await {
        gauge!("lair_chat_total_rooms").set(total_rooms as f64);
    }

    // Render metrics using the prometheus handle stored in state
    match &state.metrics_handle {
        Some(handle) => {
            let output = handle.render();
            (
                StatusCode::OK,
                [(
                    header::CONTENT_TYPE,
                    "text/plain; version=0.0.4; charset=utf-8",
                )],
                output,
            )
                .into_response()
        }
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Metrics exporter not initialized",
        )
            .into_response(),
    }
}
