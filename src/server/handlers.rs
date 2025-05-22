use actix_web::{HttpRequest, HttpResponse, web};
use base64::{Engine as _, engine::general_purpose};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::errors::AppError;
use crate::bmp::generate_hello_world_bmp;
use crate::database::Database;

// Configuration constants
pub const ACCESS_TOKEN: &str = "your-secret-access-token"; // Replace with your actual token

// Success response structure
#[derive(Serialize)]
pub struct SuccessResponse {
    pub message: String,
    pub device_id: String,
}

// Display response structure
#[derive(Serialize, Deserialize)]
pub struct DisplayResponse {
    pub filename: String,
    pub image_url: String,
    pub image_url_timeout: u32,
    pub refresh_rate: u32,
}

// Device structure
#[derive(Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub registered_at: i64,
}

/// Extract and validate device ID and access token from headers
pub fn validate_headers(req: &HttpRequest) -> Result<String, AppError> {
    // Extract device ID from header
    let device_id = req
        .headers()
        .get("ID")
        .ok_or_else(|| AppError::AuthError("Missing ID header".to_string()))?
        .to_str()
        .map_err(|_| AppError::AuthError("Invalid ID header format".to_string()))?
        .to_string();

    // Validate access token
    let token = req
        .headers()
        .get("Access-Token")
        .ok_or_else(|| AppError::AuthError("Missing Access-Token header".to_string()))?
        .to_str()
        .map_err(|_| AppError::AuthError("Invalid Access-Token header format".to_string()))?;

    if token != ACCESS_TOKEN {
        return Err(AppError::AuthError("Invalid Access-Token".to_string()));
    }

    Ok(device_id)
}

/// Setup endpoint handler
pub async fn setup_handler(
    req: HttpRequest,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let device_id = validate_headers(&req)?;

    info!("Processing setup request for device: {}", device_id);

    // Register device in database
    db.register_device(&device_id)?;

    let is_new = !db.device_exists(&device_id)?;

    let message = if is_new {
        format!("Device {} registered successfully", device_id)
    } else {
        format!("Device {} registration updated", device_id)
    };

    Ok(HttpResponse::Ok().json(SuccessResponse { message, device_id }))
}

/// Display endpoint handler
pub async fn display_handler(
    req: HttpRequest,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let device_id = validate_headers(&req)?;

    info!("Processing display request for device: {}", device_id);

    // Check if device is registered
    if !db.device_exists(&device_id)? {
        return Err(AppError::AuthError(format!(
            "Device {} not registered",
            device_id
        )));
    }

    // Generate BMP image
    let bmp_data = generate_hello_world_bmp().map_err(|e| {
        error!("Error generating BMP: {}", e);
        AppError::DbError(rusqlite::Error::ExecuteReturnedResults)
    })?;

    // Encode to base64
    let base64_image = general_purpose::STANDARD.encode(&bmp_data);
    let image_url = format!("data:image/bmp;base64,{}", base64_image);

    // Create response
    let response = DisplayResponse {
        filename: "demo.bmp".to_string(),
        image_url,
        image_url_timeout: 0,
        refresh_rate: 200,
    };

    Ok(HttpResponse::Ok().json(response))
}
