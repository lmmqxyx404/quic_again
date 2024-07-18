use std::sync::Arc;

use bytes::BytesMut;

use crate::{
    endpoint::ConnectError, shared::ConnectionId, transport_parameters::TransportParameters, Side,
};

/// 1. Cryptography interface based on *ring*
#[cfg(feature = "ring")]
pub(crate) mod ring;
/// TLS interface based on rustls
#[cfg(feature = "rustls")]
pub mod rustls;

/// 3. A cryptographic session (commonly TLS)
pub trait Session: Send + Sync + 'static {
    /// 1. Create the initial set of keys given the client's initial destination ConnectionId
    fn initial_keys(&self, dst_cid: &ConnectionId, side: Side) -> Keys;
    /// 2. Writes handshake bytes into the given buffer and optionally returns the negotiated keys
    ///
    /// When the handshake proceeds to the next phase, this method will return a new set of
    /// keys to encrypt data with.
    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys>;
    /// 3. Get the 0-RTT keys if available (clients only)
    ///
    /// On the client side, this method can be used to see if 0-RTT key material is available
    /// to start sending data before the protocol handshake has completed.
    ///
    /// Returns `None` if the key material is not available. This might happen if you have
    /// not connected to this server before.
    fn early_crypto(&self) -> Option<(Box<dyn HeaderKey>, Box<dyn PacketKey>)>;
}

/// 1. A key for signing with HMAC-based algorithms
pub trait HmacKey: Send + Sync {
    /// Method for signing a message
    fn sign(&self, data: &[u8], signature_out: &mut [u8]);
    /// Length of `sign`'s output
    fn signature_len(&self) -> usize;
    /// Method for verifying a message
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), CryptoError>;
}

/// 1. Generic crypto errors
#[derive(Debug)]
pub struct CryptoError;

/// 2. Client-side configuration for the crypto protocol
pub trait ClientConfig: Send + Sync {
    // Start a client session with this configuration
    fn start_session(
        self: Arc<Self>,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Box<dyn Session>, ConnectError>;
}

/// 2. Error indicating that the specified QUIC version is not supported
#[derive(Debug)]
pub struct UnsupportedVersion;

impl From<UnsupportedVersion> for ConnectError {
    fn from(_: UnsupportedVersion) -> Self {
        Self::UnsupportedVersion
    }
}

/// 3. A complete set of keys for a certain packet space
pub struct Keys {
    /// Header protection keys
    pub header: KeyPair<Box<dyn HeaderKey>>,
    /// Packet protection keys
    pub packet: KeyPair<Box<dyn PacketKey>>,
}

/// 4. A pair of keys for bidirectional communication
pub struct KeyPair<T> {
    /// Key for encrypting data
    pub local: T,
    /// Key for decrypting data
    pub remote: T,
}

/// 4. Keys used to protect packet payloads
pub trait PacketKey: Send + Sync {
    /// Encrypt the packet payload with the given packet number
    fn encrypt(&self, packet: u64, buf: &mut [u8], header_len: usize);
    /// Decrypt the packet payload with the given packet number
    fn decrypt(
        &self,
        packet: u64,
        header: &[u8],
        payload: &mut BytesMut,
    ) -> Result<(), CryptoError>;
    /// The length of the AEAD tag appended to packets on encryption
    fn tag_len(&self) -> usize;
    /// Maximum number of packets that may be sent using a single key
    fn confidentiality_limit(&self) -> u64;
    /// Maximum number of incoming packets that may fail decryption before the connection must be
    /// abandoned
    fn integrity_limit(&self) -> u64;
}

/// 5. Keys used to protect packet headers
pub trait HeaderKey: Send + Sync {
    /// Decrypt the given packet's header
    fn decrypt(&self, pn_offset: usize, packet: &mut [u8]);
    /// Encrypt the given packet's header
    fn encrypt(&self, pn_offset: usize, packet: &mut [u8]);
    /// The sample size used for this key's algorithm
    fn sample_size(&self) -> usize;
}

/// 6. Server-side configuration for the crypto protocol
pub trait ServerConfig: Send + Sync {
    /// 1. Create the initial set of keys given the client's initial destination ConnectionId
    fn initial_keys(
        &self,
        version: u32,
        dst_cid: &ConnectionId,
    ) -> Result<Keys, UnsupportedVersion>;
    /// 2. Start a server session with this configuration
    ///
    /// Never called if `initial_keys` rejected `version`.
    fn start_session(
        self: Arc<Self>,
        version: u32,
        params: &TransportParameters,
    ) -> Box<dyn Session>;
}

/// 7. A pseudo random key for HKDF
pub trait HandshakeTokenKey: Send + Sync {
    /// Derive AEAD using hkdf
    fn aead_from_hkdf(&self, random_bytes: &[u8]) -> Box<dyn AeadKey>;
}

/// 8. A key for sealing data with AEAD-based algorithms
pub trait AeadKey {}
