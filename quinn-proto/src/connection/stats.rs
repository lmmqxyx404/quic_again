/// 1. Connection statistics
#[derive(Debug, Default, Copy, Clone)]
#[non_exhaustive]
pub struct ConnectionStats {
    /// 1. Statistics about UDP datagrams received on a connection
    pub udp_rx: UdpStats,
    /// 2. Statistics about frames transmitted on a connection
    pub frame_tx: FrameStats,
    /// 3. Statistics about UDP datagrams transmitted on a connection
    pub udp_tx: UdpStats,
}

/// 2. Statistics about UDP datagrams transmitted or received on a connection
#[derive(Default, Debug, Copy, Clone)]
#[non_exhaustive]
pub struct UdpStats {
    /// The amount of UDP datagrams observed
    pub datagrams: u64,
    /// The total amount of bytes which have been transferred inside UDP datagrams
    pub bytes: u64,
    /// The amount of I/O operations executed
    ///
    /// Can be less than `datagrams` when GSO, GRO, and/or batched system calls are in use.
    pub ios: u64,
}

impl UdpStats {
    pub(crate) fn on_sent(&mut self, datagrams: u64, bytes: usize) {
        self.datagrams += datagrams;
        self.bytes += bytes as u64;
        self.ios += 1;
    }
}

/// Number of frames transmitted of each frame type
#[derive(Default, Copy, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub struct FrameStats {
    /// 1.
    pub path_response: u64,
    /// 2.
    pub handshake_done: u8,
    /// 3.
    pub immediate_ack: u64,
    /// 4.
    pub ack_frequency: u64,
    /// 5.
    pub path_challenge: u64,
    /// 6.
    pub crypto: u64,
    /// 7.
    pub new_connection_id: u64,
    /// 8.
    pub retire_connection_id: u64,
    /// 9.
    pub datagram: u64,
    /// 10.
    pub stream: u64,
}

impl std::fmt::Debug for FrameStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameStats")
            .field("PATH_RESPONSE", &self.path_response)
            .field("HANDSHAKE_DONE", &self.handshake_done)
            .field("IMMEDIATE_ACK", &self.immediate_ack)
            .field("ACK_FREQUENCY", &self.ack_frequency)
            .field("PATH_CHALLENGE", &self.path_challenge)
            .finish()
    }
}
