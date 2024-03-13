use bytes::{Buf, BufMut};
use thiserror::Error;

// use crate::coding::BufMutExt;

use crate::coding::{self, BufExt, BufMutExt};
/// 1.
// An encoded packet number
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
}

#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PacketDecodeError {
    /* todo2
    #[error("unsupported version {version:x}")]
    UnsupportedVersion {
        src_cid: ConnectionId,
        dst_cid: ConnectionId,
        version: u32,
    }, */
    #[error("invalid header: {0}")]
    InvalidHeader(&'static str),
}

// the impl is forst used for decode r.get()? parse
impl From<coding::UnexpectedEnd> for PacketDecodeError {
    fn from(_: coding::UnexpectedEnd) -> Self {
        Self::InvalidHeader("unexpected end of packet")
    }
}

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

}
