use std::{
    future::{poll_fn, Future},
    io,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use proto::{Chunk, Chunks, ClosedStream, ConnectionError, ReadableError, StreamId};
use thiserror::Error;

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
    conn: ConnectionRef,
    stream: StreamId,
    is_0rtt: bool,
    all_data_read: bool,
    reset: Option<VarInt>,
}

impl RecvStream {
    pub(crate) fn new(conn: ConnectionRef, stream: StreamId, is_0rtt: bool) -> Self {
        Self {
            conn,
            stream,
            is_0rtt,
            all_data_read: false,
            reset: None,
        }
    }

    /// Convenience method to read all remaining data into a buffer
    ///
    /// Fails with [`ReadToEndError::TooLong`] on reading more than `size_limit` bytes, discarding
    /// all data read. Uses unordered reads to be more efficient than using `AsyncRead` would
    /// allow. `size_limit` should be set to limit worst-case memory use.
    ///
    /// If unordered reads have already been made, the resulting buffer may have gaps containing
    /// arbitrary data.
    ///
    /// This operation is *not* cancel-safe.
    ///
    /// [`ReadToEndError::TooLong`]: crate::ReadToEndError::TooLong
    pub async fn read_to_end(&mut self, size_limit: usize) -> Result<Vec<u8>, ReadToEndError> {
        ReadToEnd {
            stream: self,
            size_limit,
            read: Vec::new(),
            start: u64::MAX,
            end: 0,
        }
        .await
    }

    /// Attempts to read a chunk from the stream.
    ///
    /// On success, returns `Poll::Ready(Ok(Some(chunk)))`. If `Poll::Ready(Ok(None))`
    /// is returned, it implies that EOF has been reached.
    ///
    /// If no data is available for reading, the method returns `Poll::Pending`
    /// and arranges for the current task (via cx.waker()) to receive a notification
    /// when the stream becomes readable or is closed.
    fn poll_read_chunk(
        &mut self,
        cx: &mut Context,
        max_length: usize,
        ordered: bool,
    ) -> Poll<Result<Option<Chunk>, ReadError>> {
        self.poll_read_generic(cx, ordered, |chunks| match chunks.next(max_length) {
            Ok(Some(chunk)) => ReadStatus::Readable(chunk),
            res => (None, res.err()).into(),
        })
    }

    /// Handle common logic related to reading out of a receive stream
    ///
    /// This takes an `FnMut` closure that takes care of the actual reading process, matching
    /// the detailed read semantics for the calling function with a particular return type.
    /// The closure can read from the passed `&mut Chunks` and has to return the status after
    /// reading: the amount of data read, and the status after the final read call.
    fn poll_read_generic<T, U>(
        &mut self,
        cx: &mut Context,
        ordered: bool,
        mut read_fn: T,
    ) -> Poll<Result<Option<U>, ReadError>>
    where
        T: FnMut(&mut Chunks) -> ReadStatus<U>,
    {
        use proto::ReadError::*;
        if self.all_data_read {
            todo!() // return Poll::Ready(Ok(None));
        }
        let mut conn = self.conn.state.lock("RecvStream::poll_read");
        if self.is_0rtt {
            todo!() // conn.check_0rtt().map_err(|()| ReadError::ZeroRttRejected)?;
        }
        // If we stored an error during a previous call, return it now. This can happen if a
        // `read_fn` both wants to return data and also returns an error in its final stream status.
        let status: ReadStatus<U> = match self.reset {
            Some(code) => {
                todo!() // ReadStatus::Failed(None, Reset(code)),
            }
            None => {
                let mut recv = conn.inner.recv_stream(self.stream);
                let mut chunks = recv.read(ordered)?;
                let status = read_fn(&mut chunks);
                if chunks.finalize().should_transmit() {
                    todo!() // conn.wake();
                }
                status
            }
        };
        match status {
            ReadStatus::Readable(read) => Poll::Ready(Ok(Some(read))),
            ReadStatus::Finished(read) => {
                todo!()
                /* self.all_data_read = true;
                Poll::Ready(Ok(read)) */
            }
            ReadStatus::Failed(read, Blocked) => match read {
                Some(val) => {
                    todo!() // Poll::Ready(Ok(Some(val))),
                }
                None => {
                    todo!()
                    /* if let Some(ref x) = conn.error {
                        return Poll::Ready(Err(ReadError::ConnectionLost(x.clone())));
                    }
                    conn.blocked_readers.insert(self.stream, cx.waker().clone());
                    Poll::Pending */
                }
            },
            ReadStatus::Failed(read, Reset(error_code)) => match read {
                None => {
                    todo!()
                    /*  self.all_data_read = true;
                    self.reset = Some(error_code);
                    Poll::Ready(Err(ReadError::Reset(error_code))) */
                }
                done => {
                    todo!()
                    /* self.reset = Some(error_code);
                    Poll::Ready(Ok(done)) */
                }
            },
        }
    }
}

/// Future produced by [`RecvStream::read_to_end()`].
///
/// [`RecvStream::read_to_end()`]: crate::RecvStream::read_to_end
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
struct ReadToEnd<'a> {
    stream: &'a mut RecvStream,
    read: Vec<(Bytes, u64)>,
    start: u64,
    end: u64,
    size_limit: usize,
}

impl Future for ReadToEnd<'_> {
    type Output = Result<Vec<u8>, ReadToEndError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match ready!(self.stream.poll_read_chunk(cx, usize::MAX, false))? {
                Some(chunk) => {
                    todo!()

                    /* self.start = self.start.min(chunk.offset);
                    let end = chunk.bytes.len() as u64 + chunk.offset;
                    if (end - self.start) > self.size_limit as u64 {
                        return Poll::Ready(Err(ReadToEndError::TooLong));
                    }
                    self.end = self.end.max(end);
                    self.read.push((chunk.bytes, chunk.offset)); */
                }
                None => {
                    todo!()

                    /* if self.end == 0 {
                        // Never received anything
                        return Poll::Ready(Ok(Vec::new()));
                    }
                    let start = self.start;
                    let mut buffer = vec![0; (self.end - start) as usize];
                    for (data, offset) in self.read.drain(..) {
                        let offset = (offset - start) as usize;
                        buffer[offset..offset + data.len()].copy_from_slice(&data);
                    }
                    return Poll::Ready(Ok(buffer)); */
                }
            }
        }
    }
}

/// Errors from [`RecvStream::read_to_end`]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReadToEndError {
    /// An error occurred during reading
    #[error("read error: {0}")]
    Read(#[from] ReadError),
}

/// Errors that arise from reading from a stream.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The stream has already been stopped, finished, or reset
    #[error("closed stream")]
    ClosedStream,
    /// Attempted an ordered read following an unordered read
    ///
    /// Performing an unordered read allows discontinuities to arise in the receive buffer of a
    /// stream which cannot be recovered, making further ordered reads impossible.
    #[error("ordered read after unordered read")]
    IllegalOrderedRead,
}

impl From<ReadableError> for ReadError {
    fn from(e: ReadableError) -> Self {
        match e {
            ReadableError::ClosedStream => Self::ClosedStream,
            ReadableError::IllegalOrderedRead => Self::IllegalOrderedRead,
        }
    }
}

enum ReadStatus<T> {
    Readable(T),
    Finished(Option<T>),
    Failed(Option<T>, proto::ReadError),
}

impl<T> From<(Option<T>, Option<proto::ReadError>)> for ReadStatus<T> {
    fn from(status: (Option<T>, Option<proto::ReadError>)) -> Self {
        match status {
            (read, None) => Self::Finished(read),
            (read, Some(e)) => Self::Failed(read, e),
        }
    }
}
