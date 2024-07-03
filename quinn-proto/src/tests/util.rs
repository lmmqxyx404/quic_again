use std::sync::Arc;

use crate::config::ClientConfig;

pub(super) fn client_config() -> ClientConfig {
    ClientConfig::new(Arc::new(client_crypto()))
}
