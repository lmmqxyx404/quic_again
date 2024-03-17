use std::{fmt, num::TryFromIntError, sync::Arc, time::Duration};

use thiserror::Error;

#[cfg(feature = "ring")]
use rand::RngCore;




/// Global configuration for the endpoint, affecting all connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
pub struct EndpointConfig {
    pub(crate) reset_key: Arc<dyn HmacKey>,
    pub(crate) max_udp_payload_size: VarInt,
    /// CID generator factory
    ///
    /// Create a cid generator for local cid in Endpoint struct
    pub(crate) connection_id_generator_factory:
        Arc<dyn Fn() -> Box<dyn ConnectionIdGenerator> + Send + Sync>,
    pub(crate) supported_versions: Vec<u32>,
    pub(crate) grease_quic_bit: bool,
}

impl EndpointConfig {
    /// Create a default config with a particular `reset_key`
    pub fn new(reset_key: Arc<dyn HmacKey>) -> Self {
        let cid_factory: fn() -> Box<dyn ConnectionIdGenerator> =
            || Box::<RandomConnectionIdGenerator>::default();
        Self {
            reset_key,
            max_udp_payload_size: (1500u32 - 28).into(), // Ethernet MTU minus IP + UDP headers
            connection_id_generator_factory: Arc::new(cid_factory),
            supported_versions: DEFAULT_SUPPORTED_VERSIONS.to_vec(),
            grease_quic_bit: true,
        }
    }

    /// Supply a custom connection ID generator factory
    ///
    /// Called once by each `Endpoint` constructed from this configuration to obtain the CID
    /// generator which will be used to generate the CIDs used for incoming packets on all
    /// connections involving that  `Endpoint`. A custom CID generator allows applications to embed
    /// information in local connection IDs, e.g. to support stateless packet-level load balancers.
    ///
    /// `EndpointConfig::new()` applies a default random CID generator factory. This functions
    /// accepts any customized CID generator to reset CID generator factory that implements
    /// the `ConnectionIdGenerator` trait.
    pub fn cid_generator<F: Fn() -> Box<dyn ConnectionIdGenerator> + Send + Sync + 'static>(
        &mut self,
        factory: F,
    ) -> &mut Self {
        self.connection_id_generator_factory = Arc::new(factory);
        self
    }

    /// Private key used to send authenticated connection resets to peers who were
    /// communicating with a previous instance of this endpoint.
    pub fn reset_key(&mut self, key: Arc<dyn HmacKey>) -> &mut Self {
        self.reset_key = key;
        self
    }

    /// Maximum UDP payload size accepted from peers (excluding UDP and IP overhead).
    ///
    /// Must be greater or equal than 1200.
    ///
    /// Defaults to 1472, which is the largest UDP payload that can be transmitted in the typical
    /// 1500 byte Ethernet MTU. Deployments on links with larger MTUs (e.g. loopback or Ethernet
    /// with jumbo frames) can raise this to improve performance at the cost of a linear increase in
    /// datagram receive buffer size.
    pub fn max_udp_payload_size(&mut self, value: u16) -> Result<&mut Self, ConfigError> {
        if !(1200..=65_527).contains(&value) {
            return Err(ConfigError::OutOfBounds);
        }

        self.max_udp_payload_size = value.into();
        Ok(self)
    }

    /// Get the current value of `max_udp_payload_size`
    ///
    /// While most parameters don't need to be readable, this must be exposed to allow higher-level
    /// layers, e.g. the `quinn` crate, to determine how large a receive buffer to allocate to
    /// support an externally-defined `EndpointConfig`.
    ///
    /// While `get_` accessors are typically unidiomatic in Rust, we favor concision for setters,
    /// which will be used far more heavily.
    #[doc(hidden)]
    pub fn get_max_udp_payload_size(&self) -> u64 {
        self.max_udp_payload_size.into()
    }

    /// Override supported QUIC versions
    pub fn supported_versions(&mut self, supported_versions: Vec<u32>) -> &mut Self {
        self.supported_versions = supported_versions;
        self
    }

    /// Whether to accept QUIC packets containing any value for the fixed bit
    ///
    /// Enabled by default. Helps protect against protocol ossification and makes traffic less
    /// identifiable to observers. Disable if helping observers identify this traffic as QUIC is
    /// desired.
    pub fn grease_quic_bit(&mut self, value: bool) -> &mut Self {
        self.grease_quic_bit = value;
        self
    }
}

impl fmt::Debug for EndpointConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("EndpointConfig")
            .field("reset_key", &"[ elided ]")
            .field("max_udp_payload_size", &self.max_udp_payload_size)
            .field("cid_generator_factory", &"[ elided ]")
            .field("supported_versions", &self.supported_versions)
            .field("grease_quic_bit", &self.grease_quic_bit)
            .finish()
    }
}

#[cfg(feature = "ring")]
impl Default for EndpointConfig {
    fn default() -> Self {
        let mut reset_key = [0; 64];
        rand::thread_rng().fill_bytes(&mut reset_key);

        Self::new(Arc::new(ring::hmac::Key::new(
            ring::hmac::HMAC_SHA256,
            &reset_key,
        )))
    }
}

/// Parameters governing incoming connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
pub struct ServerConfig {
    /// Transport configuration to use for incoming connections
    pub transport: Arc<TransportConfig>,

    /// TLS configuration used for incoming connections.
    ///
    /// Must be set to use TLS 1.3 only.
    pub crypto: Arc<dyn crypto::ServerConfig>,

    /// Used to generate one-time AEAD keys to protect handshake tokens
    pub(crate) token_key: Arc<dyn HandshakeTokenKey>,

    /// Whether to require clients to prove ownership of an address before committing resources.
    ///
    /// Introduces an additional round-trip to the handshake to make denial of service attacks more difficult.
    pub(crate) use_retry: bool,
    /// Microseconds after a stateless retry token was issued for which it's considered valid.
    pub(crate) retry_token_lifetime: Duration,

    /// Maximum number of concurrent connections
    pub(crate) concurrent_connections: u32,

    /// Whether to allow clients to migrate to new addresses
    ///
    /// Improves behavior for clients that move between different internet connections or suffer NAT
    /// rebinding. Enabled by default.
    pub(crate) migration: bool,
}

impl ServerConfig {
    /// Create a default config with a particular handshake token key
    pub fn new(
        crypto: Arc<dyn crypto::ServerConfig>,
        token_key: Arc<dyn HandshakeTokenKey>,
    ) -> Self {
        Self {
            transport: Arc::new(TransportConfig::default()),
            crypto,

            token_key,
            use_retry: false,
            retry_token_lifetime: Duration::from_secs(15),

            concurrent_connections: 100_000,

            migration: true,
        }
    }

    /// Set a custom [`TransportConfig`]
    pub fn transport_config(&mut self, transport: Arc<TransportConfig>) -> &mut Self {
        self.transport = transport;
        self
    }

    /// Private key used to authenticate data included in handshake tokens.
    pub fn token_key(&mut self, value: Arc<dyn HandshakeTokenKey>) -> &mut Self {
        self.token_key = value;
        self
    }

    /// Whether to require clients to prove ownership of an address before committing resources.
    ///
    /// Introduces an additional round-trip to the handshake to make denial of service attacks more difficult.
    pub fn use_retry(&mut self, value: bool) -> &mut Self {
        self.use_retry = value;
        self
    }

    /// Duration after a stateless retry token was issued for which it's considered valid.
    pub fn retry_token_lifetime(&mut self, value: Duration) -> &mut Self {
        self.retry_token_lifetime = value;
        self
    }

    /// Maximum number of simultaneous connections to accept.
    ///
    /// New incoming connections are only accepted if the total number of incoming or outgoing
    /// connections is less than this. Outgoing connections are unaffected.
    pub fn concurrent_connections(&mut self, value: u32) -> &mut Self {
        self.concurrent_connections = value;
        self
    }

    /// Whether to allow clients to migrate to new addresses
    ///
    /// Improves behavior for clients that move between different internet connections or suffer NAT
    /// rebinding. Enabled by default.
    pub fn migration(&mut self, value: bool) -> &mut Self {
        self.migration = value;
        self
    }
}

#[cfg(feature = "rustls")]
impl ServerConfig {
    /// Create a server config with the given certificate chain to be presented to clients
    ///
    /// Uses a randomized handshake token key.
    pub fn with_single_cert(
        cert_chain: Vec<rustls::Certificate>,
        key: rustls::PrivateKey,
    ) -> Result<Self, rustls::Error> {
        let crypto = crypto::rustls::server_config(cert_chain, key)?;
        Ok(Self::with_crypto(Arc::new(crypto)))
    }
}

#[cfg(feature = "ring")]
impl ServerConfig {
    /// Create a server config with the given [`crypto::ServerConfig`]
    ///
    /// Uses a randomized handshake token key.
    pub fn with_crypto(crypto: Arc<dyn crypto::ServerConfig>) -> Self {
        let rng = &mut rand::thread_rng();
        let mut master_key = [0u8; 64];
        rng.fill_bytes(&mut master_key);
        let master_key = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, &[]).extract(&master_key);

        Self::new(crypto, Arc::new(master_key))
    }
}

impl fmt::Debug for ServerConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ServerConfig<T>")
            .field("transport", &self.transport)
            .field("crypto", &"ServerConfig { elided }")
            .field("token_key", &"[ elided ]")
            .field("use_retry", &self.use_retry)
            .field("retry_token_lifetime", &self.retry_token_lifetime)
            .field("concurrent_connections", &self.concurrent_connections)
            .field("migration", &self.migration)
            .finish()
    }
}

/// Configuration for outgoing connections
///
/// Default values should be suitable for most internet applications.
#[derive(Clone)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Transport configuration to use
    pub(crate) transport: Arc<TransportConfig>,

    /// Cryptographic configuration to use
    pub(crate) crypto: Arc<dyn crypto::ClientConfig>,

    /// QUIC protocol version to use
    pub(crate) version: u32,
}

impl ClientConfig {
    /// Create a default config with a particular cryptographic config
    pub fn new(crypto: Arc<dyn crypto::ClientConfig>) -> Self {
        Self {
            transport: Default::default(),
            crypto,
            version: 1,
        }
    }

    /// Set a custom [`TransportConfig`]
    pub fn transport_config(&mut self, transport: Arc<TransportConfig>) -> &mut Self {
        self.transport = transport;
        self
    }

    /// Set the QUIC version to use
    pub fn version(&mut self, version: u32) -> &mut Self {
        self.version = version;
        self
    }
}

#[cfg(feature = "rustls")]
impl ClientConfig {
    /// Create a client configuration that trusts the platform's native roots
    #[cfg(feature = "platform-verifier")]
    pub fn with_platform_verifier() -> Self {
        let mut cfg = rustls::ClientConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_custom_certificate_verifier(Arc::new(rustls_platform_verifier::Verifier::new()))
            .with_no_client_auth();
        cfg.enable_early_data = true;
        Self::new(Arc::new(cfg))
    }

    /// Create a client configuration that trusts specified trust anchors
    pub fn with_root_certificates(roots: rustls::RootCertStore) -> Self {
        Self::new(Arc::new(crypto::rustls::client_config(roots)))
    }
}

impl fmt::Debug for ClientConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ClientConfig<T>")
            .field("transport", &self.transport)
            .field("crypto", &"ClientConfig { elided }")
            .field("version", &self.version)
            .finish()
    }
}

/// Errors in the configuration of an endpoint
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// Value exceeds supported bounds
    #[error("value exceeds supported bounds")]
    OutOfBounds,
}

impl From<TryFromIntError> for ConfigError {
    fn from(_: TryFromIntError) -> Self {
        Self::OutOfBounds
    }
}

impl From<VarIntBoundsExceeded> for ConfigError {
    fn from(_: VarIntBoundsExceeded) -> Self {
        Self::OutOfBounds
    }
}

/// Maximum duration of inactivity to accept before timing out the connection.
///
/// This wraps an underlying [`VarInt`], representing the duration in milliseconds. Values can be
/// constructed by converting directly from `VarInt`, or using `TryFrom<Duration>`.
///
/// ```
/// # use std::{convert::TryFrom, time::Duration};
/// # use quinn_proto::{IdleTimeout, VarIntBoundsExceeded, VarInt};
/// # fn main() -> Result<(), VarIntBoundsExceeded> {
/// // A `VarInt`-encoded value in milliseconds
/// let timeout = IdleTimeout::from(VarInt::from_u32(10_000));
///
/// // Try to convert a `Duration` into a `VarInt`-encoded timeout
/// let timeout = IdleTimeout::try_from(Duration::from_secs(10))?;
/// # Ok(())
/// # }
/// ```
#[derive(Default, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdleTimeout(VarInt);

impl From<VarInt> for IdleTimeout {
    fn from(inner: VarInt) -> Self {
        Self(inner)
    }
}

impl std::convert::TryFrom<Duration> for IdleTimeout {
    type Error = VarIntBoundsExceeded;

    fn try_from(timeout: Duration) -> Result<Self, Self::Error> {
        let inner = VarInt::try_from(timeout.as_millis())?;
        Ok(Self(inner))
    }
}

impl fmt::Debug for IdleTimeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
