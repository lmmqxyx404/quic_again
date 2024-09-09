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
            todo!()
            /* conn.check_0rtt()
            .map_err(|()| WriteError::ZeroRttRejected)?; */
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
                todo!() // return Poll::Ready(Ok(()));
            }
            let buf = this.buf;
            let n = ready!(this.stream.execute_poll(cx, |s| s.write(buf)))?;
            this.buf = &this.buf[n..];
        }
    }
}

/// Errors that arise from writing to a stream
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WriteError {}
