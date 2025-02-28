use std::{ops::ControlFlow, sync::Arc};

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
                    if let ControlFlow::Break(_) =
                        match_message(&self_cloned, &ws, &connection, incoming).await
                    {
                        continue;
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

async fn match_message<MR, DB>(
    actor: &WebSocketActor<MR, DB>,
    ws: &Arc<Mutex<WebSocketManager>>,
    connection: &Arc<WebSocketConnection>,
    incoming: IncomeMessage,
) -> ControlFlow<()>
where
    MR: MessagesRepository + Clone,
    DB: MessagesDB + Clone,
{
    match incoming.clone() {
        IncomeMessage::Ping => {
            let _ = actor
                .messages_use_case
                .status_response(connection, true)
                .await;
        }
        IncomeMessage::Send(msg) => {
            if !actor
                .messages_use_case
                .is_valid_message(msg.clone().into())
                .await
            {
                let _ = actor
                    .messages_use_case
                    .status_response(connection, false)
                    .await;
                return ControlFlow::Break(());
            }

            let message = entity::websocket::ConnectedMessage {
                connection: connection.clone(),
                message: incoming,
            };

            let mut ws_lock = ws.lock().await;

            if ws_lock.message_queues.contains_key(&msg.chat_id) {
                let _ = ws_lock
                    .message_queues
                    .get_mut(&msg.chat_id)
                    .unwrap()
                    .0
                    .send(message);
                log::info!("Message has been successfully added to the queue");
            } else {
                log::info!("There is no subscribers to receive message in the queue");
                if let Err(err) = actor.messages_use_case.db.insert_message(msg).await {
                    log::info!("Error inserting message into database: {}", err);
                    let _ = actor
                        .messages_use_case
                        .status_response(connection, false)
                        .await;
                    return ControlFlow::Break(());
                }

                let _ = actor
                    .messages_use_case
                    .status_response(connection, true)
                    .await;
            }
        }
        IncomeMessage::Subscribe(msg) => {
            let chat_id = match decode_base64(msg.chat_id.clone()).await {
                Ok(chat_id) => chat_id,
                Err(err) => {
                    log::error!("Error decoding chat ID: {}", err);
                    let _ = actor
                        .messages_use_case
                        .status_response(connection, false)
                        .await;
                    return ControlFlow::Break(());
                }
            };

            actor
                .websocket_use_case
                .handle_subscribe(ws.clone(), connection.clone(), &msg.chat_id)
                .await;
            let _ = actor
                .messages_use_case
                .status_response(connection, true)
                .await;
            let _ = actor
                .messages_use_case
                .unread_message_response(connection, &chat_id, msg.nonce)
                .await;
            let _ = actor
                .messages_use_case
                .wait_event_response(connection, &msg.chat_id)
                .await;
        }
        IncomeMessage::Unsubscribe(msg) => {
            actor
                .websocket_use_case
                .handle_unsubscribe(ws.clone(), connection.clone(), &msg.chat_id)
                .await;
            let _ = actor
                .messages_use_case
                .status_response(connection, true)
                .await;
        }
        IncomeMessage::None => {}
    }
    ControlFlow::Continue(())
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct WebSocketActorMessage {
    pub connection: WebSocketConnection,
    pub stream: flume::Receiver<Message>,
}
