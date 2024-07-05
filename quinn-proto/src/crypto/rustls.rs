use std::sync::Arc;

use rustls::{
    client::danger::ServerCertVerifier, pki_types::ServerName, quic::{Connection, Secrets, Suite, Version}
};

use crate::{crypto, endpoint::ConnectError, transport_parameters::TransportParameters};

use super::UnsupportedVersion;

/// 1. A QUIC-compatible TLS client configuration
///
/// Can be constructed via [`ClientConfig::with_root_certificates()`][root_certs],
/// [`ClientConfig::with_platform_verifier()`][platform] or by using the [`TryFrom`] implementation with a
/// custom [`rustls::ClientConfig`]. A pre-existing `ClientConfig` must have TLS 1.3 support enabled for
/// this to work. 0-RTT support is available if `enable_early_data` is set to `true`.
///
/// [root_certs]: crate::config::ClientConfig::with_root_certificates()
/// [platform]: crate::config::ClientConfig::with_platform_verifier()
pub struct QuicClientConfig {
    pub(crate) inner: Arc<rustls::ClientConfig>,
    initial: Suite,
}

impl crypto::ClientConfig for QuicClientConfig {
    fn start_session(
        self: std::sync::Arc<Self>,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Box<dyn crypto::Session>, ConnectError> {
        let version = interpret_version(version)?;
        Ok(Box::new(TlsSession {
            version,
            got_handshake_data: false,
            next_secrets: None,
            inner: rustls::quic::Connection::Client(
                rustls::quic::ClientConnection::new(
                    self.inner.clone(),
                    version,
                    ServerName::try_from(server_name)
                        .map_err(|_| ConnectError::InvalidServerName(server_name.into()))?
                        .to_owned(),
                    to_vec(params),
                )
                .unwrap(),
            ),
            suite: self.initial,
        }))
    }
}

impl QuicClientConfig {
    pub(crate) fn inner(verifier: Arc<dyn ServerCertVerifier>) -> rustls::ClientConfig {
        let mut config = rustls::ClientConfig::builder_with_provider(
            rustls::crypto::ring::default_provider().into(),
        )
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap() // The *ring* default provider supports TLS 1.3
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();

        config.enable_early_data = true;
        config
    }
}

impl TryFrom<rustls::ClientConfig> for QuicClientConfig {
    type Error = NoInitialCipherSuite;

    fn try_from(inner: rustls::ClientConfig) -> Result<Self, Self::Error> {
        Arc::new(inner).try_into()
    }
}

impl TryFrom<Arc<rustls::ClientConfig>> for QuicClientConfig {
    type Error = NoInitialCipherSuite;

    fn try_from(inner: Arc<rustls::ClientConfig>) -> Result<Self, Self::Error> {
        Ok(Self {
            initial: initial_suite_from_provider(inner.crypto_provider())
                .ok_or(NoInitialCipherSuite { specific: false })?,
            inner,
        })
    }
}

/// 2. The initial cipher suite (AES-128-GCM-SHA256) is not available
///
/// When the cipher suite is supplied `with_initial()`, it must be
/// [`CipherSuite::TLS13_AES_128_GCM_SHA256`]. When the cipher suite is derived from a config's
/// [`CryptoProvider`][provider], that provider must reference a cipher suite with the same ID.
///
/// [provider]: rustls::crypto::CryptoProvider
#[derive(Clone, Debug)]
pub struct NoInitialCipherSuite {
    /// Whether the initial cipher suite was supplied by the caller
    specific: bool,
}

pub(crate) fn initial_suite_from_provider(
    provider: &Arc<rustls::crypto::CryptoProvider>,
) -> Option<Suite> {
    provider
        .cipher_suites
        .iter()
        .find_map(|cs| match (cs.suite(), cs.tls13()) {
            (rustls::CipherSuite::TLS13_AES_128_GCM_SHA256, Some(suite)) => {
                Some(suite.quic_suite())
            }
            _ => None,
        })
        .flatten()
}

fn interpret_version(version: u32) -> Result<Version, UnsupportedVersion> {
    match version {
        0xff00_001d..=0xff00_0020 => Ok(Version::V1Draft),
        0x0000_0001 | 0xff00_0021..=0xff00_0022 => Ok(Version::V1),
        _ => Err(UnsupportedVersion),
    }
}

/// A rustls TLS session
pub struct TlsSession {
    version: Version,
    got_handshake_data: bool,
    next_secrets: Option<Secrets>,
    inner: Connection,
    suite: Suite,
}

impl crypto::Session for TlsSession {}


fn to_vec(params: &TransportParameters) -> Vec<u8> {
    let mut bytes = Vec::new();
    // todo: very important
    // params.write(&mut bytes);
    bytes
}