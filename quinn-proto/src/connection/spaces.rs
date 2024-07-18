use rand::Rng;

use crate::frame;
use crate::packet::SpaceId;
use crate::range_set::ArrayRangeSet;
use crate::{crypto::Keys, shared::IssuedCid};
use std::collections::VecDeque;
use std::ops::{Index, IndexMut};
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
    /// 7. The packet number of the next packet that will be sent, if any. In the Data space, the
    /// packet number stored here is sometimes skipped by [`PacketNumberFilter`] logic.
    pub(super) next_packet_number: u64,
    /// 8. The largest packet number the remote peer acknowledged in an ACK frame.
    pub(super) largest_acked_packet: Option<u64>,
    /// 9. Packet numbers to acknowledge
    pub(super) pending_acks: PendingAcks,
    /// 10.
    pub(super) ping_pending: bool,
    /// 11. Number of packets sent in the current key phase
    pub(super) sent_with_keys: u64,
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
            next_packet_number: 0,
            largest_acked_packet: None,

            pending_acks: PendingAcks::new(),
            ping_pending: false,
            sent_with_keys: 0,
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
    /// 3
    pub(super) fn can_send(&self, streams: &StreamsState) -> SendableFrames {
        let acks = self.pending_acks.can_send();
        let other =
            !self.pending.is_empty(streams) || self.ping_pending || self.immediate_ack_pending;

        SendableFrames { acks, other }
    }

    /// 4. Get the next outgoing packet number in this space
    ///
    /// In the Data space, the connection's [`PacketNumberFilter`] must be used rather than calling
    /// this directly.
    pub(super) fn get_tx_number(&mut self) -> u64 {
        // TODO: Handle packet number overflow gracefully
        assert!(self.next_packet_number < 2u64.pow(62));
        let x = self.next_packet_number;
        self.next_packet_number += 1;
        self.sent_with_keys += 1;
        x
    }
}

impl Index<SpaceId> for [PacketSpace; 3] {
    type Output = PacketSpace;
    fn index(&self, space: SpaceId) -> &PacketSpace {
        &self.as_ref()[space as usize]
    }
}

impl IndexMut<SpaceId> for [PacketSpace; 3] {
    fn index_mut(&mut self, space: SpaceId) -> &mut PacketSpace {
        &mut self.as_mut()[space as usize]
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

/// Helper for mitigating [optimistic ACK attacks]
///
/// A malicious peer could prompt the local application to begin a large data transfer, and then
/// send ACKs without first waiting for data to be received. This could defeat congestion control,
/// allowing the connection to consume disproportionate resources. We therefore occasionally skip
/// packet numbers, and classify any ACK referencing a skipped packet number as a transport error.
///
/// Skipped packet numbers occur only in the application data space (where costly transfers might
/// take place) and are distributed exponentially to reflect the reduced likelihood and impact of
/// bad behavior from a peer that has been well-behaved for an extended period.
///
/// ACKs for packet numbers that have not yet been allocated are also a transport error, but an
/// attacker with knowledge of the congestion control algorithm in use could time falsified ACKs to
/// arrive after the packets they reference are sent.
///
/// [optimistic ACK attacks]: https://www.rfc-editor.org/rfc/rfc9000.html#name-optimistic-ack-attack
pub(super) struct PacketNumberFilter {
    /// Next outgoing packet number to skip
    next_skipped_packet_number: u64,
}

impl PacketNumberFilter {
    /// 1
    pub(super) fn new(rng: &mut (impl Rng + ?Sized)) -> Self {
        // First skipped PN is in 0..64
        let exponent = 6;
        Self {
            next_skipped_packet_number: rng.gen_range(0..2u64.saturating_pow(exponent)),
        }
    }
    /// 2
    #[cfg(test)]
    pub(super) fn disabled() -> Self {
        Self {
            next_skipped_packet_number: u64::MAX,
        }
    }
    /// 3
    pub(super) fn peek(&self, space: &PacketSpace) -> u64 {
        let n = space.next_packet_number;
        if n != self.next_skipped_packet_number {
            return n;
        }
        n + 1
    }
    /// 4
    pub(super) fn allocate(
        &mut self,
        rng: &mut (impl Rng + ?Sized),
        space: &mut PacketSpace,
    ) -> u64 {
        todo!()
    }
}

/// Indicates which data is available for sending
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SendableFrames {
    pub(super) acks: bool,
    pub(super) other: bool,
}

impl SendableFrames {
    /// 1. Whether no data is sendable
    pub(super) fn is_empty(&self) -> bool {
        !self.acks && !self.other
    }

    /// 2. Returns that no data is available for sending
    pub(super) fn empty() -> Self {
        Self {
            acks: false,
            other: false,
        }
    }
}

#[derive(Debug)]
pub(super) struct PendingAcks {
    /// 1. Whether we should send an ACK immediately, even if that means sending an ACK-only packet
    ///
    /// When `immediate_ack_required` is false, the normal behavior is to send ACK frames only when
    /// there is other data to send, or when the `MaxAckDelay` timer expires.
    immediate_ack_required: bool,
    /// 2. The packet number ranges of ack-eliciting packets the peer hasn't confirmed receipt of ACKs
    /// for
    ranges: ArrayRangeSet,
}

impl PendingAcks {
    /// 1.
    fn new() -> Self {
        Self {
            immediate_ack_required: false,

            ranges: ArrayRangeSet::default(),
        }
    }
    /// 2. Whether any ACK frames can be sent
    pub(super) fn can_send(&self) -> bool {
        self.immediate_ack_required && !self.ranges.is_empty()
    }
    /// 3. Returns the set of currently pending ACK ranges
    pub(super) fn ranges(&self) -> &ArrayRangeSet {
        &self.ranges
    }

    /// 4. Should be called whenever ACKs have been sent
    ///
    /// This will suppress sending further ACKs until additional ACK eliciting frames arrive
    pub(super) fn acks_sent(&mut self) {
        todo!()
    }
}
