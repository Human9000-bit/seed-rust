use anyhow::Result;

use crate::{
    base64::decode_base64,
    seed::entity::{
        self,
        response::{WaitEventDetail, WaitEventResponse},
        websocket::WebSocketConnection,
    },
    traits::message::{MessagesDB, MessagesRepository},
};

const MESSAGES_LIMIT: usize = 100;

#[derive(Clone, Copy)]
pub struct MessagesUseCase<T: MessagesDB> {
    pub db: T,
}

impl<T: MessagesDB> MessagesUseCase<T> {
    pub fn new(db: T) -> Self {
        Self { db }
    }
}

impl<T: MessagesDB> MessagesRepository for MessagesUseCase<T> {
    async fn wait_event_response(
        &self,
        connection: &crate::seed::entity::websocket::WebSocketConnection,
        chat_id: &str,
    ) -> Result<()> {
        let outgoing = WaitEventResponse {
            rtype: "event".to_string(),
            event: WaitEventDetail {
                rtype: "wait".to_string(),
                chat_id: chat_id.to_string(),
            },
        };

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;
        Ok(())
    }

    async fn new_event_response(
        &self,
        connection: &crate::seed::entity::websocket::WebSocketConnection,
        message: crate::seed::entity::message::OutcomeMessage,
    ) -> Result<()> {
        let outgoing = entity::response::NewEventResponse {
            rtype: "event".to_string(),
            event: entity::response::NewEventDetail {
                rtype: "new".to_string(),
                message: message.clone(),
            },
        };

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;

        Ok(())
    }

    async fn status_response(
        &self,
        connection: &crate::seed::entity::websocket::WebSocketConnection,
        status: bool,
    ) -> Result<()> {
        let outgoing = entity::response::StatusResponse {
            rtype: "response".to_string(),
            status,
        };

        let mut session = connection.session.lock().await;

        session.text(serde_json::to_string(&outgoing)?).await?;

        Ok(())
    }

    async fn unread_message_response(
        &self,
        connection: &WebSocketConnection,
        chat_id: &[u8],
        nonce: usize,
    ) {
        let mut current_nonce = nonce;

        loop {
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

            let mut futures = Vec::new();
            for msg in messages {
                futures.push(self.new_event_response(connection, msg));
            }

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

            futures::future::join_all(futures)
                .await
                .into_iter()
                .for_each(|r| {
                    if let Err(e) = r {
                        log::error!("failed to send history message: {e}");
                    }
                });

            current_nonce += MESSAGES_LIMIT;
        }
    }

    async fn is_valid_message(&self, message: entity::message::OutcomeMessage) -> bool {
        let chat_id = decode_base64(message.chat_id).await;

        if chat_id.is_err() {
            log::error!("invalid chat id");
            return false;
        }

        let signature = decode_base64(message.signature).await;

        if signature.is_err() {
            log::error!("invalid signature");
            return false;
        }

        let content_iv = decode_base64(message.content_iv).await;

        if content_iv.is_err() {
            log::error!("invalid content iv");
            return false;
        }

        true
    }

    async fn insert_message(&self, message: entity::message::Message) -> Result<()> {
        self.db.insert_message(message).await?;
        Ok(())
    }
}
