use crate::crypto;

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
        params: &crate::transport_parameters::TransportParameters,
    ) {
        todo!()
    }
}
