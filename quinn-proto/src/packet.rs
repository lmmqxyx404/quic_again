use std::ops::Range;
use std::{cmp::Ordering, io};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

// use crate::coding::BufMutExt;

use crate::crypto;
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
    /// 5
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
    /// 6
    fn tag(self) -> u8 {
        use self::PacketNumber::*;
        match self {
            U8(_) => 0b00,
            U16(_) => 0b01,
            U24(_) => 0b10,
            U32(_) => 0b11,
        }
    }
    /// 7
    pub(crate) fn decode_len(tag: u8) -> usize {
        1 + (tag & 0x03) as usize
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
    /// 1
    pub(crate) header: Header,
    /// 2
    pub(crate) header_data: Bytes,
    /// 3
    pub(crate) payload: BytesMut,
}

impl Packet {
    pub(crate) fn reserved_bits_valid(&self) -> bool {
        let mask = match self.header {
            Header::Short { .. } => SHORT_RESERVED_BITS,
            _ => LONG_RESERVED_BITS,
        };
        self.header_data[0] & mask == 0
    }
}

pub(crate) enum Header {
    Initial(InitialHeader),
    Long {
        ty: LongType,
        dst_cid: ConnectionId,
        src_cid: ConnectionId,
        number: PacketNumber,
        version: u32,
    },
    Retry {
        dst_cid: ConnectionId,
        src_cid: ConnectionId,
        version: u32,
    },
    Short {
        spin: bool,
        key_phase: bool,
        dst_cid: ConnectionId,
        number: PacketNumber,
    },
    VersionNegotiate {
        random: u8,
        src_cid: ConnectionId,
        dst_cid: ConnectionId,
    },
}

impl Header {
    /// 1.
    pub(crate) fn encode(&self, w: &mut Vec<u8>) -> PartialEncode {
        use self::Header::*;
        let start = w.len();
        match *self {
            Initial(InitialHeader {
                ref dst_cid,
                ref src_cid,
                ref token,
                number,
                version,
            }) => {
                w.write(u8::from(LongHeaderType::Initial) | number.tag());
                w.write(version);
                dst_cid.encode_long(w);
                src_cid.encode_long(w);
                w.write_var(token.len() as u64);
                w.put_slice(token);
                w.write::<u16>(0); // Placeholder for payload length; see `set_payload_length`
                number.encode(w);
                PartialEncode {
                    start,
                    header_len: w.len() - start,
                    pn: Some((number.len(), true)),
                }
            }
            Long {
                ty,
                ref dst_cid,
                ref src_cid,
                number,
                version,
            } => {
                w.write(u8::from(LongHeaderType::Standard(ty)) | number.tag());
                w.write(version);
                dst_cid.encode_long(w);
                src_cid.encode_long(w);
                w.write::<u16>(0); // Placeholder for payload length; see `set_payload_length`
                number.encode(w);
                PartialEncode {
                    start,
                    header_len: w.len() - start,
                    pn: Some((number.len(), true)),
                }
            }
            Retry {
                ref dst_cid,
                ref src_cid,
                version,
            } => {
                w.write(u8::from(LongHeaderType::Retry));
                w.write(version);
                dst_cid.encode_long(w);
                src_cid.encode_long(w);
                PartialEncode {
                    start,
                    header_len: w.len() - start,
                    pn: None,
                }
            }
            Short {
                spin,
                key_phase,
                ref dst_cid,
                number,
            } => {
                w.write(
                    FIXED_BIT
                        | if key_phase { KEY_PHASE_BIT } else { 0 }
                        | if spin { SPIN_BIT } else { 0 }
                        | number.tag(),
                );
                w.put_slice(dst_cid);
                number.encode(w);
                PartialEncode {
                    start,
                    header_len: w.len() - start,
                    pn: Some((number.len(), false)),
                }
            }
            VersionNegotiate {
                ref random,
                ref dst_cid,
                ref src_cid,
            } => {
                w.write(0x80u8 | random);
                w.write::<u32>(0);
                dst_cid.encode_long(w);
                src_cid.encode_long(w);
                PartialEncode {
                    start,
                    header_len: w.len() - start,
                    pn: None,
                }
            }
        }
    }

    /// 2. Whether the packet is encrypted on the wire
    pub(crate) fn is_protected(&self) -> bool {
        !matches!(*self, Self::Retry { .. } | Self::VersionNegotiate { .. })
    }
    /// 3.
    pub(crate) fn space(&self) -> SpaceId {
        use self::Header::*;
        match *self {
            Short { .. } => SpaceId::Data,
            Long {
                ty: LongType::ZeroRtt,
                ..
            } => SpaceId::Data,
            Long {
                ty: LongType::Handshake,
                ..
            } => SpaceId::Handshake,
            _ => SpaceId::Initial,
        }
    }
    /// 4.
    pub(crate) fn is_short(&self) -> bool {
        matches!(*self, Self::Short { .. })
    }
    /// 5
    pub(crate) fn dst_cid(&self) -> &ConnectionId {
        use self::Header::*;
        match *self {
            Initial(InitialHeader { ref dst_cid, .. }) => dst_cid,
            Long { ref dst_cid, .. } => dst_cid,
            Retry { ref dst_cid, .. } => dst_cid,
            Short { ref dst_cid, .. } => dst_cid,
            VersionNegotiate { ref dst_cid, .. } => dst_cid,
        }
    }
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
pub struct PartialDecode {
    plain_header: ProtectedHeader,
    buf: io::Cursor<BytesMut>,
}

#[allow(clippy::len_without_is_empty)]
impl PartialDecode {
    /// 1. Begin decoding a QUIC packet from `bytes`, returning any trailing data not part of that packet
    pub fn new(
        bytes: BytesMut,
        cid_parser: &(impl ConnectionIdParser + ?Sized),
        supported_versions: &[u32],
        grease_quic_bit: bool,
    ) -> Result<(Self, Option<BytesMut>), PacketDecodeError> {
        let mut buf = io::Cursor::new(bytes);
        let plain_header =
            ProtectedHeader::decode(&mut buf, cid_parser, supported_versions, grease_quic_bit)?;
        let dgram_len = buf.get_ref().len();
        let packet_len = plain_header
            .payload_len()
            .map(|len| (buf.position() + len) as usize)
            .unwrap_or(dgram_len);
        match dgram_len.cmp(&packet_len) {
            Ordering::Equal => Ok((Self { plain_header, buf }, None)),
            Ordering::Less => Err(PacketDecodeError::InvalidHeader(
                "packet too short to contain payload length",
            )),
            Ordering::Greater => {
                let rest = Some(buf.get_mut().split_off(packet_len));
                Ok((Self { plain_header, buf }, rest))
            }
        }
    }
    /// 2. The destination connection ID of the packet
    pub fn dst_cid(&self) -> &ConnectionId {
        self.plain_header.dst_cid()
    }
    /// 3.
    pub(crate) fn is_initial(&self) -> bool {
        self.space() == Some(SpaceId::Initial)
    }
    /// 4
    pub(crate) fn space(&self) -> Option<SpaceId> {
        use self::ProtectedHeader::*;
        match self.plain_header {
            Initial { .. } => Some(SpaceId::Initial),
            Long {
                ty: LongType::Handshake,
                ..
            } => Some(SpaceId::Handshake),
            Long {
                ty: LongType::ZeroRtt,
                ..
            } => Some(SpaceId::Data),
            Short { .. } => Some(SpaceId::Data),
            _ => None,
        }
    }
    /// 5
    pub(crate) fn is_0rtt(&self) -> bool {
        match self.plain_header {
            ProtectedHeader::Long { ty, .. } => ty == LongType::ZeroRtt,
            _ => false,
        }
    }
    /// 6. The underlying partially-decoded packet data
    pub(crate) fn data(&self) -> &[u8] {
        self.buf.get_ref()
    }

    /// 7. Length of QUIC packet being decoded
    #[allow(unreachable_pub)] // fuzzing only
    pub fn len(&self) -> usize {
        self.buf.get_ref().len()
    }
    /// 8. used for `fn unprotect_header` in packet_crypto
    pub(crate) fn finish(
        self,
        header_crypto: Option<&dyn crypto::HeaderKey>,
    ) -> Result<Packet, PacketDecodeError> {
        use self::ProtectedHeader::*;
        let Self {
            plain_header,
            mut buf,
        } = self;
        if let Initial(ProtectedInitialHeader {
            dst_cid,
            src_cid,
            token_pos,
            version,
            ..
        }) = plain_header
        {
            let number = Self::decrypt_header(&mut buf, header_crypto.unwrap())?;
            let header_len = buf.position() as usize;
            let mut bytes = buf.into_inner();

            let header_data = bytes.split_to(header_len).freeze();
            let token = header_data.slice(token_pos.start..token_pos.end);
            return Ok(Packet {
                header: Header::Initial(InitialHeader {
                    dst_cid,
                    src_cid,
                    token,
                    number,
                    version,
                }),
                header_data,
                payload: bytes,
            });
        }

        let header = match plain_header {
            Long {
                ty,
                dst_cid,
                src_cid,
                version,
                ..
            } => Header::Long {
                ty,
                dst_cid,
                src_cid,
                number: Self::decrypt_header(&mut buf, header_crypto.unwrap())?,
                version,
            },
            Retry {
                dst_cid,
                src_cid,
                version,
            } => Header::Retry {
                dst_cid,
                src_cid,
                version,
            },
            Short { spin, dst_cid, .. } => {
                let number = Self::decrypt_header(&mut buf, header_crypto.unwrap())?;
                let key_phase = buf.get_ref()[0] & KEY_PHASE_BIT != 0;
                Header::Short {
                    spin,
                    key_phase,
                    dst_cid,
                    number,
                }
            }
            VersionNegotiate {
                random,
                dst_cid,
                src_cid,
            } => Header::VersionNegotiate {
                random,
                dst_cid,
                src_cid,
            },
            Initial { .. } => unreachable!(),
        };

        let header_len = buf.position() as usize;
        let mut bytes = buf.into_inner();
        Ok(Packet {
            header,
            header_data: bytes.split_to(header_len).freeze(),
            payload: bytes,
        })
    }
    /// 9.
    fn decrypt_header(
        buf: &mut io::Cursor<BytesMut>,
        header_crypto: &dyn crypto::HeaderKey,
    ) -> Result<PacketNumber, PacketDecodeError> {
        let packet_length = buf.get_ref().len();
        let pn_offset = buf.position() as usize;
        if packet_length < pn_offset + 4 + header_crypto.sample_size() {
            return Err(PacketDecodeError::InvalidHeader(
                "packet too short to extract header protection sample",
            ));
        }

        header_crypto.decrypt(pn_offset, buf.get_mut());

        let len = PacketNumber::decode_len(buf.get_ref()[0]);
        PacketNumber::decode(len, buf)
    }
    /// 10.
    pub(crate) fn initial_header(&self) -> Option<&ProtectedInitialHeader> {
        self.plain_header.as_initial()
    }
    /// 11.
    pub(crate) fn has_long_header(&self) -> bool {
        !matches!(self.plain_header, ProtectedHeader::Short { .. })
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
    /// 3. An Initial packet header
    Initial(ProtectedInitialHeader),
    /// 4. A Retry packet header
    Retry {
        /// Destination Connection ID
        dst_cid: ConnectionId,
        /// Source Connection ID
        src_cid: ConnectionId,
        /// QUIC version
        version: u32,
    },
    /// 5. A Long packet header, as used during the handshake
    Long {
        /// Type of the Long header packet
        ty: LongType,
        /// Destination Connection ID
        dst_cid: ConnectionId,
        /// Source Connection ID
        src_cid: ConnectionId,
        /// Length of the packet payload
        len: u64,
        /// QUIC version
        version: u32,
    },
}

impl ProtectedHeader {
    /// 1. Decode a plain header from given buffer, with given [`ConnectionIdParser`].
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

            match LongHeaderType::from_byte(first)? {
                LongHeaderType::Initial => {
                    let token_len = buf.get_var()? as usize;
                    let token_start = buf.position() as usize;
                    if token_len > buf.remaining() {
                        return Err(PacketDecodeError::InvalidHeader("token out of bounds"));
                    }
                    buf.advance(token_len);

                    let len = buf.get_var()?;
                    Ok(Self::Initial(ProtectedInitialHeader {
                        dst_cid,
                        src_cid,
                        token_pos: token_start..token_start + token_len,
                        len,
                        version,
                    }))
                }
                LongHeaderType::Retry => Ok(Self::Retry {
                    dst_cid,
                    src_cid,
                    version,
                }),
                LongHeaderType::Standard(ty) => Ok(Self::Long {
                    ty,
                    dst_cid,
                    src_cid,
                    len: buf.get_var()?,
                    version,
                }),
            }
        }
    }
    /// 2
    fn payload_len(&self) -> Option<u64> {
        use self::ProtectedHeader::*;
        match self {
            Initial(ProtectedInitialHeader { len, .. }) | Long { len, .. } => Some(*len),
            _ => None,
        }
    }

    /// 3. The destination Connection ID of the packet
    pub fn dst_cid(&self) -> &ConnectionId {
        use self::ProtectedHeader::*;
        match self {
            Initial(header) => &header.dst_cid,
            Long { dst_cid, .. } => dst_cid,
            Retry { dst_cid, .. } => dst_cid,
            Short { dst_cid, .. } => dst_cid,
            VersionNegotiate { dst_cid, .. } => dst_cid,
        }
    }
    /// 4
    fn as_initial(&self) -> Option<&ProtectedInitialHeader> {
        match self {
            Self::Initial(x) => Some(x),
            _ => None,
        }
    }
}

/// Long packet type including non-uniform cases
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LongHeaderType {
    Initial,
    Retry,
    Standard(LongType),
}

impl LongHeaderType {
    fn from_byte(b: u8) -> Result<Self, PacketDecodeError> {
        use self::{LongHeaderType::*, LongType::*};
        debug_assert!(b & LONG_HEADER_FORM != 0, "not a long packet");
        Ok(match (b & 0x30) >> 4 {
            0x0 => Initial,
            0x1 => Standard(ZeroRtt),
            0x2 => Standard(Handshake),
            0x3 => Retry,
            _ => unreachable!(),
        })
    }
}

impl From<LongHeaderType> for u8 {
    fn from(ty: LongHeaderType) -> Self {
        use self::{LongHeaderType::*, LongType::*};
        match ty {
            Initial => LONG_HEADER_FORM | FIXED_BIT,
            Standard(ZeroRtt) => LONG_HEADER_FORM | FIXED_BIT | (0x1 << 4),
            Standard(Handshake) => LONG_HEADER_FORM | FIXED_BIT | (0x2 << 4),
            Retry => LONG_HEADER_FORM | FIXED_BIT | (0x3 << 4),
        }
    }
}

/// Long packet types with uniform header structure
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongType {
    /// Handshake packet
    Handshake,
    /// 0-RTT packet
    ZeroRtt,
}

/// Header of an Initial packet, before decryption
#[derive(Clone, Debug)]
pub struct ProtectedInitialHeader {
    /// Destination Connection ID
    pub dst_cid: ConnectionId,
    /// Source Connection ID
    pub src_cid: ConnectionId,
    /// The position of a token in the packet buffer
    pub token_pos: Range<usize>,
    /// Length of the packet payload
    pub len: u64,
    /// QUIC version
    pub version: u32,
}

pub(crate) struct PartialEncode {
    pub(crate) start: usize,
    pub(crate) header_len: usize,
    // Packet number length, payload length needed
    pn: Option<(usize, bool)>,
}

impl PartialEncode {
    pub(crate) fn finish(
        self,
        buf: &mut [u8],
        header_crypto: &dyn crypto::HeaderKey,
        crypto: Option<(u64, &dyn crypto::PacketKey)>,
    ) {
        let Self { header_len, pn, .. } = self;
        let (pn_len, write_len) = match pn {
            Some((pn_len, write_len)) => (pn_len, write_len),
            None => return,
        };

        let pn_pos = header_len - pn_len;
        if write_len {
            let len = buf.len() - header_len + pn_len;
            assert!(len < 2usize.pow(14)); // Fits in reserved space
            let mut slice = &mut buf[pn_pos - 2..pn_pos];
            slice.put_u16(len as u16 | 0b01 << 14);
        }

        if let Some((number, crypto)) = crypto {
            crypto.encrypt(number, buf, header_len);
        }

        debug_assert!(
            pn_pos + 4 + header_crypto.sample_size() <= buf.len(),
            "packet must be padded to at least {} bytes for header protection sampling",
            pn_pos + 4 + header_crypto.sample_size()
        );
        header_crypto.encrypt(pn_pos, buf);
    }
}

/// Packet number space identifiers
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SpaceId {
    /// Unprotected packets, used to bootstrap the handshake
    Initial = 0,
    Handshake = 1,
    /// Application data space, used for 0-RTT and post-handshake/1-RTT packets
    Data = 2,
}

impl SpaceId {
    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Initial, Self::Handshake, Self::Data].iter().cloned()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InitialHeader {
    pub(crate) dst_cid: ConnectionId,
    pub(crate) src_cid: ConnectionId,
    pub(crate) token: Bytes,
    pub(crate) number: PacketNumber,
    pub(crate) version: u32,
}

pub(crate) struct InitialPacket {
    /// 1.
    pub(crate) header: InitialHeader,
}

pub(crate) const FIXED_BIT: u8 = 0x40;
pub(crate) const LONG_HEADER_FORM: u8 = 0x80;
pub(crate) const SPIN_BIT: u8 = 0x20;
const KEY_PHASE_BIT: u8 = 0x04;
const SHORT_RESERVED_BITS: u8 = 0x18;
const LONG_RESERVED_BITS: u8 = 0x0c;

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
