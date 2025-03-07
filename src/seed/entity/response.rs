use serde::Serialize;

use super::message::OutcomeMessage;

/// Response types for seed operations.
///
/// This enum represents different types of responses that can be sent
/// from the server to the client, including event notifications and status updates.
#[derive(Serialize)]
#[serde(tag = "type", content = "response")]
pub enum SeedResponse {
    /// Represents a new event notification.
    /// 
    /// This variant is used when a new event occurs that needs to be sent to the client.
    #[serde(rename = "event")]
    NewEvent(NewEventDetail),

    /// Represents a wait event notification.
    /// 
    /// This variant is used when the client needs to wait for an event to complete.
    #[serde(rename = "event")]
    WaitEvent(WaitEventDetail),

    /// Represents a status response.
    /// 
    /// This variant is used to communicate the success or failure of an operation.
    #[serde(rename = "response")]
    Status(StatusResponse),
}

/// Details for a new event notification.
///
/// Contains information about the type of event and the message content.
#[derive(Serialize)]
pub struct NewEventDetail {
    /// The type of the event.
    /// 
    /// This field is renamed to "type" in the serialized JSON.
    #[serde(rename = "type")]
    pub rtype: String,

    /// The message content associated with this event.
    pub message: OutcomeMessage,
}

/// Details for a wait event notification.
///
/// Contains information about the type of event and the chat ID to wait on.
#[derive(Serialize)]
pub struct WaitEventDetail {
    /// The type of the wait event.
    /// 
    /// This field is renamed to "type" in the serialized JSON.
    #[serde(rename = "type")]
    pub rtype: String,

    /// The chat ID associated with this wait event.
    /// 
    /// This field is renamed to "queueId" in the serialized JSON.
    #[serde(rename = "queueId")]
    pub chat_id: String,
}

/// Response containing operation status.
///
/// A simple response that indicates whether an operation succeeded or failed.
#[derive(Serialize)]
pub struct StatusResponse {
    /// The status of the operation.
    /// 
    /// true indicates success, false indicates failure.
    pub status: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that StatusResponse serializes correctly.
    /// 
    /// Verifies that the JSON serialization produces the expected format.
    #[test]
    fn test_status_serialization() {
        let response = SeedResponse::Status(StatusResponse { status: true });
        let serialized = serde_json::to_string(&response).unwrap();
        let expected = r#"{"type":"response","response":{"status":true}}"#;
        assert_eq!(serialized, expected);
    }
}
