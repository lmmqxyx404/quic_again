use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use proto::{ClosedStream, ConnectionError, FinishError, StreamId, Written};
use thiserror::Error;

use crate::{connection::ConnectionRef, VarInt};

/// A stream that can only be used to send data
///
/// If dropped, streams that haven't been explicitly [`reset()`] will be implicitly [`finish()`]ed,
/// continuing to (re)transmit previously written data until it has been fully acknowledged or the
/// connection is closed.
///
/// # Cancellation
///
/// A `write` method is said to be *cancel-safe* when dropping its future before the future becomes
/// ready will always result in no data being written to the stream. This is true of methods which
/// succeed immediately when any progress is made, and is not true of methods which might need to
/// perform multiple writes internally before succeeding. Each `write` method documents whether it is
/// cancel-safe.
///
/// [`reset()`]: SendStream::reset
/// [`finish()`]: SendStream::finish
#[derive(Debug)]
pub struct SendStream {
    conn: ConnectionRef,
    stream: StreamId,
    is_0rtt: bool,
}

impl SendStream {
    pub(crate) fn new(conn: ConnectionRef, stream: StreamId, is_0rtt: bool) -> Self {
        Self {
            conn,
            stream,
            is_0rtt,
        }
    }

    /// Convenience method to write an entire buffer to the stream
    ///
    /// This operation is *not* cancel-safe.
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteError> {
        WriteAll { stream: self, buf }.await
    }

    fn execute_poll<F, R>(&mut self, cx: &mut Context, write_fn: F) -> Poll<Result<R, WriteError>>
    where
        F: FnOnce(&mut proto::SendStream) -> Result<R, proto::WriteError>,
    {
        use proto::WriteError::*;
        let mut conn = self.conn.state.lock("SendStream::poll_write");
        if self.is_0rtt {
            conn.check_0rtt()
                .map_err(|()| WriteError::ZeroRttRejected)?;
        }
        if let Some(ref x) = conn.error {
            todo!() // return Poll::Ready(Err(WriteError::ConnectionLost(x.clone())));
        }
        let result = match write_fn(&mut conn.inner.send_stream(self.stream)) {
            Ok(result) => result,
            Err(Blocked) => {
                todo!()
            }
            Err(Stopped(error_code)) => {
                todo!() // return Poll::Ready(Err(WriteError::Stopped(error_code)));
            }
            Err(ClosedStream) => {
                todo!() // return Poll::Ready(Err(WriteError::ClosedStream));
            }
        };
        conn.wake();
        Poll::Ready(Ok(result))
    }

    /// Notify the peer that no more data will ever be written to this stream
    ///
    /// It is an error to write to a [`SendStream`] after `finish()`ing it. [`reset()`](Self::reset)
    /// may still be called after `finish` to abandon transmission of any stream data that might
    /// still be buffered.
    ///
    /// To wait for the peer to receive all buffered stream data, see [`stopped()`](Self::stopped).
    ///
    /// May fail if [`finish()`](Self::finish) or [`reset()`](Self::reset) was previously
    /// called. This error is harmless and serves only to indicate that the caller may have
    /// incorrect assumptions about the stream's state.
    pub fn finish(&mut self) -> Result<(), ClosedStream> {
        let mut conn = self.conn.state.lock("finish");
        match conn.inner.send_stream(self.stream).finish() {
            Ok(()) => {
                conn.wake();
                Ok(())
            }
            Err(FinishError::ClosedStream) => {
                todo!() // Err(ClosedStream::new()),
            }
            // Harmless. If the application needs to know about stopped streams at this point, it
            // should call `stopped`.
            Err(FinishError::Stopped(_)) => {
                todo!() // Ok(()),
            }
        }
    }

    /// Completes when the peer stops the stream or reads the stream to completion
    ///
    /// Yields `Some` with the stop error code if the peer stops the stream. Yields `None` if the
    /// local side [`finish()`](Self::finish)es the stream and then the peer acknowledges receipt
    /// of all stream data (although not necessarily the processing of it), after which the peer
    /// closing the stream is no longer meaningful.
    ///
    /// For a variety of reasons, the peer may not send acknowledgements immediately upon receiving
    /// data. As such, relying on `stopped` to know when the peer has read a stream to completion
    /// may introduce more latency than using an application-level response of some sort.
    pub async fn stopped(&mut self) -> Result<Option<VarInt>, StoppedError> {
        Stopped { stream: self }.await
    }

    #[doc(hidden)]
    pub fn poll_stopped(&mut self, cx: &mut Context) -> Poll<Result<Option<VarInt>, StoppedError>> {
        let mut conn = self.conn.state.lock("SendStream::poll_stopped");

        if self.is_0rtt {
            todo!()
            /* conn.check_0rtt()
            .map_err(|()| StoppedError::ZeroRttRejected)?; */
        }
        match conn.inner.send_stream(self.stream).stopped() {
            Err(_) => Poll::Ready(Ok(None)),
            Ok(Some(error_code)) => {
                todo!()
                // Poll::Ready(Ok(Some(error_code))),
            }
            Ok(None) => {
                if let Some(e) = &conn.error {
                    todo!() //    return Poll::Ready(Err(e.clone().into()));
                }
                conn.stopped.insert(self.stream, cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Future produced by [`SendStream::write_all()`].
///
/// [`SendStream::write_all()`]: crate::SendStream::write_all
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
struct WriteAll<'a> {
    stream: &'a mut SendStream,
    buf: &'a [u8],
}

impl<'a> Future for WriteAll<'a> {
    type Output = Result<(), WriteError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            if this.buf.is_empty() {
                return Poll::Ready(Ok(()));
            }
            let buf = this.buf;
            let n = ready!(this.stream.execute_poll(cx, |s| s.write(buf)))?;
            this.buf = &this.buf[n..];
        }
    }
}

/// Errors that arise from writing to a stream
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// This was a 0-RTT stream and the server rejected it
    ///
    /// Can only occur on clients for 0-RTT streams, which can be opened using
    /// [`Connecting::into_0rtt()`].
    ///
    /// [`Connecting::into_0rtt()`]: crate::Connecting::into_0rtt()
    #[error("0-RTT rejected")]
    ZeroRttRejected,
}

/// Errors that arise while monitoring for a send stream stop from the peer
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StoppedError {}

/// Future produced by `SendStream::stopped`
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
struct Stopped<'a> {
    stream: &'a mut SendStream,
}

impl Future for Stopped<'_> {
    type Output = Result<Option<VarInt>, StoppedError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().stream.poll_stopped(cx)
    }
}
