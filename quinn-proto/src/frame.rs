use std::{fmt, io, ops::RangeInclusive};

use bytes::{Buf, BufMut, Bytes};

use crate::{
    coding::{self, BufExt, BufMutExt},
    shared::ConnectionId,
    token::ResetToken,
    TransportError, VarInt,
};
/// 1
#[derive(Clone, Debug)]
pub enum Close {
    Connection(ConnectionClose),
}

impl From<TransportError> for Close {
    fn from(x: TransportError) -> Self {
        todo!()
    }
}

impl From<ConnectionClose> for Close {
    fn from(x: ConnectionClose) -> Self {
        todo!()
    }
}

impl From<ApplicationClose> for Close {
    fn from(x: ApplicationClose) -> Self {
        todo!()
    }
}
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Type(u64);

impl coding::Codec for Type {
    fn decode<B: Buf>(buf: &mut B) -> coding::Result<Self> {
        Ok(Self(buf.get_var()?))
    }
    fn encode<B: BufMut>(&self, buf: &mut B) {
        buf.write_var(self.0);
    }
}

pub(crate) struct Iter {
    // TODO: ditch io::Cursor after bytes 0.5
    bytes: io::Cursor<Bytes>,
    last_ty: Option<Type>,
}

impl Iter {
    pub(crate) fn new(payload: Bytes) -> Result<Self, TransportError> {
        if payload.is_empty() {
            // "An endpoint MUST treat receipt of a packet containing no frames as a
            // connection error of type PROTOCOL_VIOLATION."
            // https://www.rfc-editor.org/rfc/rfc9000.html#name-frames-and-frame-types
            todo!()
            // todo: very important
            /* return Err(TransportError::PROTOCOL_VIOLATION(
                "packet payload is empty",
            )); */
        }

        Ok(Self {
            bytes: io::Cursor::new(payload),
            last_ty: None,
        })
    }
}

impl Iterator for Iter {
    type Item = Result<Frame, InvalidFrame>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct InvalidFrame {
    // pub(crate) ty: Option<Type>,
    pub(crate) reason: &'static str,
}

impl From<InvalidFrame> for TransportError {
    fn from(err: InvalidFrame) -> Self {
        todo!()
        /* let mut te = Self::FRAME_ENCODING_ERROR(err.reason);
        te.frame = err.ty;
        te */
    }
}

#[derive(Debug)]
pub(crate) enum Frame {
    Padding,
    Ping,
}

/// Reason given by the transport for closing the connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionClose {
    /// 1. Human-readable reason for the close
    pub reason: Bytes,
    // pub error_code: TransportErrorCode,
}

impl fmt::Display for ConnectionClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // todo
        if !self.reason.as_ref().is_empty() {
            f.write_str(": ")?;
            f.write_str(&String::from_utf8_lossy(&self.reason))?;
        }
        Ok(())
    }
}

impl FrameStruct for ConnectionClose {
    const SIZE_BOUND: usize = 1 + 8 + 8 + 8;
}

/// Reason given by an application for closing the connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationClose {}

impl fmt::Display for ApplicationClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Crypto {
    pub(crate) offset: u64,
    pub(crate) data: Bytes,
}

impl Crypto {
    pub(crate) const SIZE_BOUND: usize = 17;

    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(Type::CRYPTO);
        out.write_var(self.offset);
        out.write_var(self.data.len() as u64);
        out.put_slice(&self.data);
    }
}
pub(crate) trait FrameStruct {
    /// Smallest number of bytes this type of frame is guaranteed to fit within.
    const SIZE_BOUND: usize;
}

macro_rules! frame_types {
    {$($name:ident = $val:expr,)*} => {
        impl Type {
            $(pub const $name: Type = Type($val);)*
        }

        impl fmt::Debug for Type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    $($val => f.write_str(stringify!($name)),)*
                    _ => write!(f, "Type({:02x})", self.0)
                }
            }
        }

        impl fmt::Display for Type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    $($val => f.write_str(stringify!($name)),)*
                    x if STREAM_TYS.contains(&x) => f.write_str("STREAM"),
                    x if DATAGRAM_TYS.contains(&x) => f.write_str("DATAGRAM"),
                    _ => write!(f, "<unknown {:02x}>", self.0),
                }
            }
        }
    }
}

frame_types! {
    PATH_RESPONSE = 0x1b,
    HANDSHAKE_DONE = 0x1e,
    IMMEDIATE_ACK = 0x1f,
    // ACK Frequency
    ACK_FREQUENCY = 0xaf,
    PATH_CHALLENGE = 0x1a,
    CRYPTO = 0x06,
    NEW_CONNECTION_ID = 0x18,
    RETIRE_CONNECTION_ID = 0x19,
}

const STREAM_TYS: RangeInclusive<u64> = RangeInclusive::new(0x08, 0x0f);
const DATAGRAM_TYS: RangeInclusive<u64> = RangeInclusive::new(0x30, 0x31);

/// An unreliable datagram
#[derive(Debug, Clone)]
pub struct Datagram {
    /// Payload
    pub data: Bytes,
}

impl FrameStruct for Datagram {
    const SIZE_BOUND: usize = 1 + 8;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct AckFrequency {
    pub(crate) sequence: VarInt,
    pub(crate) ack_eliciting_threshold: VarInt,
    pub(crate) request_max_ack_delay: VarInt,
    pub(crate) reordering_threshold: VarInt,
}

impl AckFrequency {
    pub(crate) fn encode<W: BufMut>(&self, buf: &mut W) {
        buf.write(Type::ACK_FREQUENCY);
        buf.write(self.sequence);
        buf.write(self.ack_eliciting_threshold);
        buf.write(self.request_max_ack_delay);
        buf.write(self.reordering_threshold);
    }
}
#[derive(Debug, Copy, Clone)]
pub(crate) struct NewConnectionId {
    pub(crate) sequence: u64,
    pub(crate) retire_prior_to: u64,
    pub(crate) id: ConnectionId,
    pub(crate) reset_token: ResetToken,
}

impl NewConnectionId {
    pub(crate) fn encode<W: BufMut>(&self, out: &mut W) {
        out.write(Type::NEW_CONNECTION_ID);
        out.write_var(self.sequence);
        out.write_var(self.retire_prior_to);
        out.write(self.id.len() as u8);
        out.put_slice(&self.id);
        out.put_slice(&self.reset_token);
    }
}

/// Smallest number of bytes this type of frame is guaranteed to fit within.
pub(crate) const RETIRE_CONNECTION_ID_SIZE_BOUND: usize = 9;
