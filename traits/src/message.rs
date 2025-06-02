use std::sync::Arc;

use anyhow::Result;

use protocol::entity::{self, websocket::WebSocketConnection};

/// Repository trait for handling websocket message events and responses
pub trait MessagesRepository {
    /// Waits for an event response on the websocket connection for a specific chat
    fn wait_event_response(
        &self,
        connecion: Arc<WebSocketConnection>,
        chat_id: &str,
    ) -> impl Future<Output = Result<()>>;

    /// Sends a new message event response over the websocket connection
    fn new_event_response(
        &self,
        connection: Arc<WebSocketConnection>,
        message: entity::message::OutcomeMessage,
    ) -> impl Future<Output = Result<()>>;

    /// Sends a status response indicating connection state
    fn status_response(
        &self,
        connection: Arc<WebSocketConnection>,
        status: bool,
    ) -> impl Future<Output = Result<()>>;

    /// Sends a response about unread messages for a chat
    fn unread_message_response(
        &self,
        connection: Arc<WebSocketConnection>,
        chat_id: &[u8],
        nonce: usize,
    ) -> impl Future<Output = ()>;

    /// Validates if a message meets required criteria
    fn is_valid_message(&self, message: entity::message::OutcomeMessage) -> impl Future<Output = bool>;

    fn insert_message(&self, message: entity::message::Message) -> impl Future<Output = Result<()>>;
}

/// Database interface for message persistence
pub trait MessagesDB {
    /// Inserts a new message into the database
    fn insert_message(&self, message: entity::message::Message) -> impl Future<Output = Result<()>>;

    /// Retrieves message history for a chat with pagination
    ///
    /// # Arguments
    /// * `chat_id` - The ID of the chat to fetch history for
    /// * `nonce` - Pagination token
    /// * `amount` - Number of messages to retrieve
    fn fetch_history(
        &self,
        chat_id: &[u8],
        nonce: usize,
        amount: usize,
    ) -> impl Future<Output = Result<Vec<entity::message::OutcomeMessage>>>;
}
