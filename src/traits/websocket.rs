use actix_ws::{MessageStream, Session};

use crate::seed::entity::{message::IncomeMessage, websocket::WebSocketManager};

/// Repository trait for handling WebSocket operations
pub trait WebsocketRepository {
    /// Handles subscription to a chat room
    async fn handle_subscribe(ws: &WebSocketManager, connection: &MessageStream, chat_id: &str);
    /// Handles unsubscription from a chat room 
    async fn handle_unsubscribe(ws: &WebSocketManager, connection: &MessageStream);
    /// Broadcasts an event to connected clients
    async fn broadcast_event(ws: &WebSocketManager, message: IncomeMessage);
    /// Handles client disconnection
    async fn disconnect(ws: &WebSocketManager, connection: &MessageStream);
}
