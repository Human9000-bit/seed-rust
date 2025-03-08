use actix_ws::Message;
use futures::StreamExt;
use std::{ops::ControlFlow, sync::Arc};

use crate::{
    base64::decode_base64,
    seed::entity::{
        self,
        message::IncomeMessage,
        websocket::{WebSocketConnection, WebSocketManager},
    },
    traits::{
        message::{MessagesDB, MessagesRepository},
        websocket::WebsocketRepository,
    },
    use_case::{messages::MessagesUseCase, websocket::WebSocketUseCase},
};

#[derive(Clone)]
pub struct WebSocketService<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> {
    manager: Arc<WebSocketManager>,
    websocket_use_case: WebSocketUseCase<MR>,
    messages_use_case: MessagesUseCase<DB>,
}

impl<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> WebSocketService<MR, DB> {
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

        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<IncomeMessage>(&text) {
                    Ok(incoming) => {
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
                        log::error!("Failed to parse message: {}", err);
                        let _ = messages_use_case.status_response(connection.clone(), false).await;
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

    async fn process_message(
        manager: Arc<WebSocketManager>,
        connection: Arc<WebSocketConnection>,
        incoming: IncomeMessage,
        websocket_use_case: &WebSocketUseCase<MR>,
        messages_use_case: &MessagesUseCase<DB>,
    ) -> ControlFlow<()> {
        match incoming.clone() {
            IncomeMessage::Ping => {
                let _ = messages_use_case.status_response(connection, true).await;
            }
            IncomeMessage::Send(msg) => {
                if !messages_use_case.is_valid_message(msg.clone().into()).await {
                    let _ = messages_use_case.status_response(connection, false).await;
                    return ControlFlow::Break(());
                }

                let message = entity::websocket::ConnectedMessage {
                    connection: connection.clone(),
                    message: incoming,
                };

                let contains_key = manager.message_queues.contains_key(&msg.chat_id);

                if contains_key {
                    if let Some(queue) = manager.message_queues.get_mut(&msg.chat_id) {
                        let _ = queue.0.send(message);
                        log::info!("Message has been successfully added to the queue");
                    }

                    let _ = messages_use_case.status_response(connection.clone(), true).await;
                } else {
                    log::info!("There is no subscribers to receive message in the queue");
                    if let Err(err) = messages_use_case.db.insert_message(msg).await {
                        log::info!("Error inserting message into database: {}", err);
                        let _ = messages_use_case.status_response(connection.clone(), false).await;
                        return ControlFlow::Break(());
                    }

                    let _ = messages_use_case.status_response(connection.clone(), true).await;
                }
            }
            IncomeMessage::Subscribe(msg) => {
                let chat_id = match decode_base64(msg.chat_id.clone()).await {
                    Ok(chat_id) => chat_id,
                    Err(err) => {
                        log::error!("Error decoding chat ID: {}", err);
                        let _ = messages_use_case.status_response(connection.clone(), false).await;
                        return ControlFlow::Break(());
                    }
                };

                websocket_use_case
                    .handle_subscribe(manager.clone(), connection.clone(), &msg.chat_id)
                    .await;
                let _ = messages_use_case.status_response(connection.clone(), true).await;
                let _ = messages_use_case
                    .unread_message_response(connection.clone(), &chat_id, msg.nonce)
                    .await;
                let _ = messages_use_case
                    .wait_event_response(connection.clone(), &msg.chat_id)
                    .await;
            }
            IncomeMessage::Unsubscribe(msg) => {
                websocket_use_case
                    .handle_unsubscribe(manager.clone(), connection.clone(), &msg.chat_id)
                    .await;
                let _ = messages_use_case.status_response(connection, true).await;
            }
            IncomeMessage::None => {}
        }
        ControlFlow::Continue(())
    }
}
