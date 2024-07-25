use std::{net::SocketAddr, time::SystemTime};

use crate::{
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
        todo!()
    }
}
