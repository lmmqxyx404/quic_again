use std::sync::Arc;
use std::time::Instant;

mod cubic;

pub use cubic::CubicConfig;

/// Common interface for different congestion controllers
pub trait Controller: Send + Sync {
    /// 1. Number of ack-eliciting bytes that may be in flight,first used for `self.path.congestion.window()`
    fn window(&self) -> u64;
}

/// Constructs controllers on demand
pub trait ControllerFactory {
    /// Construct a fresh `Controller`
    fn build(self: Arc<Self>, now: Instant, current_mtu: u16) -> Box<dyn Controller>;
}

const BASE_DATAGRAM_SIZE: u64 = 1200;
