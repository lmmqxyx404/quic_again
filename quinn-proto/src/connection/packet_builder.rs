use std::{cmp, time::Instant};

use bytes::Bytes;
use rand::Rng;
use tracing::trace_span;

use crate::{
    frame::{self, Close},
    packet::{Header, InitialHeader, PacketNumber, PartialEncode, SpaceId, FIXED_BIT},
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

        let span = trace_span!("send", space = ?space_id, pn = exact_number).entered();

        let number = PacketNumber::new(exact_number, space.largest_acked_packet.unwrap_or(0));

        let header = match space_id {
            SpaceId::Data if space.crypto.is_some() => Header::Short {
                dst_cid,
                number,
                spin: if conn.spin_enabled {
                    conn.spin
                } else {
                    conn.rng.gen()
                },
                key_phase: conn.key_phase,
            },
            SpaceId::Data => {
                todo!()
            }
            SpaceId::Handshake => {
                todo!()
            }
            SpaceId::Initial => Header::Initial(InitialHeader {
                src_cid: conn.handshake_cid,
                dst_cid,
                token: conn.retry_token.clone(),
                number,
                version,
            }),
        };
        let partial_encode = header.encode(buffer);
        if conn.peer_params.grease_quic_bit && conn.rng.gen() {
            buffer[partial_encode.start] ^= FIXED_BIT;
        }

        let (sample_size, tag_len) = if let Some(ref crypto) = space.crypto {
            (
                crypto.header.local.sample_size(),
                crypto.packet.local.tag_len(),
            )
        } else if space_id == SpaceId::Data {
            let zero_rtt = conn.zero_rtt_crypto.as_ref().unwrap();
            (zero_rtt.header.sample_size(), zero_rtt.packet.tag_len())
        } else {
            unreachable!("tried to send {:?} packet without keys", space_id);
        };

        // Each packet must be large enough for header protection sampling, i.e. the combined
        // lengths of the encoded packet number and protected payload must be at least 4 bytes
        // longer than the sample required for header protection. Further, each packet should be at
        // least tag_len + 6 bytes larger than the destination CID on incoming packets so that the
        // peer may send stateless resets that are indistinguishable from regular traffic.

        // pn_len + payload_len + tag_len >= sample_size + 4
        // payload_len >= sample_size + 4 - pn_len - tag_len
        let min_size = Ord::max(
            buffer.len() + (sample_size + 4).saturating_sub(number.len() + tag_len),
            partial_encode.start + dst_cid.len() + 6,
        );
        let max_size = buffer_capacity - tag_len;

        Some(Self {
            min_size,
            tag_len,
            partial_encode,
            short_header: header.is_short(),
        })
    }
}
