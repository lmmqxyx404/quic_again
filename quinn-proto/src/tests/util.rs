use std::{
    io::{self, Write},
    str,
    sync::Arc,
};

use lazy_static::lazy_static;
use rustls::{client::WebPkiServerVerifier, pki_types::CertificateDer, KeyLogFile};

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
    pub(crate) static ref CERTIFIED_KEY: rcgen::CertifiedKey =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
}

pub(super) fn subscribe() -> tracing::subscriber::DefaultGuard {
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(|| TestWriter)
        .finish();
    tracing::subscriber::set_default(sub)
}
struct TestWriter;
impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!(
            "{}",
            str::from_utf8(buf).expect("tried to log invalid UTF-8")
        );
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}
