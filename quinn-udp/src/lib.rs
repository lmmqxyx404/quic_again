#[cfg(unix)]
use std::os::unix::io::AsFd;

#[cfg(unix)]
#[path = "unix.rs"]
mod imp;

#[cfg(any(unix, windows))]
mod cmsg;

#[allow(unused_imports, unused_macros)]
mod log {
    #[cfg(feature = "tracing")]
    pub(crate) use tracing::{debug, error, info, trace, warn};
}

pub use imp::UdpSocketState;

/// A borrowed UDP socket
///
/// On Unix, constructible via `From<T: AsFd>`. On Windows, constructible via `From<T:
/// AsSocket>`.
// Wrapper around socket2 to avoid making it a public dependency and incurring stability risk
pub struct UdpSockRef<'a>(socket2::SockRef<'a>);

#[cfg(unix)]
impl<'s, S> From<&'s S> for UdpSockRef<'s>
where
    S: AsFd,
{
    fn from(socket: &'s S) -> Self {
        Self(socket.into())
    }
}
