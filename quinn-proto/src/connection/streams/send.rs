use thiserror::Error;

use crate::connection::send_buffer::SendBuffer;

#[derive(Debug)]
pub(super) struct Send {
    /// 1. Whether this stream is in the `connection_blocked` list of `Streams`
    pub(super) connection_blocked: bool,
    /// 2.
    pub(super) max_data: u64,
    /// 3.
    pub(super) state: SendState,
    /// 4.
    pub(super) pending: SendBuffer,
    /// 5.
    pub(super) priority: i32,
    /// 6.
    pub(super) fin_pending: bool,
}

impl Send {
    /// 1.
    pub(super) fn is_writable(&self) -> bool {
        todo!()
        // matches!(self.state, SendState::Ready)
    }
    /// 2.
    pub(super) fn offset(&self) -> u64 {
        todo!()
        //   self.pending.offset()
    }
    /// 3. Whether the stream has been reset
    pub(super) fn is_reset(&self) -> bool {
        matches!(self.state, SendState::ResetSent { .. })
    }
    /// 4.
    pub(super) fn is_pending(&self) -> bool {
        self.pending.has_unsent_data() || self.fin_pending
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum SendState {
    /// Sent RESET
    ResetSent,
}

/// Errors triggered while writing to a send stream
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum WriteError {}

/// Reasons why attempting to finish a stream might fail
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FinishError {}
