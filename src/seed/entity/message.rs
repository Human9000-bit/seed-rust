use serde::{Deserialize, Serialize};

/// Incoming message struct
#[derive(Serialize, Deserialize, Clone)]
pub struct IncomeMessage {
    /// Response type
    #[serde(rename = "type")]
    pub rtype: String,
    /// The message
    pub message: Option<Message>,
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
            message: Some(message),
        }
    }
}

impl From<(String, Message)> for IncomeMessage {
    fn from((rtype, message): (String, Message)) -> Self {
        IncomeMessage {
            rtype,
            message: Some(message),
        }
    }
}

impl From<IncomeMessage> for OutcomeMessage {
    fn from(income: IncomeMessage) -> Self {
        let message = income.message.unwrap();
        Self {
            nonce: message.nonce,
            chat_id: message.chat_id,
            signature: message.signature,
            content: message.content,
            content_iv: message.content_iv,
        }
    }
}
