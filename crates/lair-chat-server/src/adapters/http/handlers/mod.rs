//! HTTP request handlers.

pub mod admin;
pub mod auth;
pub mod health;
pub mod invitations;
pub mod messages;
pub mod rooms;
pub mod users;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::Error;

/// Standard error response format.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
}

/// Error details.
#[derive(Serialize)]
pub struct ErrorInfo {
    pub code: &'static str,
    pub message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = ErrorResponse {
            error: ErrorInfo {
                code: self.code(),
                message: self.to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}

/// Success response wrapper.
#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

impl SuccessResponse {
    pub fn ok() -> Self {
        Self { success: true }
    }
}
