#![forbid(unsafe_code)]
#![allow(dead_code)]

// In the name of the Father, and in the name of the Son, and in the name of the Holy Spirit. Amen.

/// External crates for logging functionality
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod base64;
mod infrastructure;
mod seed;
mod tls;
mod traits;
mod use_case;

use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse, get, web};
use anyhow::Result;
use infrastructure::database::PostgresDatabase;
use infrastructure::websocket::WebSocketService;
use seed::entity::websocket::{WebSocketConnection, WebSocketManager};
use use_case::messages::MessagesUseCase;

/// Main application entry point
///
/// Sets up the following components:
/// - Logging system
/// - TLS configuration
/// - Database connection pool
/// - Use cases and business logic
/// - WebSocket service
/// - HTTP server with WebSocket endpoint
///
/// # Returns
///
/// Returns a `Result` that indicates whether the application started successfully
#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize the logging system
    pretty_env_logger::init();

    // Load TLS configuration for secure connections
    let tls_config = tls::load_rustls_config()?;

    // Get the server port from environment variables or use default 8080
    let port = match std::env::var("PORT") {
        Ok(port_str) => port_str.parse().unwrap_or(8080),
        Err(_) => 8080,
    };

    // Initialize database connection pool
    let pg_pool = PostgresDatabase::new().await?;

    // Set up application use cases
    let messages_use_case = use_case::messages::MessagesUseCase::new(pg_pool);
    let websocket_use_case =
        use_case::websocket::WebSocketUseCase::new(messages_use_case.clone()).await;
    let websocket_manager = WebSocketManager::new();

    // Create the WebSocket service to handle connections
    let websocket_service = infrastructure::websocket::WebSocketService::new(
        websocket_manager,
        websocket_use_case,
        messages_use_case,
    );

    // Configure and start the HTTP server
    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(Data::new(websocket_service.clone()))
            .service(accept_websocket_connection)
    })
    .bind_rustls_0_23(format!("127.0.0.1:{port}"), tls_config)?
    .run();

    // Wait for the server to complete
    server.await?;

    Ok(())
}

/// WebSocket connection handler endpoint
///
/// This endpoint handles incoming WebSocket connection requests and passes them
/// to the WebSocket service for processing.
///
/// # Parameters
///
/// * `req` - The HTTP request containing WebSocket upgrade information
/// * `payload` - The request payload stream
/// * `websocket_service` - The WebSocket service that will handle the connection
///
/// # Returns
///
/// Returns an HTTP response that completes the WebSocket handshake
#[get("/ws")]
async fn accept_websocket_connection(
    req: HttpRequest,
    payload: web::Payload,
    websocket_service: web::Data<
        WebSocketService<MessagesUseCase<PostgresDatabase>, PostgresDatabase>,
    >,
) -> actix_web::Result<HttpResponse> {
    // Create a new WebSocket connection from the request
    let (response, conn, stream) = WebSocketConnection::new(&req, payload)?;
    debug!("New websocket connection");

    // Process the connection in a new task
    actix_web::rt::spawn(async move { websocket_service.handle_connection(conn, stream).await });

    // Return the WebSocket handshake response
    Ok(response)
}
