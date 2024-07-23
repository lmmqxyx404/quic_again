use std::sync::Arc;
use std::time::Instant;

mod cubic;

pub use cubic::CubicConfig;

use crate::connection::RttEstimator;

/// Common interface for different congestion controllers
pub trait Controller: Send + Sync {
    /// 1. Number of ack-eliciting bytes that may be in flight,first used for `self.path.congestion.window()`
    fn window(&self) -> u64;
    /// 2. Initial congestion window
    fn initial_window(&self) -> u64;
    /// 3. One or more packets were just sent
    #[allow(unused_variables)]
    fn on_sent(&mut self, now: Instant, bytes: u64, last_packet_number: u64) {}
    /// 4. Packet deliveries were confirmed
    ///
    /// `app_limited` indicates whether the connection was blocked on outgoing
    /// application data prior to receiving these acknowledgements.
    #[allow(unused_variables)]
    fn on_ack(
        &mut self,
        now: Instant,
        sent: Instant,
        bytes: u64,
        app_limited: bool,
        rtt: &RttEstimator,
    ) {
    }
}

/// Constructs controllers on demand
pub trait ControllerFactory {
    /// Construct a fresh `Controller`
    fn build(self: Arc<Self>, now: Instant, current_mtu: u16) -> Box<dyn Controller>;
}

const BASE_DATAGRAM_SIZE: u64 = 1200;
