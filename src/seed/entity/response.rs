use serde::{Deserialize, Serialize};

use super::message::OutcomeMessage;

#[derive(Deserialize, Serialize)]
pub struct NewEventResponse {
    #[serde(rename = "type")]
    rtype: String,
    event: NewEventDetail,
}

#[derive(Serialize, Deserialize)]
pub struct NewEventDetail {
    #[serde(rename = "type")]
    rtype: String,

    message: OutcomeMessage,
}

#[derive(Serialize, Deserialize)]
pub struct WaitEventResponse {
    #[serde(rename = "type")]
    rtype: String,

    event: WaitEventDetail,
}

#[derive(Serialize, Deserialize)]
pub struct WaitEventDetail {
    #[serde(rename = "type")]
    rtype: String,

    #[serde(rename = "queueId")]
    chat_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    #[serde(rename = "type")]
    rtype: String,

    status: bool,
}
