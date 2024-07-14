use std::time::Duration;

use rand::{Rng, RngCore};

use crate::shared::ConnectionId;
use crate::MAX_CID_SIZE;

/// Generates connection IDs for incoming connections
pub trait ConnectionIdGenerator: Send {
    /// Generates a new CID
    ///
    /// Connection IDs MUST NOT contain any information that can be used by
    /// an external observer (that is, one that does not cooperate with the
    /// issuer) to correlate them with other connection IDs for the same
    /// connection.
    fn generate_cid(&mut self) -> ConnectionId;
    /// Returns the length of a CID for connections created by this generator
    fn cid_len(&self) -> usize;
    /// Returns the lifetime of generated Connection IDs
    ///
    /// Connection IDs will be retired after the returned `Duration`, if any. Assumed to be constant.
    fn cid_lifetime(&self) -> Option<Duration>;
}

/// Generates purely random connection IDs of a certain length
#[derive(Debug, Clone, Copy)]
pub struct RandomConnectionIdGenerator {
    cid_len: usize,
    lifetime: Option<Duration>,
}

impl Default for RandomConnectionIdGenerator {
    fn default() -> Self {
        Self {
            cid_len: 8,
            lifetime: None,
        }
    }
}

impl RandomConnectionIdGenerator {
    /// Initialize Random CID generator with a fixed CID length
    ///
    /// The given length must be less than or equal to MAX_CID_SIZE.
    pub fn new(cid_len: usize) -> Self {
        debug_assert!(cid_len <= MAX_CID_SIZE);
        Self {
            cid_len,
            ..Self::default()
        }
    }

    /// Set the lifetime of CIDs created by this generator
    pub fn set_lifetime(&mut self, d: Duration) -> &mut Self {
        self.lifetime = Some(d);
        self
    }
}

impl ConnectionIdGenerator for RandomConnectionIdGenerator {
    fn generate_cid(&mut self) -> ConnectionId {
        let mut bytes_arr = [0; MAX_CID_SIZE];
        rand::thread_rng().fill_bytes(&mut bytes_arr[..self.cid_len]);

        ConnectionId::new(&bytes_arr[..self.cid_len])
    }

    /// Provide the length of dst_cid in short header packet
    fn cid_len(&self) -> usize {
        self.cid_len
    }

    fn cid_lifetime(&self) -> Option<Duration> {
        self.lifetime
    }
}

/// Generates 8-byte connection IDs that can be efficiently
/// [`validate`](ConnectionIdGenerator::validate)d
///
/// This generator uses a non-cryptographic hash and can therefore still be spoofed, but nonetheless
/// helps prevents Quinn from responding to non-QUIC packets at very low cost.
pub struct HashedConnectionIdGenerator {
    key: u64,
    lifetime: Option<Duration>,
}

impl HashedConnectionIdGenerator {
    /// Create a generator with a random key
    pub fn new() -> Self {
        Self::from_key(rand::thread_rng().gen())
    }

    /// Create a generator with a specific key
    ///
    /// Allows [`validate`](ConnectionIdGenerator::validate) to recognize a consistent set of
    /// connection IDs across restarts
    pub fn from_key(key: u64) -> Self {
        Self {
            key,
            lifetime: None,
        }
    }
}

impl Default for HashedConnectionIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionIdGenerator for HashedConnectionIdGenerator {
    fn generate_cid(&mut self) -> ConnectionId {
        todo!()
    }

    fn cid_len(&self) -> usize {
        NONCE_LEN + SIGNATURE_LEN
    }

    fn cid_lifetime(&self) -> Option<Duration> {
        todo!()
    }
}

const NONCE_LEN: usize = 3; // Good for more than 16 million connections
const SIGNATURE_LEN: usize = 8 - NONCE_LEN; // 8-byte total CID length
