use std::{
    net::{SocketAddrV4, SocketAddrV6},
    sync::Arc,
    time::Duration,
};

use rand::RngCore;

use crate::{
    cid_generator::HashedConnectionIdGenerator,
    congestion,
    crypto::{self, HandshakeTokenKey, HmacKey},
    shared::ConnectionId,
    ConnectionIdGenerator, RandomConnectionIdGenerator, VarInt, DEFAULT_SUPPORTED_VERSIONS,
    INITIAL_MTU, MAX_CID_SIZE,
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
    /// 4
    pub(crate) supported_versions: Vec<u32>,
    /// 5.
    pub(crate) grease_quic_bit: bool,
    /// 6.
    pub(crate) reset_key: Arc<dyn HmacKey>,
    /// 7. Minimum interval between outgoing stateless reset packets
    pub(crate) min_reset_interval: Duration,
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
            supported_versions: DEFAULT_SUPPORTED_VERSIONS.to_vec(),
            grease_quic_bit: true,
            reset_key,
            min_reset_interval: Duration::from_millis(20),
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
    /// Supply a custom connection ID generator factory
    ///
    /// Called once by each `Endpoint` constructed from this configuration to obtain the CID
    /// generator which will be used to generate the CIDs used for incoming packets on all
    /// connections involving that  `Endpoint`. A custom CID generator allows applications to embed
    /// information in local connection IDs, e.g. to support stateless packet-level load balancers.
    ///
    /// Defaults to [`HashedConnectionIdGenerator`].
    pub fn cid_generator<F: Fn() -> Box<dyn ConnectionIdGenerator> + Send + Sync + 'static>(
        &mut self,
        factory: F,
    ) -> &mut Self {
        self.connection_id_generator_factory = Arc::new(factory);
        self
    }
}
/// Parameters governing incoming connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
pub struct ServerConfig {
    /// 1
    pub(crate) incoming_buffer_size: u64,
    /// 2
    pub(crate) incoming_buffer_size_total: u64,
    /// 3. Whether to allow clients to migrate to new addresses
    ///
    /// Improves behavior for clients that move between different internet connections or suffer NAT
    /// rebinding. Enabled by default.
    pub(crate) migration: bool,
    /// 4. TLS configuration used for incoming connections.
    ///
    /// Must be set to use TLS 1.3 only.
    pub crypto: Arc<dyn crypto::ServerConfig>,
    /// 5.
    pub(crate) max_incoming: usize,
    /// 6. Transport configuration to use for incoming connections
    pub transport: Arc<TransportConfig>,
    /// 7
    pub(crate) preferred_address_v4: Option<SocketAddrV4>,
    /// 8
    pub(crate) preferred_address_v6: Option<SocketAddrV6>,
    /// 9. Used to generate one-time AEAD keys to protect handshake tokens
    pub(crate) token_key: Arc<dyn HandshakeTokenKey>,
    /// 10. Microseconds after a stateless retry token was issued for which it's considered valid.
    pub(crate) retry_token_lifetime: Duration,
}

impl ServerConfig {
    /// 1. Create a default config with a particular handshake token key
    pub fn new(
        crypto: Arc<dyn crypto::ServerConfig>,
        token_key: Arc<dyn HandshakeTokenKey>,
    ) -> Self {
        Self {
            migration: true,

            incoming_buffer_size: 10 << 20,
            incoming_buffer_size_total: 100 << 20,

            crypto,
            max_incoming: 1 << 16,
            transport: Arc::new(TransportConfig::default()),

            preferred_address_v4: None,
            preferred_address_v6: None,

            token_key,
            retry_token_lifetime: Duration::from_secs(15),
        }
    }
    /// 2. Maximum number of received bytes to buffer for each [`Incoming`][crate::Incoming]
    ///
    /// An [`Incoming`][crate::Incoming] comes into existence when an incoming connection attempt
    /// is received and stops existing when the application either accepts it or otherwise disposes
    /// of it. This limit governs only packets received within that period, and does not include
    /// the first packet. Packets received in excess of this limit are dropped, which may cause
    /// 0-RTT or handshake data to have to be retransmitted.
    ///
    /// The default value is set to 10 MiB--an amount such that in most situations a client would
    /// not transmit that much 0-RTT data faster than the server handles the corresponding
    /// [`Incoming`][crate::Incoming].
    pub fn incoming_buffer_size(&mut self, incoming_buffer_size: u64) -> &mut Self {
        self.incoming_buffer_size = incoming_buffer_size;
        self
    }
    /// 3. Maximum number of received bytes to buffer for all [`Incoming`][crate::Incoming]
    /// collectively
    ///
    /// An [`Incoming`][crate::Incoming] comes into existence when an incoming connection attempt
    /// is received and stops existing when the application either accepts it or otherwise disposes
    /// of it. This limit governs only packets received within that period, and does not include
    /// the first packet. Packets received in excess of this limit are dropped, which may cause
    /// 0-RTT or handshake data to have to be retransmitted.
    ///
    /// The default value is set to 100 MiB--a generous amount that still prevents memory
    /// exhaustion in most contexts.
    pub fn incoming_buffer_size_total(&mut self, incoming_buffer_size_total: u64) -> &mut Self {
        self.incoming_buffer_size_total = incoming_buffer_size_total;
        self
    }
}

#[cfg(feature = "ring")]
impl ServerConfig {
    /// Create a server config with the given [`crypto::ServerConfig`]
    ///
    /// Uses a randomized handshake token key.
    pub fn with_crypto(crypto: Arc<dyn crypto::ServerConfig>) -> Self {
        let rng = &mut rand::thread_rng();
        let mut master_key = [0u8; 64];
        rng.fill_bytes(&mut master_key);
        let master_key = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, &[]).extract(&master_key);

        Self::new(crypto, Arc::new(master_key))
    }
}

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

    /// Set the QUIC version to use
    pub fn version(&mut self, version: u32) -> &mut Self {
        self.version = version;
        self
    }
}

/// Parameters governing MTU discovery.
///
/// # The why of MTU discovery
///
/// By design, QUIC ensures during the handshake that the network path between the client and the
/// server is able to transmit unfragmented UDP packets with a body of 1200 bytes. In other words,
/// once the connection is established, we know that the network path's maximum transmission unit
/// (MTU) is of at least 1200 bytes (plus IP and UDP headers). Because of this, a QUIC endpoint can
/// split outgoing data in packets of 1200 bytes, with confidence that the network will be able to
/// deliver them (if the endpoint were to send bigger packets, they could prove too big and end up
/// being dropped).
///
/// There is, however, a significant overhead associated to sending a packet. If the same
/// information can be sent in fewer packets, that results in higher throughput. The amount of
/// packets that need to be sent is inversely proportional to the MTU: the higher the MTU, the
/// bigger the packets that can be sent, and the fewer packets that are needed to transmit a given
/// amount of bytes.
///
/// Most networks have an MTU higher than 1200. Through MTU discovery, endpoints can detect the
/// path's MTU and, if it turns out to be higher, start sending bigger packets.
///
/// # MTU discovery internals
///
/// Quinn implements MTU discovery through DPLPMTUD (Datagram Packetization Layer Path MTU
/// Discovery), described in [section 14.3 of RFC
/// 9000](https://www.rfc-editor.org/rfc/rfc9000.html#section-14.3). This method consists of sending
/// QUIC packets padded to a particular size (called PMTU probes), and waiting to see if the remote
/// peer responds with an ACK. If an ACK is received, that means the probe arrived at the remote
/// peer, which in turn means that the network path's MTU is of at least the packet's size. If the
/// probe is lost, it is sent another 2 times before concluding that the MTU is lower than the
/// packet's size.
///
/// MTU discovery runs on a schedule (e.g. every 600 seconds) specified through
/// [`MtuDiscoveryConfig::interval`]. The first run happens right after the handshake, and
/// subsequent discoveries are scheduled to run when the interval has elapsed, starting from the
/// last time when MTU discovery completed.
///
/// Since the search space for MTUs is quite big (the smallest possible MTU is 1200, and the highest
/// is 65527), Quinn performs a binary search to keep the number of probes as low as possible. The
/// lower bound of the search is equal to [`TransportConfig::initial_mtu`] in the
/// initial MTU discovery run, and is equal to the currently discovered MTU in subsequent runs. The
/// upper bound is determined by the minimum of [`MtuDiscoveryConfig::upper_bound`] and the
/// `max_udp_payload_size` transport parameter received from the peer during the handshake.
///
/// # Black hole detection
///
/// If, at some point, the network path no longer accepts packets of the detected size, packet loss
/// will eventually trigger black hole detection and reset the detected MTU to 1200. In that case,
/// MTU discovery will be triggered after [`MtuDiscoveryConfig::black_hole_cooldown`] (ignoring the
/// timer that was set based on [`MtuDiscoveryConfig::interval`]).
///
/// # Interaction between peers
///
/// There is no guarantee that the MTU on the path between A and B is the same as the MTU of the
/// path between B and A. Therefore, each peer in the connection needs to run MTU discovery
/// independently in order to discover the path's MTU.
#[derive(Clone, Debug)]
pub struct MtuDiscoveryConfig {
    /// 1
    pub(crate) upper_bound: u16,
    /// 2.
    pub(crate) interval: Duration,
    /// 3.
    pub(crate) minimum_change: u16,
    /// 4.
    pub(crate) black_hole_cooldown: Duration,
}

impl Default for MtuDiscoveryConfig {
    fn default() -> Self {
        Self {
            upper_bound: 1452,
            interval: Duration::from_secs(600),
            minimum_change: 20,
            black_hole_cooldown: Duration::from_secs(60),
        }
    }
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
pub struct TransportConfig {
    /// 1
    pub(crate) initial_rtt: Duration,
    /// 2
    pub(crate) max_concurrent_bidi_streams: VarInt,
    /// 3
    pub(crate) max_concurrent_uni_streams: VarInt,
    /// 4
    pub(crate) stream_receive_window: VarInt,
    /// 5
    pub(crate) receive_window: VarInt,
    /// 6
    pub(crate) send_window: u64,
    /// 7.
    pub(crate) enable_segmentation_offload: bool,
    /// 8.
    pub(crate) mtu_discovery_config: Option<MtuDiscoveryConfig>,
    /// 9.
    pub(crate) min_mtu: u16,
    /// 10.
    pub(crate) initial_mtu: u16,
    /// 11.
    pub(crate) ack_frequency_config: Option<AckFrequencyConfig>,
    /// 12.
    #[cfg(test)]
    pub(crate) deterministic_packet_numbers: bool,
    /// 13.
    pub(crate) congestion_controller_factory: Arc<dyn congestion::ControllerFactory + Send + Sync>,
    /// 14.
    pub(crate) allow_spin: bool,
    /// 15.
    pub(crate) keep_alive_interval: Option<Duration>,
    /// 16.
    pub(crate) max_idle_timeout: Option<VarInt>,
    /// 17.
    pub(crate) crypto_buffer_size: usize,
    /// 18.
    pub(crate) time_threshold: f32,
    /// 19.
    pub(crate) packet_threshold: u32,
    /// 20.
    pub(crate) persistent_congestion_threshold: u32,
    /// 21.
    pub(crate) datagram_receive_buffer_size: Option<usize>,
    /// 22.
    pub(crate) datagram_send_buffer_size: usize,
}

impl TransportConfig {
    pub(crate) fn get_initial_mtu(&self) -> u16 {
        self.initial_mtu.max(self.min_mtu)
    }

    /// Maximum duration of inactivity to accept before timing out the connection.
    ///
    /// The true idle timeout is the minimum of this and the peer's own max idle timeout. `None`
    /// represents an infinite timeout.
    ///
    /// **WARNING**: If a peer or its network path malfunctions or acts maliciously, an infinite
    /// idle timeout can result in permanently hung futures!
    ///
    /// ```
    /// # use std::{convert::TryInto, time::Duration};
    /// # use quinn_proto::{TransportConfig, VarInt, VarIntBoundsExceeded};
    /// # fn main() -> Result<(), VarIntBoundsExceeded> {
    /// let mut config = TransportConfig::default();
    ///
    /// // Set the idle timeout as `VarInt`-encoded milliseconds
    /// config.max_idle_timeout(Some(VarInt::from_u32(10_000).into()));
    ///
    /// // Set the idle timeout as a `Duration`
    /// config.max_idle_timeout(Some(Duration::from_secs(10).try_into()?));
    /// # Ok(())
    /// # }
    /// ```
    pub fn max_idle_timeout(&mut self, value: Option<IdleTimeout>) -> &mut Self {
        self.max_idle_timeout = value.map(|t| t.0);
        self
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        const EXPECTED_RTT: u32 = 100; // ms
        const MAX_STREAM_BANDWIDTH: u32 = 12500 * 1000; // bytes/s
                                                        // Window size needed to avoid pipeline
                                                        // stalls
        const STREAM_RWND: u32 = MAX_STREAM_BANDWIDTH / 1000 * EXPECTED_RTT;

        Self {
            initial_rtt: Duration::from_millis(333), // per spec, intentionally distinct from EXPECTED_RTT
            max_concurrent_bidi_streams: 100u32.into(),
            max_concurrent_uni_streams: 100u32.into(),
            stream_receive_window: STREAM_RWND.into(),
            receive_window: VarInt::MAX,
            send_window: (8 * STREAM_RWND).into(),

            enable_segmentation_offload: true,

            min_mtu: INITIAL_MTU,
            mtu_discovery_config: Some(MtuDiscoveryConfig::default()),

            initial_mtu: INITIAL_MTU,
            ack_frequency_config: None,
            #[cfg(test)]
            deterministic_packet_numbers: false,

            congestion_controller_factory: Arc::new(congestion::CubicConfig::default()),
            allow_spin: true,

            keep_alive_interval: None,
            max_idle_timeout: Some(VarInt(10_000)),
            crypto_buffer_size: 16 * 1024,
            time_threshold: 9.0 / 8.0,
            packet_threshold: 3,
            persistent_congestion_threshold: 3,
            datagram_receive_buffer_size: Some(STREAM_RWND as usize),
            datagram_send_buffer_size: 1024 * 1024,
        }
    }
}

/// Parameters for controlling the peer's acknowledgement frequency
///
/// The parameters provided in this config will be sent to the peer at the beginning of the
/// connection, so it can take them into account when sending acknowledgements (see each parameter's
/// description for details on how it influences acknowledgement frequency).
///
/// Quinn's implementation follows the fourth draft of the
/// [QUIC Acknowledgement Frequency extension](https://datatracker.ietf.org/doc/html/draft-ietf-quic-ack-frequency-04).
/// The defaults produce behavior slightly different than the behavior without this extension,
/// because they change the way reordered packets are handled (see
/// [`AckFrequencyConfig::reordering_threshold`] for details).
#[derive(Clone, Debug)]
pub struct AckFrequencyConfig {
    /// 1
    pub(crate) ack_eliciting_threshold: VarInt,
    /// 2
    pub(crate) reordering_threshold: VarInt,
}

/// Maximum duration of inactivity to accept before timing out the connection.
///
/// This wraps an underlying [`VarInt`], representing the duration in milliseconds. Values can be
/// constructed by converting directly from `VarInt`, or using `TryFrom<Duration>`.
///
/// ```
/// # use std::{convert::TryFrom, time::Duration};
/// # use quinn_proto::{IdleTimeout, VarIntBoundsExceeded, VarInt};
/// # fn main() -> Result<(), VarIntBoundsExceeded> {
/// // A `VarInt`-encoded value in milliseconds
/// let timeout = IdleTimeout::from(VarInt::from_u32(10_000));
///
/// // Try to convert a `Duration` into a `VarInt`-encoded timeout
/// let timeout = IdleTimeout::try_from(Duration::from_secs(10))?;
/// # Ok(())
/// # }
/// ```
#[derive(Default, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdleTimeout(VarInt);
