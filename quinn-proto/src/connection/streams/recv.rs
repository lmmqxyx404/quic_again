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
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum RecvState {
    Recv { size: Option<u64> },
}


impl Default for RecvState {
  fn default() -> Self {
      Self::Recv { size: None }
  }
}