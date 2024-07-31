use tracing::debug;

use crate::{frame, TransportError, VarInt};

#[derive(Debug, Default)]
pub(super) struct Recv {
    /// 1
    state: RecvState,
    /// 2
    pub(super) stopped: bool,
}

impl Recv {
    /// 1. Whether stream-level flow control updates should be sent for this stream
    pub(super) fn can_send_flow_control(&self) -> bool {
        // Stream-level flow control is redundant if the sender has already sent the whole stream,
        // and moot if we no longer want data on this stream.
        self.final_offset_unknown() && !self.stopped
    }

    /// 2. Whether the total amount of data that the peer will send on this stream is unknown
    ///
    /// True until we've received either a reset or the final frame.
    ///
    /// Implies that the sender might benefit from stream-level flow control updates, and we might
    /// need to issue connection-level flow control updates due to flow control budget use by this
    /// stream in the future, even if it's been stopped.
    pub(super) fn final_offset_unknown(&self) -> bool {
        matches!(self.state, RecvState::Recv { size: None })
    }
    /// 3
    pub(super) fn new(initial_max_data: u64) -> Box<Self> {
        Box::new(Self {
            state: RecvState::default(),
            /* assembler: Assembler::new(),
            sent_max_stream_data: initial_max_data,
            end: 0, */
            stopped: false,
        })
    }
    /// 4. Whether data is still being accepted from the peer
    pub(super) fn is_receiving(&self) -> bool {
        matches!(self.state, RecvState::Recv { .. })
    }
    /// Process a STREAM frame
    ///
    /// Return value is `(number_of_new_bytes_ingested, stream_is_closed)`
    pub(super) fn ingest(
        &mut self,
        frame: frame::Stream,
        payload_len: usize,
        received: u64,
        max_data: u64,
    ) -> Result<(u64, bool), TransportError> {
        let end = frame.offset + frame.data.len() as u64;
        if end >= 2u64.pow(62) {
            return Err(TransportError::FLOW_CONTROL_ERROR(
                "maximum stream offset too large",
            ));
        }

        if let Some(final_offset) = self.final_offset() {
            if end > final_offset || (frame.fin && end != final_offset) {
                debug!(end, final_offset, "final size error");
                return Err(TransportError::FINAL_SIZE_ERROR(""));
            }
        }
        todo!()
    }

    fn final_offset(&self) -> Option<u64> {
        match self.state {
            RecvState::Recv { size } => size,
            RecvState::ResetRecvd { size, .. } => Some(size),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum RecvState {
    /// 1
    Recv { size: Option<u64> },
    /// 2
    ResetRecvd { size: u64, error_code: VarInt },
}

impl Default for RecvState {
    fn default() -> Self {
        Self::Recv { size: None }
    }
}
