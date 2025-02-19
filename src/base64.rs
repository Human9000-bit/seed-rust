use base64::{engine::general_purpose, Engine as _};

pub async fn encode_base64(input: &[u8]) -> String {
    general_purpose::STANDARD.encode(input)
}

pub async fn decode_base64(input: String) -> Result<Vec<u8>, base64::DecodeError> {
    general_purpose::STANDARD.decode(input)
}
