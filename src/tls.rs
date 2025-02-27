use std::{fs::File, io::BufReader};

use anyhow::Ok;
use rustls_pemfile::{certs, pkcs8_private_keys};

pub fn load_rustls_config() -> Result<rustls::ServerConfig, anyhow::Error> {
    let mut cert_file = BufReader::new(File::open("cert.pem")?);
    let mut key_file = BufReader::new(File::open("key.pem")?);

    let cert_chain = certs(&mut cert_file).collect::<Result<Vec<_>, _>>()?;

    let mut keys = pkcs8_private_keys(&mut key_file).collect::<Result<Vec<_>, _>>()?;

    let key = keys.remove(0);

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key.into())?;

    Ok(config)
}
