use std::{collections::hash_map, net::SocketAddr, sync::Arc, time::Instant};

use rand::RngCore;
use rustc_hash::FxHashMap;
use slab::Slab;
use thiserror::Error;

use crate::{
    config::{ClientConfig, EndpointConfig, ServerConfig},
    connection::Connection,
    shared::ConnectionId,
    transport_parameters::TransportParameters,
    ConnectionIdGenerator,
};

/// 1. The main entry point to the library
///
/// This object performs no I/O whatsoever. Instead, it consumes incoming packets and
/// connection-generated events via `handle` and `handle_event`.
pub struct Endpoint {
    /// 1.
    connections: Slab<ConnectionMeta>,
    /// 2
    local_cid_generator: Box<dyn ConnectionIdGenerator>,
    /// 3.
    index: ConnectionIndex,
}

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
        Self {
            connections: Slab::new(),
            local_cid_generator: (config.connection_id_generator_factory.as_ref())(),
            index: ConnectionIndex::default(),
        }
    }

    /// Initiate a connection
    pub fn connect(
        &mut self,
        now: Instant,
        config: ClientConfig,
        remote: SocketAddr,
        server_name: &str,
    ) -> Result<(ConnectionHandle, Connection), ConnectError> {
        let remote_id = (config.initial_dst_cid_provider)();
        let ch = ConnectionHandle(self.connections.vacant_key());
        let loc_cid = self.new_cid(ch);

        let params = TransportParameters::new(&config.transport);

        let tls = config
            .crypto
            .start_session(config.version, server_name, &params)?;
        todo!()
    }

    /// Generate a connection ID for `ch`
    fn new_cid(&mut self, ch: ConnectionHandle) -> ConnectionId {
        loop {
            let cid = self.local_cid_generator.generate_cid();
            if cid.len() == 0 {
                // Zero-length CID; nothing to track
                debug_assert_eq!(self.local_cid_generator.cid_len(), 0);
                return cid;
            }
            if let hash_map::Entry::Vacant(e) = self.index.connection_ids.entry(cid) {
                e.insert(ch);
                break cid;
            }
        }
    }
}

/// 2. Internal identifier for a `Connection` currently associated with an endpoint
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ConnectionHandle(pub usize);

/// 3. Errors in the parameters being used to create a new connection
///
/// These arise before any I/O has been performed.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConnectError {
    /// 1. The local endpoint does not support the QUIC version specified in the client configuration
    #[error("unsupported QUIC version")]
    UnsupportedVersion,
    /// 2. The given server name was malformed
    #[error("invalid server name: {0}")]
    InvalidServerName(String),
}

/// 4. connection meta data
#[derive(Debug)]
pub(crate) struct ConnectionMeta {}

/// 5. Maps packets to existing connections
#[derive(Default, Debug)]
struct ConnectionIndex {
    /// 1. Identifies connections based on locally created CIDs
    ///
    /// Uses a cheaper hash function since keys are locally created
    connection_ids: FxHashMap<ConnectionId, ConnectionHandle>,
}

/// 6. Parameters governing the core QUIC state machine
///
/// Default values should be suitable for most internet applications. Applications protocols which
/// forbid remotely-initiated streams should set `max_concurrent_bidi_streams` and
/// `max_concurrent_uni_streams` to zero.
///
/// In some cases, performance or resource requirements can be improved by tuning these values to
/// suit a particular application and/or network connection. In particular, data window sizes can be
/// tuned for a particular expected round trip time, link capacity, and memory availability. Tuning
/// for higher bandwidths and latencies increases worst-case memory consumption, but does not impair
/// performance at lower bandwidths and latencies. The default configuration is tuned for a 100Mbps
/// link with a 100ms round trip time.
pub struct TransportConfig {}

impl Default for TransportConfig {
    fn default() -> Self {
        const EXPECTED_RTT: u32 = 100; // ms
        const MAX_STREAM_BANDWIDTH: u32 = 12500 * 1000; // bytes/s
                                                        // Window size needed to avoid pipeline
                                                        // stalls
        const STREAM_RWND: u32 = MAX_STREAM_BANDWIDTH / 1000 * EXPECTED_RTT;

        Self {}
    }
}
