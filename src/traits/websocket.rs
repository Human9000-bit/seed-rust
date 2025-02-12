use actix_ws::{MessageStream, Session};

use crate::seed::entity::{message::IncomeMessage, websocket::WebSocketManager};

pub trait WebsocketRepository {
    async fn handle_subscribe(ws: &WebSocketManager, connection: &MessageStream, chat_id: &str);
    async fn handle_unsubscribe(ws: &WebSocketManager, connection: &MessageStream);
    async fn broadcast_event(ws: &WebSocketManager, message: IncomeMessage);
    async fn disconnect(ws: &WebSocketManager, connection: &MessageStream);
}
