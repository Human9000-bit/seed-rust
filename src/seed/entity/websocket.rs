use std::collections::HashMap;

use actix_web::{web::Payload, HttpRequest, HttpResponse};
use actix_ws::{MessageStream, Session};
use serde::{Deserialize, Serialize};

use super::message::IncomeMessage;

/// A request to subscribe to a chat queue
#[derive(Serialize, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "type")]
    rtype: String,

    #[serde(rename = "queueId")]
    chat_id: String,

    nonce: usize,
}

/// A message received from a connected WebSocket client
pub struct ConnectedMessage {
    pub connection: WebSocketConnection,
    pub message: IncomeMessage,
}

/// Manages WebSocket connections and message routing
pub struct WebSocketManager {
    connections: HashMap<WebSocketConnection, String>,
    chats: HashMap<WebSocketConnection, Session>,
    message_queues: HashMap<String, flume::Sender<ConnectedMessage>>,
}

/// Wraps both [Session] and [MessageStream] into one struct
pub struct WebSocketConnection {
    pub session: Session,
    pub messages: MessageStream,
}

impl WebSocketConnection {
    /// Construct new [WebSocketConnection] from [HttpRequest] and [Payload]
    /// 
    /// Think of it as calling [actix_ws::handle], but you get [WebSocketConnection]
    fn new(
        req: &HttpRequest,
        body: Payload,
    ) -> std::result::Result<(HttpResponse, Self), actix_web::Error> {
        let (response, session, messages) = actix_ws::handle(req, body)?;
        let wsconn = WebSocketConnection { session, messages };
        Ok((response, wsconn))
    }
}
