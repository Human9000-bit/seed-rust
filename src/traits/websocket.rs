use crate::seed::entity::{
    message::IncomeMessage,
    websocket::{WebSocketConnection, WebSocketManager},
};
use futures::lock::Mutex;
use std::sync::Arc;

/// Repository trait for handling WebSocket operations
pub trait WebsocketRepository {
    /// Handles subscription to a chat room
    async fn handle_subscribe(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    );
    /// Handles unsubscription from a chat room
    async fn handle_unsubscribe(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    );
    /// Broadcasts an event to connected clients
    async fn broadcast_event(&self, ws: Arc<Mutex<WebSocketManager>>, message: IncomeMessage);
    /// Handles client disconnection
    async fn disconnect(&self, ws: Arc<Mutex<WebSocketManager>>, connection: WebSocketConnection);
}
