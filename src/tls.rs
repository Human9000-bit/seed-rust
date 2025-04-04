use std::{fs::File, io::BufReader};

use anyhow::Result;
use rustls_pemfile::{certs, pkcs8_private_keys};

/// Loads and configures TLS settings for a Rustls server.
///
/// This function reads certificate and private key files from the current directory,
/// and creates a server configuration with no client authentication required.
///
/// # Files Required
/// - `cert.pem`: PEM-encoded certificate chain file
/// - `key.pem`: PEM-encoded private key file in PKCS#8 format
///
/// # Returns
/// - A `Result` containing the configured `ServerConfig` or an error
///
/// # Errors
/// - If certificate or key files cannot be read
/// - If PEM parsing fails
/// - If the certificate or key are invalid
pub fn load_rustls_config() -> Result<rustls::ServerConfig> {
    // Install AWS-LC as the cryptographic provider
    let _ = rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    // Open and prepare certificate and key files for reading
    let mut cert_file = BufReader::new(File::open("cert.pem")?);
    let mut key_file = BufReader::new(File::open("key.pem")?);

    // Parse certificate chain from PEM file
    let cert_chain = certs(&mut cert_file).collect::<Result<Vec<_>, _>>()?;

    // Parse private keys from PEM file
    let mut keys = pkcs8_private_keys(&mut key_file).collect::<Result<Vec<_>, _>>()?;

    // Extract the first key (assuming there's at least one)
    let key = keys.remove(0);

    // Build server configuration with the parsed certificates and key
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth() // Don't require client certificates
        .with_single_cert(cert_chain, key.into())?;

    Ok(config)
}
