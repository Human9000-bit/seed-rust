use std::sync::Arc;

use actix::{Actor, Context, Handler, Message as ActixMessage, ResponseFuture};
use actix_ws::Message;

use futures::lock::Mutex;

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
pub struct WebSocketActor<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> {
    manager: WebSocketManager,
    websocket_use_case: WebSocketUseCase<MR>,
    messages_use_case: MessagesUseCase<DB>,
}

impl<MR: MessagesRepository + Clone, DB: MessagesDB + Clone> WebSocketActor<MR, DB> {
    pub fn new(
        manager: WebSocketManager,
        websocket_use_case: WebSocketUseCase<MR>,
        messages_use_case: MessagesUseCase<DB>,
    ) -> Self {
        Self {
            manager,
            websocket_use_case,
            messages_use_case,
        }
    }
}

impl<MR: MessagesRepository + Clone + Unpin + 'static, DB: MessagesDB + Clone + Unpin + 'static>
    Actor for WebSocketActor<MR, DB>
{
    type Context = Context<Self>;
}

impl<MR: MessagesRepository + Clone + Unpin + 'static, DB: MessagesDB + Clone + Unpin + 'static>
    Handler<WebSocketActorMessage> for WebSocketActor<MR, DB>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: WebSocketActorMessage, _ctx: &mut Self::Context) -> Self::Result {
        let self_cloned = self.clone();
        Box::pin(async move {
            let ws = Arc::new(Mutex::new(self_cloned.manager.clone()));
            let connection = Arc::new(msg.connection);

            while let Ok(msg) = msg.stream.recv() {
                if let Message::Text(text) = msg {
                    let incoming: IncomeMessage = serde_json::from_str(&text).unwrap(); // TODO: proper error handling
                    match incoming.rtype.as_str() {
                        "ping" => {
                            let _ = self_cloned
                                .messages_use_case
                                .status_response(&connection, true)
                                .await;
                        }
                        "send" => {
                            let inner_message = match incoming.message.clone() {
                                Some(val) => val,
                                None => {
                                    log::error!("Error parsing send message");
                                    continue;
                                }
                            };

                            if !self_cloned
                                .messages_use_case
                                .is_valid_message(incoming.clone().into())
                                .await
                            {
                                let _ = self_cloned
                                    .messages_use_case
                                    .status_response(&connection, false)
                                    .await;
                                continue;
                            }

                            let message = entity::websocket::ConnectedMessage {
                                connection: connection.clone(),
                                message: incoming,
                            };

                            let mut ws_lock = ws.lock().await;

                            if ws_lock.message_queues.contains_key(&inner_message.chat_id) {
                                let _ = ws_lock
                                    .message_queues
                                    .get_mut(&inner_message.chat_id)
                                    .unwrap()
                                    .0
                                    .send(message);
                                log::info!("Message has been successfully added to the queue");
                            } else {
                                log::info!(
                                    "There is no subscribers to receive message in the queue"
                                );
                                if let Err(err) = self_cloned
                                    .messages_use_case
                                    .db
                                    .insert_message(message.message)
                                    .await
                                {
                                    log::info!("Error inserting message into database: {}", err);
                                    let _ = self_cloned
                                        .messages_use_case
                                        .status_response(&connection, false)
                                        .await;
                                    continue;
                                }

                                let _ = self_cloned
                                    .messages_use_case
                                    .status_response(&connection, true)
                                    .await;
                            }
                        }
                        "subscribe" => {
                            let message_inner = match incoming.message {
                                Some(val) => val,
                                None => {
                                    log::error!("Error parsing subscription message");
                                    let _ = self_cloned
                                        .messages_use_case
                                        .status_response(&connection, false)
                                        .await;
                                    continue;
                                }
                            };

                            let chat_id = match decode_base64(message_inner.chat_id.clone()).await {
                                Ok(chat_id) => chat_id,
                                Err(err) => {
                                    log::error!("Error decoding chat ID: {}", err);
                                    let _ = self_cloned
                                        .messages_use_case
                                        .status_response(&connection, false)
                                        .await;
                                    continue;
                                }
                            };

                            self_cloned
                                .websocket_use_case
                                .handle_subscribe(
                                    ws.clone(),
                                    connection.clone(),
                                    &message_inner.chat_id,
                                )
                                .await;
                            let _ = self_cloned
                                .messages_use_case
                                .status_response(&connection, true)
                                .await;
                            let _ = self_cloned
                                .messages_use_case
                                .unread_message_response(&connection, &chat_id, message_inner.nonce)
                                .await;
                            let _ = self_cloned
                                .messages_use_case
                                .wait_event_response(&connection, &message_inner.chat_id)
                                .await;
                        }
                        "unsubscribe" => {
                            let message_inner = match incoming.message {
                                Some(val) => val,
                                None => {
                                    log::error!("Error parsing unsubscribe message");
                                    let _ = self_cloned
                                        .messages_use_case
                                        .status_response(&connection, false)
                                        .await;
                                    continue;
                                }
                            };

                            self_cloned
                                .websocket_use_case
                                .handle_unsubscribe(
                                    ws.clone(),
                                    connection.clone(),
                                    &message_inner.chat_id,
                                )
                                .await;
                            let _ = self_cloned
                                .messages_use_case
                                .status_response(&connection, true)
                                .await;
                        }
                        _ => {
                            log::warn!("unknown request type: {}", incoming.rtype);
                            let _ = self_cloned
                                .messages_use_case
                                .status_response(&connection, false)
                                .await;
                        }
                    }
                }
            }

            self_cloned
                .websocket_use_case
                .disconnect(ws, connection)
                .await;
        })
    }
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct WebSocketActorMessage {
    pub connection: WebSocketConnection,
    pub stream: flume::Receiver<Message>,
}
