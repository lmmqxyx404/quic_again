use std::time::Instant;

use crate::packet::PartialEncode;

use super::{Connection, SentFrames};

pub(super) struct PacketBuilder {
    /// 1. Smallest absolute position in the associated buffer that must be occupied by this packet's
    /// frames
    pub(super) min_size: usize,
    /// 2
    pub(super) tag_len: usize,
    /// 3.
    pub(super) partial_encode: PartialEncode,
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
}
