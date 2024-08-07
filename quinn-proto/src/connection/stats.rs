use std::time::Duration;

use crate::{frame::Frame, Dir};

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
    /// 4. Statistics related to the current transmission path
    pub path: PathStats,
    /// 5. Statistics about frames received on a connection
    pub frame_rx: FrameStats,
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
    /// 11.
    pub ping: u64,
    /// 12.
    pub acks: u64,
    /// 13.
    pub connection_close: u64,
    /// 14.
    pub reset_stream: u64,
    /// 15.
    pub stop_sending: u64,
    /// 16
    pub max_streams_bidi: u64,
    /// 17
    pub max_streams_uni: u64,
    /// 18.
    pub max_stream_data: u64,
    /// 19.
    pub max_data: u64,
}

impl FrameStats {
    /// 1.
    pub(crate) fn record(&mut self, frame: &Frame) {
        match frame {
            Frame::Padding => {}
            Frame::Ping => self.ping += 1,
            Frame::Ack(_) => self.acks += 1,

            Frame::Crypto(_) => self.crypto += 1,

            Frame::Stream(_) => self.stream += 1,

            Frame::Close(_) => self.connection_close += 1,
            Frame::Datagram(_) => self.datagram += 1,

            Frame::NewConnectionId(_) => self.new_connection_id += 1,
            Frame::HandshakeDone => self.handshake_done = self.handshake_done.saturating_add(1),
            Frame::RetireConnectionId { .. } => self.retire_connection_id += 1,
            Frame::ResetStream(_) => self.reset_stream += 1,
            Frame::StopSending(_) => self.stop_sending += 1,
            Frame::MaxStreams { dir, .. } => {
                if *dir == Dir::Bi {
                    self.max_streams_bidi += 1;
                } else {
                    self.max_streams_uni += 1;
                }
            }
            Frame::PathChallenge(_) => self.path_challenge += 1,
            Frame::PathResponse(_) => self.path_response += 1,
            Frame::ImmediateAck => self.immediate_ack += 1,

            Frame::MaxStreamData { .. } => self.max_stream_data += 1,
            Frame::MaxData(_) => self.max_data += 1,
            Frame::AckFrequency(_) => self.ack_frequency += 1,
        }
    }
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

/// Statistics related to a transmission path
#[derive(Debug, Default, Copy, Clone)]
#[non_exhaustive]
pub struct PathStats {
    /// 1. The amount of packets sent on this path
    pub sent_packets: u64,
    /// 2. Congestion events on the connection
    pub congestion_events: u64,
    /// 3. The amount of PLPMTUD probe packets sent on this path (also counted by `sent_packets`)
    pub sent_plpmtud_probes: u64,
    /// 4. The amount of packets lost on this path
    pub lost_packets: u64,
    /// 5. The amount of bytes lost on this path
    pub lost_bytes: u64,
    /// 6. Current best estimate of this connection's latency (round-trip-time)
    pub rtt: Duration,
    /// 7. Current congestion window of the connection
    pub cwnd: u64,
    /// 8. The amount of PLPMTUD probe packets lost on this path (ignored by `lost_packets` and
    /// `lost_bytes`)
    pub lost_plpmtud_probes: u64,
    /// 9. The number of times a black hole was detected in the path
    pub black_holes_detected: u64,
}
