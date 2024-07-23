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
    streams: BinaryHeap<PendingStream>,
}

impl PendingStreamsQueue {
    fn new() -> Self {
        Self {
            streams: BinaryHeap::new(),
            // recency: u64::MAX,
        }
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
