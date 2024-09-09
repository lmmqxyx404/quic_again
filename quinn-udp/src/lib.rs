use std::net::{IpAddr, Ipv6Addr, SocketAddr};
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

/// Number of UDP packets to send/receive at a time
pub const BATCH_SIZE: usize = imp::BATCH_SIZE;

/// Metadata for a single buffer filled with bytes received from the network
///
/// This associated buffer can contain one or more datagrams, see [`stride`].
///
/// [`stride`]: RecvMeta::stride
#[derive(Debug, Copy, Clone)]
pub struct RecvMeta {
    /// The source address of the datagram(s) contained in the buffer
    pub addr: SocketAddr,
    /// The number of bytes the associated buffer has
    pub len: usize,
    /// The size of a single datagram in the associated buffer
    ///
    /// When GRO (Generic Receive Offload) is used this indicates the size of a single
    /// datagram inside the buffer. If the buffer is larger, that is if [`len`] is greater
    /// then this value, then the individual datagrams contained have their boundaries at
    /// `stride` increments from the start. The last datagram could be smaller than
    /// `stride`.
    ///
    /// [`len`]: RecvMeta::len
    pub stride: usize,
    /// The Explicit Congestion Notification bits for the datagram(s) in the buffer
    pub ecn: Option<EcnCodepoint>,
    /// The destination IP address which was encoded in this datagram
    ///
    /// Populated on platforms: Windows, Linux, Android, FreeBSD, OpenBSD, NetBSD, macOS,
    /// and iOS.
    pub dst_ip: Option<IpAddr>,
}

impl Default for RecvMeta {
    /// Constructs a value with arbitrary fields, intended to be overwritten
    fn default() -> Self {
        Self {
            addr: SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0),
            len: 0,
            stride: 0,
            ecn: None,
            dst_ip: None,
        }
    }
}

/// An outgoing packet
#[derive(Debug, Clone)]
pub struct Transmit<'a> {
    /// The socket this datagram should be sent to
    pub destination: SocketAddr,
    /// Contents of the datagram
    pub contents: &'a [u8],
    /// Explicit congestion notification bits to set on the packet
    pub ecn: Option<EcnCodepoint>,
    /// The segment size if this transmission contains multiple datagrams.
    /// This is `None` if the transmit only contains a single datagram
    pub segment_size: Option<usize>,
    /// Optional source IP address for the datagram
    pub src_ip: Option<IpAddr>,
}

/// Explicit congestion notification codepoint
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EcnCodepoint {
    #[doc(hidden)]
    Ect0 = 0b10,
    #[doc(hidden)]
    Ect1 = 0b01,
    #[doc(hidden)]
    Ce = 0b11,
}

impl EcnCodepoint {
    /// Create new object from the given bits
    pub fn from_bits(x: u8) -> Option<Self> {
        use self::EcnCodepoint::*;
        Some(match x & 0b11 {
            0b10 => Ect0,
            0b01 => Ect1,
            0b11 => Ce,
            _ => {
                return None;
            }
        })
    }
}
