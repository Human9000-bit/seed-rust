#![forbid(unsafe_code)]
#![allow(dead_code)]

// In the name of the Father, and in the name of the Son, and in the name of the Holy Spirit. Amen.

#[macro_use]
extern crate log;

mod base64;
mod infrastructure;
mod seed;
mod traits;
mod use_case;

use std::sync::Arc;

use actix_web::{get, web, HttpRequest, HttpResponse};
use anyhow::Result;
use infrastructure::{database::PostgresDatabase, websocket::handle_websocket_connection};
use seed::entity::websocket::{WebSocketConnection, WebSocketManager};
use use_case::{
    messages::{self, MessagesUseCase},
    websocket::WebSocketUseCase,
};

#[actix_web::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let port = std::env::var("PORT").unwrap_or("8080".to_string()).parse::<u16>().unwrap();

    let pg_pool = PostgresDatabase::new().await?;

    let messages_use_case = use_case::messages::MessagesUseCase::new(pg_pool);
    let websocket_use_case =
        use_case::websocket::WebSocketUseCase::new(messages_use_case.clone()).await;
    let websocket_manager = Arc::new(WebSocketManager::new());

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(web::Data::new(messages_use_case.clone()))
            .app_data(web::Data::new(websocket_use_case.clone()))
            .app_data(web::Data::new(websocket_manager.clone()))
            .service(accept_websocket_connection)
    })
    .bind(("127.0.0.1", port))?
    .run();

    server.await?;

    Ok(())
}

#[get("/ws")]
async fn accept_websocket_connection(
    req: HttpRequest,
    payload: web::Payload,
    message_use_case: web::Data<messages::MessagesUseCase<PostgresDatabase>>,
    websocket_use_case: web::Data<WebSocketUseCase<MessagesUseCase<PostgresDatabase>>>,
    websocket_manager: web::Data<WebSocketManager>,
) -> actix_web::Result<HttpResponse> {
    let (response, conn, mut stream) = WebSocketConnection::new(&req, payload)?;
    handle_websocket_connection(
        conn,
        &mut stream,
        message_use_case.get_ref(),
        websocket_use_case.get_ref(),
        websocket_manager.get_ref(),
    )
    .await;
    Ok(response)
}
