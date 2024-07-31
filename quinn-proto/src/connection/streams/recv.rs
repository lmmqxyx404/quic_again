use std::collections::hash_map::Entry;

use bytes::Bytes;
use thiserror::Error;
use tracing::debug;

use crate::{
    connection::{
        assembler::{Assembler, IllegalOrderedRead},
        spaces::Retransmits,
        streams::{state::get_or_insert_recv, StreamHalf},
        Chunk,
    },
    frame, StreamId, TransportError, VarInt,
};

use super::StreamsState;

#[derive(Debug, Default)]
pub(super) struct Recv {
    /// 1
    state: RecvState,
    /// 2
    pub(super) stopped: bool,
    /// 3.
    pub(super) end: u64,
    /// 4.
    pub(super) assembler: Assembler,
    /// 5.
    sent_max_stream_data: u64,
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
            assembler: Assembler::new(),
            sent_max_stream_data: initial_max_data,
            end: 0,
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

        let new_bytes = self.credit_consumed_by(end, received, max_data)?;

        // Stopped streams don't need to wait for the actual data, they just need to know
        // how much there was.
        if frame.fin && !self.stopped {
            if let RecvState::Recv { ref mut size } = self.state {
                *size = Some(end);
            }
        }

        self.end = self.end.max(end);
        // Don't bother storing data or releasing stream-level flow control credit if the stream's
        // already stopped
        if !self.stopped {
            self.assembler.insert(frame.offset, frame.data, payload_len);
        }

        Ok((new_bytes, frame.fin && self.stopped))
    }

    fn final_offset(&self) -> Option<u64> {
        match self.state {
            RecvState::Recv { size } => size,
            RecvState::ResetRecvd { size, .. } => Some(size),
        }
    }

    /// Compute the amount of flow control credit consumed, or return an error if more was consumed
    /// than issued
    fn credit_consumed_by(
        &self,
        offset: u64,
        received: u64,
        max_data: u64,
    ) -> Result<u64, TransportError> {
        let prev_end = self.end;
        let new_bytes = offset.saturating_sub(prev_end);
        if offset > self.sent_max_stream_data || received + new_bytes > max_data {
            debug!(
                received,
                new_bytes,
                max_data,
                offset,
                stream_max_data = self.sent_max_stream_data,
                "flow control error"
            );
            return Err(TransportError::FLOW_CONTROL_ERROR(""));
        }

        Ok(new_bytes)
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

/// Chunks
pub struct Chunks<'a> {
    /// 1
    pending: &'a mut Retransmits,
    /// 2
    state: ChunksState,
    /// 3
    ordered: bool,
    /// 4
    read: u64,
    /// 5.
    streams: &'a mut StreamsState,
    /// 6.
    id: StreamId,
}

impl<'a> Chunks<'a> {
    pub(super) fn new(
        id: StreamId,
        ordered: bool,
        streams: &'a mut StreamsState,
        pending: &'a mut Retransmits,
    ) -> Result<Self, ReadableError> {
        let mut entry = match streams.recv.entry(id) {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(_) => return Err(ReadableError::ClosedStream),
        };

        let mut recv =
            match get_or_insert_recv(streams.stream_receive_window)(entry.get_mut()).stopped {
                true => return Err(ReadableError::ClosedStream),
                false => entry.remove().unwrap(), // this can't fail due to the previous get_or_insert_with
            };

        recv.assembler.ensure_ordering(ordered)?;
        Ok(Self {
            id,
            ordered,
            streams,
            pending,
            state: ChunksState::Readable(recv),
            read: 0,
        })
    }
    /// Next
    ///
    /// Should call finalize() when done calling this.
    pub fn next(&mut self, max_length: usize) -> Result<Option<Chunk>, ReadError> {
        let rs = match self.state {
            ChunksState::Readable(ref mut rs) => rs,
            ChunksState::Reset(error_code) => {
                return Err(ReadError::Reset(error_code));
            }
            ChunksState::Finished => {
                return Ok(None);
            }
            ChunksState::Finalized => panic!("must not call next() after finalize()"),
        };
        if let Some(chunk) = rs.assembler.read(max_length, self.ordered) {
            self.read += chunk.bytes.len() as u64;
            return Ok(Some(chunk));
        }

        match rs.state {
            RecvState::ResetRecvd { error_code, .. } => {
                debug_assert_eq!(self.read, 0, "reset streams have empty buffers");
                self.streams.stream_freed(self.id, StreamHalf::Recv);
                self.state = ChunksState::Reset(error_code);
                Err(ReadError::Reset(error_code))
            }
            RecvState::Recv { size } => {
                if size == Some(rs.end) && rs.assembler.bytes_read() == rs.end {
                    self.streams.stream_freed(self.id, StreamHalf::Recv);
                    self.state = ChunksState::Finished;
                    Ok(None)
                } else {
                    // We don't need a distinct `ChunksState` variant for a blocked stream because
                    // retrying a read harmlessly re-traces our steps back to returning
                    // `Err(Blocked)` again. The buffers can't refill and the stream's own state
                    // can't change so long as this `Chunks` exists.
                    Err(ReadError::Blocked)
                }
            }
        }
    }
}

/// Errors triggered when opening a recv stream for reading
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ReadableError {
    /// 1.The stream has not been opened or was already stopped, finished, or reset
    #[error("closed stream")]
    ClosedStream,
    /// 2.Attempted an ordered read following an unordered read
    ///
    /// Performing an unordered read allows discontinuities to arise in the receive buffer of a
    /// stream which cannot be recovered, making further ordered reads impossible.
    #[error("ordered read after unordered read")]
    IllegalOrderedRead,
}

impl From<IllegalOrderedRead> for ReadableError {
    fn from(_: IllegalOrderedRead) -> Self {
        Self::IllegalOrderedRead
    }
}

/// Errors triggered when reading from a recv stream
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ReadError {
    /// 1. The peer abandoned transmitting data on this stream.
    ///
    /// Carries an application-defined error code.
    #[error("reset by peer: code {0}")]
    Reset(VarInt),
    /// 2. No more data is currently available on this stream.
    ///
    /// If more data on this stream is received from the peer, an `Event::StreamReadable` will be
    /// generated for this stream, indicating that retrying the read might succeed.
    #[error("blocked")]
    Blocked,
}

enum ChunksState {
    Readable(Box<Recv>),
    Reset(VarInt),
    Finished,
    Finalized,
}
