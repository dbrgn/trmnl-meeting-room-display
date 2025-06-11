use anyhow::Error as AnyhowError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// Custom API error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("{0}")]
    Anyhow(#[from] AnyhowError),
}

/// Error response structure
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

/// Implement conversion from AppError to axum::response::Response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Auth(_) => StatusCode::UNAUTHORIZED,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_response = ErrorResponse {
            error: self.to_string(),
            code: status.as_u16(),
        };

        (status, Json(error_response)).into_response()
    }
}
