use crate::shared::ConnectionId;

/// Sliding window of active Connection IDs
///
/// May contain gaps due to packet loss or reordering
#[derive(Debug)]
pub(crate) struct CidQueue {}

impl CidQueue {
    pub(crate) fn new(cid: ConnectionId) -> Self {
        Self {}
    }

    /// Return active remote CID itself
    pub(crate) fn active(&self) -> ConnectionId {
        todo!()
        // self.buffer[self.cursor].unwrap().0
    }
}
