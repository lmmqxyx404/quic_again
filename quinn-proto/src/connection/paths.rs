use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::endpoint::TransportConfig;

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
        Self {
            remote,
            validated,
            total_sent: 0,
            total_recvd: 0,
            rtt: RttEstimator::new(config.initial_rtt),
        }
    }
    /// 2. Indicates whether we're a server that hasn't validated the peer's address and hasn't
    /// received enough data from the peer to permit sending `bytes_to_send` additional bytes
    pub(super) fn anti_amplification_blocked(&self, bytes_to_send: u64) -> bool {
        !self.validated && self.total_recvd * 3 < self.total_sent + bytes_to_send
    }
}

/// RTT estimation for a particular network path
#[derive(Copy, Clone)]
pub struct RttEstimator {}

impl RttEstimator {
    /// 1
    fn new(initial_rtt: Duration) -> Self {
        todo!()
    }

    /// 2.PTO computed as described in RFC9002#6.2.1
    pub(crate) fn pto_base(&self) -> Duration {
        todo!()
        // self.get() + cmp::max(4 * self.var, TIMER_GRANULARITY)
    }
}
