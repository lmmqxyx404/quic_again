use std::ops::Range;

use crate::{frame::NewConnectionId, shared::ConnectionId, token::ResetToken};

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
    /// 4. Handle a `NEW_CONNECTION_ID` frame
    ///
    /// Returns a non-empty range of retired sequence numbers and the reset token of the new active
    /// CID iff any CIDs were retired.
    pub(crate) fn insert(
        &mut self,
        cid: NewConnectionId,
    ) -> Result<Option<(Range<u64>, ResetToken)>, InsertError> {
        // Position of new CID wrt. the current active CID
        let index = match cid.sequence.checked_sub(self.offset) {
            None => return Err(InsertError::Retired),
            Some(x) => x,
        };

        let retired_count = cid.retire_prior_to.saturating_sub(self.offset);
        if index >= Self::LEN as u64 + retired_count {
            return Err(InsertError::ExceedsLimit);
        }

        // Discard retired CIDs, if any
        for i in 0..(retired_count.min(Self::LEN as u64) as usize) {
            self.buffer[(self.cursor + i) % Self::LEN] = None;
        }

        // Record the new CID
        let index = ((self.cursor as u64 + index) % Self::LEN as u64) as usize;
        self.buffer[index] = Some((cid.id, Some(cid.reset_token)));

        if retired_count == 0 {
            tracing::debug!("returned CidQueue::insert Ok(None)");
            return Ok(None);
        }
        todo!()
    }
    /// 5. Return the sequence number of active remote CID
    pub(crate) fn active_seq(&self) -> u64 {
        self.offset
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum InsertError {
    /// 1. CID was already retired
    Retired,
    /// 2. Sequence number violates the leading edge of the window
    ExceedsLimit,
}
