use serde::{Deserialize, Serialize};

/// Represents incoming messages from clients with different action types.
/// Uses a tagged enum format for JSON serialization/deserialization.
#[derive(Deserialize, Clone)]
#[serde(tag = "type", content = "message")]
pub enum IncomeMessage {
    /// Simple ping message for connection checking
    #[serde(rename = "ping")]
    Ping,
    /// Message to send content to a specific chat
    #[serde(rename = "send")]
    Send(Message),
    /// Message to subscribe to a specific chat
    #[serde(rename = "subscribe")]
    Subscribe(Message),
    /// Message to unsubscribe from a specific chat
    #[serde(rename = "unsubscribe")]
    Unsubscribe(Message),
    /// Empty message or placeholder
    None,
}

/// Represents the core message structure used for communication.
/// Contains encryption and identification details.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Message {
    /// Unique number for message sequencing and identification
    pub nonce: usize,
    /// Identifier for the chat/queue this message belongs to
    #[serde(rename = "queueId")]
    pub chat_id: String,
    /// Cryptographic signature for message verification
    pub signature: String,
    /// Encrypted message content
    pub content: String,
    /// Initialization vector used for content encryption
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}

/// Outcoming message struct for sending responses back to clients.
/// Has the same structure as Message but separated for clear direction indication.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct OutcomeMessage {
    /// Unique number for message sequencing and identification
    pub nonce: usize,
    /// Identifier for the chat/queue this message belongs to
    #[serde(rename = "queueId")]
    pub chat_id: String,
    /// Cryptographic signature for message verification
    pub signature: String,
    /// Encrypted message content
    pub content: String,
    /// Initialization vector used for content encryption
    #[serde(rename = "contentIV")]
    pub content_iv: String,
}

/// Conversion implementation from OutcomeMessage to Message.
/// Allows direct conversion between outgoing and internal message formats.
impl From<OutcomeMessage> for Message {
    /// Converts an OutcomeMessage to a Message, preserving all fields.
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

/// Conversion implementation from Message to OutcomeMessage.
/// Enables preparing internal messages for sending to clients.
impl From<Message> for OutcomeMessage {
    /// Converts a Message to an OutcomeMessage, copying all fields directly.
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

/// Conversion implementation from IncomeMessage to Option<Message>.
/// Extracts the Message content from action-type messages when present.
impl From<IncomeMessage> for Option<Message> {
    /// Converts an IncomeMessage to Option<Message>.
    /// Returns Some(message) for action messages containing a Message payload,
    /// or None for messages without content (like Ping).
    fn from(msg: IncomeMessage) -> Self {
        match msg {
            IncomeMessage::Send(message) => Some(message),
            IncomeMessage::Subscribe(message) => Some(message),
            IncomeMessage::Unsubscribe(message) => Some(message),
            _ => None,
        }
    }
}

/// Conversion implementation from IncomeMessage to OutcomeMessage.
/// Allows direct handling of incoming messages as outgoing responses.
impl From<IncomeMessage> for OutcomeMessage {
    /// Converts an IncomeMessage to an OutcomeMessage.
    /// For action messages containing a Message payload, converts that Message.
    /// For other message types, returns a default OutcomeMessage.
    fn from(msg: IncomeMessage) -> Self {
        match msg {
            IncomeMessage::Send(message) => OutcomeMessage::from(message),
            IncomeMessage::Subscribe(message) => OutcomeMessage::from(message),
            IncomeMessage::Unsubscribe(message) => OutcomeMessage::from(message),
            _ => OutcomeMessage::from(Message::default()),
        }
    }
}

/// Test module for message serialization and conversion functions
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that IncomeMessage::Send deserializes correctly from JSON
    /// Verifies the tag/content structure and field names are as expected
    #[test]
    fn test_income_message_send_deserialization() {
        let json_str = r#"{"type":"send","message":{"nonce":12345,"queueId":"chat-123456","signature":"abc123def456","content":"encrypted_content","contentIV":"iv_data"}}"#;

        // Deserialize from JSON string
        let deserialized: IncomeMessage = serde_json::from_str(json_str).unwrap();

        // Check if it deserialized to the expected type and with correct values
        match deserialized {
            IncomeMessage::Send(message) => {
                assert_eq!(message.nonce, 12345);
                assert_eq!(message.chat_id, "chat-123456");
                assert_eq!(message.signature, "abc123def456");
                assert_eq!(message.content, "encrypted_content");
                assert_eq!(message.content_iv, "iv_data");
            },
            _ => panic!("Deserialized to wrong variant, expected IncomeMessage::Send"),
        }
    }
}
