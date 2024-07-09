use std::{net::SocketAddr, time::Instant};

use crate::endpoint::TransportConfig;

/// Description of a particular network path
pub(super) struct PathData {
    pub(super) remote: SocketAddr,
}

impl PathData {
    pub(super) fn new(
        remote: SocketAddr,
        allow_mtud: bool,
        peer_max_udp_payload_size: Option<u16>,
        now: Instant,
        validated: bool,
        config: &TransportConfig,
    ) -> Self {
        Self { remote }
    }
}
