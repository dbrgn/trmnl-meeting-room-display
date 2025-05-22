pub mod errors;
pub mod handlers;

use actix_web::{App, HttpServer, middleware, web};
use std::sync::Arc;

use crate::database::Database;
use handlers::{display_handler, setup_handler};

// Configuration constants
pub const SERVER_PORT: u16 = 8080;

/// Create app for testing
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
    App::new()
        .app_data(web::Data::new(database))
        .service(web::resource("/api/setup/").route(web::get().to(setup_handler)))
        .service(web::resource("/api/display").route(web::get().to(display_handler)))
}

/// Start the server
pub async fn start_server(database: Arc<Database>) -> std::io::Result<()> {
    log::info!("Starting server at http://localhost:{}", SERVER_PORT);

    // Start HTTP server
    HttpServer::new(move || test_app(database.clone()).wrap(middleware::Logger::default()))
        .bind(("127.0.0.1", SERVER_PORT))?
        .run()
        .await
}
