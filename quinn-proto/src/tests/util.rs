use std::sync::Arc;

use rustls::pki_types::CertificateDer;

use crate::{config::ClientConfig, crypto::rustls::QuicClientConfig};

pub(super) fn client_config() -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto()))
}

pub(super) fn client_crypto() -> QuicClientConfig {
    client_crypto_inner(None, None)
}

fn client_crypto_inner(
    certs: Option<Vec<CertificateDer<'static>>>,
    alpn: Option<Vec<Vec<u8>>>,
) -> QuicClientConfig {
    todo!()
}
