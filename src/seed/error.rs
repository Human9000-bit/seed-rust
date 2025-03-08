use thiserror::Error;

/// Represents errors that can occur in seed protocol.
#[derive(Error, Debug)]
pub enum SeedError {
    /// Error returned when a nonce is invalid.
    #[error("invalid nonce")]
    InvalidNonce,
}
