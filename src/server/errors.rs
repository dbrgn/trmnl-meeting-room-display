use actix_web::{HttpResponse, error};
use rusqlite;
use serde::Serialize;
use thiserror::Error;

/// Custom error types for the server
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),
}

/// Error response structure
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Implement conversion from AppError to actix_web::error::Error
impl error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::AuthError(_) => HttpResponse::Unauthorized().json(ErrorResponse {
                error: self.to_string(),
            }),
            _ => HttpResponse::InternalServerError().json(ErrorResponse {
                error: self.to_string(),
            }),
        }
    }
}
