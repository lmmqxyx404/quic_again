use std::sync::Arc;

use rand::RngCore;

use crate::config::{EndpointConfig, ServerConfig};

/// The main entry point to the library
///
/// This object performs no I/O whatsoever. Instead, it consumes incoming packets and
/// connection-generated events via `handle` and `handle_event`.
pub struct Endpoint {}

impl Endpoint {
    /// Create a new endpoint
    ///
    /// `allow_mtud` enables path MTU detection when requested by `Connection` configuration for
    /// better performance. This requires that outgoing packets are never fragmented, which can be
    /// achieved via e.g. the `IPV6_DONTFRAG` socket option.
    ///
    /// If `rng_seed` is provided, it will be used to initialize the endpoint's rng (having priority
    /// over the rng seed configured in [`EndpointConfig`]). Note that the `rng_seed` parameter will
    /// be removed in a future release, so prefer setting it to `None` and configuring rng seeds
    /// using [`EndpointConfig::rng_seed`].
    pub fn new(
        config: Arc<EndpointConfig>,
        server_config: Option<Arc<ServerConfig>>,
        allow_mtud: bool,
        rng_seed: Option<[u8; 32]>,
    ) -> Self {
        let rng_seed = rng_seed.or(config.rng_seed);
        Self {}
    }
}
