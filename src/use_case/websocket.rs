use std::sync::Arc;

use crate::{
    seed::entity::{
        message::{IncomeMessage, OutcomeMessage},
        websocket::{WebSocketConnection, WebSocketManager},
    },
    traits::{message::MessagesRepository, websocket::WebsocketRepository},
};

#[derive(Clone, Copy)]
pub struct WebSocketUseCase<T: MessagesRepository> {
    messages_repository: T,
}

impl<T: MessagesRepository> WebSocketUseCase<T> {
    pub async fn new(messages_repository: T) -> Self {
        Self {
            messages_repository,
        }
    }

    pub async fn start_message_processor(&self, ws: Arc<WebSocketManager>, chat_id: &str) {
        let chat_id = chat_id.to_string();
        let (sender, reciever) = flume::bounded(100);
        ws.message_queues
            .insert(chat_id.clone(), (sender, reciever.clone()));

        for event in ws.message_queues.get(&chat_id).unwrap().1.iter() {
            let message = match event.message {
                IncomeMessage::Send(msg) => msg,
                IncomeMessage::Subscribe(msg) => msg,
                IncomeMessage::Unsubscribe(msg) => msg,
                _ => continue,
            };
            let _ = self
                .messages_repository
                .insert_message(message)
                .await
                .inspect_err(|e| error!("Error inserting message: {e}"));
        }

        info!("All users have unsubscribed from chat {chat_id}")
    }

    async fn subscribe_to_chat(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        ws.connections.entry(connection).or_default();
        ws.chats.entry(chat_id.to_string()).or_default();

        if !ws.message_queues.contains_key(chat_id) {
            self.start_message_processor(ws, chat_id).await;
        }
    }

    pub async fn unsubscribe_from_chat(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: String,
    ) {
        if let Some(conn) = ws.connections.get_mut(&connection) {
            conn.remove(&chat_id);

            if conn.is_empty() {
                ws.connections.remove(&connection);
            }
        }

        if let Some(chats) = ws.chats.get_mut(&chat_id) {
            chats.remove(&connection);

            if chats.is_empty() {
                ws.chats.remove(&chat_id);
            }
        }
    }
}

impl<T: MessagesRepository> WebsocketRepository for WebSocketUseCase<T> {
    async fn handle_subscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.subscribe_to_chat(ws, connection, chat_id).await;
    }

    async fn handle_unsubscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.unsubscribe_from_chat(ws, connection, chat_id.to_owned())
            .await;
    }

    async fn broadcast_event(
        &self,
        ws: Arc<WebSocketManager>,
        message: crate::seed::entity::message::IncomeMessage,
    ) {
        let message: OutcomeMessage = message.into();

        let connections = ws.chats.get(&message.chat_id).unwrap();

        let tasks = connections.iter().map(|conn| {
            self.messages_repository
                .new_event_response(conn.clone(), message.clone())
        });

        let results = futures::future::join_all(tasks).await;
        for result in results {
            if let Err(e) = result {
                log::error!("Error broadcasting event: {}", e);
            }
        }
    }

    async fn disconnect(&self, ws: Arc<WebSocketManager>, connection: Arc<WebSocketConnection>) {
        let _ = connection.session.lock().await.to_owned().close(None).await;

        if let Some(chat_id) = ws.connections.get(&connection) {
            let handles = chat_id
                .iter()
                .map(|id| self.unsubscribe_from_chat(ws.clone(), connection.clone(), id.to_owned()))
                .collect::<Vec<_>>();

            futures::future::join_all(handles).await;
        }

        ws.connections.remove(&connection);
    }
}
