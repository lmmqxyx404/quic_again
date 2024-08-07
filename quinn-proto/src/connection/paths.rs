use std::{
    cmp,
    net::SocketAddr,
    time::{Duration, Instant},
};

use tracing::trace;

use crate::{config::TransportConfig, congestion, packet::SpaceId, TIMER_GRANULARITY};

use super::{
    mtud::MtuDiscovery,
    pacing::Pacer,
    spaces::{PacketSpace, SentPacket},
};

/// Description of a particular network path
pub(super) struct PathData {
    /// 1.
    pub(super) remote: SocketAddr,
    /// 2. Total size of all UDP datagrams sent on this path
    pub(super) total_sent: u64,
    /// 3. Total size of all UDP datagrams received on this path
    pub(super) total_recvd: u64,
    /// 4. Whether we're certain the peer can both send and receive on this address
    ///
    /// Initially equal to `use_stateless_retry` for servers, and becomes false again on every
    /// migration. Always true for clients.
    pub(super) validated: bool,
    /// 5.
    pub(super) rtt: RttEstimator,
    /// 6. The state of the MTU discovery process
    pub(super) mtud: MtuDiscovery,
    /// 7.
    pub(super) in_flight: InFlight,
    /// 8. Congestion controller state
    pub(super) congestion: Box<dyn congestion::Controller>,
    /// 9. Pacing state
    pub(super) pacing: Pacer,
    /// 10.
    pub(super) challenge: Option<u64>,
    /// 11.
    pub(super) challenge_pending: bool,
    /// 12. Number of the first packet sent on this path
    ///
    /// Used to determine whether a packet was sent on an earlier path. Insufficient to determine if
    /// a packet was sent on a later path.
    first_packet: Option<u64>,
    /// 13. Whether we're enabling ECN on outgoing packets
    pub(super) sending_ecn: bool,
    /// 14. Packet number of the first packet sent after an RTT sample was collected on this path
    ///
    /// Used in persistent congestion determination.
    pub(super) first_packet_after_rtt_sample: Option<(SpaceId, u64)>,
}

impl PathData {
    /// 1.
    pub(super) fn new(
        remote: SocketAddr,
        allow_mtud: bool,
        peer_max_udp_payload_size: Option<u16>,
        now: Instant,
        validated: bool,
        config: &TransportConfig,
    ) -> Self {
        let congestion = config
            .congestion_controller_factory
            .clone()
            .build(now, config.get_initial_mtu());
        Self {
            remote,
            validated,
            total_sent: 0,
            total_recvd: 0,
            rtt: RttEstimator::new(config.initial_rtt),
            mtud: config
                .mtu_discovery_config
                .as_ref()
                .filter(|_| allow_mtud)
                .map_or(
                    MtuDiscovery::disabled(config.get_initial_mtu(), config.min_mtu),
                    |mtud_config| {
                        MtuDiscovery::new(
                            config.get_initial_mtu(),
                            config.min_mtu,
                            peer_max_udp_payload_size,
                            mtud_config.clone(),
                        )
                    },
                ),

            in_flight: InFlight::new(),
            pacing: Pacer::new(
                config.initial_rtt,
                congestion.initial_window(),
                config.get_initial_mtu(),
                now,
            ),
            congestion,
            challenge: None,
            challenge_pending: false,
            first_packet: None,
            sending_ecn: true,
            first_packet_after_rtt_sample: None,
        }
    }
    /// 2. Indicates whether we're a server that hasn't validated the peer's address and hasn't
    /// received enough data from the peer to permit sending `bytes_to_send` additional bytes
    pub(super) fn anti_amplification_blocked(&self, bytes_to_send: u64) -> bool {
        !self.validated && self.total_recvd * 3 < self.total_sent + bytes_to_send
    }

    /// 3. Returns the path's current MTU
    pub(super) fn current_mtu(&self) -> u16 {
        self.mtud.current_mtu()
    }

    /// 4. Account for transmission of `packet` with number `pn` in `space`
    pub(super) fn sent(&mut self, pn: u64, packet: SentPacket, space: &mut PacketSpace) {
        self.in_flight.insert(&packet);
        if self.first_packet.is_none() {
            self.first_packet = Some(pn);
        }
        self.in_flight.bytes -= space.sent(pn, packet);
    }
    /// 5. Remove `packet` with number `pn` from this path's congestion control counters, or return
    /// `false` if `pn` was sent before this path was established.
    pub(super) fn remove_in_flight(&mut self, pn: u64, packet: &SentPacket) -> bool {
        if self.first_packet.map_or(true, |first| first > pn) {
            return false;
        }
        self.in_flight.remove(packet);
        true
    }
    /// 6.
    pub(super) fn from_previous(remote: SocketAddr, prev: &Self, now: Instant) -> Self {
        todo!()
    }
}

/// RTT estimation for a particular network path
#[derive(Copy, Clone)]
pub struct RttEstimator {
    /// 1. The most recent RTT measurement made when receiving an ack for a previously unacked packet
    latest: Duration,
    /// 2. The smoothed RTT of the connection, computed as described in RFC6298
    smoothed: Option<Duration>,
    /// 3. The RTT variance, computed as described in RFC6298
    var: Duration,
    /// 4. The minimum RTT seen in the connection, ignoring ack delay.
    min: Duration,
}

impl RttEstimator {
    /// 1
    fn new(initial_rtt: Duration) -> Self {
        Self {
            latest: initial_rtt,
            smoothed: None,
            var: initial_rtt / 2,
            min: initial_rtt,
        }
    }

    /// 2.PTO computed as described in RFC9002#6.2.1
    pub(crate) fn pto_base(&self) -> Duration {
        self.get() + cmp::max(4 * self.var, TIMER_GRANULARITY)
    }

    /// 3.The current best RTT estimation.
    pub fn get(&self) -> Duration {
        self.smoothed.unwrap_or(self.latest)
    }
    /// 4.
    pub(crate) fn update(&mut self, ack_delay: Duration, rtt: Duration) {
        self.latest = rtt;
        // min_rtt ignores ack delay.
        self.min = cmp::min(self.min, self.latest);
        // Based on RFC6298.
        if let Some(smoothed) = self.smoothed {
            let adjusted_rtt = if self.min + ack_delay <= self.latest {
                self.latest - ack_delay
            } else {
                self.latest
            };
            let var_sample = if smoothed > adjusted_rtt {
                smoothed - adjusted_rtt
            } else {
                adjusted_rtt - smoothed
            };
            self.var = (3 * self.var + var_sample) / 4;
            self.smoothed = Some((7 * smoothed + adjusted_rtt) / 8);
        } else {
            self.smoothed = Some(self.latest);
            self.var = self.latest / 2;
            self.min = self.latest;
        }
    }
    /// 5. Conservative estimate of RTT
    ///
    /// Takes the maximum of smoothed and latest RTT, as recommended
    /// in 6.1.2 of the recovery spec (draft 29).
    pub fn conservative(&self) -> Duration {
        self.get().max(self.latest)
    }
}

/// Summary statistics of packets that have been sent on a particular path, but which have not yet
/// been acked or deemed lost
pub(super) struct InFlight {
    /// 1. Sum of the sizes of all sent packets considered "in flight" by congestion control
    ///
    /// The size does not include IP or UDP overhead. Packets only containing ACK frames do not
    /// count towards this to ensure congestion control does not impede congestion feedback.
    pub(super) bytes: u64,
    /// 2. Number of packets in flight containing frames other than ACK and PADDING
    ///
    /// This can be 0 even when bytes is not 0 because PADDING frames cause a packet to be
    /// considered "in flight" by congestion control. However, if this is nonzero, bytes will always
    /// also be nonzero.
    pub(super) ack_eliciting: u64,
}

impl InFlight {
    fn new() -> Self {
        Self {
            bytes: 0,
            ack_eliciting: 0,
        }
    }
    /// 2.
    fn insert(&mut self, packet: &SentPacket) {
        self.bytes += u64::from(packet.size);
        self.ack_eliciting += u64::from(packet.ack_eliciting);
    }
    /// 3. Update counters to account for a packet becoming acknowledged, lost, or abandoned
    fn remove(&mut self, packet: &SentPacket) {
        self.bytes -= u64::from(packet.size);
        self.ack_eliciting -= u64::from(packet.ack_eliciting);
    }
}

#[derive(Default)]
pub(crate) struct PathResponses {
    pending: Vec<PathResponse>,
}

impl PathResponses {
    /// 1
    pub(crate) fn pop_off_path(&mut self, remote: &SocketAddr) -> Option<(u64, SocketAddr)> {
        let response = *self.pending.last()?;
        if response.remote == *remote {
            // We don't bother searching further because we expect that the on-path response will
            // get drained in the immediate future by a call to `pop_on_path`
            return None;
        }
        self.pending.pop();
        Some((response.token, response.remote))
    }
    /// 2
    pub(crate) fn pop_on_path(&mut self, remote: &SocketAddr) -> Option<u64> {
        let response = *self.pending.last()?;
        if response.remote != *remote {
            // We don't bother searching further because we expect that the off-path response will
            // get drained in the immediate future by a call to `pop_off_path`
            return None;
        }
        self.pending.pop();
        Some(response.token)
    }
    /// 3.
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
    /// 4
    pub(crate) fn push(&mut self, packet: u64, token: u64, remote: SocketAddr) {
        /// Arbitrary permissive limit to prevent abuse
        const MAX_PATH_RESPONSES: usize = 16;
        let response = PathResponse {
            packet,
            token,
            remote,
        };
        let existing = self.pending.iter_mut().find(|x| x.remote == remote);
        if let Some(existing) = existing {
            // Update a queued response
            if existing.packet <= packet {
                *existing = response;
            }
            return;
        }
        if self.pending.len() < MAX_PATH_RESPONSES {
            self.pending.push(response);
        } else {
            // We don't expect to ever hit this with well-behaved peers, so we don't bother dropping
            // older challenges.
            trace!("ignoring excessive PATH_CHALLENGE");
        }
    }
}

#[derive(Copy, Clone)]
struct PathResponse {
    /// 1
    token: u64,
    /// 2.The address the corresponding PATH_CHALLENGE was received from
    remote: SocketAddr,
    /// 3.The packet number the corresponding PATH_CHALLENGE was received in
    packet: u64,
}
