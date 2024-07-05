use std::sync::Arc;

use rustls::client::danger::ServerCertVerifier;

use crate::{crypto, endpoint::ConnectError, transport_parameters::TransportParameters};

/// A QUIC-compatible TLS client configuration
///
/// Can be constructed via [`ClientConfig::with_root_certificates()`][root_certs],
/// [`ClientConfig::with_platform_verifier()`][platform] or by using the [`TryFrom`] implementation with a
/// custom [`rustls::ClientConfig`]. A pre-existing `ClientConfig` must have TLS 1.3 support enabled for
/// this to work. 0-RTT support is available if `enable_early_data` is set to `true`.
///
/// [root_certs]: crate::config::ClientConfig::with_root_certificates()
/// [platform]: crate::config::ClientConfig::with_platform_verifier()
pub struct QuicClientConfig {}

impl crypto::ClientConfig for QuicClientConfig {
    fn start_session(
        self: std::sync::Arc<Self>,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Box<dyn crypto::Session>, ConnectError> {
        todo!()
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
