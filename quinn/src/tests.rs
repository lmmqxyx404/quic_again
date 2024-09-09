// #![cfg(feature = "rustls")]

use core::str;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
    time::Duration,
};

use rustls::{pki_types::PrivateKeyDer, RootCertStore};
use tokio::{
    runtime::{Builder, Runtime},
    time::Instant,
};
use tracing_subscriber::EnvFilter;

use crate::{endpoint::Endpoint, TokioRuntime};

use super::{ClientConfig, EndpointConfig, TransportConfig};

#[test]
fn handshake_timeout() {
    let _guard = subscribe();
    let runtime = rt_threaded();
    let client = {
        let _guard = runtime.enter();
        Endpoint::client(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap()
    };

    // Avoid NoRootAnchors error
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(cert.cert.into()).unwrap();

    let mut client_config = crate::ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();
    const IDLE_TIMEOUT: Duration = Duration::from_millis(500);
    let mut transport_config = crate::TransportConfig::default();
    transport_config
        .max_idle_timeout(Some(IDLE_TIMEOUT.try_into().unwrap()))
        .initial_rtt(Duration::from_millis(10));
    client_config.transport_config(Arc::new(transport_config));

    let start = Instant::now();
    runtime.block_on(async move {
        match client
            .connect_with(
                client_config,
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1),
                "localhost",
            )
            .unwrap()
            .await
        {
            Err(crate::ConnectionError::TimedOut) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
            Ok(_) => panic!("unexpected success"),
        }
    });
    let dt = start.elapsed();
    assert!(dt > IDLE_TIMEOUT && dt < 2 * IDLE_TIMEOUT);
}

fn subscribe() -> tracing::subscriber::DefaultGuard {
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(|| TestWriter)
        .finish();
    tracing::subscriber::set_default(sub)
}

#[tokio::test]
async fn close_endpoint() {
    let _guard = subscribe();

    // Avoid NoRootAnchors error
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(cert.cert.into()).unwrap();

    let mut endpoint =
        Endpoint::client(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    endpoint
        .set_default_client_config(ClientConfig::with_root_certificates(Arc::new(roots)).unwrap());

    let conn = endpoint
        .connect(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1234),
            "localhost",
        )
        .unwrap();

    tokio::spawn(async move {
        let _ = conn.await;
    });

    let conn = endpoint
        .connect(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1234),
            "localhost",
        )
        .unwrap();
    endpoint.close(0u32.into(), &[]);
    match conn.await {
        Err(crate::ConnectionError::LocallyClosed) => (),
        Err(e) => panic!("unexpected error: {e}"),
        Ok(_) => {
            panic!("unexpected success");
        }
    }
}

#[test]
fn local_addr() {
    let socket = UdpSocket::bind("[::1]:0").unwrap();
    let addr = socket.local_addr().unwrap();
    let runtime = rt_basic();
    let ep = {
        let _guard = runtime.enter();
        Endpoint::new(Default::default(), None, socket, Arc::new(TokioRuntime)).unwrap()
    };
    // println!("{:?}", addr);
    assert_eq!(
        addr,
        ep.local_addr()
            .expect("Could not obtain our local endpoint")
    );
}

#[test]
fn read_after_close() {
    let _guard = subscribe();
    let runtime = rt_basic();
    let endpoint = {
        let _guard = runtime.enter();
        endpoint()
    };

    const MSG: &[u8] = b"goodbye!";
    let endpoint2 = endpoint.clone();
    runtime.spawn(async move {
        let new_conn = endpoint2
            .accept()
            .await
            .expect("endpoint")
            .await
            .expect("connection");

        let mut s = new_conn.open_uni().await.unwrap();
        s.write_all(MSG).await.unwrap();
        s.finish().unwrap();
        todo!()
        // Wait for the stream to be closed, one way or another.
        // _ = s.stopped().await;
    });
    runtime.block_on(async move {
        let new_conn = endpoint
            .connect(endpoint.local_addr().unwrap(), "localhost")
            .unwrap()
            .await
            .expect("connect");
        todo!()
        /* tokio::time::sleep_until(Instant::now() + Duration::from_millis(100)).await;
        let mut stream = new_conn.accept_uni().await.expect("incoming streams");
        let msg = stream.read_to_end(usize::MAX).await.expect("read_to_end");
        assert_eq!(msg, MSG); */
    });
}

struct TestWriter;

impl std::io::Write for TestWriter {
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

fn rt_threaded() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}

fn rt_basic() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

/// Construct an endpoint suitable for connecting to itself
fn endpoint() -> Endpoint {
    EndpointFactory::new().endpoint()
}

/// Constructs endpoints suitable for connecting to themselves and each other
struct EndpointFactory {
    cert: rcgen::CertifiedKey,
    endpoint_config: EndpointConfig,
}

impl EndpointFactory {
    fn new() -> Self {
        Self {
            cert: rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap(),
            endpoint_config: EndpointConfig::default(),
        }
    }

    fn endpoint(&self) -> Endpoint {
        self.endpoint_with_config(TransportConfig::default())
    }

    fn endpoint_with_config(&self, transport_config: TransportConfig) -> Endpoint {
        let key = PrivateKeyDer::Pkcs8(self.cert.key_pair.serialize_der().into());
        let transport_config = Arc::new(transport_config);
        let mut server_config =
            crate::ServerConfig::with_single_cert(vec![self.cert.cert.der().clone()], key).unwrap();
        server_config.transport_config(transport_config.clone());

        let mut roots = rustls::RootCertStore::empty();
        roots.add(self.cert.cert.der().clone()).unwrap();
        let mut endpoint = Endpoint::new(
            self.endpoint_config.clone(),
            Some(server_config),
            UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap(),
            Arc::new(TokioRuntime),
        )
        .unwrap();
        let mut client_config = ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();
        client_config.transport_config(transport_config);
        endpoint.set_default_client_config(client_config);

        endpoint
    }
}
