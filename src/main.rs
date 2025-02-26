#![forbid(unsafe_code)]
#![allow(dead_code)]

// In the name of the Father, and in the name of the Son, and in the name of the Holy Spirit. Amen.

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod base64;
mod infrastructure;
mod seed;
mod traits;
mod use_case;

use actix::Actor;
use actix::Addr;
use actix_web::{HttpRequest, HttpResponse, get, web};
use anyhow::Result;
use futures::StreamExt;
use infrastructure::{
    database::PostgresDatabase,
    websocket::{WebSocketActor, WebSocketActorMessage},
};
use seed::entity::websocket::{WebSocketConnection, WebSocketManager};
use use_case::messages::MessagesUseCase;

#[actix_web::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let port = std::env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap();

    let pg_pool = PostgresDatabase::new().await?;

    let messages_use_case = use_case::messages::MessagesUseCase::new(pg_pool);
    let websocket_use_case =
        use_case::websocket::WebSocketUseCase::new(messages_use_case.clone()).await;
    let websocket_manager = WebSocketManager::new();

    let websocket_actor =
        WebSocketActor::new(websocket_manager, websocket_use_case, messages_use_case).start();

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(websocket_actor.clone())
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
    websocket_actor: web::Data<
        Addr<WebSocketActor<MessagesUseCase<PostgresDatabase>, PostgresDatabase>>,
    >,
) -> actix_web::Result<HttpResponse> {
    let (response, conn, mut stream) = WebSocketConnection::new(&req, payload)?;
    let (tx, rx) = flume::unbounded();
    
    while let Some(Ok(msg)) = stream.next().await {
        if tx.send(msg).is_err() {
            break;
        }
    };
    
    let message = WebSocketActorMessage {
        connection: conn,
        stream: rx
    };
    websocket_actor
        .send(message)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to send message"))?;

    Ok(response)
}