#![forbid(unsafe_code)]
#![allow(dead_code)]

// In the name of the Father, and in the name of the Son, and in the name of the Holy Spirit. Amen.

/// External crates for logging functionality
extern crate log;
extern crate pretty_env_logger;

use std::sync::Arc;

use anyhow::Result;
use infrastructure::database::PostgresDatabase;
use infrastructure::websocket::WebSocketService;
use log::error;
use protocol::entity::websocket::{WebSocketConnection, WebSocketManager};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        http::{StatusCode},
    },
};
use traits::message::{MessagesDB, MessagesRepository};

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
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logging system
    pretty_env_logger::init();

    // Get the server port from environment variables or use default 8080
    let port = match std::env::var("PORT") {
        Ok(port_str) => port_str.parse().unwrap_or(8080),
        Err(_) => 8080,
    };

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"));

    // Initialize database connection pool
    let pg_pool = PostgresDatabase::new().await?;

    // Set up application use cases
    let messages_use_case = use_case::messages::MessagesUseCase::new(pg_pool);
    let websocket_use_case =
        use_case::websocket::WebSocketUseCase::new(messages_use_case.clone()).await;
    let websocket_manager = WebSocketManager::default();

    // Create the WebSocket service to handle connections
    let websocket_service = Arc::new(infrastructure::websocket::WebSocketService::new(
        websocket_manager,
        websocket_use_case,
        messages_use_case,
    ));

    let listener = listener.await?;
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_handshake(stream, websocket_service.clone()));
    }

    Ok(())
}

async fn handle_handshake<MR: MessagesRepository + Clone, DB: MessagesDB + Clone>(
    stream: tokio::net::TcpStream,
    ws_service: Arc<WebSocketService<MR, DB>>,
) {
    let callback = |req: &Request, resp: Response| {
        if req.uri().path() != "/ws" {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(None::<String>).unwrap();
            return Err(response)
        }
        Ok(resp)
    };

    match accept_hdr_async(stream, callback).await {
        Ok(ws_stream) => {
            let connection = WebSocketConnection::new(ws_stream);
            ws_service.handle_connection(connection).await;
        }
        Err(err) => error!("failed to accept connection: {err}"),
    }
}
