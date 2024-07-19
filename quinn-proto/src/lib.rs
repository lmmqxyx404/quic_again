#[cfg(all(test, feature = "rustls"))]
mod tests;
// 1. basic components of quic
mod packet;
// 2. todo: #[doc(hidden)]
pub mod coding;
// 3. because 2 needs the varint.
mod varint;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use shared::EcnCodepoint;
pub use varint::{VarInt, VarIntBoundsExceeded};

// 4. generate connection id
mod cid_generator;
pub use crate::cid_generator::{ConnectionIdGenerator, RandomConnectionIdGenerator};
// 5
mod shared;

/// 6. endpoint
mod endpoint;
pub use crate::endpoint::Endpoint;

/// 7.config
mod config;
/// 9
mod connection;
/// 8
pub mod crypto;

/// 10
pub mod transport_parameters;

/// 11
pub mod token;
use token::ResetToken;

/// 12 used for [`ResetToken`]
mod constant_time;

/// 13
mod frame;
/// 14
mod transport_error;
pub use crate::transport_error::{Code as TransportErrorCode, Error as TransportError};
/// 15
mod cid_queue;
/// 17.
pub mod congestion;
/// 16.
mod range_set;

/// 1
const MAX_CID_SIZE: usize = 20;
/// 2
const RESET_TOKEN_SIZE: usize = 16;
/// 3.
const TIMER_GRANULARITY: Duration = Duration::from_millis(1);
/// 4. <https://www.rfc-editor.org/rfc/rfc9000.html#name-datagram-size>
const INITIAL_MTU: u16 = 1200;
/// 5.
const MAX_UDP_PAYLOAD: u16 = 65527;
/// 6.
const MIN_INITIAL_SIZE: u16 = 1200;

/// Whether an endpoint was the initiator of a connection
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Side {
    /// The initiator of a connection
    Client = 0,
    /// The acceptor of a connection
    Server = 1,
}

impl Side {
    #[inline]
    /// 1. Shorthand for `self == Side::Server`
    pub fn is_server(self) -> bool {
        self == Self::Server
    }
    #[inline]
    /// 2. Shorthand for `self == Side::Client`
    pub fn is_client(self) -> bool {
        self == Self::Client
    }
}

/// The QUIC protocol version implemented.
pub const DEFAULT_SUPPORTED_VERSIONS: &[u32] = &[
    0x00000001,
    0xff00_001d,
    0xff00_001e,
    0xff00_001f,
    0xff00_0020,
    0xff00_0021,
    0xff00_0022,
];

/// Whether a stream communicates data in both directions or only from the initiator
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Dir {
    /// Data flows in both directions
    Bi = 0,
    /// Data flows only from the stream's initiator
    Uni = 1,
}

impl Dir {
    fn iter() -> impl Iterator<Item = Self> {
        [Self::Bi, Self::Uni].iter().cloned()
    }
}

/// Identifier for a stream within a particular connection
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StreamId(#[doc(hidden)] pub u64);

/// An outgoing packet
#[derive(Debug)]
#[must_use]
pub struct Transmit {
    /// The socket this datagram should be sent to
    pub destination: SocketAddr,
    /// Explicit congestion notification bits to set on the packet
    pub ecn: Option<EcnCodepoint>,
    /// Amount of data written to the caller-supplied buffer
    pub size: usize,
    /// The segment size if this transmission contains multiple datagrams.
    /// This is `None` if the transmit only contains a single datagram
    pub segment_size: Option<usize>,
    /// Optional source IP address for the datagram
    pub src_ip: Option<IpAddr>,
}
