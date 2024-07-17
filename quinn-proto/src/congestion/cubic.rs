use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{Controller, ControllerFactory, BASE_DATAGRAM_SIZE};

/// The RFC8312 congestion controller, as widely used for TCP
#[derive(Debug, Clone)]
pub struct Cubic {
    /// 1. Maximum number of bytes in flight that may be sent.
    window: u64,
}

impl Cubic {
    /// Construct a state using the given `config` and current time `now`
    pub fn new(config: Arc<CubicConfig>, _now: Instant, current_mtu: u16) -> Self {
        Self {
            window: config.initial_window,
        }
    }
}
impl Controller for Cubic {
    fn window(&self) -> u64 {
        self.window
    }
}

/// Configuration for the `Cubic` congestion controller
#[derive(Debug, Clone)]
pub struct CubicConfig {
    initial_window: u64,
}

impl Default for CubicConfig {
    fn default() -> Self {
        Self {
            initial_window: 14720.clamp(2 * BASE_DATAGRAM_SIZE, 10 * BASE_DATAGRAM_SIZE),
        }
    }
}

impl ControllerFactory for CubicConfig {
    fn build(self: Arc<Self>, now: Instant, current_mtu: u16) -> Box<dyn Controller> {
        Box::new(Cubic::new(self, now, current_mtu))
    }
}
