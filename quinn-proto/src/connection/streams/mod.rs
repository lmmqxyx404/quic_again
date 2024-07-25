mod state;
use std::collections::BinaryHeap;

#[allow(unreachable_pub)] // fuzzing only
pub use state::StreamsState;

use crate::{Dir, StreamId};

mod send;

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
