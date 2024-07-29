use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::connection::RttEstimator;

use super::{Controller, ControllerFactory, BASE_DATAGRAM_SIZE};

/// The RFC8312 congestion controller, as widely used for TCP
#[derive(Debug, Clone)]
pub struct Cubic {
    /// 1. Maximum number of bytes in flight that may be sent.
    window: u64,
    /// 2.
    config: Arc<CubicConfig>,
    /// 3. The time when QUIC first detects a loss, causing it to enter recovery. When a packet sent
    /// after this time is acknowledged, QUIC exits recovery.
    recovery_start_time: Option<Instant>,
    /// 4.
    current_mtu: u64,
}

impl Cubic {
    /// 1. Construct a state using the given `config` and current time `now`
    pub fn new(config: Arc<CubicConfig>, _now: Instant, current_mtu: u16) -> Self {
        Self {
            window: config.initial_window,
            config,
            recovery_start_time: None,
            current_mtu: current_mtu as u64,
        }
    }
    /// 2.
    fn minimum_window(&self) -> u64 {
        2 * self.current_mtu
    }
}
impl Controller for Cubic {
    fn window(&self) -> u64 {
        self.window
    }

    fn initial_window(&self) -> u64 {
        self.config.initial_window
    }
    fn on_ack(
        &mut self,
        now: Instant,
        sent: Instant,
        bytes: u64,
        app_limited: bool,
        rtt: &RttEstimator,
    ) {
        if app_limited
            || self
                .recovery_start_time
                .map(|recovery_start_time| sent <= recovery_start_time)
                .unwrap_or(false)
        {
            return;
        }
        todo!()
    }

    fn on_congestion_event(
        &mut self,
        now: Instant,
        sent: Instant,
        is_persistent_congestion: bool,
        _lost_bytes: u64,
    ) {
        todo!()
    }

    fn on_mtu_update(&mut self, new_mtu: u16) {
        self.current_mtu = new_mtu as u64;
        self.window = self.window.max(self.minimum_window());
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
