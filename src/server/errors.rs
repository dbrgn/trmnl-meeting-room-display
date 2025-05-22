use actix_web::{HttpResponse, error, http::StatusCode};
use serde::Serialize;
use thiserror::Error;

use crate::bmp::BmpError;
use crate::database::DatabaseError;

/// Custom error types for the server
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Database error: {0}")]
    DbError(#[from] DatabaseError),

    #[error("Image generation error: {0}")]
    ImageError(#[from] BmpError),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// Error response structure
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

/// Implement conversion from AppError to actix_web::error::Error
impl error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
            AppError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ImageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();

        HttpResponse::build(status).json(ErrorResponse {
            error: self.to_string(),
            code: status.as_u16(),
        })
    }
}
