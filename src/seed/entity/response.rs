use serde::{Deserialize, Serialize};

use super::message::OutcomeMessage;

#[derive(Deserialize, Serialize)]
pub struct NewEventResponse {
    #[serde(rename = "type")]
    pub rtype: String,
    pub event: NewEventDetail,
}

#[derive(Serialize, Deserialize)]
pub struct NewEventDetail {
    #[serde(rename = "type")]
    pub rtype: String,

    pub message: OutcomeMessage,
}

#[derive(Serialize, Deserialize)]
pub struct WaitEventResponse {
    #[serde(rename = "type")]
    pub rtype: String,

    pub event: WaitEventDetail,
}

#[derive(Serialize, Deserialize)]
pub struct WaitEventDetail {
    #[serde(rename = "type")]
    pub rtype: String,

    #[serde(rename = "queueId")]
    pub chat_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    #[serde(rename = "type")]
    pub rtype: String,

    pub status: bool,
}
