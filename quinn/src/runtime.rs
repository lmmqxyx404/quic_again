use std::{fmt::Debug, io, sync::Arc};

#[cfg(feature = "runtime-tokio")]
mod tokio;
#[cfg(feature = "runtime-tokio")]
pub use self::tokio::TokioRuntime;

/// Abstracts I/O and timer operations for runtime independence
pub trait Runtime: Send + Sync + Debug + 'static {
    /// 1. Convert `t` into the socket type used by this runtime
    fn wrap_udp_socket(&self, t: std::net::UdpSocket) -> io::Result<Arc<dyn AsyncUdpSocket>>;
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
pub trait AsyncUdpSocket: Send + Sync + Debug + 'static {}
