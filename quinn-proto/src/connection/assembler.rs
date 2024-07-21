/// Helper to assemble unordered stream frames into an ordered stream
#[derive(Debug, Default)]
pub(super) struct Assembler {
    /// 1. Number of bytes read by the application. When only ordered reads have been used, this is the
    /// length of the contiguous prefix of the stream which has been consumed by the application,
    /// aka the stream offset.
    bytes_read: u64,
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
}
