use std::sync::Arc;

use anyhow::Result;

use crate::seed::entity::{self, websocket::WebSocketConnection};

/// Repository trait for handling websocket message events and responses
pub trait MessagesRepository {
    /// Waits for an event response on the websocket connection for a specific chat
    async fn wait_event_response(
        &self,
        connecion: Arc<WebSocketConnection>,
        chat_id: &str,
    ) -> Result<()>;

    /// Sends a new message event response over the websocket connection
    async fn new_event_response(
        &self,
        connection: Arc<WebSocketConnection>,
        message: entity::message::OutcomeMessage,
    ) -> Result<()>;

    /// Sends a status response indicating connection state
    async fn status_response(&self, connecion: Arc<WebSocketConnection>, status: bool) -> Result<()>;

    /// Sends a response about unread messages for a chat
    async fn unread_message_response(
        &self,
        connecion: Arc<WebSocketConnection>,
        chat_id: &[u8],
        nonce: usize,
    );

    /// Validates if a message meets required criteria
    async fn is_valid_message(&self, message: entity::message::OutcomeMessage) -> bool;

    async fn insert_message(&self, message: entity::message::Message) -> Result<()>;
}

/// Database interface for message persistence
pub trait MessagesDB {
    /// Inserts a new message into the database
    async fn insert_message(&self, message: entity::message::Message) -> Result<()>;

    /// Retrieves message history for a chat with pagination
    ///
    /// # Arguments
    /// * `chat_id` - The ID of the chat to fetch history for
    /// * `nonce` - Pagination token
    /// * `amount` - Number of messages to retrieve
    async fn fetch_history(
        &self,
        chat_id: &[u8],
        nonce: usize,
        amount: usize,
    ) -> Result<Vec<entity::message::OutcomeMessage>>;
}
