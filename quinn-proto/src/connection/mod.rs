use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use crate::{
    config::{EndpointConfig, ServerConfig},
    crypto::{self, KeyPair, PacketKey},
    endpoint::TransportConfig,
    packet::{Packet, PartialDecode, SpaceId},
    shared::{
        ConnectionEvent, ConnectionEventInner, ConnectionId, DatagramConnectionEvent, EcnCodepoint,
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
use timer::{Timer, TimerTable};
/// 6.
mod packet_crypto;

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
        };
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
            // todo: add trace!
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
        todo!()
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
        todo!()
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
