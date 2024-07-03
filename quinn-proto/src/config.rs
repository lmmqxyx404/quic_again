use std::sync::Arc;

use rand::RngCore;

use crate::{cid_generator::HashedConnectionIdGenerator, crypto::{self, HmacKey}, ConnectionIdGenerator};

/// Global configuration for the endpoint, affecting all connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
pub struct EndpointConfig {
    /// 1. CID generator factory
    ///
    /// Create a cid generator for local cid in Endpoint struct
    pub(crate) connection_id_generator_factory:
        Arc<dyn Fn() -> Box<dyn ConnectionIdGenerator> + Send + Sync>,

    /// 2. Optional seed to be used internally for random number generation
    pub(crate) rng_seed: Option<[u8; 32]>,
}

impl EndpointConfig {
    /// Create a default config with a particular `reset_key`
    pub fn new(reset_key: Arc<dyn HmacKey>) -> Self {
        let cid_factory =
            || -> Box<dyn ConnectionIdGenerator> { Box::<HashedConnectionIdGenerator>::default() };
        Self {
            connection_id_generator_factory: Arc::new(cid_factory),
            rng_seed: None,
        }
    }
}
/// Parameters governing incoming connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
pub struct ServerConfig {}

#[cfg(feature = "ring")]
impl Default for EndpointConfig {
    fn default() -> Self {
        let mut reset_key = [0; 64];
        rand::thread_rng().fill_bytes(&mut reset_key);

        Self::new(Arc::new(ring::hmac::Key::new(
            ring::hmac::HMAC_SHA256,
            &reset_key,
        )))
    }
}

/// Configuration for outgoing connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
#[non_exhaustive]
pub struct ClientConfig {}

impl ClientConfig {
    /// 1. Create a default config with a particular cryptographic config
    pub fn new(crypto: Arc<dyn crypto::ClientConfig>) -> Self {
        todo!()
    }
}
