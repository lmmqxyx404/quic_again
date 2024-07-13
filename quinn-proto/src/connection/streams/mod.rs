mod state;
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
