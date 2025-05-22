use actix_web::{HttpResponse, error, http::StatusCode};
use anyhow::Error as AnyhowError;
use serde::Serialize;
use thiserror::Error;

/// Custom API error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Anyhow(#[from] AnyhowError),
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
            AppError::Auth(_) => StatusCode::UNAUTHORIZED,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
