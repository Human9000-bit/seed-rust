use thiserror::Error;

#[derive(Error, Debug)]
pub enum SeedError {
    #[error("invalid nonce")]
    InvalidNonce
}
