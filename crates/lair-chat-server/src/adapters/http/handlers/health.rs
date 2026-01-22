//! Health check handlers.

use axum::Json;
use serde::Serialize;

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Readiness check response.
#[derive(Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub database: &'static str,
}

/// Health check endpoint.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness check endpoint.
pub async fn readiness_check() -> Json<ReadinessResponse> {
    Json(ReadinessResponse {
        ready: true,
        database: "connected",
    })
}
