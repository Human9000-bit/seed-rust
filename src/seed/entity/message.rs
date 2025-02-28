use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "message")]
pub enum IncomeMessage {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "send")]
    Send(Message),
    #[serde(rename = "subscribe")]
    Subscribe(Message),
    #[serde(rename = "unsubscribe")]
    Unsubscribe(Message),
    None,
}

#[derive(Serialize, Deserialize, Clone, Default)]
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
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct OutcomeMessage {
    pub nonce: usize,
    #[serde(rename = "queueId")]
    pub chat_id: String,
    pub signature: String,
    pub content: String,
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}

impl From<OutcomeMessage> for Message {
    fn from(msg: OutcomeMessage) -> Self {
        Self {
            nonce: msg.nonce,
            chat_id: msg.chat_id,
            signature: msg.signature,
            content: msg.content,
            content_iv: msg.content_iv,
        }
    }
}

impl From<Message> for OutcomeMessage {
    fn from(msg: Message) -> Self {
        Self {
            nonce: msg.nonce,
            chat_id: msg.chat_id,
            signature: msg.signature,
            content: msg.content,
            content_iv: msg.content_iv,
        }
    }
}

impl From<IncomeMessage> for Option<Message> {
    fn from(msg: IncomeMessage) -> Self {
        match msg {
            IncomeMessage::Send(message) => Some(message),
            IncomeMessage::Subscribe(message) => Some(message),
            IncomeMessage::Unsubscribe(message) => Some(message),
            _ => None,
        }
    }
}

impl From<IncomeMessage> for OutcomeMessage {
    fn from(msg: IncomeMessage) -> Self {
        match msg {
            IncomeMessage::Send(message) => OutcomeMessage::from(message),
            IncomeMessage::Subscribe(message) => OutcomeMessage::from(message),
            IncomeMessage::Unsubscribe(message) => OutcomeMessage::from(message),
            _ => OutcomeMessage::from(Message::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_income_message_send_serialization() {
        let message = Message {
            nonce: 12345,
            chat_id: "chat-123456".to_string(),
            signature: "abc123def456".to_string(),
            content: "encrypted_content".to_string(),
            content_iv: "iv_data".to_string(),
        };

        let send = IncomeMessage::Send(message);

        // Serialize to JSON string
        let serialized = serde_json::to_string(&send).unwrap();

        let expected = r#"{"type":"send","message":{"nonce":12345,"queueId":"chat-123456","signature":"abc123def456","content":"encrypted_content","contentIV":"iv_data"}}"#;

        assert_eq!(serialized, expected);
    }
}
