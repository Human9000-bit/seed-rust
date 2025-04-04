use std::sync::Arc;

use anyhow::Result;

use crate::{
    base64::decode_base64,
    seed::entity::{
        self,
        response::{SeedResponse, WaitEventDetail},
        websocket::WebSocketConnection,
    },
    traits::message::{MessagesDB, MessagesRepository},
};

/// Maximum number of messages to fetch in a single request
const MESSAGES_LIMIT: usize = 100;

/// Use case for handling message operations
///
/// This struct implements the business logic for message operations
/// such as sending, receiving, and validating messages.
#[derive(Clone, Copy)]
pub struct MessagesUseCase<T: MessagesDB> {
    /// Database interface for message storage
    pub db: T,
}

impl<T: MessagesDB> MessagesUseCase<T> {
    /// Creates a new instance of MessagesUseCase
    ///
    /// # Arguments
    /// * `db` - Database implementation for message storage
    pub fn new(db: T) -> Self {
        Self { db }
    }
}

impl<T: MessagesDB> MessagesRepository for MessagesUseCase<T> {
    /// Sends a wait event response to the client
    ///
    /// Notifies the client to wait for events on a specific chat.
    ///
    /// # Arguments
    /// * `connection` - WebSocket connection to the client
    /// * `chat_id` - Identifier for the chat session
    async fn wait_event_response(
        &self,
        connection: Arc<WebSocketConnection>,
        chat_id: &str,
    ) -> Result<()> {
        let outgoing = SeedResponse::WaitEvent(WaitEventDetail {
            rtype: "wait".to_string(),
            chat_id: chat_id.to_string(),
        });

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;
        Ok(())
    }

    /// Sends a new event response to the client
    ///
    /// Delivers a new message to the client over the WebSocket connection.
    ///
    /// # Arguments
    /// * `connection` - WebSocket connection to the client
    /// * `message` - Message to be delivered
    async fn new_event_response(
        &self,
        connection: Arc<WebSocketConnection>,
        message: crate::seed::entity::message::OutcomeMessage,
    ) -> Result<()> {
        let outgoing = SeedResponse::NewEvent(entity::response::NewEventDetail {
            rtype: "new".to_string(),
            message: message.clone(),
        });

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;

        Ok(())
    }

    /// Sends a status response to the client
    ///
    /// Updates the client about the status of an operation.
    ///
    /// # Arguments
    /// * `connection` - WebSocket connection to the client
    /// * `status` - Status of the operation (true = success, false = failure)
    async fn status_response(
        &self,
        connection: Arc<WebSocketConnection>,
        status: bool,
    ) -> Result<()> {
        let outgoing = SeedResponse::Status(entity::response::StatusResponse { status });

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;

        Ok(())
    }

    /// Sends unread messages to the client
    ///
    /// Fetches and sends historical messages from the database in batches,
    /// starting from the specified nonce value.
    ///
    /// # Arguments
    /// * `connection` - WebSocket connection to the client
    /// * `chat_id` - Identifier for the chat session
    /// * `nonce` - Starting position for fetching messages
    async fn unread_message_response(
        &self,
        connection: Arc<WebSocketConnection>,
        chat_id: &[u8],
        nonce: usize,
    ) {
        let mut current_nonce = nonce;

        loop {
            // Fetch a batch of messages from the database
            let messages = self
                .db
                .fetch_history(chat_id, current_nonce, MESSAGES_LIMIT)
                .await;
            let messages = match messages {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("failed to fetch history: {e}");
                    break;
                }
            };

            // Prepare futures for sending each message
            let mut futures = Vec::new();
            for msg in messages {
                futures.push(self.new_event_response(connection.clone(), msg));
            }

            // If we have fewer messages than the limit, this is the last batch
            if futures.len() < MESSAGES_LIMIT {
                futures::future::join_all(futures)
                    .await
                    .into_iter()
                    .for_each(|r| {
                        if let Err(e) = r {
                            log::error!("failed to send history message: {e}");
                        }
                    });
                break;
            };

            // Process all message sending futures
            futures::future::join_all(futures)
                .await
                .into_iter()
                .for_each(|r| {
                    if let Err(e) = r {
                        log::error!("failed to send history message: {e}");
                    }
                });

            // Move to the next batch of messages
            // Overflow check:
            current_nonce = match current_nonce.checked_add(MESSAGES_LIMIT) {
                // If no overflow occurred, update the nonce
                Some(int) => int,
                // If overflow occurred, send a status response and finish processing
                None => {
                    let _ = self.status_response(connection, false).await;
                    return;
                }
            };
        }
    }

    /// Validates message format and encoding
    ///
    /// Checks if the message has properly encoded fields.
    ///
    /// # Arguments
    /// * `message` - Message to validate
    ///
    /// # Returns
    /// * `bool` - true if message is valid, false otherwise
    async fn is_valid_message(&self, message: entity::message::OutcomeMessage) -> bool {
        // Validate chat_id
        let chat_id = decode_base64(message.chat_id).await;
        if chat_id.is_err() {
            log::error!("invalid chat id");
            return false;
        }

        // Validate signature
        let signature = decode_base64(message.signature).await;
        if signature.is_err() {
            log::error!("invalid signature");
            return false;
        }

        // Validate content initialization vector
        let content_iv = decode_base64(message.content_iv).await;
        if content_iv.is_err() {
            log::error!("invalid content iv");
            return false;
        }

        true
    }

    /// Inserts a message into the database
    ///
    /// # Arguments
    /// * `message` - Message to be stored
    async fn insert_message(&self, message: entity::message::Message) -> Result<()> {
        self.db.insert_message(message).await?;
        Ok(())
    }
}
