use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "type")]
    rtype: String,

    #[serde(rename = "queueId")]
    chat_id: String,

    nonce: usize,
}