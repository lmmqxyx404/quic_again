use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    cid_generator::HashedConnectionIdGenerator,
    config::EndpointConfig,
    connection::{ConnectionError, Event, StreamEvent},
    endpoint::DatagramEvent,
    frame::ApplicationClose,
    ConnectionIdGenerator, Dir, Endpoint, RandomConnectionIdGenerator, Transmit, VarInt,
    DEFAULT_SUPPORTED_VERSIONS,
};

mod util;
use assert_matches::assert_matches;
use hex_literal::hex;
use rand::RngCore;
use ring::hmac;
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
    todo!()
    /* assert_matches!(
        chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG
    );
    assert_matches!(chunks.next(usize::MAX), Ok(None));
    let _ = chunks.finalize(); */
}
