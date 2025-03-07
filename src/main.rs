#![forbid(unsafe_code)]
#![allow(dead_code)]

// In the name of the Father, and in the name of the Son, and in the name of the Holy Spirit. Amen.

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

#[actix_web::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let tls_config = tls::load_rustls_config()?;

    let port = std::env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap();

    let pg_pool = PostgresDatabase::new().await?;

    let messages_use_case = use_case::messages::MessagesUseCase::new(pg_pool);
    let websocket_use_case =
        use_case::websocket::WebSocketUseCase::new(messages_use_case.clone()).await;
    let websocket_manager = WebSocketManager::new();

    // Replace actor with service
    let websocket_service = infrastructure::websocket::WebSocketService::new(
        websocket_manager, 
        websocket_use_case, 
        messages_use_case
    );

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(Data::new(websocket_service.clone()))
            .service(accept_websocket_connection)
    })
    .bind_rustls_0_23(format!("127.0.0.1:{port}"), tls_config)?
    .run();

    server.await?;

    Ok(())
}

#[get("/ws")]
async fn accept_websocket_connection(
    req: HttpRequest,
    payload: web::Payload,
    websocket_service: web::Data<
        WebSocketService<MessagesUseCase<PostgresDatabase>, PostgresDatabase>,
    >,
) -> actix_web::Result<HttpResponse> {
    let (response, conn, stream) = WebSocketConnection::new(&req, payload)?;
    debug!("New websocket connection");
    
    // Process the connection in a new task
    websocket_service.handle_connection(conn, stream).await;
    
    Ok(response)
}