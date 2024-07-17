use std::{cmp, time::Instant};

use bytes::Bytes;

use crate::{
    frame::{self, Close},
    packet::{PartialEncode, SpaceId},
    shared::ConnectionId,
    INITIAL_MTU,
};

use super::{Connection, SentFrames};

pub(super) struct PacketBuilder {
    /// 1. Smallest absolute position in the associated buffer that must be occupied by this packet's
    /// frames
    pub(super) min_size: usize,
    /// 2
    pub(super) tag_len: usize,
    /// 3.
    pub(super) partial_encode: PartialEncode,
    /// 4.
    pub(super) short_header: bool,
}

impl PacketBuilder {
    /// 1. Append the minimum amount of padding such that, after encryption, the packet will occupy at
    /// least `min_size` bytes
    pub(super) fn pad_to(&mut self, min_size: u16) {
        todo!()
    }
    /// 2.
    pub(super) fn finish_and_track(
        self,
        now: Instant,
        conn: &mut Connection,
        sent: Option<SentFrames>,
        buffer: &mut Vec<u8>,
    ) {
        todo!()
    }

    /// 3.Write a new packet header to `buffer` and determine the packet's properties
    ///
    /// Marks the connection drained and returns `None` if the confidentiality limit would be
    /// violated.
    pub(super) fn new(
        now: Instant,
        space_id: SpaceId,
        dst_cid: ConnectionId,
        buffer: &mut Vec<u8>,
        mut buffer_capacity: usize,
        datagram_start: usize,
        ack_eliciting: bool,
        conn: &mut Connection,
    ) -> Option<Self> {
        let version = conn.version;
        // Initiate key update if we're approaching the confidentiality limit
        let sent_with_keys = conn.spaces[space_id].sent_with_keys;
        if space_id == SpaceId::Data {
            if sent_with_keys >= conn.key_phase_size {
                conn.initiate_key_update();
            }
        } else {
            let confidentiality_limit = conn.spaces[space_id]
                .crypto
                .as_ref()
                .map_or_else(
                    || &conn.zero_rtt_crypto.as_ref().unwrap().packet,
                    |keys| &keys.packet.local,
                )
                .confidentiality_limit();
            if sent_with_keys.saturating_add(1) == confidentiality_limit {
                // We still have time to attempt a graceful close
                conn.close_inner(
                    now,
                    Close::Connection(frame::ConnectionClose {
                        // error_code: TransportErrorCode::AEAD_LIMIT_REACHED,
                        // frame_type: None,
                        reason: Bytes::from_static(b"confidentiality limit reached"),
                    }),
                )
            } else if sent_with_keys > confidentiality_limit {
                // Confidentiality limited violated and there's nothing we can do
                /*todo  conn.kill(
                    TransportError::AEAD_LIMIT_REACHED("confidentiality limit reached").into(),
                ); */
                return None;
            }
        }

        let space = &mut conn.spaces[space_id];

        if space.loss_probes != 0 {
            space.loss_probes -= 1;
            // Clamp the packet size to at most the minimum MTU to ensure that loss probes can get
            // through and enable recovery even if the path MTU has shrank unexpectedly.
            buffer_capacity = cmp::min(buffer_capacity, datagram_start + usize::from(INITIAL_MTU));
        }
        let exact_number = match space_id {
            SpaceId::Data => conn.packet_number_filter.allocate(&mut conn.rng, space),
            _ => space.get_tx_number(),
        };
        todo!()
    }
}
