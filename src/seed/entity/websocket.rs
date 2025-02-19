use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

use futures::lock::Mutex;

use actix_web::{web::Payload, HttpRequest, HttpResponse};
use actix_ws::{MessageStream, Session};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::message::IncomeMessage;

/// A request to subscribe to a chat queue
#[derive(Serialize, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "type")]
    pub rtype: String,

    #[serde(rename = "queueId")]
    pub chat_id: String,
    
    pub nonce: usize,
}

/// A message received from a connected WebSocket client
pub struct ConnectedMessage {
    pub connection: Arc<WebSocketConnection>,
    pub message: IncomeMessage,
}

/// Manages WebSocket connections and message routing
pub struct WebSocketManager {
    pub connections: HashMap<Arc<WebSocketConnection>, HashSet<String>>,
    pub chats: HashMap<String, HashSet<WebSocketConnection>>,
    pub message_queues: HashMap<
        String,
        (
            flume::Sender<ConnectedMessage>,
            flume::Receiver<ConnectedMessage>,
        ),
    >,
}

/// Wraps both [Session] and [MessageStream] into one struct
#[derive(Clone)]
pub struct WebSocketConnection {
    pub id: Uuid,
    pub session: Arc<Mutex<Session>>,
}

impl WebSocketConnection {
    /// Construct new [WebSocketConnection] from [HttpRequest] and [Payload]
    ///
    /// Think of it as calling [actix_ws::handle], but you get [WebSocketConnection]
    pub fn new(
        req: &HttpRequest,
        body: Payload,
    ) -> std::result::Result<(HttpResponse, Self, MessageStream), actix_web::Error> {
        let (response, session, stream) = actix_ws::handle(req, body)?;
        let wsconn = WebSocketConnection {
            id: Uuid::new_v4(),
            session: Arc::new(Mutex::new(session)),
        };
        Ok((response, wsconn, stream))
    }
}

impl PartialEq for WebSocketConnection {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WebSocketConnection {}

impl Hash for WebSocketConnection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn send_and_sync<T: Send + Sync>() {}
    fn websocket_manager_send_and_sync() {
        send_and_sync::<WebSocketManager>();
    }
}
