use serde::{Deserialize, Serialize};

/// Incoming message struct
#[derive(Serialize, Deserialize)]
pub struct IncomeMessage {
    /// Response type
    #[serde(rename = "type")]
    pub rtype: String,
    /// The message
    pub message: Message,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub nonce: usize,
    #[serde(rename = "queueId")]
    pub chat_id: String,
    pub signature: String,
    pub content: String,
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}

/// Outcoming message struct
#[derive(Serialize, Deserialize)]
pub struct OutcomeMessage {
    pub nonce: usize,
    #[serde(rename = "queueId")]
    pub chat_id: String,
    pub signature: String,
    pub content: String,
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}
