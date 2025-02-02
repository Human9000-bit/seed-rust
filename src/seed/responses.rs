//use hmac::{Mac, digest::KeyInit};
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseType {
    #[serde(rename = "subscribe")]
    Subscribe(SubscribeResponse),
    #[serde(rename = "response")]
    Response(Response),
    #[serde(rename = "event")]
    Event(EventResponse),
    #[serde(rename = "send")]
    Send(SendMessage),
}

#[derive(Serialize, Deserialize)]
pub struct EventResponse {
    #[serde(rename = "queueId")]
    pub queue_id: usize,
    #[serde(rename = "type")]
    event_type: EventType,
}

#[derive(Serialize, Deserialize)]
pub enum EventType {
    New,
    Wait
}

#[derive(Serialize, Deserialize)]
pub struct SendMessage {
    #[serde(rename = "queueId")]
    queue_id: String,
    nonce: u64,
    signature: String,
    content: String,
    #[serde(rename = "contentIV")]
    content_iv: String
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub status: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeResponse {
    #[serde(rename = "queueId")]
    queue_id: String,
    nonce: u64,
}

// async fn generate_signature(content_string: &str, private_key: &[u8]) -> String {
//     // Create HMAC-SHA256 instance
//     type HmacSha256 = hmac::Hmac<sha2::Sha256>;

//     // Prepare the data with "SIGNATURE:" prefix
//     let data = format!("SIGNATURE:{}", content_string);

//     // Create new HMAC instance
//     let mut mac = HmacSha256::from_core(hmac::HmacCore::new_from_slice(private_key)
//         .expect("HMAC can take key of any size"));

//     // Add data to MAC
//     mac.update(data.as_bytes());

//     // Calculate HMAC
//     let result = mac.finalize();
    
//     // Convert to hex string
//     hex::encode(result.into_bytes())
// }