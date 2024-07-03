use std::sync::Arc;

use crate::{endpoint::ConnectError, transport_parameters::TransportParameters};

/// 1. Cryptography interface based on *ring*
#[cfg(feature = "ring")]
pub(crate) mod ring;
/// TLS interface based on rustls
#[cfg(feature = "rustls")]
pub mod rustls;

/// 3. A cryptographic session (commonly TLS)
pub trait Session: Send + Sync + 'static {}

/// 1. A key for signing with HMAC-based algorithms
pub trait HmacKey: Send + Sync {
    /// Method for signing a message
    fn sign(&self, data: &[u8], signature_out: &mut [u8]);
    /// Length of `sign`'s output
    fn signature_len(&self) -> usize;
    /// Method for verifying a message
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), CryptoError>;
}

/// Generic crypto errors
#[derive(Debug)]
pub struct CryptoError;

/// 2. Client-side configuration for the crypto protocol
pub trait ClientConfig: Send + Sync {
    /// Start a client session with this configuration
    fn start_session(
        self: Arc<Self>,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Box<dyn Session>, ConnectError>;
}
