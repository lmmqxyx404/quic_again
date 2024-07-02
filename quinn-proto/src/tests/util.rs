use std::{
    ops::RangeFrom,
    sync::{Arc, Mutex},
};

use rustls::{client::WebPkiServerVerifier, pki_types::CertificateDer, KeyLogFile};

use crate::{config::ClientConfig, crypto::rustls::QuicClientConfig};
use lazy_static::lazy_static;

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

    let mut inner = QuicClientConfig::inner(
        WebPkiServerVerifier::builder(Arc::new(roots))
            .build()
            .unwrap(),
    );

    inner.key_log = Arc::new(KeyLogFile::new());
    if let Some(alpn) = alpn {
        inner.alpn_protocols = alpn;
    }

    inner.try_into().unwrap()
}

lazy_static! {
    pub static ref SERVER_PORTS: Mutex<RangeFrom<u16>> = Mutex::new(4433..);
    pub static ref CLIENT_PORTS: Mutex<RangeFrom<u16>> = Mutex::new(44433..);
    pub(crate) static ref CERTIFIED_KEY: rcgen::CertifiedKey =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
}
