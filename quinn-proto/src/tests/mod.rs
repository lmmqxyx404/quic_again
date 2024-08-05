use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    cid_generator::HashedConnectionIdGenerator,
    config::EndpointConfig,
    connection::{ConnectionError, Event, FinishError, ReadError, StreamEvent, WriteError},
    endpoint::DatagramEvent,
    frame::ApplicationClose,
    ConnectionIdGenerator, Dir, Endpoint, RandomConnectionIdGenerator, Transmit, VarInt,
    DEFAULT_SUPPORTED_VERSIONS,
};

mod util;
use super::*;
use assert_matches::assert_matches;
use bytes::Bytes;
use config::{ClientConfig, ServerConfig, TransportConfig};
use crypto::rustls::QuicServerConfig;
use frame::ConnectionClose;
use hex_literal::hex;
use rand::RngCore;
use ring::hmac;
use rustls::{
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
    server::WebPkiClientVerifier,
    AlertDescription, RootCertStore,
};
use tracing::info;
use util::*;

#[test]
fn version_negotiate_server() {
    let _guard = subscribe();
    let client_addr = "[::2]:7890".parse().unwrap();
    let mut server = Endpoint::new(
        Default::default(),
        Some(Arc::new(server_config())),
        true,
        None,
    );
    let now = Instant::now();
    let mut buf = Vec::with_capacity(server.config().get_max_udp_payload_size() as usize);
    let event = server.handle(
        now,
        client_addr,
        None,
        None,
        // Long-header packet with reserved version number
        hex!("80 0a1a2a3a 04 00000000 04 00000000 00")[..].into(),
        &mut buf,
    );

    let Some(DatagramEvent::Response(Transmit { .. })) = event else {
        panic!("expected a response");
    };

    assert_ne!(buf[0] & 0x80, 0);
    assert_eq!(&buf[1..15], hex!("00000000 04 00000000 04 00000000"));
    assert!(buf[15..].chunks(4).any(|x| {
        DEFAULT_SUPPORTED_VERSIONS.contains(&u32::from_be_bytes(x.try_into().unwrap()))
    }));
}

#[test]
fn version_negotiate_client() {
    let _guard = subscribe();
    let server_addr = "[::2]:7890".parse().unwrap();
    // Configure client to use empty CIDs so we can easily hardcode a server version negotiation
    // packet
    let cid_generator_factory: fn() -> Box<dyn ConnectionIdGenerator> =
        || Box::new(RandomConnectionIdGenerator::new(0));
    let mut client = Endpoint::new(
        Arc::new(EndpointConfig {
            connection_id_generator_factory: Arc::new(cid_generator_factory),
            ..Default::default()
        }),
        None,
        true,
        None,
    );
    let (_, mut client_ch) = client
        .connect(Instant::now(), client_config(), server_addr, "localhost")
        .unwrap();
    let now = Instant::now();
    let mut buf: Vec<u8> = Vec::with_capacity(client.config().get_max_udp_payload_size() as usize);

    let opt_event = client.handle(
        now,
        server_addr,
        None,
        None,
        // Version negotiation packet for reserved version, with empty DCID
        hex!(
            "80 00000000 00 04 00000000
             0a1a2a3a"
        )[..]
            .into(),
        &mut buf,
    );
    if let Some(DatagramEvent::ConnectionEvent(_, event)) = opt_event {
        client_ch.handle_event(event);
    }

    assert_matches!(
        client_ch.poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::VersionMismatch,
        })
    );
}

#[test]
fn lifecycle() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert!(pair.client_conn_mut(client_ch).using_ecn());
    assert!(pair.server_conn_mut(server_ch).using_ecn());

    const REASON: &[u8] = b"whee";
    info!("closing");
    pair.client.connections.get_mut(&client_ch).unwrap().close(
        pair.time,
        VarInt(42),
        REASON.into(),
    );
    // note, 主动更新状态
    pair.drive();
    assert_matches!(pair.server_conn_mut(server_ch).poll(),
    Some(Event::ConnectionLost { reason: ConnectionError::ApplicationClosed(
        ApplicationClose { error_code: VarInt(42), ref reason }
    )}) if reason == REASON);
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert_eq!(pair.client.known_connections(), 0);
    assert_eq!(pair.client.known_cids(), 0);
    assert_eq!(pair.server.known_connections(), 0);
    assert_eq!(pair.server.known_cids(), 0);
}

#[test]
fn draft_version_compat() {
    let _guard = subscribe();

    let mut client_config = client_config();
    client_config.version(0xff00_0020);
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect_with(client_config);

    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert!(pair.client_conn_mut(client_ch).using_ecn());
    assert!(pair.server_conn_mut(server_ch).using_ecn());

    const REASON: &[u8] = b"whee";
    info!("closing");
    pair.client.connections.get_mut(&client_ch).unwrap().close(
        pair.time,
        VarInt(42),
        REASON.into(),
    );
    pair.drive();
    assert_matches!(pair.server_conn_mut(server_ch).poll(),
                    Some(Event::ConnectionLost { reason: ConnectionError::ApplicationClosed(
                        ApplicationClose { error_code: VarInt(42), ref reason }
                    )}) if reason == REASON);
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert_eq!(pair.client.known_connections(), 0);
    assert_eq!(pair.client.known_cids(), 0);
    assert_eq!(pair.server.known_connections(), 0);
    assert_eq!(pair.server.known_cids(), 0);
}

#[test]
fn stateless_retry() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    pair.server.incoming_connection_behavior = IncomingConnectionBehavior::Validate;
    pair.connect();
}

#[test]
fn server_stateless_reset() {
    let _guard = subscribe();
    let mut key_material = vec![0; 64];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut key_material);
    let reset_key = hmac::Key::new(hmac::HMAC_SHA256, &key_material);
    rng.fill_bytes(&mut key_material);

    let mut endpoint_config = EndpointConfig::new(Arc::new(reset_key));
    endpoint_config.cid_generator(move || Box::new(HashedConnectionIdGenerator::from_key(0)));
    let endpoint_config = Arc::new(endpoint_config);

    let mut pair = Pair::new(endpoint_config.clone(), server_config());
    let (client_ch, _) = pair.connect();
    pair.drive(); // Flush any post-handshake frames
    pair.server.endpoint =
        Endpoint::new(endpoint_config, Some(Arc::new(server_config())), true, None);

    // Force the server to generate the smallest possible stateless reset
    pair.client.connections.get_mut(&client_ch).unwrap().ping();
    info!("resetting");
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::Reset
        })
    );
}

#[test]
fn client_stateless_reset() {
    let _guard = subscribe();
    let mut key_material = vec![0; 64];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut key_material);
    let reset_key = hmac::Key::new(hmac::HMAC_SHA256, &key_material);
    rng.fill_bytes(&mut key_material);

    let mut endpoint_config = EndpointConfig::new(Arc::new(reset_key));
    endpoint_config.cid_generator(move || Box::new(HashedConnectionIdGenerator::from_key(0)));
    let endpoint_config = Arc::new(endpoint_config);

    let mut pair = Pair::new(endpoint_config.clone(), server_config());
    let (_, server_ch) = pair.connect();
    pair.client.endpoint =
        Endpoint::new(endpoint_config, Some(Arc::new(server_config())), true, None);
    // Send something big enough to allow room for a smaller stateless reset.
    pair.server.connections.get_mut(&server_ch).unwrap().close(
        pair.time,
        VarInt(42),
        (&[0xab; 128][..]).into(),
    );
    info!("resetting");
    pair.drive();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::Reset
        })
    );
}

/// Verify that stateless resets are rate-limited
#[test]
fn stateless_reset_limit() {
    let _guard = subscribe();
    let remote = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 42);
    let mut endpoint_config = EndpointConfig::default();
    endpoint_config.cid_generator(move || Box::new(RandomConnectionIdGenerator::new(8)));
    let endpoint_config = Arc::new(endpoint_config);
    let mut endpoint = Endpoint::new(
        endpoint_config.clone(),
        Some(Arc::new(server_config())),
        true,
        None,
    );
    let time = Instant::now();
    let mut buf = Vec::new();
    let event = endpoint.handle(time, remote, None, None, [0u8; 1024][..].into(), &mut buf);
    assert!(matches!(event, Some(DatagramEvent::Response(_))));
    let event = endpoint.handle(time, remote, None, None, [0u8; 1024][..].into(), &mut buf);
    assert!(event.is_none());
    let event = endpoint.handle(
        time + endpoint_config.min_reset_interval - Duration::from_nanos(1),
        remote,
        None,
        None,
        [0u8; 1024][..].into(),
        &mut buf,
    );
    assert!(event.is_none());
    let event = endpoint.handle(
        time + endpoint_config.min_reset_interval,
        remote,
        None,
        None,
        [0u8; 1024][..].into(),
        &mut buf,
    );
    assert!(matches!(event, Some(DatagramEvent::Response(_))));
}

#[test]
fn export_keying_material() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();

    const LABEL: &[u8] = b"test_label";
    const CONTEXT: &[u8] = b"test_context";

    // client keying material
    let mut client_buf = [0u8; 64];
    pair.client_conn_mut(client_ch)
        .crypto_session()
        .export_keying_material(&mut client_buf, LABEL, CONTEXT)
        .unwrap();

    // server keying material
    let mut server_buf = [0u8; 64];
    pair.server_conn_mut(server_ch)
        .crypto_session()
        .export_keying_material(&mut server_buf, LABEL, CONTEXT)
        .unwrap();

    assert_eq!(&client_buf[..], &server_buf[..]);
}

#[test]
fn finish_stream_simple() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();

    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();

    const MSG: &[u8] = b"hello";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    assert_eq!(pair.client_streams(client_ch).send_streams(), 1);
    pair.client_send(client_ch, s).finish().unwrap();
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Stream(StreamEvent::Finished { id })) if id == s
    );
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert_eq!(pair.client_streams(client_ch).send_streams(), 0);
    assert_eq!(pair.server_conn_mut(client_ch).streams().send_streams(), 0);
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    // Receive-only streams do not get `StreamFinished` events
    assert_eq!(pair.server_conn_mut(client_ch).streams().send_streams(), 0);
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Uni), Some(stream) if stream == s);
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();

    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG
    );

    assert_matches!(chunks.next(usize::MAX), Ok(None));
    let _ = chunks.finalize();
}

#[test]
fn reset_stream() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();

    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();

    const MSG: &[u8] = b"hello";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    pair.drive();

    info!("resetting stream");
    const ERROR: VarInt = VarInt(42);
    pair.client_send(client_ch, s).reset(ERROR).unwrap();

    pair.drive();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );

    assert_matches!(pair.server_streams(server_ch).accept(Dir::Uni), Some(stream) if stream == s);
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(chunks.next(usize::MAX), Err(ReadError::Reset(ERROR)));
    let _ = chunks.finalize();
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
}

#[test]
fn stop_stream() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();

    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"hello";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    pair.drive();

    info!("stopping stream");
    const ERROR: VarInt = VarInt(42);
    pair.server_recv(server_ch, s).stop(ERROR).unwrap();
    pair.drive();

    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Uni), Some(stream) if stream == s);

    assert_matches!(
        pair.client_send(client_ch, s).write(b"foo"),
        Err(WriteError::Stopped(ERROR))
    );

    assert_matches!(
        pair.client_send(client_ch, s).finish(),
        Err(FinishError::Stopped(ERROR))
    );
}

#[test]
fn reject_self_signed_server_cert() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    info!("connecting");

    // Create a self-signed certificate with a different distinguished name than the default one,
    // such that path building cannot confuse the default root the server is using and the one
    // the client is trusting (in which case we'd get a different error).
    let mut cert = rcgen::CertificateParams::new(["localhost".into()]).unwrap();
    let mut issuer = rcgen::DistinguishedName::new();
    issuer.push(
        rcgen::DnType::OrganizationName,
        "Crazy Quinn's House of Certificates",
    );
    cert.distinguished_name = issuer;
    let cert = cert
        .self_signed(&rcgen::KeyPair::generate().unwrap())
        .unwrap();
    let client_ch = pair.begin_connect(client_config_with_certs(vec![cert.into()]));

    pair.drive();

    assert_matches!(pair.client_conn_mut(client_ch).poll(),
                    Some(Event::ConnectionLost { reason: ConnectionError::TransportError(ref error)})
                    if error.code == TransportErrorCode::crypto(AlertDescription::UnknownCA.into()));
}

#[test]
fn reject_missing_client_cert() {
    let _guard = subscribe();

    let mut store = RootCertStore::empty();
    // `WebPkiClientVerifier` requires a non-empty store, so we stick our own certificate into it
    // because it's convenient.
    store.add(CERTIFIED_KEY.cert.der().clone()).unwrap();

    let key = PrivatePkcs8KeyDer::from(CERTIFIED_KEY.key_pair.serialize_der());
    let cert = CERTIFIED_KEY.cert.der().clone();

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_client_cert_verifier(
            WebPkiClientVerifier::builder_with_provider(Arc::new(store), provider)
                .build()
                .unwrap(),
        )
        .with_single_cert(vec![cert], PrivateKeyDer::from(key))
        .unwrap();
    let config = QuicServerConfig::try_from(config).unwrap();

    let mut pair = Pair::new(
        Default::default(),
        ServerConfig::with_crypto(Arc::new(config)),
    );

    info!("connecting");
    let client_ch = pair.begin_connect(client_config());
    pair.drive();

    // The client completes the connection, but finds it immediately closed
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Connected)
    );
    assert_matches!(pair.client_conn_mut(client_ch).poll(),
                    Some(Event::ConnectionLost { reason: ConnectionError::ConnectionClosed(ref close)})
                    if close.error_code == TransportErrorCode::crypto(AlertDescription::CertificateRequired.into()));

    // The server never completes the connection
    let server_ch = pair.server.assert_accept();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(pair.server_conn_mut(server_ch).poll(),
                    Some(Event::ConnectionLost { reason: ConnectionError::TransportError(ref error)})
                    if error.code == TransportErrorCode::crypto(AlertDescription::CertificateRequired.into()));
}

#[test]
fn congestion() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, _) = pair.connect();

    const TARGET: u64 = 2048;
    assert!(pair.client_conn_mut(client_ch).congestion_window() > TARGET);
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    // Send data without receiving ACKs until the congestion state falls below target
    while pair.client_conn_mut(client_ch).congestion_window() > TARGET {
        let n = pair.client_send(client_ch, s).write(&[42; 1024]).unwrap();
        assert_eq!(n, 1024);
        pair.drive_client();
    }
    // Ensure that the congestion state recovers after receiving the ACKs
    pair.drive();
    assert!(pair.client_conn_mut(client_ch).congestion_window() >= TARGET);
    pair.client_send(client_ch, s).write(&[42; 1024]).unwrap();
}

#[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
#[test]
fn high_latency_handshake() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    pair.latency = Duration::from_micros(200 * 1000);
    let (client_ch, server_ch) = pair.connect();
    assert_eq!(pair.client_conn_mut(client_ch).bytes_in_flight(), 0);
    assert_eq!(pair.server_conn_mut(server_ch).bytes_in_flight(), 0);
    assert!(pair.client_conn_mut(client_ch).using_ecn());
    assert!(pair.server_conn_mut(server_ch).using_ecn());
}

#[test]
fn zero_rtt_happypath() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    pair.server.incoming_connection_behavior = IncomingConnectionBehavior::Validate;
    let config = client_config();

    // Establish normal connection
    let client_ch = pair.begin_connect(config.clone());
    pair.drive();
    pair.server.assert_accept();
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(0), [][..].into());
    pair.drive();

    pair.client.addr = SocketAddr::new(
        Ipv6Addr::LOCALHOST.into(),
        CLIENT_PORTS.lock().unwrap().next().unwrap(),
    );
    info!("resuming session");
    let client_ch = pair.begin_connect(config);
    assert!(pair.client_conn_mut(client_ch).has_0rtt());
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"Hello, 0-RTT!";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    pair.drive();

    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Connected)
    );

    assert!(pair.client_conn_mut(client_ch).accepted_0rtt());
    let server_ch = pair.server.assert_accept();

    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    // We don't currently preserve stream event order wrt. connection events
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Connected)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG
    );
    let _ = chunks.finalize();
    assert_eq!(pair.client_conn_mut(client_ch).lost_packets(), 0);
}

#[test]
fn zero_rtt_rejection() {
    let _guard = subscribe();
    let server_config = ServerConfig::with_crypto(Arc::new(server_crypto_with_alpn(vec![
        "foo".into(),
        "bar".into(),
    ])));
    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config);
    let mut client_crypto = Arc::new(client_crypto_with_alpn(vec!["foo".into()]));
    let client_config = ClientConfig::new(client_crypto.clone());

    // Establish normal connection
    let client_ch = pair.begin_connect(client_config);
    pair.drive();
    let server_ch = pair.server.assert_accept();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Connected)
    );
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(0), [][..].into());
    pair.drive();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost { .. })
    );
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
    pair.client.connections.clear();
    pair.server.connections.clear();

    // We want to have a TLS client config with the existing session cache (so resumption could
    // happen), but with different ALPN protocols (so that the server must reject it). Reuse
    // the existing `ClientConfig` and change the ALPN protocols to make that happen.
    let this = Arc::get_mut(&mut client_crypto).expect("QuicClientConfig is shared");
    let inner = Arc::get_mut(&mut this.inner).expect("QuicClientConfig.inner is shared");
    inner.alpn_protocols = vec!["bar".into()];

    // Changing protocols invalidates 0-RTT
    let client_config = ClientConfig::new(client_crypto);
    info!("resuming session");
    let client_ch = pair.begin_connect(client_config);
    assert!(pair.client_conn_mut(client_ch).has_0rtt());
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"Hello, 0-RTT!";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    pair.drive();
    assert!(!pair.client_conn_mut(client_ch).accepted_0rtt());
    let server_ch = pair.server.assert_accept();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Connected)
    );
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
    let s2 = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    assert_eq!(s, s2);

    let mut recv = pair.server_recv(server_ch, s2);
    let mut chunks = recv.read(false).unwrap();
    assert_eq!(chunks.next(usize::MAX), Err(ReadError::Blocked));
    let _ = chunks.finalize();
    assert_eq!(pair.client_conn_mut(client_ch).lost_packets(), 0);
}

fn test_zero_rtt_incoming_limit<F: FnOnce(&mut ServerConfig)>(configure_server: F) {
    // caller sets the server limit to 4000 bytes
    // the client writes 8000 bytes
    const CLIENT_WRITES: usize = 8000;
    // this gets split across 8 packets
    // the first packet is stored in the Incoming
    // the next three are incoming-buffered, bringing the incoming buffer size to 3600 bytes
    // the last four are dropped due to the buffering limit and must be retransmitted
    const EXPECTED_DROPPED: u64 = 4;

    let _guard = subscribe();
    let mut server_config = server_config();
    configure_server(&mut server_config);
    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config);
    let config = client_config();

    // Establish normal connection
    let client_ch = pair.begin_connect(config.clone());
    pair.drive();
    pair.server.assert_accept();
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(0), [][..].into());
    pair.drive();

    pair.client.addr = SocketAddr::new(
        Ipv6Addr::LOCALHOST.into(),
        CLIENT_PORTS.lock().unwrap().next().unwrap(),
    );
    info!("resuming session");
    pair.server.incoming_connection_behavior = IncomingConnectionBehavior::Wait;
    let client_ch = pair.begin_connect(config);
    assert!(pair.client_conn_mut(client_ch).has_0rtt());
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    pair.client_send(client_ch, s)
        .write(&vec![0; CLIENT_WRITES])
        .unwrap();
    pair.drive();
    let incoming = pair.server.waiting_incoming.pop().unwrap();
    assert!(pair.server.waiting_incoming.is_empty());
    let _ = pair.server.try_accept(incoming, pair.time);
    pair.drive();

    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Connected)
    );

    assert!(pair.client_conn_mut(client_ch).accepted_0rtt());
    let server_ch = pair.server.assert_accept();

    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    // We don't currently preserve stream event order wrt. connection events
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Connected)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    let mut offset = 0;
    loop {
        match chunks.next(usize::MAX) {
            Ok(Some(chunk)) => {
                assert_eq!(chunk.offset as usize, offset);
                offset += chunk.bytes.len();
            }
            Err(ReadError::Blocked) => break,
            Ok(None) => panic!("unexpected stream end"),
            Err(e) => panic!("{}", e),
        }
    }
    assert_eq!(offset, CLIENT_WRITES);
    let _ = chunks.finalize();
    assert_eq!(
        pair.client_conn_mut(client_ch).lost_packets(),
        EXPECTED_DROPPED
    );
}

#[test]
fn zero_rtt_incoming_buffer_size() {
    test_zero_rtt_incoming_limit(|config| {
        config.incoming_buffer_size(4000);
    });
}

#[test]
fn zero_rtt_incoming_buffer_size_total() {
    test_zero_rtt_incoming_limit(|config| {
        config.incoming_buffer_size_total(4000);
    });
}

#[test]
fn alpn_success() {
    let _guard = subscribe();
    let server_config = ServerConfig::with_crypto(Arc::new(server_crypto_with_alpn(vec![
        "foo".into(),
        "bar".into(),
        "baz".into(),
    ])));

    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config);
    let client_config = ClientConfig::new(Arc::new(client_crypto_with_alpn(vec![
        "bar".into(),
        "quux".into(),
        "corge".into(),
    ])));

    // Establish normal connection
    let client_ch = pair.begin_connect(client_config);
    pair.drive();
    let server_ch = pair.server.assert_accept();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Connected)
    );

    let hd = pair
        .client_conn_mut(client_ch)
        .crypto_session()
        .handshake_data()
        .unwrap()
        .downcast::<crate::crypto::rustls::HandshakeData>()
        .unwrap();
    assert_eq!(hd.protocol.unwrap(), &b"bar"[..]);
}

#[test]
fn server_alpn_unset() {
    let _guard = subscribe();
    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config());
    let client_config = ClientConfig::new(Arc::new(client_crypto_with_alpn(vec!["foo".into()])));

    let client_ch = pair.begin_connect(client_config);
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::ConnectionLost { reason: ConnectionError::ConnectionClosed(err) }) if err.error_code == TransportErrorCode::crypto(0x78)
    );
}

#[test]
fn client_alpn_unset() {
    let _guard = subscribe();
    let server_config = ServerConfig::with_crypto(Arc::new(server_crypto_with_alpn(vec![
        "foo".into(),
        "bar".into(),
        "baz".into(),
    ])));

    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config);
    let client_ch = pair.begin_connect(client_config());
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::ConnectionLost { reason: ConnectionError::ConnectionClosed(err) }) if err.error_code == TransportErrorCode::crypto(0x78)
    );
}

#[test]
fn alpn_mismatch() {
    let _guard = subscribe();
    let server_config = ServerConfig::with_crypto(Arc::new(server_crypto_with_alpn(vec![
        "foo".into(),
        "bar".into(),
        "baz".into(),
    ])));

    let mut pair = Pair::new(Arc::new(EndpointConfig::default()), server_config);
    let client_ch = pair.begin_connect(ClientConfig::new(Arc::new(client_crypto_with_alpn(vec![
        "quux".into(),
        "corge".into(),
    ]))));

    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::ConnectionLost { reason: ConnectionError::ConnectionClosed(err) }) if err.error_code == TransportErrorCode::crypto(0x78)
    );
}

#[test]
fn stream_id_limit() {
    let _guard = subscribe();
    let server = ServerConfig {
        transport: Arc::new(TransportConfig {
            max_concurrent_uni_streams: 1u32.into(),
            ..TransportConfig::default()
        }),
        ..server_config()
    };
    let mut pair = Pair::new(Default::default(), server);
    let (client_ch, server_ch) = pair.connect();

    let s = pair
        .client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .streams()
        .open(Dir::Uni)
        .expect("couldn't open first stream");
    assert_eq!(
        pair.client_streams(client_ch).open(Dir::Uni),
        None,
        "only one stream is permitted at a time"
    );
    // Generate some activity to allow the server to see the stream
    const MSG: &[u8] = b"hello";
    pair.client_send(client_ch, s).write(MSG).unwrap();
    pair.client_send(client_ch, s).finish().unwrap();
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Stream(StreamEvent::Finished { id })) if id == s
    );
    assert_eq!(
        pair.client_streams(client_ch).open(Dir::Uni),
        None,
        "server does not immediately grant additional credit"
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Uni), Some(stream) if stream == s);

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG
    );
    assert_eq!(chunks.next(usize::MAX), Ok(None));
    let _ = chunks.finalize();

    // Server will only send MAX_STREAM_ID now that the application's been notified
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Stream(StreamEvent::Available { dir: Dir::Uni }))
    );
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);

    // Try opening the second stream again, now that we've made room
    let s = pair
        .client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .streams()
        .open(Dir::Uni)
        .expect("didn't get stream id budget");
    pair.client_send(client_ch, s).finish().unwrap();
    pair.drive();
    // Make sure the server actually processes data on the newly-available stream
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Uni), Some(stream) if stream == s);
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(chunks.next(usize::MAX), Ok(None));
    let _ = chunks.finalize();
}

#[test]
fn key_update_simple() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    let s = pair
        .client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .streams()
        .open(Dir::Bi)
        .expect("couldn't open first stream");

    const MSG1: &[u8] = b"hello1";
    pair.client_send(client_ch, s).write(MSG1).unwrap();
    pair.drive();

    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Bi }))
    );
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Bi), Some(stream) if stream == s);
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG1
    );
    let _ = chunks.finalize();

    info!("initiating key update");
    pair.client_conn_mut(client_ch).initiate_key_update();

    const MSG2: &[u8] = b"hello2";
    pair.client_send(client_ch, s).write(MSG2).unwrap();
    pair.drive();

    assert_matches!(pair.server_conn_mut(server_ch).poll(), Some(Event::Stream(StreamEvent::Readable { id })) if id == s);
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 6 && chunk.bytes == MSG2
    );
    let _ = chunks.finalize();

    assert_eq!(pair.client_conn_mut(client_ch).lost_packets(), 0);
    assert_eq!(pair.server_conn_mut(server_ch).lost_packets(), 0);
}

#[test]
fn initial_retransmit() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let client_ch = pair.begin_connect(client_config());
    pair.client.drive(pair.time, pair.server.addr);
    pair.client.outbound.clear(); // Drop initial
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Connected { .. })
    );
}

#[test]
fn instant_close_1() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    info!("connecting");
    let client_ch = pair.begin_connect(client_config());
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(0), Bytes::new());
    pair.drive();
    let server_ch = pair.server.assert_accept();
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::ConnectionClosed(ConnectionClose {
                error_code: TransportErrorCode::APPLICATION_ERROR,
                ..
            }),
        })
    );
}

#[test]
fn instant_close_2() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    info!("connecting");
    let client_ch = pair.begin_connect(client_config());
    // Unlike `instant_close`, the server sees a valid Initial packet first.
    pair.drive_client();
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(42), Bytes::new());
    pair.drive();
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    let server_ch = pair.server.assert_accept();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::ConnectionClosed(ConnectionClose {
                error_code: TransportErrorCode::APPLICATION_ERROR,
                ..
            }),
        })
    );
}

#[test]
fn instant_server_close() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    info!("connecting");
    pair.begin_connect(client_config());
    pair.drive_client();
    pair.server.drive_incoming(pair.time, pair.client.addr);
    let server_ch = pair.server.assert_accept();
    info!("closing");
    pair.server
        .connections
        .get_mut(&server_ch)
        .unwrap()
        .close(pair.time, VarInt(42), Bytes::new());
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::ConnectionClosed(ConnectionClose {
                error_code: TransportErrorCode::APPLICATION_ERROR,
                ..
            }),
        })
    );
}

#[test]
fn idle_timeout() {
    let _guard = subscribe();
    const IDLE_TIMEOUT: u64 = 100;
    let server = ServerConfig {
        transport: Arc::new(TransportConfig {
            max_idle_timeout: Some(VarInt(IDLE_TIMEOUT)),
            ..TransportConfig::default()
        }),
        ..server_config()
    };
    let mut pair = Pair::new(Default::default(), server);
    let (client_ch, server_ch) = pair.connect();
    pair.client_conn_mut(client_ch).ping();
    let start = pair.time;

    while !pair.client_conn_mut(client_ch).is_closed()
        || !pair.server_conn_mut(server_ch).is_closed()
    {
        if !pair.step() {
            if let Some(t) = min_opt(pair.client.next_wakeup(), pair.server.next_wakeup()) {
                pair.time = t;
            }
        }
        pair.client.inbound.clear(); // Simulate total S->C packet loss
    }

    assert!(pair.time - start < Duration::from_millis(2 * IDLE_TIMEOUT));
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::TimedOut,
        })
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::ConnectionLost {
            reason: ConnectionError::TimedOut,
        })
    );
}

#[test]
fn connection_close_sends_acks() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, _server_ch) = pair.connect();

    let client_acks = pair.client_conn_mut(client_ch).stats().frame_rx.acks;

    pair.client_conn_mut(client_ch).ping();
    pair.drive_client();

    let time = pair.time;
    pair.server_conn_mut(client_ch)
        .close(time, VarInt(42), Bytes::new());

    pair.drive();

    let client_acks_2 = pair.client_conn_mut(client_ch).stats().frame_rx.acks;
    assert!(
        client_acks_2 > client_acks,
        "Connection close should send pending ACKs"
    );
}

#[test]
fn server_hs_retransmit() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let client_ch = pair.begin_connect(client_config());
    pair.step();
    assert!(!pair.client.inbound.is_empty()); // Initial + Handshakes
    pair.client.inbound.clear();
    pair.drive();
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::HandshakeDataReady)
    );
    assert_matches!(
        pair.client_conn_mut(client_ch).poll(),
        Some(Event::Connected { .. })
    );
}

#[test]
fn migration() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    pair.drive();

    let client_stats_after_connect = pair.client_conn_mut(client_ch).stats();

    pair.client.addr = SocketAddr::new(
        Ipv4Addr::new(127, 0, 0, 1).into(),
        CLIENT_PORTS.lock().unwrap().next().unwrap(),
    );
    pair.client_conn_mut(client_ch).ping();

    // Assert that just receiving the ping message is accounted into the servers
    // anti-amplification budget
    pair.drive_client();
    pair.drive_server();
    assert_ne!(pair.server_conn_mut(server_ch).total_recvd(), 0);

    pair.drive();
    assert_matches!(pair.client_conn_mut(client_ch).poll(), None);
    assert_eq!(
        pair.server_conn_mut(server_ch).remote_address(),
        pair.client.addr
    );

    // Assert that the client's response to the PATH_CHALLENGE was an IMMEDIATE_ACK, instead of a
    // second ping
    let client_stats_after_migrate = pair.client_conn_mut(client_ch).stats();
    assert_eq!(
        client_stats_after_migrate.frame_tx.ping - client_stats_after_connect.frame_tx.ping,
        1
    );
    assert_eq!(
        client_stats_after_migrate.frame_tx.immediate_ack
            - client_stats_after_connect.frame_tx.immediate_ack,
        1
    );
}

fn test_flow_control(config: TransportConfig, window_size: usize) {
    let _guard = subscribe();
    let mut pair = Pair::new(
        Default::default(),
        ServerConfig {
            transport: Arc::new(config),
            ..server_config()
        },
    );
    let (client_ch, server_ch) = pair.connect();
    let msg = vec![0xAB; window_size + 10];

    // Stream reset before read
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    info!("writing");
    assert_eq!(pair.client_send(client_ch, s).write(&msg), Ok(window_size));
    assert_eq!(
        pair.client_send(client_ch, s).write(&msg[window_size..]),
        Err(WriteError::Blocked)
    );
    pair.drive();
    info!("resetting");
    pair.client_send(client_ch, s).reset(VarInt(42)).unwrap();
    pair.drive();

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(true).unwrap();
    assert_eq!(
        chunks.next(usize::MAX).err(),
        Some(ReadError::Reset(VarInt(42)))
    );
    let _ = chunks.finalize();

    // Happy path
    info!("writing");
    let s = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    assert_eq!(pair.client_send(client_ch, s).write(&msg), Ok(window_size));
    assert_eq!(
        pair.client_send(client_ch, s).write(&msg[window_size..]),
        Err(WriteError::Blocked)
    );

    pair.drive();
    let mut cursor = 0;
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(true).unwrap();
    loop {
        match chunks.next(usize::MAX) {
            Ok(Some(chunk)) => {
                cursor += chunk.bytes.len();
            }
            Ok(None) => {
                panic!("end of stream");
            }
            Err(ReadError::Blocked) => {
                break;
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
    let _ = chunks.finalize();

    info!("finished reading");
    assert_eq!(cursor, window_size);
    pair.drive();
    info!("writing");
    assert_eq!(pair.client_send(client_ch, s).write(&msg), Ok(window_size));
    assert_eq!(
        pair.client_send(client_ch, s).write(&msg[window_size..]),
        Err(WriteError::Blocked)
    );

    pair.drive();
    let mut cursor = 0;
    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(true).unwrap();
    loop {
        match chunks.next(usize::MAX) {
            Ok(Some(chunk)) => {
                cursor += chunk.bytes.len();
            }
            Ok(None) => {
                panic!("end of stream");
            }
            Err(ReadError::Blocked) => {
                break;
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
    assert_eq!(cursor, window_size);
    let _ = chunks.finalize();
    info!("finished reading");
}

#[test]
fn stream_flow_control() {
    test_flow_control(
        TransportConfig {
            stream_receive_window: 2000u32.into(),
            ..TransportConfig::default()
        },
        2000,
    );
}

#[test]
fn conn_flow_control() {
    test_flow_control(
        TransportConfig {
            receive_window: 2000u32.into(),
            ..TransportConfig::default()
        },
        2000,
    );
}

#[test]
fn stop_opens_bidi() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    assert_eq!(pair.client_streams(client_ch).send_streams(), 0);
    let s = pair.client_streams(client_ch).open(Dir::Bi).unwrap();
    assert_eq!(pair.client_streams(client_ch).send_streams(), 1);
    const ERROR: VarInt = VarInt(42);
    pair.client
        .connections
        .get_mut(&server_ch)
        .unwrap()
        .recv_stream(s)
        .stop(ERROR)
        .unwrap();
    pair.drive();

    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Bi }))
    );
    assert_eq!(pair.server_conn_mut(client_ch).streams().send_streams(), 0);
    assert_matches!(pair.server_streams(server_ch).accept(Dir::Bi), Some(stream) if stream == s);
    assert_eq!(pair.server_conn_mut(client_ch).streams().send_streams(), 1);

    let mut recv = pair.server_recv(server_ch, s);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(chunks.next(usize::MAX), Err(ReadError::Blocked));
    let _ = chunks.finalize();

    assert_matches!(
        pair.server_send(server_ch, s).write(b"foo"),
        Err(WriteError::Stopped(ERROR))
    );
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Stopped {
            id: _,
            error_code: ERROR
        }))
    );
    assert_matches!(pair.server_conn_mut(server_ch).poll(), None);
}

#[test]
fn implicit_open() {
    let _guard = subscribe();
    let mut pair = Pair::default();
    let (client_ch, server_ch) = pair.connect();
    let s1 = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    let s2 = pair.client_streams(client_ch).open(Dir::Uni).unwrap();
    pair.client_send(client_ch, s2).write(b"hello").unwrap();
    pair.drive();
    assert_matches!(
        pair.server_conn_mut(server_ch).poll(),
        Some(Event::Stream(StreamEvent::Opened { dir: Dir::Uni }))
    );
    assert_eq!(pair.server_streams(server_ch).accept(Dir::Uni), Some(s1));
    assert_eq!(pair.server_streams(server_ch).accept(Dir::Uni), Some(s2));
    assert_eq!(pair.server_streams(server_ch).accept(Dir::Uni), None);
}

#[test]
fn zero_length_cid() {
    let _guard = subscribe();
    let cid_generator_factory: fn() -> Box<dyn ConnectionIdGenerator> =
        || Box::new(RandomConnectionIdGenerator::new(0));
    let mut pair = Pair::new(
        Arc::new(EndpointConfig {
            connection_id_generator_factory: Arc::new(cid_generator_factory),
            ..EndpointConfig::default()
        }),
        server_config(),
    );
    let (client_ch, server_ch) = pair.connect();
    // Ensure we can reconnect after a previous connection is cleaned up
    info!("closing");
    pair.client
        .connections
        .get_mut(&client_ch)
        .unwrap()
        .close(pair.time, VarInt(42), Bytes::new());
    pair.drive();
    pair.server
        .connections
        .get_mut(&server_ch)
        .unwrap()
        .close(pair.time, VarInt(42), Bytes::new());
    pair.connect();
}

#[test]
fn keep_alive() {
    let _guard = subscribe();
    const IDLE_TIMEOUT: u64 = 10;
    let server = ServerConfig {
        transport: Arc::new(TransportConfig {
            keep_alive_interval: Some(Duration::from_millis(IDLE_TIMEOUT / 2)),
            max_idle_timeout: Some(VarInt(IDLE_TIMEOUT)),
            ..TransportConfig::default()
        }),
        ..server_config()
    };
    let mut pair = Pair::new(Default::default(), server);
    let (client_ch, server_ch) = pair.connect();
    // Run a good while longer than the idle timeout
    let end = pair.time + Duration::from_millis(20 * IDLE_TIMEOUT);
    while pair.time < end {
        if !pair.step() {
            if let Some(time) = min_opt(pair.client.next_wakeup(), pair.server.next_wakeup()) {
                pair.time = time;
            }
        }
        assert!(!pair.client_conn_mut(client_ch).is_closed());
        assert!(!pair.server_conn_mut(server_ch).is_closed());
    }
}

#[test]
fn cid_rotation() {
    let _guard = subscribe();
    const CID_TIMEOUT: Duration = Duration::from_secs(2);

    let cid_generator_factory: fn() -> Box<dyn ConnectionIdGenerator> =
        || Box::new(*RandomConnectionIdGenerator::new(8).set_lifetime(CID_TIMEOUT));

    // Only test cid rotation on server side to have a clear output trace
    let server = Endpoint::new(
        Arc::new(EndpointConfig {
            connection_id_generator_factory: Arc::new(cid_generator_factory),
            ..EndpointConfig::default()
        }),
        Some(Arc::new(server_config())),
        true,
        None,
    );
    let client = Endpoint::new(Arc::new(EndpointConfig::default()), None, true, None);

    let mut pair = Pair::new_from_endpoint(client, server);
    let (_, server_ch) = pair.connect();

    let mut round: u64 = 1;
    let mut stop = pair.time;
    let end = pair.time + 5 * CID_TIMEOUT;

    use crate::cid_queue::CidQueue;
    use crate::LOC_CID_COUNT;
    let mut active_cid_num = CidQueue::LEN as u64 + 1;
    active_cid_num = active_cid_num.min(LOC_CID_COUNT);
    let mut left_bound = 0;
    let mut right_bound = active_cid_num - 1;

    while pair.time < end {
        stop += CID_TIMEOUT;
        // Run a while until PushNewCID timer fires
        while pair.time < stop {
            if !pair.step() {
                if let Some(time) = min_opt(pair.client.next_wakeup(), pair.server.next_wakeup()) {
                    pair.time = time;
                }
            }
        }
        info!(
            "Checking active cid sequence range before {:?} seconds",
            round * CID_TIMEOUT.as_secs()
        );
        let _bound = (left_bound, right_bound);
        assert_matches!(
            pair.server_conn_mut(server_ch).active_local_cid_seq(),
            _bound
        );
        round += 1;
        left_bound += active_cid_num;
        right_bound += active_cid_num;
        pair.drive_server();
    }
}
