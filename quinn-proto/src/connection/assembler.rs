use std::{cmp::Ordering, collections::BinaryHeap};

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
    /// 4. Total number of buffered bytes, including duplicates in ordered mode.
    buffered: usize,
    /// 5. Estimated number of allocated bytes, will never be less than `buffered`.
    allocated: usize,
    /// 6. pay attention to use `BinaryHeap` that required the inner to implement `Ord` trait.
    data: BinaryHeap<Buffer>,
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
    /// 3. Note: If a packet contains many frames from the same stream, the estimated over-allocation
    /// will be much higher because we are counting the same allocation multiple times.
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

        let buffer = Buffer::new(offset, bytes, allocation_size);
        self.buffered += buffer.bytes.len();
        self.allocated += buffer.allocation_size;
        self.data.push(buffer);
        // `self.buffered` also counts duplicate bytes, therefore we use
        // `self.end - self.bytes_read` as an upper bound of buffered unique
        // bytes. This will cause a defragmentation if the amount of duplicate
        // bytes exceedes a proportion of the receive window size.
        let buffered = self.buffered.min((self.end - self.bytes_read) as usize);
        let over_allocation = self.allocated - buffered;
        // Rationale: on the one hand, we want to defragment rarely, ideally never
        // in non-pathological scenarios. However, a pathological or malicious
        // peer could send us one-byte frames, and since we use reference-counted
        // buffers in order to prevent copying, this could result in keeping a lot
        // of memory allocated. This limits over-allocation in proportion to the
        // buffered data. The constants are chosen somewhat arbitrarily and try to
        // balance between defragmentation overhead and over-allocation.
        let threshold = 32768.max(buffered * 3 / 2);
        if over_allocation > threshold {
            self.defragment()
        }
    }
    /// 4. Copy fragmented chunk data to new chunks backed by a single buffer
    ///
    /// This makes sure we're not unnecessarily holding on to many larger allocations.
    /// We merge contiguous chunks in the process of doing so.
    fn defragment(&mut self) {
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

#[derive(Debug, Eq)]
struct Buffer {
    offset: u64,
    bytes: Bytes,
    /// Size of the allocation behind `bytes`, if `defragmented == false`.
    /// Otherwise this will be set to `bytes.len()` by `try_mark_defragment`.
    /// Will never be less than `bytes.len()`.
    allocation_size: usize,
}
impl Buffer {
    /// Constructs a new fragmented Buffer
    fn new(offset: u64, bytes: Bytes, allocation_size: usize) -> Self {
        Self {
            offset,
            bytes,
            allocation_size,
        }
    }
}

impl Ord for Buffer {
    // Invert ordering based on offset (max-heap, min offset first),
    // prioritize longer chunks at the same offset.
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset
            .cmp(&other.offset)
            .reverse()
            .then(self.bytes.len().cmp(&other.bytes.len()))
    }
}

impl PartialOrd for Buffer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        (self.offset, self.bytes.len()) == (other.offset, other.bytes.len())
    }
}
