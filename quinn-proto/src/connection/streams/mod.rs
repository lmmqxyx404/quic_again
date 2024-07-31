mod state;
use std::collections::BinaryHeap;

use send::{ByteSlice, BytesSource, FinishError, WriteError, Written};
use state::get_or_insert_send;
#[allow(unreachable_pub)] // fuzzing only
pub use state::StreamsState;
use tracing::trace;

use crate::{Dir, StreamId};

mod send;

/// 3
mod recv;
use recv::Recv;

use super::spaces::Retransmits;
/// Application events about streams
#[derive(Debug, PartialEq, Eq)]
pub enum StreamEvent {
    /// 1. One or more new streams has been opened and might be readable
    Opened {
        /// Directionality for which streams have been opened
        dir: Dir,
    },
    /// 2. A formerly write-blocked stream might be ready for a write or have been stopped
    ///
    /// Only generated for streams that are currently open.
    Writable {
        /// Which stream is now writable
        id: StreamId,
    },
}

/// A queue of streams with pending outgoing data, sorted by priority
struct PendingStreamsQueue {
    /// 1.
    streams: BinaryHeap<PendingStream>,
    /// 2. A monotonically decreasing counter, used to implement round-robin scheduling for streams of the same priority.
    /// Underflowing is not a practical concern, as it is initialized to u64::MAX and only decremented by 1 in `push_pending`
    recency: u64,
}

impl PendingStreamsQueue {
    /// 1.
    fn new() -> Self {
        Self {
            streams: BinaryHeap::new(),
            recency: u64::MAX,
        }
    }
    /// 2. Push a pending stream ID with the given priority, queued after any already-queued streams for the priority
    fn push_pending(&mut self, id: StreamId, priority: i32) {
        // As the recency counter is monotonically decreasing, we know that using its value to sort this stream will queue it
        // after all other queued streams of the same priority.
        // This is enough to implement round-robin scheduling for streams that are still pending even after being handled,
        // as in that case they are removed from the `BinaryHeap`, handled, and then immediately reinserted.
        self.recency -= 1;
        self.streams.push(PendingStream {
            // priority,
            // recency: self.recency,
            id,
        });
    }
}

/// The [`StreamId`] of a stream with pending data queued, ordered by its priority and recency
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct PendingStream {
    /// The ID of the stream
    // The way this type is used ensures that every instance has a unique `recency` value, so this field should be kept below
    // the `priority` and `recency` fields, so that it does not interfere with the behaviour of the `Ord` derive
    id: StreamId,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum StreamHalf {
    Send,
    Recv,
}

/// Indicates whether a frame needs to be transmitted
///
/// This type wraps around bool and uses the `#[must_use]` attribute in order
/// to prevent accidental loss of the frame transmission requirement.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[must_use = "A frame might need to be enqueued"]
pub struct ShouldTransmit(bool);

impl ShouldTransmit {
    /// Returns whether a frame should be transmitted
    pub fn should_transmit(self) -> bool {
        self.0
    }
}

/// Access to streams
pub struct Streams<'a> {
    pub(super) state: &'a mut StreamsState,
    pub(super) conn_state: &'a super::State,
}

impl<'a> Streams<'a> {
    /// 1. Open a single stream if possible
    ///
    /// Returns `None` if the streams in the given direction are currently exhausted.
    pub fn open(&mut self, dir: Dir) -> Option<StreamId> {
        if self.conn_state.is_closed() {
            return None;
        }
        // TODO: Queue STREAM_ID_BLOCKED if this fails
        if self.state.next[dir as usize] >= self.state.max[dir as usize] {
            return None;
        }

        self.state.next[dir as usize] += 1;
        let id = StreamId::new(self.state.side, dir, self.state.next[dir as usize] - 1);
        self.state.insert(false, id);
        self.state.send_streams += 1;
        Some(id)
    }
    /// 2. The number of streams that may have unacknowledged data.
    pub fn send_streams(&self) -> usize {
        self.state.send_streams
    }
}

/// Access to streams
pub struct SendStream<'a> {
    /// 3
    pub(super) id: StreamId,
    /// 1
    pub(super) state: &'a mut StreamsState,
    /// 4
    pub(super) pending: &'a mut Retransmits,
    /// 2
    pub(super) conn_state: &'a super::State,
}

impl<'a> SendStream<'a> {
    /// 1. Send data on the given stream
    ///
    /// Returns the number of bytes successfully written.
    pub fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        Ok(self.write_source(&mut ByteSlice::from_slice(data))?.bytes)
    }
    /// 2. Finish a send stream, signalling that no more data will be sent.
    ///
    /// If this fails, no [`StreamEvent::Finished`] will be generated.
    ///
    /// [`StreamEvent::Finished`]: crate::StreamEvent::Finished
    pub fn finish(&mut self) -> Result<(), FinishError> {
        let max_send_data = self.state.max_send_data(self.id);
        let stream = self
            .state
            .send
            .get_mut(&self.id)
            .map(get_or_insert_send(max_send_data))
            .ok_or(FinishError::ClosedStream)?;

        let was_pending = stream.is_pending();
        stream.finish()?;
        if !was_pending {
            self.state.pending.push_pending(self.id, stream.priority);
        }

        Ok(())
    }
    /// 3.
    fn write_source<B: BytesSource>(&mut self, source: &mut B) -> Result<Written, WriteError> {
        if self.conn_state.is_closed() {
            trace!(%self.id, "write blocked; connection draining");
            return Err(WriteError::Blocked);
        }

        let limit = self.state.write_limit();

        let max_send_data = self.state.max_send_data(self.id);
        let stream = self
            .state
            .send
            .get_mut(&self.id)
            .map(get_or_insert_send(max_send_data))
            .ok_or(WriteError::ClosedStream)?;

        if limit == 0 {
            trace!(
                stream = %self.id, max_data = self.state.max_data, data_sent = self.state.data_sent,
                "write blocked by connection-level flow control or send window"
            );
            if !stream.connection_blocked {
                stream.connection_blocked = true;
                self.state.connection_blocked.push(self.id);
            }
            return Err(WriteError::Blocked);
        }
        let was_pending = stream.is_pending();
        let written = stream.write(source, limit)?;
        self.state.data_sent += written.bytes as u64;
        self.state.unacked_data += written.bytes as u64;
        trace!(stream = %self.id, "wrote {} bytes", written.bytes);
        if !was_pending {
            self.state.pending.push_pending(self.id, stream.priority);
        }
        Ok(written)
    }
}
