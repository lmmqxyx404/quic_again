use bytes::{Buf, Bytes};

use crate::range_set::RangeSet;

/// Helper to assemble unordered stream frames into an ordered stream
#[derive(Debug, Default)]
pub(super) struct Assembler {
    /// 1. Number of bytes read by the application. When only ordered reads have been used, this is the
    /// length of the contiguous prefix of the stream which has been consumed by the application,
    /// aka the stream offset.
    bytes_read: u64,
    /// 2.
    end: u64,
    /// 3.
    state: State,
}

impl Assembler {
    /// 1.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 2. Number of bytes consumed by the application
    pub(super) fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
    // Note: If a packet contains many frames from the same stream, the estimated over-allocation
    // will be much higher because we are counting the same allocation multiple times.
    pub(super) fn insert(&mut self, mut offset: u64, mut bytes: Bytes, allocation_size: usize) {
        debug_assert!(
            bytes.len() <= allocation_size,
            "allocation_size less than bytes.len(): {:?} < {:?}",
            allocation_size,
            bytes.len()
        );
        self.end = self.end.max(offset + bytes.len() as u64);
        if let State::Unordered { ref mut recvd } = self.state {
            todo!()
        } else if offset < self.bytes_read {
            if (offset + bytes.len() as u64) <= self.bytes_read {
                return;
            } else {
                let diff = self.bytes_read - offset;
                offset += diff;
                bytes.advance(diff as usize);
            }
        }

        if bytes.is_empty() {
            return;
        }

        todo!()
    }
}

#[derive(Debug)]
enum State {
    Ordered,
    Unordered {
        /// The set of offsets that have been received from the peer, including portions not yet
        /// read by the application.
        recvd: RangeSet,
    },
}

impl Default for State {
    fn default() -> Self {
        Self::Ordered
    }
}
