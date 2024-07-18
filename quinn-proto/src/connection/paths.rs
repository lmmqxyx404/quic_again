use std::{
    cmp,
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::{config::TransportConfig, congestion, TIMER_GRANULARITY};

use super::{mtud::MtuDiscovery, pacing::Pacer};

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
}

impl RttEstimator {
    /// 1
    fn new(initial_rtt: Duration) -> Self {
        Self {
            latest: initial_rtt,
            smoothed: None,
            var: initial_rtt / 2,
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
}

/// Summary statistics of packets that have been sent on a particular path, but which have not yet
/// been acked or deemed lost
pub(super) struct InFlight {
    /// 1. Sum of the sizes of all sent packets considered "in flight" by congestion control
    ///
    /// The size does not include IP or UDP overhead. Packets only containing ACK frames do not
    /// count towards this to ensure congestion control does not impede congestion feedback.
    pub(super) bytes: u64,
}

impl InFlight {
    fn new() -> Self {
        Self { bytes: 0 }
    }
}

#[derive(Default)]
pub(crate) struct PathResponses {}

impl PathResponses {
    /// 1
    pub(crate) fn pop_off_path(&mut self, remote: &SocketAddr) -> Option<(u64, SocketAddr)> {
        todo!()
    }
    /// 2
    pub(crate) fn pop_on_path(&mut self, remote: &SocketAddr) -> Option<u64> {
        todo!()
    }
}
