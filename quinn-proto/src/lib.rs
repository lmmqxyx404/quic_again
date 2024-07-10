mod tests;
// 1. basic components of quic
mod packet;
// 2. todo: #[doc(hidden)]
pub mod coding;
// 3. because 2 needs the varint.
mod varint;

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

/// 1
const MAX_CID_SIZE: usize = 20;
/// 2
const RESET_TOKEN_SIZE: usize = 16;

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
