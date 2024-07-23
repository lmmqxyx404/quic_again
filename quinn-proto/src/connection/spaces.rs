use rand::Rng;
use rustc_hash::FxHashSet;
use tracing::trace;

use crate::packet::SpaceId;
use crate::range_set::ArrayRangeSet;
use crate::{crypto::Keys, shared::IssuedCid};
use crate::{frame, StreamId, TransportError, VarInt};
use std::collections::{BTreeMap, VecDeque};
use std::ops::{Bound, Index, IndexMut};
use std::time::{Duration, Instant};
use std::{cmp, mem};

use super::assembler::Assembler;
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
    /// 12. The highest-numbered ACK-eliciting packet we've sent
    pub(super) largest_ack_eliciting_sent: u64,
    /// 13. Number of packets in `sent_packets` with numbers above `largest_ack_eliciting_sent`
    pub(super) unacked_non_ack_eliciting_tail: u64,
    /// 14. Transmitted but not acked
    /// We use a BTreeMap here so we can efficiently query by range on ACK and for loss detection
    pub(super) sent_packets: BTreeMap<u64, SentPacket>,
    /// 15. Number of congestion control "in flight" bytes
    pub(super) in_flight: u64,
    /// 16. The time the most recently sent retransmittable packet was sent.
    pub(super) time_of_last_ack_eliciting_packet: Option<Instant>,
    /// 17. The time at which the earliest sent packet in this space will be considered lost based on
    /// exceeding the reordering window in time. Only set for packets numbered prior to a packet
    /// that has been acknowledged.
    pub(super) loss_time: Option<Instant>,
    /// 18. Number of explicit congestion notification codepoints seen on incoming packets
    pub(super) ecn_counters: frame::EcnCounts,
    /// 19. Highest received packet number
    pub(super) rx_packet: u64,
    /// 20.Incoming cryptographic handshake stream
    pub(super) crypto_stream: Assembler,
    /// 21
    pub(super) largest_acked_packet_sent: Instant,
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

            largest_ack_eliciting_sent: 0,
            unacked_non_ack_eliciting_tail: 0,
            sent_packets: BTreeMap::new(),
            in_flight: 0,
            time_of_last_ack_eliciting_packet: None,
            loss_time: None,
            ecn_counters: frame::EcnCounts::ZERO,

            rx_packet: 0,

            crypto_stream: Assembler::new(),
            largest_acked_packet_sent: now,
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
    /// Returns the number of bytes to *remove* from the connection's in-flight count
    pub(super) fn sent(&mut self, number: u64, packet: SentPacket) -> u64 {
        // Retain state for at most this many non-ACK-eliciting packets sent after the most recently
        // sent ACK-eliciting packet. We're never guaranteed to receive an ACK for those, and we
        // can't judge them as lost without an ACK, so to limit memory in applications which receive
        // packets but don't send ACK-eliciting data for long periods use we must eventually start
        // forgetting about them, although it might also be reasonable to just kill the connection
        // due to weird peer behavior.
        const MAX_UNACKED_NON_ACK_ELICTING_TAIL: u64 = 1_000;

        let mut forgotten_bytes = 0;
        if packet.ack_eliciting {
            self.unacked_non_ack_eliciting_tail = 0;
            self.largest_ack_eliciting_sent = number;
        } else if self.unacked_non_ack_eliciting_tail > MAX_UNACKED_NON_ACK_ELICTING_TAIL {
            let oldest_after_ack_eliciting = *self
                .sent_packets
                .range((
                    Bound::Excluded(self.largest_ack_eliciting_sent),
                    Bound::Unbounded,
                ))
                .next()
                .unwrap()
                .0;
            // Per https://www.rfc-editor.org/rfc/rfc9000.html#name-frames-and-frame-types,
            // non-ACK-eliciting packets must only contain PADDING, ACK, and CONNECTION_CLOSE
            // frames, which require no special handling on ACK or loss beyond removal from
            // in-flight counters if padded.
            let packet = self
                .sent_packets
                .remove(&oldest_after_ack_eliciting)
                .unwrap();
            forgotten_bytes = u64::from(packet.size);
            self.in_flight -= forgotten_bytes;
        } else {
            self.unacked_non_ack_eliciting_tail += 1;
        }

        self.in_flight += u64::from(packet.size);
        self.sent_packets.insert(number, packet);
        forgotten_bytes
    }

    /// Stop tracking sent packet `number`, and return what we knew about it
    pub(super) fn take(&mut self, number: u64) -> Option<SentPacket> {
        let packet = self.sent_packets.remove(&number)?;
        self.in_flight -= u64::from(packet.size);
        if !packet.ack_eliciting && number > self.largest_ack_eliciting_sent {
            self.unacked_non_ack_eliciting_tail =
                self.unacked_non_ack_eliciting_tail.checked_sub(1).unwrap();
        }
        Some(packet)
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
    /// 4.
    pub(super) handshake_done: bool,
    /// 5.
    pub(super) retire_cids: Vec<u64>,
    /// 6.
    pub(super) reset_stream: Vec<(StreamId, VarInt)>,
    /// 7.
    pub(super) stop_sending: Vec<frame::StopSending>,
    /// 8.
    pub(super) max_data: bool,
    /// 9.
    pub(super) max_stream_data: FxHashSet<StreamId>,
    /// 10.
    pub(super) max_stream_id: [bool; 2],
}

impl Retransmits {
    /// todo: change the fn
    pub(super) fn is_empty(&self, streams: &StreamsState) -> bool {
        // tracing::error!("to delete is_empty");
        self.crypto.is_empty()
            && self.new_cids.is_empty()
            && !self.ack_frequency
            && !self.handshake_done
            && self.retire_cids.is_empty()
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
    /// Most recently skipped packet number
    prev_skipped_packet_number: Option<u64>,
    /// Next packet number to skip is randomly selected from 2^n..2^n+1
    exponent: u32,
}

impl PacketNumberFilter {
    /// 1
    pub(super) fn new(rng: &mut (impl Rng + ?Sized)) -> Self {
        // First skipped PN is in 0..64
        let exponent = 6;
        Self {
            next_skipped_packet_number: rng.gen_range(0..2u64.saturating_pow(exponent)),
            prev_skipped_packet_number: None,
            exponent,
        }
    }
    /// 2
    #[cfg(test)]
    pub(super) fn disabled() -> Self {
        Self {
            next_skipped_packet_number: u64::MAX,
            prev_skipped_packet_number: None,
            exponent: u32::MAX,
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
        let n = space.get_tx_number();
        if n != self.next_skipped_packet_number {
            return n;
        }

        trace!("skipping pn {n}");
        // Skip this packet number, and choose the next one to skip
        self.prev_skipped_packet_number = Some(self.next_skipped_packet_number);
        let next_exponent = self.exponent.saturating_add(1);
        self.next_skipped_packet_number =
            rng.gen_range(2u64.saturating_pow(self.exponent)..2u64.saturating_pow(next_exponent));
        self.exponent = next_exponent;

        space.get_tx_number()
    }
    /// 5.
    pub(super) fn check_ack(
        &self,
        space_id: SpaceId,
        range: std::ops::RangeInclusive<u64>,
    ) -> Result<(), TransportError> {
        if space_id == SpaceId::Data
            && self
                .prev_skipped_packet_number
                .map_or(false, |x| range.contains(&x))
        {
            return Err(TransportError::PROTOCOL_VIOLATION("unsent packet acked"));
        }
        Ok(())
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
    /// 3.
    non_ack_eliciting_since_last_ack_sent: u64,
    /// 4. The packet with the largest packet number, and the time upon which it was received (used to
    /// calculate ACK delay in [`PendingAcks::ack_delay`])
    largest_packet: Option<(u64, Instant)>,
}

impl PendingAcks {
    /// 1.
    fn new() -> Self {
        Self {
            immediate_ack_required: false,
            ranges: ArrayRangeSet::default(),
            non_ack_eliciting_since_last_ack_sent: 0,
            largest_packet: None,
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
        // It is possible (though unlikely) that the ACKs we just sent do not cover all the
        // ACK-eliciting packets we have received (e.g. if there is not enough room in the packet to
        // fit all the ranges). To keep things simple, however, we assume they do. If there are
        // indeed some ACKs that weren't covered, the packets might be ACKed later anyway, because
        // they are still contained in `self.ranges`. If we somehow fail to send the ACKs at a later
        // moment, the peer will assume the packets got lost and will retransmit their frames in a
        // new packet, which is suboptimal, because we already received them. Our assumption here is
        // that simplicity results in code that is more performant, even in the presence of
        // occasional redundant retransmits.
        // todo
        self.immediate_ack_required = false;
        // self.ack_eliciting_since_last_ack_sent = 0;
        self.non_ack_eliciting_since_last_ack_sent = 0;
        // self.earliest_ack_eliciting_since_last_ack_sent = None;
        // self.largest_acked = self.largest_ack_eliciting_packet;
    }
    /// 5. Queue an ACK if a significant number of non-ACK-eliciting packets have not yet been
    /// acknowledged
    ///
    /// Should be called immediately before a non-probing packet is composed, when we've already
    /// committed to sending a packet regardless.
    pub(super) fn maybe_ack_non_eliciting(&mut self) {
        // If we're going to send a packet anyway, and we've received a significant number of
        // non-ACK-eliciting packets, then include an ACK to help the peer perform timely loss
        // detection even if they're not sending any ACK-eliciting packets themselves. Exact
        // threshold chosen somewhat arbitrarily.
        const LAZY_ACK_THRESHOLD: u64 = 10;
        if self.non_ack_eliciting_since_last_ack_sent > LAZY_ACK_THRESHOLD {
            self.immediate_ack_required = true;
        }
    }
    /// 6.
    pub(super) fn set_immediate_ack_required(&mut self) {
        self.immediate_ack_required = true;
    }
    /// 7. Insert one packet that needs to be acknowledged
    pub(super) fn insert_one(&mut self, packet: u64, now: Instant) {
        self.ranges.insert_one(packet);

        if self.largest_packet.map_or(true, |(pn, _)| packet > pn) {
            self.largest_packet = Some((packet, now));
        }

        if self.ranges.len() > MAX_ACK_BLOCKS {
            self.ranges.pop_min();
        }
    }
    /// 8. Returns the delay since the packet with the largest packet number was received
    pub(super) fn ack_delay(&self, now: Instant) -> Duration {
        self.largest_packet
            .map_or(Duration::default(), |(_, received)| now - received)
    }
    /// 9. Remove ACKs of packets numbered at or below `max` from the set of pending ACKs
    pub(super) fn subtract_below(&mut self, max: u64) {
        self.ranges.remove(0..(max + 1));
    }
}

/// A variant of `Retransmits` which only allocates storage when required
#[derive(Debug, Default, Clone)]
pub(super) struct ThinRetransmits {
    retransmits: Option<Box<Retransmits>>,
}

impl ThinRetransmits {
    /// 1. Returns a mutable reference to the stored retransmits
    ///
    /// This function will allocate a backing storage if required.
    pub(super) fn get_or_create(&mut self) -> &mut Retransmits {
        if self.retransmits.is_none() {
            self.retransmits = Some(Box::default());
        }
        self.retransmits.as_deref_mut().unwrap()
    }
    /// 2.Returns `true` if no retransmits are necessary
    pub(super) fn is_empty(&self, streams: &StreamsState) -> bool {
        match &self.retransmits {
            Some(retransmits) => retransmits.is_empty(streams),
            None => true,
        }
    }
    /// 3.Returns a reference to the retransmits stored in this box
    pub(super) fn get(&self) -> Option<&Retransmits> {
        self.retransmits.as_deref()
    }
}

/// Represents one or more packets subject to retransmission
#[derive(Debug, Clone)]
pub(super) struct SentPacket {
    /// 1. The number of bytes sent in the packet, not including UDP or IP overhead, but including QUIC
    /// framing overhead. Zero if this packet is not counted towards congestion control, i.e. not an
    /// "in flight" packet.
    pub(super) size: u16,
    /// 2. Whether an acknowledgement is expected directly in response to this packet.
    pub(super) ack_eliciting: bool,
    /// 3. The time the packet was sent.
    pub(super) time_sent: Instant,
    /// 4. The largest packet number acknowledged by this packet
    pub(super) largest_acked: Option<u64>,
    /// 5. Data which needs to be retransmitted in case the packet is lost.
    /// The data is boxed to minimize `SentPacket` size for the typical case of
    /// packets only containing ACKs and STREAM frames.
    pub(super) retransmits: ThinRetransmits,
    /// 6. Metadata for stream frames in a packet
    ///
    /// The actual application data is stored with the stream state.
    pub(super) stream_frames: frame::StreamMetaVec,
}

/// Ensures we can always fit all our ACKs in a single minimum-MTU packet with room to spare
const MAX_ACK_BLOCKS: usize = 64;
