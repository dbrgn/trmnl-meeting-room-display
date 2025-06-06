use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, web};
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose};
use log::info;
use serde::{Deserialize, Serialize};

use super::config::Config;
use super::errors::AppError;
use crate::bmp::{ImageConfig, generate_bmp};
use crate::database::Database;

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
    // Get config for access token validation
    let config = Config::get()
        .map_err(|e| AppError::Config(format!("Failed to get configuration: {}", e)))?;

    // Extract device ID from header
    let device_id = req
        .headers()
        .get("ID")
        .ok_or_else(|| AppError::Auth("Missing ID header".to_string()))?
        .to_str()
        .map_err(|e| AppError::Auth(format!("Invalid ID header format: {}", e)))?
        .to_string();

    // Validate access token
    let token = req
        .headers()
        .get("Access-Token")
        .ok_or_else(|| AppError::Auth("Missing Access-Token header".to_string()))?
        .to_str()
        .map_err(|e| AppError::Auth(format!("Invalid Access-Token header format: {}", e)))?;

    if token != config.access_token {
        return Err(AppError::Auth("Invalid Access-Token".to_string()));
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

    // Check if device exists before registration to determine if it's new
    let exists = db
        .device_exists(&device_id)
        .with_context(|| format!("Failed to check if device exists: {}", device_id))
        .map_err(AppError::from)?;

    // Register device in database
    db.register_device(&device_id)
        .with_context(|| format!("Failed to register device: {}", device_id))
        .map_err(AppError::from)?;

    let message = if !exists {
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
    let config = Config::get()
        .context("Failed to get configuration")
        .map_err(AppError::from)?;

    info!("Processing display request for device: {}", device_id);

    // Check if device is registered
    let device = db
        .get_device(&device_id)
        .with_context(|| format!("Failed to check if device exists: {}", device_id))
        .map_err(AppError::from)?;
    if device.is_none() {
        return Err(AppError::Auth(format!(
            "Device {} not registered",
            device_id
        )));
    }

    // Set up image configuration using app config
    let image_config = ImageConfig {
        font_path: config.font_path.clone(),
        font_size: 50.0,
        ..ImageConfig::default()
    };

    // Generate BMP image
    let bmp_data = generate_bmp(&image_config)
        .with_context(|| format!("Failed to generate BMP image for device {}", device_id))
        .map_err(AppError::from)?;

    // Encode to base64
    let base64_image = general_purpose::STANDARD.encode(&bmp_data);
    let image_url = format!("data:image/bmp;base64,{}", base64_image);

    // Create response
    let response = DisplayResponse {
        filename: "demo.bmp".to_string(),
        image_url,
        image_url_timeout: 0,
        refresh_rate: config.refresh_rate,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Log endpoint handler - captures and logs ESP32 device requests
pub async fn log_handler(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    // Extract headers for context
    let device_id = req
        .headers()
        .get("ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Convert body to string, handling potential encoding issues
    let body_str = match std::str::from_utf8(&body) {
        Ok(s) => s.to_string(),
        Err(_) => {
            // If not valid UTF-8, show as hex for binary data
            format!(
                "Binary data ({} bytes): {:02x?}",
                body.len(),
                &body[..std::cmp::min(body.len(), 100)]
            )
        }
    };

    info!(
        "ESP32 Log Request - Device: {}, User-Agent: {}, Content-Type: {}, Body length: {} bytes",
        device_id,
        user_agent,
        content_type,
        body.len()
    );

    if !body_str.is_empty() {
        info!("ESP32 Log Body: {}", body_str);
    }

    // Log all headers for debugging
    info!("ESP32 Log Headers:");
    for (name, value) in req.headers() {
        if let Ok(value_str) = value.to_str() {
            info!("  {}: {}", name, value_str);
        }
    }

    // Return a simple success response
    HttpResponse::Ok().json(serde_json::json!({
        "status": "received",
        "message": "Log entry processed successfully"
    }))
}

/// Health check endpoint
pub async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
