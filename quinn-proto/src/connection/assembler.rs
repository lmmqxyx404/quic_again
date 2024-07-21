use bytes::Bytes;

/// Helper to assemble unordered stream frames into an ordered stream
#[derive(Debug, Default)]
pub(super) struct Assembler {
    /// 1. Number of bytes read by the application. When only ordered reads have been used, this is the
    /// length of the contiguous prefix of the stream which has been consumed by the application,
    /// aka the stream offset.
    bytes_read: u64,
    /// 2.
    end: u64,
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

    /// 3.Note: If a packet contains many frames from the same stream, the estimated over-allocation
    /// will be much higher because we are counting the same allocation multiple times.
    pub(super) fn insert(&mut self, mut offset: u64, mut bytes: Bytes, allocation_size: usize) {
        debug_assert!(
            bytes.len() <= allocation_size,
            "allocation_size less than bytes.len(): {:?} < {:?}",
            allocation_size,
            bytes.len()
        );
        self.end = self.end.max(offset + bytes.len() as u64);

        todo!()
    }
}
