use crate::{shared::ConnectionId, token::ResetToken};

/// DataType stored in CidQueue buffer
type CidData = (ConnectionId, Option<ResetToken>);

/// Sliding window of active Connection IDs
///
/// May contain gaps due to packet loss or reordering
#[derive(Debug)]
pub(crate) struct CidQueue {
    /// 1. Ring buffer indexed by `self.cursor`
    buffer: [Option<CidData>; Self::LEN],
    /// 2. Index at which circular buffer addressing is based
    cursor: usize,
}

impl CidQueue {
    pub(crate) const LEN: usize = 5;
    /// 1.
    pub(crate) fn new(cid: ConnectionId) -> Self {
        let mut buffer = [None; Self::LEN];
        buffer[0] = Some((cid, None));
        Self { buffer, cursor: 0 }
    }
    /// 2.Return active remote CID itself
    pub(crate) fn active(&self) -> ConnectionId {
        self.buffer[self.cursor].unwrap().0
    }
}
