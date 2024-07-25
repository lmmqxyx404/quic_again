use crate::range_set::RangeSet;

/// Buffer of outgoing retransmittable stream data
#[derive(Default, Debug)]
pub(super) struct SendBuffer {
    /// 1. Total size of `unacked_segments`
    unacked_len: usize,
    /// 2. The first offset that hasn't been written by the application, i.e. the offset past the end of `unacked`
    offset: u64,
    /// 3. The first offset that hasn't been sent
    ///
    /// Always lies in (offset - unacked.len())..offset
    unsent: u64,
    /// 4. Previously transmitted ranges deemed lost
    retransmits: RangeSet,
}

impl SendBuffer {
    /// 1. Whether all sent data has been acknowledged
    pub(super) fn is_fully_acked(&self) -> bool {
        self.unacked_len == 0
    }
    /// 2
    pub(super) fn retransmit_all_for_0rtt(&mut self) {
        debug_assert_eq!(self.offset, self.unacked_len as u64);
        self.unsent = 0;
    }
    /// 3. Whether there's data to send
    ///
    /// There may be sent unacknowledged data even when this is false.
    pub(super) fn has_unsent_data(&self) -> bool {
        self.unsent != self.offset || !self.retransmits.is_empty()
    }
}