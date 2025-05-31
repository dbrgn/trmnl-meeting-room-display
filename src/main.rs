mod bmp;
mod calendar;
mod database;
mod server;

use std::process;

use anyhow::Context;
use log::{error, info};

use crate::database::init_database;
use crate::server::{config::Config, start_server};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Initialize configuration
    if let Err(e) = Config::init().context("Failed to initialize configuration") {
        error!("Configuration error: {:#}", e);
        process::exit(1);
    }

    let config = Config::get().expect("Configuration should be initialized");
    info!("Configuration loaded successfully");
    info!("Server port: {}", config.server_port);
    info!("Database path: {}", config.database_path);
    info!("Font path: {}", config.font_path);

    // Initialize database
    let database =
        match init_database(&config.database_path).context("Failed to initialize database") {
            Ok(db) => {
                info!("Database initialized successfully");
                db
            }
            Err(e) => {
                error!("Database initialization error: {:#}", e);
                process::exit(1);
            }
        };

    // Start the web server
    info!("Starting server...");
    start_server(database).await.map_err(|e| {
        error!("Server error: {:#}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use actix_web::test;
    use dotenv::dotenv;

    use crate::database::Database;
    use crate::server::handlers::DisplayResponse;
    use crate::server::test_app;

    // Helper function to get the access token for tests
    fn get_test_access_token() -> String {
        dotenv().ok();
        std::env::var("ACCESS_TOKEN").unwrap_or_else(|_| "your-secret-access-token".to_string())
    }

    #[actix_web::test]
    async fn test_setup_endpoint_success() {
        let test_db_path = "test_devices.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with valid headers
        let req = test::TestRequest::get()
            .uri("/api/setup/")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", access_token))
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
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create POST request with valid headers
        let req = test::TestRequest::post()
            .uri("/api/setup/")
            .insert_header(("ID", "00:11:22:33:44:55"))
            .insert_header(("Access-Token", access_token))
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
        let access_token = get_test_access_token();

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
            .insert_header(("Access-Token", access_token))
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
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request with missing ID header
        let req = test::TestRequest::get()
            .uri("/api/setup/")
            .insert_header(("Access-Token", access_token))
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
    async fn test_health_endpoint() {
        let test_db_path = "test_health.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test::init_service(test_app(db.clone())).await;

        // Create test request
        let req = test::TestRequest::get().uri("/health").to_request();

        // Send request and get response
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }
}
