use std::sync::Arc;

use futures::lock::Mutex;

use crate::{
    seed::entity::{
        message::OutcomeMessage,
        websocket::{WebSocketConnection, WebSocketManager},
    },
    traits::{message::MessagesRepository, websocket::WebsocketRepository},
};

pub struct WebSocketUseCase<T: MessagesRepository> {
    messages_repository: T,
}

impl<T: MessagesRepository> WebSocketUseCase<T> {
    pub async fn new(messages_repository: T) -> Self {
        Self {
            messages_repository,
        }
    }

    pub async fn start_message_processor(&self, ws: Arc<Mutex<WebSocketManager>>, chat_id: &str) {
        let mut ws = ws.lock().await;

        let chat_id = chat_id.to_string();
        let (sender, reciever) = flume::bounded(100);
        ws.message_queues
            .insert(chat_id.clone(), (sender, reciever.clone()));

        for event in ws.message_queues.get(&chat_id).unwrap().1.iter() {
            let _ = self
                .messages_repository
                .insert_message(event.message.into())
                .await
                .inspect_err(|e| error!("Error inserting message: {e}"));
        }

        info!("All users have unsubscribed from chat {chat_id}")
    }

    async fn subscribe_to_chat(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        let mut ws_guard = ws.lock().await;

        ws_guard.connections.entry(connection).or_default();
        ws_guard.chats.entry(chat_id.to_string()).or_default();

        if !ws_guard.message_queues.contains_key(chat_id) {
            drop(ws_guard);
            self.start_message_processor(ws, chat_id).await;
        }
    }

    pub async fn unsubscribe_from_chat(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        let mut ws = ws.lock().await;

        if let Some(conn) = ws.connections.get_mut(&connection) {
            conn.remove(chat_id);

            if conn.is_empty() {
                ws.connections.remove(&connection);
            }
        }

        if let Some(chats) = ws.chats.get_mut(chat_id) {
            chats.remove(&connection);

            if chats.is_empty() {
                ws.chats.remove(chat_id);
            }
        }
    }
}

impl<T: MessagesRepository> WebsocketRepository for WebSocketUseCase<T> {
    async fn handle_subscribe(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.subscribe_to_chat(ws, connection, chat_id).await;
    }

    async fn handle_unsubscribe(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.unsubscribe_from_chat(ws, connection, chat_id).await;
    }

    async fn broadcast_event(
        &self,
        ws: Arc<Mutex<WebSocketManager>>,
        message: crate::seed::entity::message::IncomeMessage,
    ) {
        let message: OutcomeMessage = OutcomeMessage::from(message);

        let ws = ws.lock().await;

        let tasks = ws.chats.get(&message.chat_id).unwrap().iter().map(|conn| {
            self.messages_repository
                .new_event_response(conn, message.clone())
        });

        let results = futures::future::join_all(tasks).await;
        for result in results {
            if let Err(e) = result {
                log::error!("Error broadcasting event: {}", e);
            }
        }
    }

    async fn disconnect(&self, ws: Arc<Mutex<WebSocketManager>>, connection: WebSocketConnection) {
        let _ = connection.session.lock().await.to_owned().close(None).await;

        let ws_unlocked = ws.lock();
        let connection = Arc::new(connection);

        let mut ws_unlocked = ws_unlocked.await;

        if let Some(chat_id) = ws_unlocked.connections.get(&connection) {
            let handles = chat_id
                .iter()
                .map(|id| self.unsubscribe_from_chat(ws.clone(), connection.clone(), id))
                .collect::<Vec<_>>();

            futures::future::join_all(handles).await;
        }

        ws_unlocked.connections.remove(&connection);
    }
}
