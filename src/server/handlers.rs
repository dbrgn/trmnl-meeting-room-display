use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, web};
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

use super::config::Config;
use super::errors::AppError;
use crate::bmp::{ImageConfig, generate_bmp};
use crate::database::Database;

// Success response structure
#[derive(Serialize)]
pub struct SetupResponse {
    /// Status code, should be 200
    pub status: u16,
    /// API key for the device
    pub api_key: String,
    /// Friendly ID for the device
    pub friendly_id: String,
    /// Image to show on the setup screen (BMP, 800x480px)
    pub image_url: String,
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

/// Extract device ID from headers
pub fn extract_device_id(req: &HttpRequest) -> Result<String, AppError> {
    Ok(req
        .headers()
        .get("ID")
        .ok_or_else(|| AppError::Auth("Missing ID header".to_string()))?
        .to_str()
        .map_err(|e| AppError::Auth(format!("Invalid ID header format: {}", e)))?
        .to_string())
}

/// Extract and validate access token in headers
pub fn validate_headers(req: &HttpRequest, config: &Config) -> Result<(), AppError> {
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

    Ok(())
}

/// Setup endpoint handler
pub async fn setup_handler(
    req: HttpRequest,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let config = Config::get()
        .map_err(|e| AppError::Config(format!("Failed to get configuration: {}", e)))?;

    let device_id = extract_device_id(&req)?;

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
    if !exists {
        info!("Device {} registered successfully", device_id)
    } else {
        info!("Device {} registration updated", device_id)
    };

    Ok(HttpResponse::Ok().json(SetupResponse {
        status: 200,
        api_key: "my-api-key".into(),
        friendly_id: "TRMNL001".into(),
        image_url: "/assets/setup-logo.bmp".into(),
    }))
}

/// Display endpoint handler
pub async fn display_handler(
    req: HttpRequest,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let config = Config::get()
        .context("Failed to get configuration")
        .map_err(AppError::from)?;

    validate_headers(&req, &config)?;

    let device_id = extract_device_id(&req)?;

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

/// Log endpoint handler - captures and logs device log requests
pub async fn log_handler(req: HttpRequest, body: web::Bytes) -> Result<HttpResponse, AppError> {
    // Note: Not validating access token for this endpoint, since we want to
    // capture logs even for misconfigured devices.

    // Extract headers
    let device_id = req
        .headers()
        .get("ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Only accept text or JSON
    if !matches!(content_type, "application/json" | "text/plain") {
        warn!("Invalid content type for log request: {}", content_type);
        return Err(AppError::BadRequest(format!(
            "Invalid content type: {content_type}"
        )));
    }

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
        "Log request: Device: {}, Content-Type: {}, Body length: {} bytes",
        device_id,
        content_type,
        body.len()
    );

    // Append log entry to file
    if !body_str.is_empty() {
        let log_entry = format!("[{}] {}\n", device_id, body_str);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("device-log.txt")
        {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                error!("Failed to write log entry: {}", e);
            }
        }
    }

    // Return a simple success response
    Ok(HttpResponse::NoContent().finish())
}

/// Health check endpoint
pub async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
