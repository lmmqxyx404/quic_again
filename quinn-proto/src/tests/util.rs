use std::sync::Arc;

use rustls::pki_types::CertificateDer;
use lazy_static::lazy_static;

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
    let mut roots = rustls::RootCertStore::empty();
    for cert in certs.unwrap_or_else(|| vec![CERTIFIED_KEY.cert.der().clone()]) {
        roots.add(cert).unwrap();
    }

    todo!()
}


lazy_static! {
    
    pub(crate) static ref CERTIFIED_KEY: rcgen::CertifiedKey =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
}
