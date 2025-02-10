use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IncomeMessage {
    #[serde(rename = "type")]
    pub rtype: String,
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
