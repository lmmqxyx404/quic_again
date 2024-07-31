use bytes::Bytes;
use thiserror::Error;

use crate::{connection::send_buffer::SendBuffer, frame, VarInt};

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
    /// 7. The reason the peer wants us to stop, if `STOP_SENDING` was received
    pub(super) stop_reason: Option<VarInt>,
}

impl Send {
    /// 1.
    pub(super) fn is_writable(&self) -> bool {
        matches!(self.state, SendState::Ready)
    }
    /// 2.
    pub(super) fn offset(&self) -> u64 {
        self.pending.offset()
    }
    /// 3. Whether the stream has been reset
    pub(super) fn is_reset(&self) -> bool {
        matches!(self.state, SendState::ResetSent { .. })
    }
    /// 4.
    pub(super) fn is_pending(&self) -> bool {
        self.pending.has_unsent_data() || self.fin_pending
    }
    /// 5.
    pub(super) fn new(max_data: VarInt) -> Box<Self> {
        Box::new(Self {
            max_data: max_data.into(),
            state: SendState::Ready,
            pending: SendBuffer::new(),
            priority: 0,
            fin_pending: false,
            connection_blocked: false,
            stop_reason: None,
        })
    }
    /// 6.
    pub(super) fn write<S: BytesSource>(
        &mut self,
        source: &mut S,
        limit: u64,
    ) -> Result<Written, WriteError> {
        if !self.is_writable() {
            return Err(WriteError::ClosedStream);
        }
        if let Some(error_code) = self.stop_reason {
            return Err(WriteError::Stopped(error_code));
        }
        let budget = self.max_data - self.pending.offset();
        if budget == 0 {
            return Err(WriteError::Blocked);
        }
        let mut limit = limit.min(budget) as usize;

        let mut result = Written::default();
        loop {
            let (chunk, chunks_consumed) = source.pop_chunk(limit);
            result.chunks += chunks_consumed;
            result.bytes += chunk.len();

            if chunk.is_empty() {
                break;
            }

            limit -= chunk.len();
            self.pending.write(chunk);
        }

        Ok(result)
    }
    /// 7
    pub(super) fn finish(&mut self) -> Result<(), FinishError> {
        if let Some(error_code) = self.stop_reason {
            Err(FinishError::Stopped(error_code))
        } else if self.state == SendState::Ready {
            self.state = SendState::DataSent {
                finish_acked: false,
            };
            self.fin_pending = true;
            Ok(())
        } else {
            Err(FinishError::ClosedStream)
        }
    }
    /// 8.Returns whether the stream has been finished and all data has been acknowledged by the peer
    pub(super) fn ack(&mut self, frame: frame::StreamMeta) -> bool {
        self.pending.ack(frame.offsets);
        match self.state {
            SendState::DataSent {
                ref mut finish_acked,
            } => {
                *finish_acked |= frame.fin;
                *finish_acked && self.pending.is_fully_acked()
            }
            _ => false,
        }
    }
    /// 9. Update stream state due to a reset sent by the local application
    pub(super) fn reset(&mut self) {
        use SendState::*;
        if let DataSent { .. } | Ready = self.state {
            self.state = ResetSent;
        }
    }
    /// 10. Handle STOP_SENDING
    ///
    /// Returns true if the stream was stopped due to this frame, and false
    /// if it had been stopped before
    pub(super) fn try_stop(&mut self, error_code: VarInt) -> bool {
        if self.stop_reason.is_none() {
            self.stop_reason = Some(error_code);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum SendState {
    /// 1. Sent RESET
    ResetSent,
    /// 2. Sending new data
    Ready,
    /// 3. Stream was finished; now sending retransmits only
    DataSent { finish_acked: bool },
}

/// Errors triggered while writing to a send stream
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum WriteError {
    /// 1. The peer is not able to accept additional data, or the connection is congested.
    ///
    /// If the peer issues additional flow control credit, a [`StreamEvent::Writable`] event will
    /// be generated, indicating that retrying the write might succeed.
    ///
    /// [`StreamEvent::Writable`]: crate::StreamEvent::Writable
    #[error("unable to accept further writes")]
    Blocked,
    /// 2. The stream has not been opened or has already been finished or reset
    #[error("closed stream")]
    ClosedStream,
    /// 3. The peer is no longer accepting data on this stream, and it has been implicitly reset. The
    /// stream cannot be finished or further written to.
    ///
    /// Carries an application-defined error code.
    ///
    /// [`StreamEvent::Finished`]: crate::StreamEvent::Finished
    #[error("stopped by peer: code {0}")]
    Stopped(VarInt),
}

/// Reasons why attempting to finish a stream might fail
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FinishError {
    /// 1. The stream has not been opened or was already finished or reset
    #[error("closed stream")]
    ClosedStream,
    /// 2. The peer is no longer accepting data on this stream. No
    /// [`StreamEvent::Finished`] event will be emitted for this stream.
    ///
    /// Carries an application-defined error code.
    ///
    /// [`StreamEvent::Finished`]: crate::StreamEvent::Finished
    #[error("stopped by peer: code {0}")]
    Stopped(VarInt),
}

/// A source of one or more buffers which can be converted into `Bytes` buffers on demand
///
/// The purpose of this data type is to defer conversion as long as possible,
/// so that no heap allocation is required in case no data is writable.
pub trait BytesSource {
    /// Returns the next chunk from the source of owned chunks.
    ///
    /// This method will consume parts of the source.
    /// Calling it will yield `Bytes` elements up to the configured `limit`.
    ///
    /// The method returns a tuple:
    /// - The first item is the yielded `Bytes` element. The element will be
    ///   empty if the limit is zero or no more data is available.
    /// - The second item returns how many complete chunks inside the source had
    ///   had been consumed. This can be less than 1, if a chunk inside the
    ///   source had been truncated in order to adhere to the limit. It can also
    ///   be more than 1, if zero-length chunks had been skipped.
    fn pop_chunk(&mut self, limit: usize) -> (Bytes, usize);
}

/// A [`BytesSource`] implementation for `&[u8]`
///
/// The type allows to dequeue a single [`Bytes`] chunk, which will be lazily
/// created from a reference. This allows to defer the allocation until it is
/// known how much data needs to be copied.
pub(crate) struct ByteSlice<'a> {
    /// The wrapped byte slice
    data: &'a [u8],
}

impl<'a> ByteSlice<'a> {
    pub(crate) fn from_slice(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl<'a> BytesSource for ByteSlice<'a> {
    fn pop_chunk(&mut self, limit: usize) -> (Bytes, usize) {
        let limit = limit.min(self.data.len());
        if limit == 0 {
            return (Bytes::new(), 0);
        }

        let chunk = Bytes::from(self.data[..limit].to_owned());
        self.data = &self.data[chunk.len()..];

        let chunks_consumed = usize::from(self.data.is_empty());
        (chunk, chunks_consumed)
    }
}

/// Indicates how many bytes and chunks had been transferred in a write operation
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Written {
    /// 1. The amount of bytes which had been written
    pub bytes: usize,
    /// 2. The amount of full chunks which had been written
    ///
    /// If a chunk was only partially written, it will not be counted by this field.
    pub chunks: usize,
}
