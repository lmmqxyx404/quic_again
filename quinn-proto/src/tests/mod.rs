use std::{sync::Arc, time::Instant};

use crate::{config::EndpointConfig, ConnectionIdGenerator, Endpoint, RandomConnectionIdGenerator};


mod util;
use util::*;
#[test]
fn version_negotiate_client() {
    // let _guard = subscribe();
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
    println!("hello world");
}
