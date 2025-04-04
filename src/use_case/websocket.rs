use std::sync::Arc;

use crate::{
    seed::entity::{
        message::{IncomeMessage, OutcomeMessage},
        websocket::{WebSocketConnection, WebSocketManager},
    },
    traits::{message::MessagesRepository, websocket::WebsocketRepository},
};

/// WebSocketUseCase handles WebSocket communication and message processing
/// for chat functionality. It manages connections, subscriptions, and message
/// broadcasting.
///
/// Type parameter `T` represents a repository implementation for message persistence.
#[derive(Clone, Copy)]
pub struct WebSocketUseCase<T: MessagesRepository> {
    /// Repository for storing and retrieving messages
    messages_repository: T,
}

impl<T: MessagesRepository> WebSocketUseCase<T> {
    /// Creates a new WebSocketUseCase instance with the provided message repository
    ///
    /// # Arguments
    /// * `messages_repository` - Repository implementation for message handling
    ///
    /// # Returns
    /// A new WebSocketUseCase instance
    pub async fn new(messages_repository: T) -> Self {
        Self {
            messages_repository,
        }
    }

    /// Starts a message processor for a specific chat
    ///
    /// This function sets up a message queue for a chat and processes incoming messages,
    /// persisting them to the message repository.
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `chat_id` - ID of the chat to process messages for
    pub async fn start_message_processor(&self, ws: Arc<WebSocketManager>, chat_id: &str) {
        let chat_id = chat_id.to_string();
        // Create unbounded channel for message queue
        let (sender, reciever) = flume::unbounded();
        ws.message_queues
            .insert(chat_id.clone(), (sender, reciever.clone()));

        // Process each message in the queue
        match ws.message_queues.get(&chat_id) {
            Some(reciever) => {
                for event in reciever.1.iter() {
                    let message = match event.message {
                        IncomeMessage::Send(msg) => msg,
                        IncomeMessage::Subscribe(msg) => msg,
                        IncomeMessage::Unsubscribe(msg) => msg,
                        _ => continue, // Skip other message types
                    };
                    // Persist the message to the repository
                    let _ = self
                        .messages_repository
                        .insert_message(message)
                        .await
                        .inspect_err(|e| error!("Error inserting message: {e}"));
                }

                info!("All users have unsubscribed from chat {chat_id}");
            }
            None => {
                error!("Failed to start message processor for chat {chat_id}: channel not found")
            }
        }
    }

    /// Subscribes a connection to a chat
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `connection` - Connection to subscribe
    /// * `chat_id` - ID of the chat to subscribe to
    async fn subscribe_to_chat(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        // Add connection to connection map
        ws.connections.entry(connection).or_default();
        // Add chat to chat map
        ws.chats.entry(chat_id.to_string()).or_default();

        // Start message processor if it doesn't exist for this chat
        if !ws.message_queues.contains_key(chat_id) {
            self.start_message_processor(ws, chat_id).await;
        }
    }

    /// Unsubscribes a connection from a chat
    ///
    /// Removes the connection from the chat and cleans up resources if needed.
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `connection` - Connection to unsubscribe
    /// * `chat_id` - ID of the chat to unsubscribe from
    pub async fn unsubscribe_from_chat(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: String,
    ) {
        // Remove chat from connection's subscribed chats
        if let Some(conn) = ws.connections.get_mut(&connection) {
            conn.remove(&chat_id);

            // Remove connection entirely if it's not subscribed to any chats
            if conn.is_empty() {
                ws.connections.remove(&connection);
            }
        }

        // Remove connection from chat's subscribers
        if let Some(chats) = ws.chats.get_mut(&chat_id) {
            chats.remove(&connection);

            // Remove chat entirely if it has no subscribers
            if chats.is_empty() {
                ws.chats.remove(&chat_id);
            }
        }
    }
}

impl<T: MessagesRepository> WebsocketRepository for WebSocketUseCase<T> {
    /// Handles subscription requests to a chat
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `connection` - Connection requesting subscription
    /// * `chat_id` - ID of the chat to subscribe to
    async fn handle_subscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.subscribe_to_chat(ws, connection, chat_id).await;
    }

    /// Handles unsubscription requests from a chat
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `connection` - Connection requesting unsubscription
    /// * `chat_id` - ID of the chat to unsubscribe from
    async fn handle_unsubscribe(
        &self,
        ws: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) {
        self.unsubscribe_from_chat(ws, connection, chat_id.to_owned())
            .await;
    }

    /// Broadcasts an event to all connections subscribed to a chat
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `message` - Message to broadcast
    async fn broadcast_event(
        &self,
        ws: Arc<WebSocketManager>,
        message: crate::seed::entity::message::IncomeMessage,
    ) {
        // Convert incoming message to outgoing format
        let message: OutcomeMessage = message.into();

        // Get all connections subscribed to this chat
        let connections = match ws.chats.get(&message.chat_id) {
            Some(chats) => chats,
            None => {
                error!(
                    "Error broadcasting event to chat {}: Chat not found",
                    message.chat_id
                );
                return;
            }
        };

        // Create tasks to send the message to each connection
        let tasks = connections.iter().map(|conn| {
            self.messages_repository
                .new_event_response(conn.clone(), message.clone())
        });

        // Execute all tasks concurrently
        let results = futures::future::join_all(tasks).await;
        for result in results {
            if let Err(e) = result {
                log::error!("Error broadcasting event: {}", e);
            }
        }
    }

    /// Handles disconnection of a client
    ///
    /// Closes the connection and removes it from all subscribed chats.
    ///
    /// # Arguments
    /// * `ws` - WebSocketManager instance
    /// * `connection` - Connection that is disconnecting
    async fn disconnect(&self, ws: Arc<WebSocketManager>, connection: Arc<WebSocketConnection>) {
        // Close the WebSocket session
        let _ = connection.session.lock().await.to_owned().close(None).await;

        // Unsubscribe from all chats this connection was subscribed to
        if let Some(chat_id) = ws.connections.get(&connection) {
            let handles = chat_id
                .iter()
                .map(|id| self.unsubscribe_from_chat(ws.clone(), connection.clone(), id.to_owned()))
                .collect::<Vec<_>>();

            // Wait for all unsubscribe operations to complete
            futures::future::join_all(handles).await;
        }

        // Remove the connection completely
        ws.connections.remove(&connection);
    }
}
