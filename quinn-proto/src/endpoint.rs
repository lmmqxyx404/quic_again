use std::{
    collections::{hash_map, HashMap},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::BytesMut;
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use rustc_hash::FxHashMap;
use slab::Slab;
use thiserror::Error;
use tracing::{debug, trace};

use crate::{
    coding::BufMutExt,
    config::{ClientConfig, EndpointConfig, ServerConfig, TransportConfig},
    connection::Connection,
    crypto,
    packet::{FixedLengthConnectionIdParser, Header, PacketDecodeError, PartialDecode},
    shared::{
        ConnectionEvent, ConnectionEventInner, ConnectionId, DatagramConnectionEvent, EcnCodepoint,
        EndpointEvent,
    },
    token::ResetToken,
    transport_parameters::TransportParameters,
    ConnectionIdGenerator, Side, Transmit, RESET_TOKEN_SIZE,
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
    /// 4.
    rng: StdRng,
    /// 5.
    config: Arc<EndpointConfig>,
    /// 6. Whether the underlying UDP socket promises not to fragment packets
    allow_mtud: bool,
    /// 7.
    server_config: Option<Arc<ServerConfig>>,
    /// 8. Buffered Initial and 0-RTT messages for pending incoming connections
    incoming_buffers: Slab<IncomingBuffer>,
    /// 9
    all_incoming_buffers_total_bytes: u64,
}

impl Endpoint {
    /// 1. Create a new endpoint
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
            rng: rng_seed.map_or(StdRng::from_entropy(), StdRng::from_seed),
            config,
            allow_mtud,
            server_config,
            incoming_buffers: Slab::new(),
            all_incoming_buffers_total_bytes: 0,
        }
    }

    /// 2. Initiate a connection
    pub fn connect(
        &mut self,
        now: Instant,
        config: ClientConfig,
        remote: SocketAddr,
        server_name: &str,
    ) -> Result<(ConnectionHandle, Connection), ConnectError> {
        let remote_id = (config.initial_dst_cid_provider)();
        trace!(initial_dcid = %remote_id);

        let ch = ConnectionHandle(self.connections.vacant_key());
        let loc_cid = self.new_cid(ch);

        let params = TransportParameters::new(&config.transport);

        let tls = config
            .crypto
            .start_session(config.version, server_name, &params)?;

        let conn = self.add_connection(
            ch,
            config.version,
            remote_id,
            loc_cid,
            remote_id,
            None,
            FourTuple {
                remote,
                local_ip: None,
            },
            now,
            tls,
            None,
            config.transport,
            true,
        );
        Ok((ch, conn))
    }

    /// 3. Generate a connection ID for `ch`
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
    /// 4.
    fn add_connection(
        &mut self,
        ch: ConnectionHandle,
        version: u32,
        init_cid: ConnectionId,
        loc_cid: ConnectionId,
        rem_cid: ConnectionId,
        pref_addr_cid: Option<ConnectionId>,
        addresses: FourTuple,
        now: Instant,
        tls: Box<dyn crypto::Session>,
        server_config: Option<Arc<ServerConfig>>,
        transport_config: Arc<TransportConfig>,
        path_validated: bool,
    ) -> Connection {
        let mut rng_seed = [0; 32];
        self.rng.fill_bytes(&mut rng_seed);
        let side = match server_config.is_some() {
            true => Side::Server,
            false => Side::Client,
        };
        let conn = Connection::new(
            self.config.clone(),
            server_config,
            transport_config,
            init_cid,
            loc_cid,
            rem_cid,
            pref_addr_cid,
            addresses.remote,
            addresses.local_ip,
            tls,
            self.local_cid_generator.as_ref(),
            now,
            version,
            self.allow_mtud,
            rng_seed,
            path_validated,
        );

        let mut cids_issued = 0;
        let mut loc_cids = FxHashMap::default();
        loc_cids.insert(cids_issued, loc_cid);
        cids_issued += 1;

        if let Some(cid) = pref_addr_cid {
            debug_assert_eq!(cids_issued, 1, "preferred address cid seq must be 1");
            loc_cids.insert(cids_issued, cid);
            cids_issued += 1;
        }

        let id = self.connections.insert(ConnectionMeta {
            init_cid,
            cids_issued,
            loc_cids,
            addresses,
            reset_token: None,
        });
        debug_assert_eq!(id, ch.0, "connection handle allocation out of sync");

        self.index.insert_conn(addresses, loc_cid, ch, side);

        conn
    }
    /// 5. Access the configuration used by this endpoint
    pub fn config(&self) -> &EndpointConfig {
        &self.config
    }

    /// 6. Process an incoming UDP datagram
    pub fn handle(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        local_ip: Option<IpAddr>,
        ecn: Option<EcnCodepoint>,
        data: BytesMut,
        buf: &mut Vec<u8>,
    ) -> Option<DatagramEvent> {
        let datagram_len = data.len();
        let (first_decode, remaining) = match PartialDecode::new(
            data,
            &FixedLengthConnectionIdParser::new(self.local_cid_generator.cid_len()),
            &self.config.supported_versions,
            self.config.grease_quic_bit,
        ) {
            Ok(x) => x,
            Err(PacketDecodeError::UnsupportedVersion {
                src_cid,
                dst_cid,
                version,
            }) => {
                if self.server_config.is_none() {
                    debug!("dropping packet with unsupported version");
                    return None;
                }
                trace!("sending version negotiation");
                // Negotiate versions
                Header::VersionNegotiate {
                    random: self.rng.gen::<u8>() | 0x40,
                    src_cid: dst_cid,
                    dst_cid: src_cid,
                }
                .encode(buf);
                // Grease with a reserved version
                if version != 0x0a1a_2a3a {
                    buf.write::<u32>(0x0a1a_2a3a);
                } else {
                    buf.write::<u32>(0x0a1a_2a4a);
                }
                for &version in &self.config.supported_versions {
                    buf.write(version);
                }
                return Some(DatagramEvent::Response(Transmit {
                    destination: remote,
                    ecn: None,
                    size: buf.len(),
                    segment_size: None,
                    src_ip: local_ip,
                }));
            }
            Err(e) => {
                trace!("malformed header: {}", e);
                return None;
            }
        };

        let addresses = FourTuple { remote, local_ip };
        if let Some(route_to) = self.index.get(&addresses, &first_decode) {
            let event = DatagramConnectionEvent {
                now,
                remote: addresses.remote,
                ecn,
                first_decode,
                remaining,
            };

            match route_to {
                RouteDatagramTo::Incoming(incoming_idx) => {
                    let incoming_buffer = &mut self.incoming_buffers[incoming_idx];
                    let config = &self.server_config.as_ref().unwrap();

                    if incoming_buffer
                        .total_bytes
                        .checked_add(datagram_len as u64)
                        .map_or(false, |n| n <= config.incoming_buffer_size)
                        && self
                            .all_incoming_buffers_total_bytes
                            .checked_add(datagram_len as u64)
                            .map_or(false, |n| n <= config.incoming_buffer_size_total)
                    {
                        incoming_buffer.datagrams.push(event);
                        incoming_buffer.total_bytes += datagram_len as u64;
                        self.all_incoming_buffers_total_bytes += datagram_len as u64;
                    }

                    return None;
                }
                RouteDatagramTo::Connection(ch) => {
                    return Some(DatagramEvent::ConnectionEvent(
                        ch,
                        ConnectionEvent(ConnectionEventInner::Datagram(event)),
                    ))
                }
            }
        }

        //
        // Potentially create a new connection
        //

        let dst_cid = first_decode.dst_cid();
        let server_config = match &self.server_config {
            Some(config) => config,
            None => {
                debug!("packet for unrecognized connection {}", dst_cid);
                return self
                    .stateless_reset(now, datagram_len, addresses, dst_cid, buf)
                    .map(DatagramEvent::Response);
            }
        };

        todo!()
    }

    /// 7. Process `EndpointEvent`s emitted from related `Connection`s
    ///
    /// In turn, processing this event may return a `ConnectionEvent` for the same `Connection`.
    pub fn handle_event(
        &mut self,
        ch: ConnectionHandle,
        event: EndpointEvent,
    ) -> Option<ConnectionEvent> {
        todo!()
    }
    /// 8.
    fn stateless_reset(
        &mut self,
        now: Instant,
        inciting_dgram_len: usize,
        addresses: FourTuple,
        dst_cid: &ConnectionId,
        buf: &mut Vec<u8>,
    ) -> Option<Transmit> {
        todo!()
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
pub(crate) struct ConnectionMeta {
    init_cid: ConnectionId,
    /// Number of local connection IDs that have been issued in NEW_CONNECTION_ID frames.
    cids_issued: u64,
    loc_cids: FxHashMap<u64, ConnectionId>,
    /// Remote/local addresses the connection began with
    ///
    /// Only needed to support connections with zero-length CIDs, which cannot migrate, so we don't
    /// bother keeping it up to date.
    addresses: FourTuple,
    /// Reset token provided by the peer for the CID we're currently sending to, and the address
    /// being sent to
    reset_token: Option<(SocketAddr, ResetToken)>,
}

/// 5. Maps packets to existing connections
#[derive(Default, Debug)]
struct ConnectionIndex {
    /// 1. Identifies connections based on locally created CIDs
    ///
    /// Uses a cheaper hash function since keys are locally created
    connection_ids: FxHashMap<ConnectionId, ConnectionHandle>,
    /// 2. Identifies incoming connections with zero-length CIDs
    ///
    /// Uses a standard `HashMap` to protect against hash collision attacks.
    incoming_connection_remotes: HashMap<FourTuple, ConnectionHandle>,
    /// 3. Identifies outgoing connections with zero-length CIDs
    ///
    /// We don't yet support explicit source addresses for client connections, and zero-length CIDs
    /// require a unique four-tuple, so at most one client connection with zero-length local CIDs
    /// may be established per remote. We must omit the local address from the key because we don't
    /// necessarily know what address we're sending from, and hence receiving at.
    ///
    /// Uses a standard `HashMap` to protect against hash collision attacks.
    outgoing_connection_remotes: HashMap<SocketAddr, ConnectionHandle>,
    /// 4. Identifies connections based on the initial DCID the peer utilized
    ///
    /// Uses a standard `HashMap` to protect against hash collision attacks.
    connection_ids_initial: HashMap<ConnectionId, RouteDatagramTo>,
    /// 5. Reset tokens provided by the peer for the CID each connection is currently sending to
    ///
    /// Incoming stateless resets do not have correct CIDs, so we need this to identify the correct
    /// recipient, if any.
    connection_reset_tokens: ResetTokenTable,
}

impl ConnectionIndex {
    /// 1. Associate a connection with its first locally-chosen destination CID if used, or otherwise
    /// its current 4-tuple
    fn insert_conn(
        &mut self,
        addresses: FourTuple,
        dst_cid: ConnectionId,
        connection: ConnectionHandle,
        side: Side,
    ) {
        match dst_cid.len() {
            0 => match side {
                Side::Server => {
                    self.incoming_connection_remotes
                        .insert(addresses, connection);
                }
                Side::Client => {
                    self.outgoing_connection_remotes
                        .insert(addresses.remote, connection);
                }
            },
            _ => {
                self.connection_ids.insert(dst_cid, connection);
            }
        }
    }
    /// 2. Find the existing connection that `datagram` should be routed to, if any
    fn get(&self, addresses: &FourTuple, datagram: &PartialDecode) -> Option<RouteDatagramTo> {
        if datagram.dst_cid().len() != 0 {
            if let Some(&ch) = self.connection_ids.get(datagram.dst_cid()) {
                return Some(RouteDatagramTo::Connection(ch));
            }
        }
        if datagram.is_initial() || datagram.is_0rtt() {
            if let Some(&ch) = self.connection_ids_initial.get(datagram.dst_cid()) {
                return Some(ch);
            }
        }
        if datagram.dst_cid().len() == 0 {
            if let Some(&ch) = self.incoming_connection_remotes.get(addresses) {
                return Some(RouteDatagramTo::Connection(ch));
            }
            if let Some(&ch) = self.outgoing_connection_remotes.get(&addresses.remote) {
                return Some(RouteDatagramTo::Connection(ch));
            }
        }
        let data = datagram.data();
        if data.len() < RESET_TOKEN_SIZE {
            return None;
        }
        self.connection_reset_tokens
            .get(addresses.remote, &data[data.len() - RESET_TOKEN_SIZE..])
            .cloned()
            .map(RouteDatagramTo::Connection)
    }
}

/// 7. Identifies a connection by the combination of remote and local addresses
///
/// Including the local ensures good behavior when the host has multiple IP addresses on the same
/// subnet and zero-length connection IDs are in use.
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
struct FourTuple {
    remote: SocketAddr,
    // A single socket can only listen on a single port, so no need to store it explicitly
    local_ip: Option<IpAddr>,
}

/// Event resulting from processing a single datagram
#[allow(clippy::large_enum_variant)] // Not passed around extensively
pub enum DatagramEvent {
    /// 2. The datagram is redirected to its `Connection`
    ConnectionEvent(ConnectionHandle, ConnectionEvent),
    /// 3. The datagram may result in starting a new `Connection`
    NewConnection(Incoming),
    /// 1. Response generated directly by the endpoint
    Response(Transmit),
}

/// Part of protocol state incoming datagrams can be routed to
#[derive(Copy, Clone, Debug)]
enum RouteDatagramTo {
    Incoming(usize),
    Connection(ConnectionHandle),
}

/// Buffered Initial and 0-RTT messages for a pending incoming connection
#[derive(Default)]
struct IncomingBuffer {
    datagrams: Vec<DatagramConnectionEvent>,
    total_bytes: u64,
}

/// Reset Tokens which are associated with peer socket addresses
///
/// The standard `HashMap` is used since both `SocketAddr` and `ResetToken` are
/// peer generated and might be usable for hash collision attacks.
#[derive(Default, Debug)]
struct ResetTokenTable(HashMap<SocketAddr, HashMap<ResetToken, ConnectionHandle>>);

impl ResetTokenTable {
    fn get(&self, remote: SocketAddr, token: &[u8]) -> Option<&ConnectionHandle> {
        let token = ResetToken::from(<[u8; RESET_TOKEN_SIZE]>::try_from(token).ok()?);
        self.0.get(&remote)?.get(&token)
    }
}

/// An incoming connection for which the server has not yet begun its part of the handshake.
pub struct Incoming {}
