use actix_ws::Message;
use futures::StreamExt;
use log::debug;
use std::{ops::ControlFlow, sync::Arc};

use crate::{
    base64::decode_base64,
    traits::{
        message::{MessagesDB, MessagesRepository},
        websocket::WebsocketRepository,
    },
    use_case::{messages::MessagesUseCase, websocket::WebSocketUseCase},
};

use protocol::entity::{
    self,
    message::IncomeMessage,
    websocket::{WebSocketConnection, WebSocketManager},
};

/// Service for handling WebSocket connections and messages.
///
/// This service manages the lifecycle of WebSocket connections, processes incoming
/// messages, and coordinates between the WebSocket manager and various use cases.
#[derive(Clone)]
pub struct WebSocketService<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> {
    /// Central manager for all WebSocket connections
    manager: Arc<WebSocketManager>,
    /// Use case for WebSocket-specific operations
    websocket_use_case: WebSocketUseCase<MR>,
    /// Use case for message handling operations
    messages_use_case: MessagesUseCase<DB>,
}

impl<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> WebSocketService<MR, DB> {
    /// Creates a new WebSocket service instance.
    ///
    /// # Arguments
    ///
    /// * `manager` - The WebSocket manager to handle connections
    /// * `websocket_use_case` - The use case for WebSocket operations
    /// * `messages_use_case` - The use case for message operations
    ///
    /// # Returns
    ///
    /// A new `WebSocketService` instance
    pub fn new(
        manager: WebSocketManager,
        websocket_use_case: WebSocketUseCase<MR>,
        messages_use_case: MessagesUseCase<DB>,
    ) -> Self {
        Self {
            manager: Arc::new(manager),
            websocket_use_case,
            messages_use_case,
        }
    }

    /// Handles a new WebSocket connection by processing its message stream.
    ///
    /// This method continuously processes incoming messages from the WebSocket stream
    /// until the connection is closed or an unrecoverable error occurs.
    ///
    /// # Arguments
    ///
    /// * `connection` - The WebSocket connection to handle
    /// * `stream` - The message stream from the WebSocket connection
    pub async fn handle_connection(
        &self,
        connection: WebSocketConnection,
        mut stream: actix_ws::MessageStream,
    ) {
        let connection = Arc::new(connection);
        let manager = self.manager.clone();
        let websocket_use_case = self.websocket_use_case.clone();
        let messages_use_case = self.messages_use_case.clone();

        // Spawn a task to handle this connection

        debug!(
            "Starting to handle websocket messages for connection: {}",
            connection.id
        );

        // Process each message in the stream until connection closes
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<IncomeMessage>(&text) {
                    Ok(incoming) => {
                        // Process the message and break the loop if needed
                        if let ControlFlow::Break(_) = Self::process_message(
                            manager.clone(),
                            connection.clone(),
                            incoming,
                            &websocket_use_case,
                            &messages_use_case,
                        )
                        .await
                        {
                            break;
                        }
                    }
                    Err(err) => {
                        // Log parsing errors and send failure status
                        log::error!("Failed to parse message: {}", err);
                        let _ = messages_use_case
                            .status_response(connection.clone(), false)
                            .await;
                    }
                },
                Message::Close(_) => {
                    log::info!("WebSocket connection closed by client");
                    break;
                }
                _ => {} // Handle other message types if needed
            }
        }

        // Clean up on disconnect
        websocket_use_case
            .disconnect(manager.clone(), connection.clone())
            .await;
    }

    /// Processes an incoming WebSocket message based on its type.
    ///
    /// This method handles different types of incoming messages (ping, send, subscribe, unsubscribe)
    /// and performs the appropriate actions for each type.
    ///
    /// # Arguments
    ///
    /// * `manager` - The WebSocket manager
    /// * `connection` - The WebSocket connection that sent the message
    /// * `incoming` - The parsed incoming message
    /// * `websocket_use_case` - The WebSocket use case for processing operations
    /// * `messages_use_case` - The messages use case for message handling
    ///
    /// # Returns
    ///
    /// A `ControlFlow` indicating whether to continue processing messages or break the connection
    async fn process_message(
        manager: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        incoming: IncomeMessage,
        websocket_use_case: &WebSocketUseCase<MR>,
        messages_use_case: &MessagesUseCase<DB>,
    ) -> ControlFlow<()> {
        match incoming.clone() {
            IncomeMessage::Ping => {
                // Handle ping messages by sending a positive status response
                let _ = messages_use_case.status_response(connection, true).await;
            }
            IncomeMessage::Send(msg) => {
                // Validate the message before processing
                if !messages_use_case.is_valid_message(msg.clone().into()).await {
                    let _ = messages_use_case.status_response(connection, false).await;
                    return ControlFlow::Break(());
                }

                // Create a connected message to send
                let message = entity::websocket::ConnectedMessage {
                    connection: connection.clone(),
                    message: incoming,
                };

                // Check if there are subscribers for this chat
                let contains_key = manager.message_queues.contains_key(&msg.chat_id);

                if contains_key {
                    // If there are subscribers, add the message to the queue
                    if let Some(queue) = manager.message_queues.get_mut(&msg.chat_id) {
                        let _ = queue.0.send(message);
                        log::info!("Message has been successfully added to the queue");
                    }

                    // Send a positive status response
                    let _ = messages_use_case
                        .status_response(connection.clone(), true)
                        .await;
                } else {
                    // If no subscribers, store the message in the database
                    log::info!("There is no subscribers to receive message in the queue");
                    if let Err(err) = messages_use_case.db.insert_message(msg).await {
                        log::info!("Error inserting message into database: {}", err);
                        let _ = messages_use_case
                            .status_response(connection.clone(), false)
                            .await;
                        return ControlFlow::Break(());
                    }

                    // Send a positive status response
                    let _ = messages_use_case
                        .status_response(connection.clone(), true)
                        .await;
                }
            }
            IncomeMessage::Subscribe(msg) => {
                // Decode the chat ID from base64
                let chat_id = match decode_base64(msg.chat_id.clone()).await {
                    Ok(chat_id) => chat_id,
                    Err(err) => {
                        log::error!("Error decoding chat ID: {}", err);
                        let _ = messages_use_case
                            .status_response(connection.clone(), false)
                            .await;
                        return ControlFlow::Break(());
                    }
                };

                // Handle the subscription
                websocket_use_case
                    .handle_subscribe(manager.clone(), connection.clone(), &msg.chat_id)
                    .await;

                // Send various responses indicating successful subscription
                let _ = messages_use_case
                    .status_response(connection.clone(), true)
                    .await;
                let _ = messages_use_case
                    .unread_message_response(connection.clone(), &chat_id, msg.nonce)
                    .await;
                let _ = messages_use_case
                    .wait_event_response(connection.clone(), &msg.chat_id)
                    .await;
            }
            IncomeMessage::Unsubscribe(msg) => {
                // Handle unsubscription
                websocket_use_case
                    .handle_unsubscribe(manager.clone(), connection.clone(), &msg.chat_id)
                    .await;
                let _ = messages_use_case.status_response(connection, true).await;
            }
            IncomeMessage::None => {
                // No-op for None messages
            }
        }
        // Continue processing messages
        ControlFlow::Continue(())
    }
}
