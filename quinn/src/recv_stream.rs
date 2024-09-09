use proto::{Chunk, Chunks, ClosedStream, ConnectionError, ReadableError, StreamId};

use crate::{connection::ConnectionRef, VarInt};

/// A stream that can only be used to receive data
///
/// `stop(0)` is implicitly called on drop unless:
/// - A variant of [`ReadError`] has been yielded by a read call
/// - [`stop()`] was called explicitly
///
/// # Cancellation
///
/// A `read` method is said to be *cancel-safe* when dropping its future before the future becomes
/// ready cannot lead to loss of stream data. This is true of methods which succeed immediately when
/// any progress is made, and is not true of methods which might need to perform multiple reads
/// internally before succeeding. Each `read` method documents whether it is cancel-safe.
///
/// # Common issues
///
/// ## Data never received on a locally-opened stream
///
/// Peers are not notified of streams until they or a later-numbered stream are used to send
/// data. If a bidirectional stream is locally opened but never used to send, then the peer may
/// never see it. Application protocols should always arrange for the endpoint which will first
/// transmit on a stream to be the endpoint responsible for opening it.
///
/// ## Data never received on a remotely-opened stream
///
/// Verify that the stream you are receiving is the same one that the server is sending on, e.g. by
/// logging the [`id`] of each. Streams are always accepted in the same order as they are created,
/// i.e. ascending order by [`StreamId`]. For example, even if a sender first transmits on
/// bidirectional stream 1, the first stream yielded by [`Connection::accept_bi`] on the receiver
/// will be bidirectional stream 0.
///
/// [`ReadError`]: crate::ReadError
/// [`stop()`]: RecvStream::stop
/// [`SendStream::finish`]: crate::SendStream::finish
/// [`WriteError::Stopped`]: crate::WriteError::Stopped
/// [`id`]: RecvStream::id
/// [`Connection::accept_bi`]: crate::Connection::accept_bi
#[derive(Debug)]
pub struct RecvStream {
   /*  conn: ConnectionRef,
    stream: StreamId,
    is_0rtt: bool,
    all_data_read: bool,
    reset: Option<VarInt>, */
}

impl RecvStream {
    pub(crate) fn new(conn: ConnectionRef, stream: StreamId, is_0rtt: bool) -> Self {
        Self {
            /* conn,
            stream,
            is_0rtt,
            all_data_read: false,
            reset: None, */
        }
    }
}
