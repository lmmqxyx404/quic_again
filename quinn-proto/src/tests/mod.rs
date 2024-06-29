use std::sync::Arc;

use crate::{
    config::EndpointConfig, endpoint::Endpoint, ConnectionIdGenerator, RandomConnectionIdGenerator,
};

#[test]
fn version_negotiate_server() {
    // let client_addr = "[::2]:7890".parse().unwrap();
    println!("hello world");
}

#[test]
fn version_negotiate_client() {
    // let server_addr: _ = "[::2]:7890".parse().unwrap();
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
    println!("hello world");
}
