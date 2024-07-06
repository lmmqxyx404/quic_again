use core::fmt;

use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::coding::{self, Codec, UnexpectedEnd};

/// 3.the third local mod, first used for [BufMutExt] trait
/// An integer less than 2^62
///
/// Values of this type are suitable for encoding as QUIC variable-length integer.
// It would be neat if we could express to Rust that the top two bits are available for use as enum
// discriminants
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VarInt(pub(crate) u64);

impl VarInt {
    /// 1
    pub fn from_u64(x: u64) -> Result<Self, VarIntBoundsExceeded> {
        if x < 2u64.pow(62) {
            Ok(Self(x))
        } else {
            Err(VarIntBoundsExceeded)
        }
    }

    /// 2.Extract the integer value
    pub const fn into_inner(self) -> u64 {
        self.0
    }

    /// 3 Construct a `VarInt` infallibly used for `$($name: VarInt::from_u32($default),)*`
    pub const fn from_u32(x: u32) -> Self {
        Self(x as u64)
    }
    /// 4.Compute the number of bytes needed to encode this value
    pub(crate) fn size(self) -> usize {
        let x = self.0;
        if x < 2u64.pow(6) {
            1
        } else if x < 2u64.pow(14) {
            2
        } else if x < 2u64.pow(30) {
            4
        } else if x < 2u64.pow(62) {
            8
        } else {
            unreachable!("malformed VarInt");
        }
    }
}
/// used for [`$($(#[$doc])* pub(crate) $name : VarInt,)*`]
impl fmt::Debug for VarInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::convert::TryFrom<usize> for VarInt {
    type Error = VarIntBoundsExceeded;
    /// Succeeds iff `x` < 2^62
    fn try_from(x: usize) -> Result<Self, VarIntBoundsExceeded> {
        Self::try_from(x as u64)
    }
}

impl std::convert::TryFrom<u64> for VarInt {
    type Error = VarIntBoundsExceeded;
    /// Succeeds iff `x` < 2^62
    fn try_from(x: u64) -> Result<Self, VarIntBoundsExceeded> {
        Self::from_u64(x)
    }
}
/// used for `EndpointConfig:: fn new`
impl From<u32> for VarInt {
    fn from(x: u32) -> Self {
        Self(x.into())
    }
}
/// used for `pub fn get_max_udp_payload_size`
impl From<VarInt> for u64 {
    fn from(x: VarInt) -> Self {
        x.0
    }
}

/// Error returned when constructing a `VarInt` from a value >= 2^62
#[derive(Debug, Copy, Clone, Eq, PartialEq, Error)]
#[error("value too large for varint encoding")]
pub struct VarIntBoundsExceeded;

impl Codec for VarInt {
    fn decode<B: Buf>(r: &mut B) -> coding::Result<Self> {
        if !r.has_remaining() {
            return Err(UnexpectedEnd);
        }
        let mut buf = [0; 8];
        buf[0] = r.get_u8();
        let tag = buf[0] >> 6;
        buf[0] &= 0b0011_1111;
        let x = match tag {
            0b00 => u64::from(buf[0]),
            0b01 => {
                if r.remaining() < 1 {
                    return Err(UnexpectedEnd);
                }
                r.copy_to_slice(&mut buf[1..2]);
                u64::from(u16::from_be_bytes(buf[..2].try_into().unwrap()))
            }
            0b10 => {
                if r.remaining() < 3 {
                    return Err(UnexpectedEnd);
                }
                r.copy_to_slice(&mut buf[1..4]);
                u64::from(u32::from_be_bytes(buf[..4].try_into().unwrap()))
            }
            0b11 => {
                if r.remaining() < 7 {
                    return Err(UnexpectedEnd);
                }
                r.copy_to_slice(&mut buf[1..8]);
                u64::from_be_bytes(buf)
            }
            _ => unreachable!(),
        };
        Ok(Self(x))
    }

    fn encode<B: BufMut>(&self, w: &mut B) {
        let x = self.0;
        if x < 2u64.pow(6) {
            w.put_u8(x as u8);
        } else if x < 2u64.pow(14) {
            w.put_u16(0b01 << 14 | x as u16);
        } else if x < 2u64.pow(30) {
            w.put_u32(0b10 << 30 | x as u32);
        } else if x < 2u64.pow(62) {
            w.put_u64(0b11 << 62 | x);
        } else {
            unreachable!("malformed VarInt")
        }
    }
}
