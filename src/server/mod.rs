pub mod config;
pub mod errors;
pub mod handlers;

use actix_web::{App, HttpServer, middleware, web};
use log::info;
use std::sync::Arc;

use crate::database::Database;
use config::Config;
use handlers::{display_handler, health_handler, setup_handler};

/// Create app for testing or production
pub fn create_app(
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
        .service(web::resource("/health").route(web::get().to(health_handler)))
}

/// Start the server with the given database connection
pub async fn start_server(database: Arc<Database>) -> std::io::Result<()> {
    // Get configuration
    let config = Config::get().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to load configuration: {}", e),
        )
    })?;

    let port = config.server_port;
    info!("Starting server at http://localhost:{}", port);

    // Start HTTP server
    HttpServer::new(move || create_app(database.clone()).wrap(middleware::Logger::default()))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}

/// Create test app for testing
pub fn test_app(
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
    create_app(database)
}
