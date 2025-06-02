use protocol::entity::{
    message::IncomeMessage,
    websocket::{WebSocketConnection, WebSocketManager},
};
use std::sync::Arc;

/// Repository trait for handling WebSocket operations
pub trait WebsocketRepository {
    /// Handles subscription to a chat room
    fn handle_subscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) -> impl Future<Output = ()>;
    /// Handles unsubscription from a chat room
    fn handle_unsubscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) -> impl Future<Output = ()>;
    /// Broadcasts an event to connected clients
    fn broadcast_event(&self, ws: Arc<WebSocketManager>, message: IncomeMessage) -> impl Future<Output = ()>;
    /// Handles client disconnection
    fn disconnect(&self, ws: Arc<WebSocketManager>, connection: Arc<WebSocketConnection>) -> impl Future<Output = ()>;
}
