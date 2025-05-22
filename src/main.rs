use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error, middleware, web};
use anyhow::Result;
use log::{error, info};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
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
    App::new().app_data(web::Data::new(database)).service(
        web::resource("/api/setup/")
            .route(web::get().to(setup_handler))
            .route(web::post().to(setup_handler)),
    )
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
