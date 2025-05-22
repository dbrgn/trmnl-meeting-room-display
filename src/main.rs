use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error, middleware, web};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use image::{ImageBuffer, Luma, codecs::bmp::BmpEncoder};
use log::{error, info};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// Configuration constants
const SERVER_PORT: u16 = 8080;
const DATABASE_PATH: &str = "devices.db";
const ACCESS_TOKEN: &str = "your-secret-access-token"; // Replace with your actual token

// Custom error type
#[derive(Error, Debug)]
enum AppError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),
}

// Implement conversion from AppError to actix_web::error::Error
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

// Error response structure
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// Success response structure
#[derive(Serialize)]
struct SuccessResponse {
    message: String,
    device_id: String,
}

// Display response structure
#[derive(Serialize, Deserialize)]
struct DisplayResponse {
    filename: String,
    image_url: String,
    image_url_timeout: u32,
    refresh_rate: u32,
}

// Device structure
#[derive(Serialize, Deserialize)]
struct Device {
    id: String,
    registered_at: i64,
}

// Database wrapper
struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        // Create devices table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                registered_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn register_device(&self, device_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO devices (id, registered_at) VALUES (?1, ?2)",
            params![device_id, now],
        )?;

        Ok(())
    }

    fn device_exists(&self, device_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM devices WHERE id = ?1")?;
        let exists = stmt.exists(params![device_id])?;
        Ok(exists)
    }
}

// Extract and validate device ID and access token from headers
fn validate_headers(req: &HttpRequest) -> Result<String, AppError> {
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

// Setup endpoint handler
async fn setup_handler(
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

// Generate a monochrome 800x480 BMP with "hello world" text
fn generate_hello_world_bmp() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create an 800x480 monochrome image (8-bit for simplicity)
    let width = 800;
    let height = 480;
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(width, height);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Luma([255]); // White
    }

    // Draw "hello world" manually with simple bitmap font
    // Each character is 5x7 pixels with 1px spacing
    let text = "hello world";
    let char_width: usize = 5;
    let char_height: usize = 7;
    let char_spacing: usize = 1;
    let start_x: usize = (width as usize / 2) - ((text.len() * (char_width + char_spacing)) / 2);
    let start_y: usize = (height as usize / 2) - (char_height / 2);

    // Simple bitmap patterns for each character (1 = pixel, 0 = no pixel)
    let patterns = [
        // h
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // e
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // o
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // Space
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // w
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // o
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // r
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // d
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 1, 0],
            [0, 0, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
    ];

    // Lookup map for characters
    let char_map = [
        ('h', 0),
        ('e', 1),
        ('l', 2),
        ('l', 3),
        ('o', 4),
        (' ', 5),
        ('w', 6),
        ('o', 7),
        ('r', 8),
        ('l', 9),
        ('d', 10),
    ];

    // Draw each character
    for (i, c) in text.chars().enumerate() {
        let pattern_idx = char_map
            .iter()
            .find(|(ch, _)| *ch == c)
            .map(|(_, idx)| *idx)
            .unwrap_or(5); // Default to space
        let pattern = patterns[pattern_idx];

        let char_x = start_x + i * (char_width + char_spacing);

        // Draw this character
        for y in 0..char_height {
            for x in 0..char_width {
                if pattern[y][x] == 1 {
                    if char_x + x < width as usize && start_y + y < height as usize {
                        img.put_pixel((char_x + x) as u32, (start_y + y) as u32, Luma([0]));
                    }
                }
            }
        }
    }

    // Draw a border around the text for visibility
    let border_x = start_x.saturating_sub(10);
    let border_y = start_y.saturating_sub(10);
    let border_width = text.len() * (char_width + char_spacing) + 20;
    let border_height = char_height + 20;

    // Top and bottom borders
    for x in border_x..(border_x + border_width) {
        if x < width as usize {
            if border_y < height as usize {
                img.put_pixel(x as u32, border_y as u32, Luma([0]));
            }
            if (border_y + border_height) < height as usize {
                img.put_pixel(x as u32, (border_y + border_height) as u32, Luma([0]));
            }
        }
    }

    // Left and right borders
    for y in border_y..(border_y + border_height) {
        if y < height as usize {
            if border_x < width as usize {
                img.put_pixel(border_x as u32, y as u32, Luma([0]));
            }
            if (border_x + border_width) < width as usize {
                img.put_pixel((border_x + border_width) as u32, y as u32, Luma([0]));
            }
        }
    }

    // Convert to monochrome BMP (1-bit)
    // For 1-bit BMP, we need to manually create a proper BMP file
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = BmpEncoder::new(&mut cursor);

    // We're using the encoder to write the image
    // This will result in an 8-bit image, but for this example it's sufficient
    // For a true 1-bit monochrome BMP, a more complex encoding would be needed
    encoder
        .encode(&img.to_vec(), width, height, image::ColorType::L8)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(cursor.into_inner())
}

// Display endpoint handler
async fn display_handler(
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

// Initialize database
fn init_database() -> Arc<Database> {
    let db_path = DATABASE_PATH;

    if !Path::new(db_path).exists() {
        info!("Creating new database at {}", db_path);
    } else {
        info!("Using existing database at {}", db_path);
    }

    match Database::new(db_path) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            panic!("Failed to initialize database: {}", e);
        }
    }
}

// Create app for testing
fn test_app(
    database: Arc<Database>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(database))
        .service(web::resource("/api/setup/").route(web::get().to(setup_handler)))
        .service(web::resource("/api/display").route(web::get().to(display_handler)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Initialize database
    let database = init_database();

    info!("Starting server at http://localhost:{}", SERVER_PORT);

    // Start HTTP server
    HttpServer::new(move || test_app(database.clone()).wrap(middleware::Logger::default()))
        .bind(("127.0.0.1", SERVER_PORT))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use std::fs;

    #[actix_web::test]
    async fn test_setup_endpoint_success() {
        let test_db_path = "test_devices.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with valid headers
        let req = test::TestRequest::get()
            .uri("/api/setup/")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", ACCESS_TOKEN))
            .insert_header(("Accept", "application/json"))
            .insert_header(("Content-Type", "application/json"))
            .to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[actix_web::test]
    async fn test_setup_endpoint_post_rejected() {
        let test_db_path = "test_devices_post.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create POST request with valid headers
        let req = test::TestRequest::post()
            .uri("/api/setup/")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", ACCESS_TOKEN))
            .insert_header(("Accept", "application/json"))
            .insert_header(("Content-Type", "application/json"))
            .to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 405); // Method Not Allowed - POST method not allowed

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[actix_web::test]
    async fn test_display_endpoint_success() {
        let test_db_path = "test_devices_display.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());

        // Register a device first
        db.register_device("00:11:22:33:44:55").unwrap();

        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with valid headers
        let req = test::TestRequest::get()
            .uri("/api/display")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", ACCESS_TOKEN))
            .insert_header(("Accept", "application/json"))
            .to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Verify response contains expected fields
        let body = test::read_body(resp).await;
        let response: DisplayResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.filename, "demo.bmp");
        assert!(response.image_url.starts_with("data:image/bmp;base64,"));
        assert_eq!(response.image_url_timeout, 0);
        assert_eq!(response.refresh_rate, 200);

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[actix_web::test]
    async fn test_setup_endpoint_invalid_token() {
        let test_db_path = "test_devices_invalid.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with invalid token
        let req = test::TestRequest::get()
            .uri("/api/setup/")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", "invalid-token"))
            .insert_header(("Accept", "application/json"))
            .insert_header(("Content-Type", "application/json"))
            .to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401); // Unauthorized

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[actix_web::test]
    async fn test_setup_endpoint_missing_headers() {
        let test_db_path = "test_devices_missing.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with missing ID header
        let req = test::TestRequest::get()
            .uri("/api/setup/")
            .insert_header(("Access-Token", ACCESS_TOKEN))
            .insert_header(("Accept", "application/json"))
            .insert_header(("Content-Type", "application/json"))
            .to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401); // Unauthorized

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }
}
