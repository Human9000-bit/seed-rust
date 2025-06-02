use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use dashmap::{DashMap, DashSet};
use futures::lock::Mutex;

use actix_web::{HttpRequest, HttpResponse, web::Payload};
use actix_ws::{MessageStream, Session};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::message::IncomeMessage;

///
/// This structure represents the JSON payload sent by clients
/// when they want to subscribe to messages from a specific chat queue.
#[derive(Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// The type of the request, typically "subscribe"
    #[serde(rename = "type")]
    pub rtype: String,

    /// The unique identifier of the chat queue to subscribe to
    #[serde(rename = "queueId")]
    pub chat_id: String,

    /// A client-provided identifier to correlate requests with responses
    pub nonce: usize,
}

/// A message received from a connected WebSocket client.
///
/// Associates an incoming message with the connection it was received from.
pub struct ConnectedMessage {
    /// The WebSocket connection that sent this message
    pub connection: Arc<WebSocketConnection>,

    /// The actual message content received from the client
    pub message: IncomeMessage,
}

/// Manages WebSocket connections and message routing between clients and chat queues.
///
/// This central manager keeps track of all active connections and their subscriptions,
/// enabling efficient message distribution to the appropriate subscribers.
#[derive(Clone, Default)]
pub struct WebSocketManager {
    /// Maps each connection to the set of chat IDs it is subscribed to
    pub connections: DashMap<Arc<WebSocketConnection>, DashSet<String>>,

    /// Maps each chat ID to the set of connections subscribed to it
    pub chats: DashMap<String, DashSet<Arc<WebSocketConnection>>>,

    /// Message queues for each chat, containing sender and receiver channels for message distribution
    pub message_queues: DashMap<
        String,
        (
            flume::Sender<ConnectedMessage>,
            flume::Receiver<ConnectedMessage>,
        ),
    >,
}

impl WebSocketManager {
    /// Creates a new empty WebSocketManager instance.
    ///
    /// Initializes the connection tracking maps and message queues with no entries.
    pub fn new(connections: DashMap<Arc<WebSocketConnection>, DashSet<String>>, chats: DashMap<String, DashSet<Arc<WebSocketConnection>>>, message_queues: DashMap<String, (flume::Sender<ConnectedMessage>, flume::Receiver<ConnectedMessage>)>) -> Self {
        Self {
            connections,
            chats,
            message_queues,
        }
    }
}

/// Represents a WebSocket connection to a client.
///
/// Wraps both the WebSocket session for sending messages and a unique identifier
/// to track this specific connection throughout the system.
#[derive(Clone)]
pub struct WebSocketConnection {
    /// Unique identifier for this connection
    pub id: Uuid,

    /// The WebSocket session wrapped in Arc<Mutex<>> for thread-safe access
    pub session: Arc<Mutex<Session>>,
}

impl WebSocketConnection {
    /// Constructs a new WebSocketConnection from an HTTP request and payload.
    ///
    /// This method handles the WebSocket handshake and returns the HTTP response
    /// to complete the handshake, the WebSocketConnection object for sending messages,
    /// and the MessageStream for receiving messages.
    ///
    /// # Arguments
    ///
    /// * `req` - The HTTP request triggering the WebSocket connection
    /// * `body` - The request payload
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The HTTP response to send back to the client
    /// * The WebSocketConnection for tracking and sending messages
    /// * The MessageStream for receiving messages from this connection
    ///
    /// # Errors
    ///
    /// Returns an actix_web::Error if the WebSocket handshake fails
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
