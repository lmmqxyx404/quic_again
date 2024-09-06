use std::{
    fmt::Debug,
    future::Future,
    io::{self, IoSliceMut},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

use udp::RecvMeta;

#[cfg(feature = "runtime-tokio")]
mod tokio;

#[cfg(feature = "runtime-tokio")]
pub use self::tokio::TokioRuntime;

/// Abstracts I/O and timer operations for runtime independence
pub trait Runtime: Send + Sync + Debug + 'static {
    /// 1. Convert `t` into the socket type used by this runtime
    fn wrap_udp_socket(&self, t: std::net::UdpSocket) -> io::Result<Arc<dyn AsyncUdpSocket>>;
    /// 2. Drive `future` to completion in the background
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
    /// 3. Look up the current time
    ///
    /// Allows simulating the flow of time for testing.
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Automatically select an appropriate runtime from those enabled at compile time
///
/// If `runtime-tokio` is enabled and this function is called from within a Tokio runtime context,
/// then `TokioRuntime` is returned. Otherwise, if `runtime-async-std` is enabled, `AsyncStdRuntime`
/// is returned. Otherwise, if `runtime-smol` is enabled, `SmolRuntime` is returned.
/// Otherwise, `None` is returned.
#[allow(clippy::needless_return)] // Be sure we return the right thing
pub fn default_runtime() -> Option<Arc<dyn Runtime>> {
    #[cfg(feature = "runtime-tokio")]
    {
        if ::tokio::runtime::Handle::try_current().is_ok() {
            return Some(Arc::new(TokioRuntime));
        }
    }
    todo!()
}

/// Abstract implementation of a UDP socket for runtime independence
pub trait AsyncUdpSocket: Send + Sync + Debug + 'static {
    /// 1. Look up the local IP address and port used by this socket
    fn local_addr(&self) -> io::Result<SocketAddr>;
    /// 2. Whether datagrams might get fragmented into multiple parts
    ///
    /// Sockets should prevent this for best performance. See e.g. the `IPV6_DONTFRAG` socket
    /// option.
    fn may_fragment(&self) -> bool {
        true
    }
    /// 3. Maximum number of datagrams that might be described by a single [`RecvMeta`]
    fn max_receive_segments(&self) -> usize {
        1
    }

    /// 4. Receive UDP datagrams, or register to be woken if receiving may succeed in the future
    fn poll_recv(
        &self,
        cx: &mut Context,
        bufs: &mut [IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> Poll<io::Result<usize>>;

    /// 5. Maximum number of datagrams that a [`Transmit`] may encode
    fn max_transmit_segments(&self) -> usize {
        1
    }
    
}

/* /// An object polled to detect when an associated [`AsyncUdpSocket`] is writable
///
/// Any number of `UdpPoller`s may exist for a single [`AsyncUdpSocket`]. Each `UdpPoller` is
/// responsible for notifying at most one task when that socket becomes writable.
pub trait UdpPoller: Send + Sync + Debug + 'static {
    /// Check whether the associated socket is likely to be writable
    ///
    /// Must be called after [`AsyncUdpSocket::try_send`] returns [`io::ErrorKind::WouldBlock`] to
    /// register the task associated with `cx` to be woken when a send should be attempted
    /// again. Unlike in [`Future::poll`], a [`UdpPoller`] may be reused indefinitely no matter how
    /// many times `poll_writable` returns [`Poll::Ready`].
    fn poll_writable(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>>;
} */
