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
/// 11. used for mod `transport_parameters`
mod token;
/// 10
pub mod transport_parameters;
/// 12
mod constant_time;

use token::ResetToken;

const MAX_CID_SIZE: usize = 20;
/// used for 11
const RESET_TOKEN_SIZE: usize = 16;