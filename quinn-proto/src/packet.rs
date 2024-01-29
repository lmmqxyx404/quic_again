use bytes::BufMut;

use crate::coding::BufMutExt;

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
}

mod tests {
    use super::PacketNumber;

    /// check packet number
    fn check_pn(typed: PacketNumber, encoded: &[u8]) {
        let mut buf = Vec::new();
        typed.encode(&mut buf);
        assert_eq!(&buf[..], encoded);
    }
}
