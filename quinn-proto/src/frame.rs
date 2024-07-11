use std::{fmt, io};

use bytes::Bytes;

use crate::TransportError;
/// 1
#[derive(Clone, Debug)]
pub enum Close {}

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
pub struct ConnectionClose {}

impl fmt::Display for ConnectionClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Reason given by an application for closing the connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationClose {}

impl fmt::Display for ApplicationClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
