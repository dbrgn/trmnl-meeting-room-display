use std::process;

use anyhow::Context;
use log::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    database::init_database,
    server::{config::Config, start_server},
};

mod bmp;
mod calendar;
mod database;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger and tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize configuration
    if let Err(e) = Config::init().context("Failed to initialize configuration") {
        error!("Configuration error: {:#}", e);
        process::exit(1);
    }

    let config = Config::get().expect("Configuration should be initialized");
    info!("Configuration loaded successfully");
    info!("Server host: {}", config.server_host);
    info!("Server port: {}", config.server_port);
    info!("Server URL: {}", config.server_url);
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
    if let Err(e) = start_server(database).await {
        error!("Server error: {:#}", e);
        return Err(e.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use dotenv::dotenv;
    use tower::util::ServiceExt;

    use crate::{
        database::Database,
        server::{
            handlers::{DisplayResponse, SetupResponse},
            test_app,
        },
    };

    /// Helper function to get the access token for tests
    fn get_test_access_token() -> String {
        dotenv().ok();
        std::env::var("ACCESS_TOKEN").unwrap_or_else(|_| "your-secret-access-token".to_string())
    }

    #[tokio::test]
    async fn test_setup_endpoint_success() {
        let test_db_path = "test_devices.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request with valid headers
        let req = Request::builder()
            .uri("/api/setup/")
            .method("GET")
            .header("ID", "00:11:22:33:44:55")
            .header("Access-Token", access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_setup_endpoint_post_rejected() {
        let test_db_path = "test_devices_post.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create POST request with valid headers
        let req = Request::builder()
            .uri("/api/setup/")
            .method("POST")
            .header("ID", "00:11:22:33:44:55")
            .header("Access-Token", access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED); // Method Not Allowed - POST method not allowed

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_display_endpoint_success() {
        let test_db_path = "test_devices_display.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());

        // Register a device first
        db.register_device("00:11:22:33:44:55").unwrap();

        let app = test_app(db.clone());

        // Create test request with valid headers
        let req = Request::builder()
            .uri("/api/display")
            .method("GET")
            .header("ID", "00:11:22:33:44:55")
            .header("Access-Token", access_token)
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Verify response contains expected fields
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: DisplayResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.filename, "demo.bmp");
        assert!(response.image_url.starts_with("data:image/bmp;base64,"));
        assert_eq!(response.image_url_timeout, 0);

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_setup_endpoint_invalid_token() {
        let test_db_path = "test_devices_invalid.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request with invalid token
        let req = Request::builder()
            .uri("/api/setup/")
            .method("GET")
            .header("ID", "00:11:22:33:44:55")
            .header("Access-Token", "invalid-token")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED); // Unauthorized

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_setup_endpoint_missing_headers() {
        let test_db_path = "test_devices_missing.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request with missing ID header
        let req = Request::builder()
            .uri("/api/setup/")
            .method("GET")
            .header("Access-Token", access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED); // Unauthorized

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let test_db_path = "test_health.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request
        let req = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_static_file_endpoint() {
        let test_db_path = "test_static.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request for static BMP file
        let req = Request::builder()
            .uri("/static/setup-logo.bmp")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Verify that we get BMP file content back with correct magic bytes
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let bytes = body.to_vec();

        // Check that file starts with BMP magic bytes (0x42 0x4d = "BM")
        assert!(bytes.len() >= 2);
        assert_eq!(bytes[0], 0x42);
        assert_eq!(bytes[1], 0x4d);

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }

    #[tokio::test]
    async fn test_setup_endpoint_returns_full_url() {
        let test_db_path = "test_setup_url.db";
        let access_token = get_test_access_token();

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = Arc::new(Database::new(test_db_path).unwrap());
        let app = test_app(db.clone());

        // Create test request with valid headers
        let req = Request::builder()
            .uri("/api/setup/")
            .method("GET")
            .header("ID", "00:11:22:33:44:55")
            .header("Access-Token", access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // Send request and get response
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Verify response contains full URL for image_url
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: SetupResponse = serde_json::from_slice(&body).unwrap();

        // Check that image_url contains the full server URL
        assert_eq!(
            response.image_url,
            "http://127.0.0.1:8080/static/setup-logo.bmp"
        );
        assert!(response.image_url.starts_with("http://"));

        // Clean up
        let _ = fs::remove_file(test_db_path);
    }
}
