use std::sync::Arc;

use crate::{crypto, endpoint::ConnectError, transport_parameters::TransportParameters};
use rustls::{client::danger::ServerCertVerifier, quic::Algorithm, Tls13CipherSuite};
/// A QUIC-compatible TLS client configuration
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

/// Produces QUIC initial keys from a TLS 1.3 ciphersuite and a QUIC key generation algorithm.
#[derive(Clone, Copy)]
pub struct Suite {
    /// The TLS 1.3 ciphersuite used to derive keys.
    pub suite: &'static Tls13CipherSuite,
    /// The QUIC key generation algorithm used to derive keys.
    pub quic: &'static dyn Algorithm,
}

impl crypto::ClientConfig for QuicClientConfig {
    fn start_session(
        self: Arc<Self>,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Box<dyn crypto::Session>, ConnectError> {
        todo!()
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

/// The initial cipher suite (AES-128-GCM-SHA256) is not available
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

impl std::fmt::Display for NoInitialCipherSuite {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self.specific {
            true => "invalid cipher suite specified",
            false => "no initial cipher suite found",
        })
    }
}

impl std::error::Error for NoInitialCipherSuite {}

pub(crate) fn initial_suite_from_provider(
    provider: &Arc<rustls::crypto::CryptoProvider>,
) -> Option<Suite> {
    todo!()
}
