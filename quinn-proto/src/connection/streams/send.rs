#[derive(Debug)]
pub(super) struct Send {
    /// 1. Whether this stream is in the `connection_blocked` list of `Streams`
    pub(super) connection_blocked: bool,
    /// 2.
    pub(super) max_data: u64,
    /// 3.
    pub(super) state: SendState,
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
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum SendState {
    /// Sent RESET
    ResetSent,
}
