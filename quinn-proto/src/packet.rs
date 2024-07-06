use std::io;

use bytes::{Buf, BufMut, BytesMut};
use thiserror::Error;

// use crate::coding::BufMutExt;

use crate::{
    coding::{self, BufExt, BufMutExt},
    shared::ConnectionId,
};
/// 1. An encoded packet number
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum PacketNumber {
    U8(u8),
    U16(u16),
    U24(u32),
    U32(u32),
}

impl PacketNumber {
    /// 1.1
    pub(crate) fn encode<W: BufMut>(self, w: &mut W) {
        use self::PacketNumber::*;
        match self {
            // write function is vital.
            U8(x) => w.write(x),
            U16(x) => w.write(x),
            U24(x) => w.put_uint(u64::from(x), 3),
            U32(x) => w.write(x),
        }
    }

    /// 1.2
    pub(crate) fn decode<R: Buf>(len: usize, r: &mut R) -> Result<Self, PacketDecodeError> {
        // done1 completed
        use self::PacketNumber::*;
        let pn = match len {
            1 => U8(r.get()?),
            2 => U16(r.get()?),
            3 => U24(r.get_uint(3) as u32),
            4 => U32(r.get()?),
            _ => unreachable!(),
        };
        Ok(pn)
    }

    /// 1.3
    pub(crate) fn len(self) -> usize {
        use self::PacketNumber::*;
        match self {
            U8(_) => 1,
            U16(_) => 2,
            U24(_) => 3,
            U32(_) => 4,
        }
    }

    /// 1.4
    pub(crate) fn new(n: u64, largest_acked: u64) -> Self {
        let range = (n - largest_acked) * 2;
        if range < 1 << 8 {
            Self::U8(n as u8)
        } else if range < 1 << 16 {
            Self::U16(n as u16)
        } else if range < 1 << 24 {
            Self::U24(n as u32)
        } else if range < 1 << 32 {
            Self::U32(n as u32)
        } else {
            panic!("packet number too large to encode")
        }
    }

    pub(crate) fn expand(self, expected: u64) -> u64 {
        use self::PacketNumber::*;
        let truncated = match self {
            U8(x) => u64::from(x),
            U16(x) => u64::from(x),
            U24(x) => u64::from(x),
            U32(x) => u64::from(x),
        };
        let nbits = self.len() * 8;
        let win = 1 << nbits;
        let hwin = win / 2;
        let mask = win - 1;
        // The incoming packet number should be greater than expected - hwin and less than or equal
        // to expected + hwin
        //
        // This means we can't just strip the trailing bits from expected and add the truncated
        // because that might yield a value outside the window.
        //
        // The following code calculates a candidate value and makes sure it's within the packet
        // number window.
        let candidate = (expected & !mask) | truncated;
        if expected.checked_sub(hwin).map_or(false, |x| candidate <= x) {
            candidate + win
        } else if candidate > expected + hwin && candidate > win {
            candidate - win
        } else {
            candidate
        }
    }
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PacketDecodeError {
    /// 1. 比较早的
    #[error("invalid header: {0}")]
    InvalidHeader(&'static str),
    /// 2. Packet uses a QUIC version that is not supported
    #[error("unsupported version {version:x}")]
    UnsupportedVersion {
        /// Source Connection ID
        src_cid: ConnectionId,
        /// Destination Connection ID
        dst_cid: ConnectionId,
        /// The version that was unsupported
        version: u32,
    },
}

// the impl is forst used for decode r.get()? parse
impl From<coding::UnexpectedEnd> for PacketDecodeError {
    fn from(_: coding::UnexpectedEnd) -> Self {
        Self::InvalidHeader("unexpected end of packet")
    }
}

pub(crate) struct Packet {
    pub(crate) header: Header,
}

pub(crate) enum Header {
    Initial,
    Long,
    Retry,
    Short,
    VersionNegotiate,
}

/// Decodes a QUIC packet's invariant header
///
/// Due to packet number encryption, it is impossible to fully decode a header
/// (which includes a variable-length packet number) without crypto context.
/// The crypto context (represented by the `Crypto` type in Quinn) is usually
/// part of the `Connection`, or can be derived from the destination CID for
/// Initial packets.
///
/// To cope with this, we decode the invariant header (which should be stable
/// across QUIC versions), which gives us the destination CID and allows us
/// to inspect the version and packet type (which depends on the version).
/// This information allows us to fully decode and decrypt the packet.
#[cfg_attr(test, derive(Clone))]
#[derive(Debug)]
pub struct PartialDecode {}

#[allow(clippy::len_without_is_empty)]
impl PartialDecode {
    /// Begin decoding a QUIC packet from `bytes`, returning any trailing data not part of that packet
    pub fn new(
        bytes: BytesMut,
        cid_parser: &(impl ConnectionIdParser + ?Sized),
        supported_versions: &[u32],
        grease_quic_bit: bool,
    ) -> Result<(Self, Option<BytesMut>), PacketDecodeError> {
        let mut buf = io::Cursor::new(bytes);
        let plain_header =
            ProtectedHeader::decode(&mut buf, cid_parser, supported_versions, grease_quic_bit)?;
        todo!()
    }
}

/// Parse connection id in short header packet
pub trait ConnectionIdParser {
    /// Parse a connection id from given buffer
    fn parse(&self, buf: &mut dyn Buf) -> Result<ConnectionId, PacketDecodeError>;
}

/// A [`ConnectionIdParser`] implementation that assumes the connection ID is of fixed length
pub struct FixedLengthConnectionIdParser {
    expected_len: usize,
}

impl FixedLengthConnectionIdParser {
    /// Create a new instance of `FixedLengthConnectionIdParser`
    pub fn new(expected_len: usize) -> Self {
        Self { expected_len }
    }
}

impl ConnectionIdParser for FixedLengthConnectionIdParser {
    fn parse(&self, buffer: &mut dyn Buf) -> Result<ConnectionId, PacketDecodeError> {
        (buffer.remaining() >= self.expected_len)
            .then(|| ConnectionId::from_buf(buffer, self.expected_len))
            .ok_or(PacketDecodeError::InvalidHeader("packet too small"))
    }
}

/// Plain packet header
#[derive(Clone, Debug)]
pub enum ProtectedHeader {
    /// 1. A short packet header, as used during the data phase
    Short {
        /// Spin bit
        spin: bool,
        /// Destination Connection ID
        dst_cid: ConnectionId,
    },
    /// 2. A Version Negotiation packet header
    VersionNegotiate {
        /// Random value
        random: u8,
        /// Destination Connection ID
        dst_cid: ConnectionId,
        /// Source Connection ID
        src_cid: ConnectionId,
    },
}

impl ProtectedHeader {
    /// Decode a plain header from given buffer, with given [`ConnectionIdParser`].
    pub fn decode(
        buf: &mut io::Cursor<BytesMut>,
        cid_parser: &(impl ConnectionIdParser + ?Sized),
        supported_versions: &[u32],
        grease_quic_bit: bool,
    ) -> Result<Self, PacketDecodeError> {
        let first = buf.get::<u8>()?;
        if !grease_quic_bit && first & FIXED_BIT == 0 {
            return Err(PacketDecodeError::InvalidHeader("fixed bit unset"));
        }
        if first & LONG_HEADER_FORM == 0 {
            let spin = first & SPIN_BIT != 0;

            Ok(Self::Short {
                spin,
                dst_cid: cid_parser.parse(buf)?,
            })
        } else {
            let version = buf.get::<u32>()?;

            let dst_cid = ConnectionId::decode_long(buf)
                .ok_or(PacketDecodeError::InvalidHeader("malformed cid"))?;
            let src_cid = ConnectionId::decode_long(buf)
                .ok_or(PacketDecodeError::InvalidHeader("malformed cid"))?;

            // TODO: Support long CIDs for compatibility with future QUIC versions
            if version == 0 {
                let random = first & !LONG_HEADER_FORM;
                return Ok(Self::VersionNegotiate {
                    random,
                    dst_cid,
                    src_cid,
                });
            }

            if !supported_versions.contains(&version) {
                return Err(PacketDecodeError::UnsupportedVersion {
                    src_cid,
                    dst_cid,
                    version,
                });
            }
            todo!()
        }
    }
}

pub(crate) const FIXED_BIT: u8 = 0x40;
pub(crate) const LONG_HEADER_FORM: u8 = 0x80;
pub(crate) const SPIN_BIT: u8 = 0x20;

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use std::io;

    use super::PacketNumber;

    /// check packet number
    fn check_pn(typed: PacketNumber, encoded: &[u8]) {
        let mut buf = Vec::new();
        typed.encode(&mut buf);
        assert_eq!(&buf[..], encoded);
        let decoded = PacketNumber::decode(typed.len(), &mut io::Cursor::new(&buf)).unwrap();
        assert_eq!(typed, decoded);
    }

    #[test]
    fn roundtrip_packet_numbers() {
        check_pn(PacketNumber::U8(0x7f), &hex!("7f"));
        check_pn(PacketNumber::U16(0x80), &hex!("0080"));
        check_pn(PacketNumber::U16(0x3fff), &hex!("3fff"));
        check_pn(PacketNumber::U32(0x0000_4000), &hex!("0000 4000"));
        check_pn(PacketNumber::U32(0xffff_ffff), &hex!("ffff ffff"));
    }

    #[test]
    fn pn_encode() {
        check_pn(PacketNumber::new(0x10, 0), &hex!("10"));
        check_pn(PacketNumber::new(0x100, 0), &hex!("0100"));
        check_pn(PacketNumber::new(0x10000, 0), &hex!("010000"));
    }

    #[test]
    fn pn_expand_roundtrip() {
        for expected in 0..1024 {
            for actual in expected..1024 {
                assert_eq!(actual, PacketNumber::new(actual, expected).expand(expected));
            }
        }
    }

    #[test]
    fn header_encoding() {}
}
