use std::sync::Arc;

use rand::RngCore;

use crate::{
    cid_generator::HashedConnectionIdGenerator, crypto::{self, HmacKey}, endpoint::TransportConfig, shared::ConnectionId, ConnectionIdGenerator, RandomConnectionIdGenerator, VarInt, MAX_CID_SIZE
};

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
    /// 3.
    pub(crate) max_udp_payload_size: VarInt,
}

impl EndpointConfig {
    /// 1. Create a default config with a particular `reset_key`
    pub fn new(reset_key: Arc<dyn HmacKey>) -> Self {
        let cid_factory =
            || -> Box<dyn ConnectionIdGenerator> { Box::<HashedConnectionIdGenerator>::default() };
        Self {
            connection_id_generator_factory: Arc::new(cid_factory),
            rng_seed: None,
            max_udp_payload_size: (1500u32 - 28).into(), // Ethernet MTU minus IP + UDP headers
        }
    }

    /// 2. Get the current value of `max_udp_payload_size`
    ///
    /// While most parameters don't need to be readable, this must be exposed to allow higher-level
    /// layers, e.g. the `quinn` crate, to determine how large a receive buffer to allocate to
    /// support an externally-defined `EndpointConfig`.
    ///
    /// While `get_` accessors are typically unidiomatic in Rust, we favor concision for setters,
    /// which will be used far more heavily.
    #[doc(hidden)]
    pub fn get_max_udp_payload_size(&self) -> u64 {
        self.max_udp_payload_size.into()
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
pub struct ClientConfig {
    /// 1. Provider that populates the destination connection ID of Initial Packets
    pub(crate) initial_dst_cid_provider: Arc<dyn Fn() -> ConnectionId + Send + Sync>,
    /// 2. Transport configuration to use
    pub(crate) transport: Arc<TransportConfig>,
    /// 3. Cryptographic configuration to use
    pub(crate) crypto: Arc<dyn crypto::ClientConfig>,
    /// 4.QUIC protocol version to use
    pub(crate) version: u32,
}

impl ClientConfig {
    /// 1. Create a default config with a particular cryptographic config
    pub fn new(crypto: Arc<dyn crypto::ClientConfig>) -> Self {
        Self {
            initial_dst_cid_provider: Arc::new(|| {
                RandomConnectionIdGenerator::new(MAX_CID_SIZE).generate_cid()
            }),
            transport: Default::default(),
            crypto,
            version: 1,
        }
    }
}
