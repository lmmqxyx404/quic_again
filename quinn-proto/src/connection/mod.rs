use std::{
    collections::VecDeque,
    fmt,
    io::Read,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::{EndpointConfig, ServerConfig},
    crypto::{self, KeyPair, Keys, PacketKey},
    endpoint::TransportConfig,
    frame::{self, Close, Frame},
    packet::{Header, InitialHeader, LongType, Packet, PartialDecode, SpaceId},
    shared::{
        ConnectionEvent, ConnectionEventInner, ConnectionId, DatagramConnectionEvent, EcnCodepoint,
        EndpointEventInner,
    },
    transport_parameters::TransportParameters,
    ConnectionIdGenerator, Side, TransportError,
};
use bytes::{Bytes, BytesMut};

/// 1.
mod paths;
use packet_crypto::{PrevCrypto, ZeroRttCrypto};
use paths::PathData;
/// 2.
mod stats;
pub use stats::ConnectionStats;
/// 3.
mod cid_state;
use cid_state::CidState;
/// 4.
mod spaces;
use spaces::PacketSpace;
/// 5.
mod timer;
use thiserror::Error;
use timer::{Timer, TimerTable};
use tracing::{debug, trace, trace_span, warn};
/// 7.
mod ack_frequency;
/// 6.
mod packet_crypto;
use ack_frequency::AckFrequencyState;
/// 8.
mod streams;
use streams::StreamEvent;
#[cfg(fuzzing)]
pub use streams::StreamsState;
#[cfg(not(fuzzing))]
use streams::StreamsState;

// #[cfg(fuzzing)]
// pub use spaces::Retransmits;

/// Protocol state and logic for a single QUIC connection
///
/// Objects of this type receive [`ConnectionEvent`]s and emit [`EndpointEvent`]s and application
/// [`Event`]s to make progress. To handle timeouts, a `Connection` returns timer updates and
/// expects timeouts through various methods. A number of simple getter methods are exposed
/// to allow callers to inspect some of the connection state.
///
/// `Connection` has roughly 4 types of methods:
///
/// - A. Simple getters, taking `&self`
/// - B. Handlers for incoming events from the network or system, named `handle_*`.
/// - C. State machine mutators, for incoming commands from the application. For convenience we
///   refer to this as "performing I/O" below, however as per the design of this library none of the
///   functions actually perform system-level I/O. For example, [`read`](RecvStream::read) and
///   [`write`](SendStream::write), but also things like [`reset`](SendStream::reset).
/// - D. Polling functions for outgoing events or actions for the caller to
///   take, named `poll_*`.
///
/// The simplest way to use this API correctly is to call (B) and (C) whenever
/// appropriate, then after each of those calls, as soon as feasible call all
/// polling methods (D) and deal with their outputs appropriately, e.g. by
/// passing it to the application or by making a system-level I/O call. You
/// should call the polling functions in this order:
///
/// 1. [`poll_transmit`](Self::poll_transmit)
/// 2. [`poll_timeout`](Self::poll_timeout)
/// 3. [`poll_endpoint_events`](Self::poll_endpoint_events)
/// 4. [`poll`](Self::poll)
///
/// Currently the only actual dependency is from (2) to (1), however additional
/// dependencies may be added in future, so the above order is recommended.
///
/// (A) may be called whenever desired.
///
/// Care should be made to ensure that the input events represent monotonically
/// increasing time. Specifically, calling [`handle_timeout`](Self::handle_timeout)
/// with events of the same [`Instant`] may be interleaved in any order with a
/// call to [`handle_event`](Self::handle_event) at that same instant; however
/// events or timeouts with different instants must not be interleaved.
/// Todo: many tasks to be done.
pub struct Connection {
    /// 1.
    path: PathData,
    /// 2.
    server_config: Option<Arc<ServerConfig>>,
    /// 3. Connection level statistics
    stats: ConnectionStats,
    /// 4. Attributes of CIDs generated by local peer
    local_cid_state: CidState,
    /// 5. Packet number spaces: initial, handshake, 1-RTT
    spaces: [PacketSpace; 3],
    /// 6.
    timers: TimerTable,
    /// 7. Set if 0-RTT is supported, then cleared when no longer needed.
    zero_rtt_crypto: Option<ZeroRttCrypto>,
    /// 8. Transport parameters set by the peer
    peer_params: TransportParameters,
    /// 9.
    state: State,
    /// 10.
    key_phase: bool,
    /// 11. 1-RTT keys used prior to a key update
    prev_crypto: Option<PrevCrypto>,
    /// 12. 1-RTT keys to be used for the next key update
    ///
    /// These are generated in advance to prevent timing attacks and/or DoS by third-party attackers
    /// spoofing key updates.
    next_crypto: Option<KeyPair<Box<dyn PacketKey>>>,
    /// 13.
    side: Side,
    /// 14. Number of packets authenticated
    total_authed_packets: u64,
    /// 15. QUIC version used for the connection.
    version: u32,
    /// 16. Why the connection was lost, if it has been
    error: Option<ConnectionError>,
    /// 17.
    endpoint_events: VecDeque<EndpointEventInner>,
    /// 18.
    close: bool,
    /// 19. Highest usable packet number space
    highest_space: SpaceId,
    /// 20. ACK frequency
    ack_frequency: AckFrequencyState,
    /// 21.
    crypto: Box<dyn crypto::Session>,
    /// 22
    events: VecDeque<Event>,
    /// 23.
    streams: StreamsState,
}

impl Connection {
    /// 1.
    pub(crate) fn new(
        endpoint_config: Arc<EndpointConfig>,
        server_config: Option<Arc<ServerConfig>>,
        config: Arc<TransportConfig>,
        init_cid: ConnectionId,
        loc_cid: ConnectionId,
        rem_cid: ConnectionId,
        pref_addr_cid: Option<ConnectionId>,
        remote: SocketAddr,
        local_ip: Option<IpAddr>,
        crypto: Box<dyn crypto::Session>,
        cid_gen: &dyn ConnectionIdGenerator,
        now: Instant,
        version: u32,
        allow_mtud: bool,
        rng_seed: [u8; 32],
        path_validated: bool,
    ) -> Self {
        let side = if server_config.is_some() {
            Side::Server
        } else {
            Side::Client
        };
        let initial_space = PacketSpace {
            crypto: Some(crypto.initial_keys(&init_cid, side)),
            ..PacketSpace::new(now)
        };
        let state = State::Handshake(state::Handshake {
            rem_cid_set: side.is_server(),
            expected_token: Bytes::new(),
            client_hello: None,
        });

        let mut this = Self {
            path: PathData::new(remote, allow_mtud, None, now, path_validated, &config),
            server_config,
            stats: ConnectionStats::default(),
            local_cid_state: CidState::new(
                cid_gen.cid_len(),
                cid_gen.cid_lifetime(),
                now,
                if pref_addr_cid.is_some() { 2 } else { 1 },
            ),
            spaces: [initial_space, PacketSpace::new(now), PacketSpace::new(now)],
            timers: TimerTable::default(),
            zero_rtt_crypto: None,
            peer_params: TransportParameters::default(),
            state,
            key_phase: false,
            prev_crypto: None,
            next_crypto: None,
            side,
            total_authed_packets: 0,
            version,
            error: None,
            endpoint_events: VecDeque::new(),
            close: false,
            highest_space: SpaceId::Initial,
            ack_frequency: AckFrequencyState::new(get_max_ack_delay(
                &TransportParameters::default(),
            )),
            crypto,
            events: VecDeque::new(),
            streams: StreamsState::new(
                side,
                config.max_concurrent_uni_streams,
                config.max_concurrent_bidi_streams,
                config.send_window,
                config.receive_window,
                config.stream_receive_window,
            ),
        };

        if side.is_client() {
            // Kick off the connection
            this.write_crypto();
            this.init_0rtt();
        }
        this
    }

    /// 2. Process `ConnectionEvent`s generated by the associated `Endpoint`
    ///
    /// Will execute protocol logic upon receipt of a connection event, in turn preparing signals
    /// (including application `Event`s, `EndpointEvent`s and outgoing datagrams) that should be
    /// extracted through the relevant methods.
    pub fn handle_event(&mut self, event: ConnectionEvent) {
        use self::ConnectionEventInner::*;

        match event.0 {
            Datagram(DatagramConnectionEvent {
                now,
                remote,
                ecn,
                first_decode,
                remaining,
            }) => {
                // If this packet could initiate a migration and we're a client or a server that
                // forbids migration, drop the datagram. This could be relaxed to heuristically
                // permit NAT-rebinding-like migration.
                if remote != self.path.remote
                    && self.server_config.as_ref().map_or(true, |x| !x.migration)
                {
                    // trace!("discarding packet from unrecognized peer {}", remote);
                    return;
                }

                let was_anti_amplification_blocked = self.path.anti_amplification_blocked(1);

                self.stats.udp_rx.datagrams += 1;
                self.stats.udp_rx.bytes += first_decode.len() as u64;
                let data_len = first_decode.len();

                self.handle_decode(now, remote, ecn, first_decode);
                // The current `path` might have changed inside `handle_decode`,
                // since the packet could have triggered a migration. Make sure
                // the data received is accounted for the most recent path by accessing
                // `path` after `handle_decode`.
                self.path.total_recvd = self.path.total_recvd.saturating_add(data_len as u64);

                if let Some(data) = remaining {
                    self.stats.udp_rx.bytes += data.len() as u64;
                    self.handle_coalesced(now, remote, ecn, data);
                }

                if was_anti_amplification_blocked {
                    // A prior attempt to set the loss detection timer may have failed due to
                    // anti-amplification, so ensure it's set now. Prevents a handshake deadlock if
                    // the server's first flight is lost.
                    self.set_loss_detection_timer(now);
                }
            }
            NewIdentifiers(ids, now) => {
                self.local_cid_state.new_cids(&ids, now);
                ids.into_iter().rev().for_each(|frame| {
                    // todo: 为什么这里需要显示声明为 usize
                    self.spaces[SpaceId::Data as usize]
                        .pending
                        .new_cids
                        .push(frame);
                });
                // Update Timer::PushNewCid
                if self
                    .timers
                    .get(Timer::PushNewCid)
                    .map_or(true, |x| x <= now)
                {
                    self.reset_cid_retirement();
                }
            }
        }
    }
    /// 3.
    fn handle_decode(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        ecn: Option<EcnCodepoint>,
        partial_decode: PartialDecode,
    ) {
        if let Some(decoded) = packet_crypto::unprotect_header(
            partial_decode,
            &self.spaces,
            self.zero_rtt_crypto.as_ref(),
            self.peer_params.stateless_reset_token,
        ) {
            self.handle_packet(now, remote, ecn, decoded.packet, decoded.stateless_reset);
        }
    }
    /// 4.
    fn handle_coalesced(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        ecn: Option<EcnCodepoint>,
        data: BytesMut,
    ) {
        todo!()
    }
    /// 5.
    fn set_loss_detection_timer(&mut self, now: Instant) {
        todo!()
    }
    /// 6.
    fn reset_cid_retirement(&mut self) {
        todo!()
    }
    /// 7.
    fn handle_packet(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        ecn: Option<EcnCodepoint>,
        packet: Option<Packet>,
        stateless_reset: bool,
    ) {
        self.stats.udp_rx.ios += 1;
        if let Some(ref packet) = packet {
            trace!(
                "got {:?} packet ({} bytes) from {} using id {}",
                packet.header.space(),
                packet.payload.len() + packet.header_data.len(),
                remote,
                packet.header.dst_cid(),
            );
        }

        if self.is_handshaking() && remote != self.path.remote {
            // debug!("discarding packet with unexpected remote during handshake");
            return;
        }

        let was_closed = self.state.is_closed();
        let was_drained = self.state.is_drained();

        let decrypted = match packet {
            None => Err(None),
            Some(mut packet) => self
                .decrypt_packet(now, &mut packet)
                .map(move |number| (packet, number)),
        };

        let result: Result<(), ConnectionError> = match decrypted {
            _ if stateless_reset => {
                debug!("got stateless reset");
                Err(ConnectionError::Reset)
            }
            Err(Some(e)) => {
                warn!("illegal packet: {}", e);
                Err(e.into())
            }
            Err(None) => {
                todo!()
            }
            Ok((packet, number)) => {
                let span = match number {
                    Some(pn) => trace_span!("recv", space = ?packet.header.space(), pn),
                    None => trace_span!("recv", space = ?packet.header.space()),
                };
                let _guard = span.enter();

                let is_duplicate = |n| self.spaces[packet.header.space() as usize].dedup.insert(n);

                if number.map_or(false, is_duplicate) {
                    debug!("discarding possible duplicate packet");
                    return;
                } else if self.state.is_handshake() && packet.header.is_short() {
                    // TODO: SHOULD buffer these to improve reordering tolerance.
                    trace!("dropping short packet during handshake");
                    return;
                } else {
                    if let Header::Initial(InitialHeader { ref token, .. }) = packet.header {
                        if let State::Handshake(ref hs) = self.state {
                            if self.side.is_server() && token != &hs.expected_token {
                                // Clients must send the same retry token in every Initial. Initial
                                // packets can be spoofed, so we discard rather than killing the
                                // connection.
                                warn!("discarding Initial with invalid retry token");
                                return;
                            }
                        }
                    }
                    self.process_decrypted_packet(now, remote, number, packet)
                }
            }
        };

        // State transitions for error cases
        if let Err(conn_err) = result {
            self.error = Some(conn_err.clone());
            self.state = match conn_err {
                ConnectionError::ApplicationClosed(reason) => State::closed(reason),
                ConnectionError::ConnectionClosed(reason) => State::closed(reason),
                // todo: change the condition
                ConnectionError::Reset => State::Drained,
                ConnectionError::TimedOut => {
                    unreachable!("timeouts aren't generated by packet processing");
                }
                ConnectionError::TransportError(err) => {
                    debug!("closing connection due to transport error: {}", err);
                    State::closed(err)
                }
                ConnectionError::VersionMismatch => State::Draining,
                ConnectionError::LocallyClosed => {
                    unreachable!("LocallyClosed isn't generated by packet processing");
                }
                ConnectionError::CidsExhausted => {
                    unreachable!("CidsExhausted isn't generated by packet processing");
                }
            };
        }
        if !was_closed && self.state.is_closed() {
            self.close_common();
            if !self.state.is_drained() {
                self.set_close_timer(now);
            }
        }

        if !was_drained && self.state.is_drained() {
            self.endpoint_events.push_back(EndpointEventInner::Drained);
            // Close timer may have been started previously, e.g. if we sent a close and got a
            // stateless reset in response
            self.timers.stop(Timer::Close);
        }
        // Transmit CONNECTION_CLOSE if necessary
        if let State::Closed(_) = self.state {
            self.close = remote == self.path.remote;
        }
    }

    /// 8. Whether the connection is in the process of being established
    ///
    /// If this returns `false`, the connection may be either established or closed, signaled by the
    /// emission of a `Connected` or `ConnectionLost` message respectively.
    pub fn is_handshaking(&self) -> bool {
        self.state.is_handshake()
    }
    /// 9.
    fn decrypt_packet(
        &mut self,
        now: Instant,
        packet: &mut Packet,
    ) -> Result<Option<u64>, Option<TransportError>> {
        let result = packet_crypto::decrypt_packet_body(
            packet,
            &self.spaces,
            self.zero_rtt_crypto.as_ref(),
            self.key_phase,
            self.prev_crypto.as_ref(),
            self.next_crypto.as_ref(),
        )?;

        let result = match result {
            Some(r) => r,
            None => return Ok(None),
        };

        if result.outgoing_key_update_acked {
            if let Some(prev) = self.prev_crypto.as_mut() {
                prev.end_packet = Some((result.number, now));
                self.set_key_discard_timer(now, packet.header.space());
            }
        }
        todo!()
    }
    /// 10
    fn set_key_discard_timer(&mut self, now: Instant, space: SpaceId) {
        todo!()
    }
    /// 11
    fn process_decrypted_packet(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        number: Option<u64>,
        packet: Packet,
    ) -> Result<(), ConnectionError> {
        let state = match self.state {
            State::Established => {
                match packet.header.space() {
                    SpaceId::Data => self.process_payload(now, remote, number.unwrap(), packet)?,
                    _ => self.process_early_payload(now, packet)?,
                }
                return Ok(());
            }
            State::Closed(_) => {
                for result in frame::Iter::new(packet.payload.freeze())? {
                    let frame = match result {
                        Ok(frame) => frame,
                        Err(err) => {
                            debug!("frame decoding error: {err:?}");
                            continue;
                        }
                    };

                    if let Frame::Padding = frame {
                        continue;
                    };

                    todo!()
                }
                return Ok(());
            }
            State::Draining | State::Drained => return Ok(()),
            State::Handshake(ref mut state) => state,
        };

        match packet.header {
            Header::Retry {
                src_cid: rem_cid, ..
            } => {
                todo!()
            }
            Header::Long {
                ty: LongType::Handshake,
                src_cid: rem_cid,
                ..
            } => {
                todo!()
            }
            Header::Initial(InitialHeader {
                src_cid: rem_cid, ..
            }) => {
                todo!()
            }
            Header::Long {
                ty: LongType::ZeroRtt,
                ..
            } => {
                todo!()
            }
            Header::VersionNegotiate { .. } => {
                if self.total_authed_packets > 1 {
                    return Ok(());
                }

                let supported = packet
                    .payload
                    .chunks(4)
                    .any(|x| match <[u8; 4]>::try_from(x) {
                        Ok(version) => self.version == u32::from_be_bytes(version),
                        Err(_) => false,
                    });
                if supported {
                    return Ok(());
                }
                debug!("remote doesn't support our version");
                Err(ConnectionError::VersionMismatch)
            }
            Header::Short { .. } => unreachable!(
                "short packets received during handshake are discarded in handle_packet"
            ),
        }
    }
    /// 12
    fn process_payload(
        &mut self,
        now: Instant,
        remote: SocketAddr,
        number: u64,
        packet: Packet,
    ) -> Result<(), TransportError> {
        todo!()
    }
    /// 13. Process an Initial or Handshake packet payload
    fn process_early_payload(
        &mut self,
        now: Instant,
        packet: Packet,
    ) -> Result<(), TransportError> {
        todo!()
    }
    /// 14
    fn close_common(&mut self) {
        trace!("connection closed");
        for &timer in &Timer::VALUES {
            self.timers.stop(timer);
        }
    }
    /// 15
    fn set_close_timer(&mut self, now: Instant) {
        self.timers
            .set(Timer::Close, now + 3 * self.pto(self.highest_space));
    }
    /// 16. Probe Timeout
    fn pto(&self, space: SpaceId) -> Duration {
        let max_ack_delay = match space {
            SpaceId::Initial | SpaceId::Handshake => Duration::new(0, 0),
            SpaceId::Data => self.ack_frequency.max_ack_delay_for_pto(),
        };

        self.path.rtt.pto_base() + max_ack_delay
    }
    /// 17 first called by client
    fn write_crypto(&mut self) {
        loop {
            let space = self.highest_space;
            let mut outgoing = Vec::new();
            if let Some(crypto) = self.crypto.write_handshake(&mut outgoing) {
                match space {
                    SpaceId::Initial => {
                        trace!("ini 1{:?}", outgoing);
                        self.upgrade_crypto(SpaceId::Handshake, crypto);
                        trace!("ini 2{:?}", outgoing);
                    }
                    SpaceId::Handshake => {
                        trace!("Handshake 1{:?}", outgoing);
                        self.upgrade_crypto(SpaceId::Data, crypto);
                        trace!("Handshake 2{:?}", outgoing);
                    }
                    _ => unreachable!("got updated secrets during 1-RTT"),
                }
            }
            if outgoing.is_empty() {
                if space == self.highest_space {
                    break;
                } else {
                    // Keys updated, check for more data to send
                    continue;
                }
            }
            let offset = self.spaces[space as usize].crypto_offset;
            let outgoing = Bytes::from(outgoing);
            if let State::Handshake(ref mut state) = self.state {
                if space == SpaceId::Initial && offset == 0 && self.side.is_client() {
                    state.client_hello = Some(outgoing.clone());
                }
            }

            self.spaces[space as usize].crypto_offset += outgoing.len() as u64;
            trace!(
                "wrote {} {:?} CRYPTO bytes {:?}",
                outgoing.len(),
                space,
                outgoing.clone().bytes()
            );

            self.spaces[space as usize]
                .pending
                .crypto
                .push_back(frame::Crypto {
                    offset,
                    data: outgoing,
                });
        }
    }

    /// 18
    fn init_0rtt(&mut self) {
        // 最开始的时候，直接就是返回None
        let (header, packet) = match self.crypto.early_crypto() {
            Some(x) => x,
            None => return,
        };
        todo!()
    }

    /// 19. Switch to stronger cryptography during handshake
    fn upgrade_crypto(&mut self, space: SpaceId, crypto: Keys) {
        todo!()
    }

    /// 20. Returns application-facing events
    ///
    /// Connections should be polled for events after:
    /// - a call was made to `handle_event`
    /// - a call was made to `handle_timeout`
    #[must_use]
    pub fn poll(&mut self) -> Option<Event> {
        if let Some(x) = self.events.pop_front() {
            return Some(x);
        }

        if let Some(event) = self.streams.poll() {
            return Some(Event::Stream(event));
        }

        if let Some(err) = self.error.take() {
            return Some(Event::ConnectionLost { reason: err });
        }

        None
    }
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Clone)]
pub enum State {
    /// 1.
    Handshake(state::Handshake),
    /// 2.
    Closed(state::Closed),
    /// 3
    Draining,
    /// 4. Waiting for application to call close so we can dispose of the resources
    Drained,
    /// 5.
    Established,
}

impl State {
    /// 1
    fn is_handshake(&self) -> bool {
        matches!(*self, Self::Handshake(_))
    }
    /// 2
    fn is_closed(&self) -> bool {
        matches!(*self, Self::Closed(_) | Self::Draining | Self::Drained)
    }
    /// 3
    fn is_drained(&self) -> bool {
        matches!(*self, Self::Drained)
    }
    /// 4.
    fn closed<R: Into<Close>>(reason: R) -> Self {
        Self::Closed(state::Closed {
            reason: reason.into(),
        })
    }
}

mod state {
    use bytes::Bytes;

    use crate::frame::Close;

    use super::*;

    #[allow(unreachable_pub)] // fuzzing only
    #[derive(Clone)]
    pub struct Handshake {
        /// Whether the remote CID has been set by the peer yet
        ///
        /// Always set for servers
        pub(super) rem_cid_set: bool,
        /// Stateless retry token received in the first Initial by a server.
        ///
        /// Must be present in every Initial. Always empty for clients.
        pub(super) expected_token: Bytes,
        /// First cryptographic message
        ///
        /// Only set for clients
        pub(super) client_hello: Option<Bytes>,
    }

    #[allow(unreachable_pub)] // fuzzing only
    #[derive(Clone)]
    pub struct Closed {
        pub(super) reason: Close,
    }
}

/// Reasons why a connection might be lost
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    /// 1. The peer is unable to continue processing this connection, usually due to having restarted
    #[error("reset by peer")]
    Reset,
    /// 2. The peer violated the QUIC specification as understood by this implementation
    #[error(transparent)]
    TransportError(#[from] TransportError),
    /// 3. The peer doesn't implement any supported version
    #[error("peer doesn't implement any supported version")]
    VersionMismatch,
    /// 4. The peer's QUIC stack aborted the connection automatically
    #[error("aborted by peer: {0}")]
    ConnectionClosed(frame::ConnectionClose),
    /// 5. The peer closed the connection
    #[error("closed by peer: {0}")]
    ApplicationClosed(frame::ApplicationClose),
    /// 6. Communication with the peer has lapsed for longer than the negotiated idle timeout
    ///
    /// If neither side is sending keep-alives, a connection will time out after a long enough idle
    /// period even if the peer is still reachable. See also [`TransportConfig::max_idle_timeout()`]
    /// and [`TransportConfig::keep_alive_interval()`].
    #[error("timed out")]
    TimedOut,
    /// 7. The local application closed the connection
    #[error("closed")]
    LocallyClosed,
    /// 8.The connection could not be created because not enough of the CID space is available
    ///
    /// Try using longer connection IDs.
    #[error("CIDs exhausted")]
    CidsExhausted,
}
/// used for AckFrequencyState::new
fn get_max_ack_delay(params: &TransportParameters) -> Duration {
    Duration::from_micros(params.max_ack_delay.0 * 1000)
}

/// Events of interest to the application
#[derive(Debug)]
pub enum Event {
    /// 1. The connection was lost
    ///
    /// Emitted if the peer closes the connection or an error is encountered.
    ConnectionLost {
        /// Reason that the connection was closed
        reason: ConnectionError,
    },
    /// 2. Stream events
    Stream(StreamEvent),
}
