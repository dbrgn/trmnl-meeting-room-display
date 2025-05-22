mod bmp;
mod database;
mod server;

use crate::database::init_database;
use crate::server::start_server;
use env_logger;

// Configuration constants
const DATABASE_PATH: &str = "devices.db";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Initialize database
    let database = init_database(DATABASE_PATH);

    // Start the web server
    start_server(database).await
}

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::server::handlers::DisplayResponse;
    use crate::server::{handlers::ACCESS_TOKEN, test_app};
    use actix_web::test;
    use std::fs;

    #[actix_web::test]
    async fn test_setup_endpoint_success() {
        let test_db_path = "test_devices.db";

        // Ensure test database doesn't exist
        let _ = fs::remove_file(test_db_path);

        let db = std::sync::Arc::new(Database::new(test_db_path).unwrap());
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

        let db = std::sync::Arc::new(Database::new(test_db_path).unwrap());
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

        let db = std::sync::Arc::new(Database::new(test_db_path).unwrap());

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

        let db = std::sync::Arc::new(Database::new(test_db_path).unwrap());
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

        let db = std::sync::Arc::new(Database::new(test_db_path).unwrap());
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
