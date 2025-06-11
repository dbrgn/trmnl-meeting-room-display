pub mod config;
pub mod errors;
pub mod handlers;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use log::info;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::database::Database;
use config::Config;
use handlers::{display_handler, health_handler, log_handler, setup_handler};

/// Create app for testing or production
pub fn create_app(database: Arc<Database>) -> Router {
    Router::new()
        .route("/api/setup/", get(setup_handler))
        .route("/api/display", get(display_handler))
        .route("/api/log", post(log_handler))
        .route("/health", get(health_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(
            ServiceBuilder::new().layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            ),
        )
        .with_state(database)
}

/// Start the server with the given database connection
pub async fn start_server(database: Arc<Database>) -> Result<()> {
    // Get configuration
    let config = Config::get().context("Failed to load configuration")?;

    let host = &config.server_host;
    let port = config.server_port;
    let addr = format!("{}:{}", host, port);

    info!("Starting server at http://{}", addr);

    // Create the app
    let app = create_app(database);

    // Create listener
    let listener = TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    // Start the server
    axum::serve(listener, app).await.context("Server error")
}

/// Create test app for testing
#[cfg(test)]
pub fn test_app(database: Arc<Database>) -> Router {
    create_app(database)
}
