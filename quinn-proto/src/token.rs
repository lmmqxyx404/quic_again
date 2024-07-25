use std::{
    net::{IpAddr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::BufMut;

use crate::{
    coding::BufMutExt,
    crypto::{HandshakeTokenKey, HmacKey},
    shared::ConnectionId,
    RESET_TOKEN_SIZE,
};

/// Stateless reset token
///
/// Used for an endpoint to securely communicate that it has lost state for a connection.
#[allow(clippy::derived_hash_with_manual_eq)] // Custom PartialEq impl matches derived semantics
#[derive(Debug, Copy, Clone, Hash)]
pub(crate) struct ResetToken([u8; RESET_TOKEN_SIZE]);

impl ResetToken {
    pub(crate) fn new(key: &dyn HmacKey, id: &ConnectionId) -> Self {
        let mut signature = vec![0; key.signature_len()];
        key.sign(id, &mut signature);
        // TODO: Server ID??
        let mut result = [0; RESET_TOKEN_SIZE];
        result.copy_from_slice(&signature[..RESET_TOKEN_SIZE]);
        result.into()
    }
}

impl PartialEq for ResetToken {
    fn eq(&self, other: &Self) -> bool {
        crate::constant_time::eq(&self.0, &other.0)
    }
}

impl Eq for ResetToken {}

impl From<[u8; RESET_TOKEN_SIZE]> for ResetToken {
    fn from(x: [u8; RESET_TOKEN_SIZE]) -> Self {
        Self(x)
    }
}
/// 用于 `w.put_slice(x);`
impl std::ops::Deref for ResetToken {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) struct RetryToken {
    /// The destination connection ID set in the very first packet from the client
    pub(crate) orig_dst_cid: ConnectionId,
    /// The time at which this token was issued
    pub(crate) issued: SystemTime,
}

impl RetryToken {
    pub(crate) fn encode(
        &self,
        key: &dyn HandshakeTokenKey,
        address: &SocketAddr,
        retry_src_cid: &ConnectionId,
    ) -> Vec<u8> {
        let aead_key = key.aead_from_hkdf(retry_src_cid);

        let mut buf = Vec::new();
        encode_addr(&mut buf, address);
        self.orig_dst_cid.encode_long(&mut buf);
        buf.write::<u64>(
            self.issued
                .duration_since(UNIX_EPOCH)
                .map(|x| x.as_secs())
                .unwrap_or(0),
        );
        aead_key.seal(&mut buf, &[]).unwrap();

        buf
    }

    pub(crate) fn from_bytes(
        key: &dyn HandshakeTokenKey,
        address: &SocketAddr,
        retry_src_cid: &ConnectionId,
        raw_token_bytes: &[u8],
    ) -> Result<Self, TokenDecodeError> {
        todo!()
    }
}

fn encode_addr(buf: &mut Vec<u8>, address: &SocketAddr) {
    match address.ip() {
        IpAddr::V4(x) => {
            buf.put_u8(0);
            buf.put_slice(&x.octets());
        }
        IpAddr::V6(x) => {
            buf.put_u8(1);
            buf.put_slice(&x.octets());
        }
    }
    buf.put_u16(address.port());
}

/// Reasons why a retry token might fail to validate a client's address
#[derive(Debug, Copy, Clone)]
pub(crate) enum TokenDecodeError {
    /// 1. Token was not recognized. It should be silently ignored.
    UnknownToken,
}
