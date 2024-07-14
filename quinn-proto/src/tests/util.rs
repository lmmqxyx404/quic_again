use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    net::{Ipv6Addr, SocketAddr, UdpSocket},
    ops::RangeFrom,
    str,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use lazy_static::lazy_static;
use rustls::{
    client::WebPkiServerVerifier,
    pki_types::{CertificateDer, PrivateKeyDer},
    KeyLogFile,
};
use tracing::{info, info_span};

use crate::{
    config::{ClientConfig, EndpointConfig, ServerConfig},
    connection::Connection,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    endpoint::ConnectionHandle,
    Endpoint,
};

pub(super) fn client_config() -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto()))
}

pub(super) fn client_crypto() -> QuicClientConfig {
    client_crypto_inner(None, None)
}

fn client_crypto_inner(
    certs: Option<Vec<CertificateDer<'static>>>,
    alpn: Option<Vec<Vec<u8>>>,
) -> QuicClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    for cert in certs.unwrap_or_else(|| vec![CERTIFIED_KEY.cert.der().clone()]) {
        roots.add(cert).unwrap();
    }
    let mut inner = QuicClientConfig::inner(
        WebPkiServerVerifier::builder(Arc::new(roots))
            .build()
            .unwrap(),
    );
    inner.key_log = Arc::new(KeyLogFile::new());
    if let Some(alpn) = alpn {
        inner.alpn_protocols = alpn;
    }
    inner.try_into().unwrap()
}

lazy_static! {
    pub static ref SERVER_PORTS: Mutex<RangeFrom<u16>> = Mutex::new(4433..);
    pub static ref CLIENT_PORTS: Mutex<RangeFrom<u16>> = Mutex::new(44433..);
    pub(crate) static ref CERTIFIED_KEY: rcgen::CertifiedKey =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
}

pub(super) fn subscribe() -> tracing::subscriber::DefaultGuard {
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(|| TestWriter)
        .finish();
    tracing::subscriber::set_default(sub)
}
struct TestWriter;
impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!(
            "{}",
            str::from_utf8(buf).expect("tried to log invalid UTF-8")
        );
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

pub(super) fn server_config() -> ServerConfig {
    ServerConfig::with_crypto(Arc::new(server_crypto()))
}

pub(super) fn server_crypto() -> QuicServerConfig {
    server_crypto_inner(None, None)
}

fn server_crypto_inner(
    identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    alpn: Option<Vec<Vec<u8>>>,
) -> QuicServerConfig {
    let (cert, key) = identity.unwrap_or_else(|| {
        (
            CERTIFIED_KEY.cert.der().clone(),
            PrivateKeyDer::Pkcs8(CERTIFIED_KEY.key_pair.serialize_der().into()),
        )
    });

    let mut config = QuicServerConfig::inner(vec![cert], key);
    if let Some(alpn) = alpn {
        config.alpn_protocols = alpn;
    }

    config.try_into().unwrap()
}

pub(super) struct Pair {
    pub(super) server: TestEndpoint,
    pub(super) client: TestEndpoint,
    /// Current time
    pub(super) time: Instant,
}

impl Pair {
    /// 1
    pub(super) fn new(endpoint_config: Arc<EndpointConfig>, server_config: ServerConfig) -> Self {
        let server = Endpoint::new(
            endpoint_config.clone(),
            Some(Arc::new(server_config)),
            true,
            None,
        );
        let client = Endpoint::new(endpoint_config, None, true, None);

        Self::new_from_endpoint(client, server)
    }
    /// 2
    pub(super) fn new_from_endpoint(client: Endpoint, server: Endpoint) -> Self {
        let server_addr = SocketAddr::new(
            Ipv6Addr::LOCALHOST.into(),
            SERVER_PORTS.lock().unwrap().next().unwrap(),
        );
        let client_addr = SocketAddr::new(
            Ipv6Addr::LOCALHOST.into(),
            CLIENT_PORTS.lock().unwrap().next().unwrap(),
        );
        let now = Instant::now();
        Self {
            server: TestEndpoint::new(server, server_addr),
            client: TestEndpoint::new(client, client_addr),
            time: now,
        }
    }
    /// 3
    pub(super) fn connect(&mut self) -> (ConnectionHandle, ConnectionHandle) {
        self.connect_with(client_config())
    }
    /// 4
    pub(super) fn connect_with(
        &mut self,
        config: ClientConfig,
    ) -> (ConnectionHandle, ConnectionHandle) {
        info!("connecting");
        let client_ch = self.begin_connect(config);
        self.drive();
        let server_ch = self.server.assert_accept();
        self.finish_connect(client_ch, server_ch);
        (client_ch, server_ch)
    }

    /// 5. Just start connecting the client
    pub(super) fn begin_connect(&mut self, config: ClientConfig) -> ConnectionHandle {
        let span = info_span!("client");
        let _guard = span.enter();

        let (client_ch, client_conn) = self
            .client
            .connect(self.time, config, self.server.addr, "localhost")
            .unwrap();
        self.client.connections.insert(client_ch, client_conn);
        client_ch
    }

    /// 6.Advance time until both connections are idle
    pub(super) fn drive(&mut self) {
        while self.step() {}
    }
    /// 7. Returns whether the connection is not idle
    pub(super) fn step(&mut self) -> bool {
        self.drive_client();
        self.drive_server();
        if self.client.is_idle() && self.server.is_idle() {
            return false;
        }

        todo!()
    }
    /// 8.
    fn finish_connect(&mut self, client_ch: ConnectionHandle, server_ch: ConnectionHandle) {
        todo!()
    }
    /// 9
    pub(super) fn drive_client(&mut self) {
        todo!()
    }
    /// 10
    pub(super) fn drive_server(&mut self) {
        todo!()
    }
}

impl Default for Pair {
    fn default() -> Self {
        Self::new(Default::default(), server_config())
    }
}

pub(super) struct TestEndpoint {
    /// 1
    pub(super) endpoint: Endpoint,
    /// 2
    pub(super) addr: SocketAddr,
    /// 3
    pub(super) connections: HashMap<ConnectionHandle, Connection>,
}

impl TestEndpoint {
    /// 1
    fn new(endpoint: Endpoint, addr: SocketAddr) -> Self {
        let socket = if env::var_os("SSLKEYLOGFILE").is_some() {
            let socket = UdpSocket::bind(addr).expect("failed to bind UDP socket");
            socket
                .set_read_timeout(Some(Duration::new(0, 10_000_000)))
                .unwrap();
            Some(socket)
        } else {
            None
        };
        Self {
            endpoint,
            addr,
            connections: HashMap::default(),
        }
    }
    /// 2
    pub(super) fn assert_accept(&mut self) -> ConnectionHandle {
        todo!()
    }
    /// 3
    fn is_idle(&self) -> bool {
        self.connections.values().all(|x| x.is_idle())
    }
}

impl ::std::ops::Deref for TestEndpoint {
    type Target = Endpoint;
    fn deref(&self) -> &Endpoint {
        &self.endpoint
    }
}

impl ::std::ops::DerefMut for TestEndpoint {
    fn deref_mut(&mut self) -> &mut Endpoint {
        &mut self.endpoint
    }
}
