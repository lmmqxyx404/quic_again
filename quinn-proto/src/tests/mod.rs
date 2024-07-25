use std::{sync::Arc, time::Instant};

use crate::{
    config::EndpointConfig,
    connection::{ConnectionError, Event},
    endpoint::DatagramEvent,
    frame::ApplicationClose,
    ConnectionIdGenerator, Endpoint, RandomConnectionIdGenerator, Transmit, VarInt,
    DEFAULT_SUPPORTED_VERSIONS,
};

mod util;
use assert_matches::assert_matches;
use hex_literal::hex;
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
