use serde::{Deserialize, Serialize};

/// Incoming message struct
#[derive(Serialize, Deserialize, Clone)]
pub struct IncomeMessage {
    /// Response type
    #[serde(rename = "type")]
    pub rtype: String,
    /// The message
    pub message: Message,
}

#[derive(Serialize, Deserialize, Clone)]
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
#[derive(Serialize, Deserialize, Clone)]
pub struct OutcomeMessage {
    pub nonce: usize,
    #[serde(rename = "queueId")]
    pub chat_id: String,
    pub signature: String,
    pub content: String,
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}

impl From<Message> for IncomeMessage {
    fn from(message: Message) -> Self {
        IncomeMessage {
            rtype: "message".to_string(),
            message,
        }
    }
}

impl From<(String, Message)> for IncomeMessage {
    fn from((rtype, message): (String, Message)) -> Self {
        IncomeMessage { rtype, message }
    }
}

impl From<IncomeMessage> for OutcomeMessage {
    fn from(income: IncomeMessage) -> Self {
        Self {
            nonce: income.message.nonce,
            chat_id: income.message.chat_id,
            signature: income.message.signature,
            content: income.message.content,
            content_iv: income.message.content_iv,
        }
    }
}
