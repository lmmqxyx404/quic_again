

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

mod shared;

const MAX_CID_SIZE: usize = 20;

// 5. endpoint
mod endpoint;
pub use crate::endpoint::{ConnectError, ConnectionHandle, DatagramEvent, Endpoint};

// 6.config
mod config;