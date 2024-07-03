use std::sync::Arc;

use crate::{config::ClientConfig, crypto::rustls::QuicClientConfig};

pub(super) fn client_config() -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto()))
}

pub(super) fn client_crypto() -> QuicClientConfig {
    todo!()
}
