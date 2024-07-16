use crate::frame;
use crate::{crypto::Keys, shared::IssuedCid};
use std::collections::VecDeque;
use std::time::Instant;
use std::{cmp, mem};

use super::streams::StreamsState;

/// 1.
pub(super) struct PacketSpace {
    /// 1.
    pub(super) crypto: Option<Keys>,
    /// 2. Data to send
    pub(super) pending: Retransmits,
    /// 3.
    pub(super) dedup: Dedup,
    /// 4. Current offset of outgoing cryptographic handshake stream
    pub(super) crypto_offset: u64,
    /// 5. Number of tail loss probes to send
    pub(super) loss_probes: u32,
    /// 6.
    pub(super) immediate_ack_pending: bool,
}

impl PacketSpace {
    /// 1.
    pub(super) fn new(now: Instant) -> Self {
        Self {
            crypto: None,
            pending: Retransmits::default(),
            dedup: Dedup::new(),

            crypto_offset: 0,
            loss_probes: 0,
            immediate_ack_pending: false,
        }
    }

    /// 2. Queue data for a tail loss probe (or anti-amplification deadlock prevention) packet
    ///
    /// Probes are sent similarly to normal packets when an expect ACK has not arrived. We never
    /// deem a packet lost until we receive an ACK that should have included it, but if a trailing
    /// run of packets (or their ACKs) are lost, this might not happen in a timely fashion. We send
    /// probe packets to force an ACK, and exempt them from congestion control to prevent a deadlock
    /// when the congestion window is filled with lost tail packets.
    ///
    /// We prefer to send new data, to make the most efficient use of bandwidth. If there's no data
    /// waiting to be sent, then we retransmit in-flight data to reduce odds of loss. If there's no
    /// in-flight data either, we're probably a client guarding against a handshake
    /// anti-amplification deadlock and we just make something up.
    pub(super) fn maybe_queue_probe(
        &mut self,
        request_immediate_ack: bool,
        streams: &StreamsState,
    ) {
        if self.loss_probes == 0 {
            return;
        }

        if request_immediate_ack {
            // The probe should be ACKed without delay (should only be used in the Data space and
            // when the peer supports the acknowledgement frequency extension)
            self.immediate_ack_pending = true;
        }

        // Retransmit the data of the oldest in-flight packet
        if !self.pending.is_empty(streams) {
            // There's real data to send here, no need to make something up
            return;
        }
        todo!()
    }
}

/// 2. Retransmittable data queue
#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Default, Clone)]
pub struct Retransmits {
    /// 1
    pub(super) new_cids: Vec<IssuedCid>,
    /// 2
    pub(super) crypto: VecDeque<frame::Crypto>,
    /// 3.
    pub(super) ack_frequency: bool,
}

impl Retransmits {
    /// todo: change the fn
    pub(super) fn is_empty(&self, streams: &StreamsState) -> bool {
        tracing::error!("to delete is_empty");
        self.crypto.is_empty() && self.new_cids.is_empty() && !self.ack_frequency
    }
}

/// RFC4303-style sliding window packet number deduplicator.
///
/// A contiguous bitfield, where each bit corresponds to a packet number and the rightmost bit is
/// always set. A set bit represents a packet that has been successfully authenticated. Bits left of
/// the window are assumed to be set.
///
/// ```text
/// ...xxxxxxxxx 1 0
///     ^        ^ ^
/// window highest next
/// ```
pub(super) struct Dedup {
    window: Window,
    /// Lowest packet number higher than all yet authenticated.
    next: u64,
}

/// Inner bitfield type.
///
/// Because QUIC never reuses packet numbers, this only needs to be large enough to deal with
/// packets that are reordered but still delivered in a timely manner.
type Window = u128;

/// Number of packets tracked by `Dedup`.
const WINDOW_SIZE: u64 = 1 + mem::size_of::<Window>() as u64 * 8;

impl Dedup {
    /// 1. Construct an empty window positioned at the start.
    pub(super) fn new() -> Self {
        Self { window: 0, next: 0 }
    }
    /// 2. Record a newly authenticated packet number.
    ///
    /// Returns whether the packet might be a duplicate.
    pub(super) fn insert(&mut self, packet: u64) -> bool {
        if let Some(diff) = packet.checked_sub(self.next) {
            // Right of window
            self.window = (self.window << 1 | 1)
                .checked_shl(cmp::min(diff, u64::from(u32::MAX)) as u32)
                .unwrap_or(0);
            self.next = packet + 1;
            false
        } else if self.highest() - packet < WINDOW_SIZE {
            // Within window
            if let Some(bit) = (self.highest() - packet).checked_sub(1) {
                // < highest
                let mask = 1 << bit;
                let duplicate = self.window & mask != 0;
                self.window |= mask;
                duplicate
            } else {
                // == highest
                true
            }
        } else {
            // Left of window
            true
        }
    }
    /// 3. Highest packet number authenticated.
    fn highest(&self) -> u64 {
        self.next - 1
    }
}
