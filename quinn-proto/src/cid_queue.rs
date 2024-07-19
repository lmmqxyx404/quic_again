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
    /// 3. Sequence number of `self.buffer[cursor]`
    ///
    /// The sequence number of the active CID; must be the smallest among CIDs in `buffer`.
    offset: u64,
}

impl CidQueue {
    pub(crate) const LEN: usize = 5;
    /// 1.
    pub(crate) fn new(cid: ConnectionId) -> Self {
        let mut buffer = [None; Self::LEN];
        buffer[0] = Some((cid, None));
        Self {
            buffer,
            cursor: 0,
            offset: 0,
        }
    }
    /// 2.Return active remote CID itself
    pub(crate) fn active(&self) -> ConnectionId {
        self.buffer[self.cursor].unwrap().0
    }
    /// 3.Replace the initial CID
    pub(crate) fn update_initial_cid(&mut self, cid: ConnectionId) {
        debug_assert_eq!(self.offset, 0);
        self.buffer[self.cursor] = Some((cid, None));
    }
}
