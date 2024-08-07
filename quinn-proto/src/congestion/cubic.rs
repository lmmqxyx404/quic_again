use std::cmp;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::connection::RttEstimator;

use super::{Controller, ControllerFactory, BASE_DATAGRAM_SIZE};

/// CUBIC Constants.
///
/// These are recommended value in RFC8312.
const BETA_CUBIC: f64 = 0.7;

const C: f64 = 0.4;

/// CUBIC State Variables.
///
/// We need to keep those variables across the connection.
/// k, w_max are described in the RFC.
#[derive(Debug, Default, Clone)]
pub(super) struct State {
    k: f64,

    w_max: f64,

    // Store cwnd increment during congestion avoidance.
    cwnd_inc: u64,
}

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
    /// 5.
    cubic_state: State,
    /// 6. Slow start threshold in bytes. When the congestion window is below ssthresh, the mode is
    /// slow start and the window grows by the number of bytes acknowledged.
    ssthresh: u64,
}

/// CUBIC Functions.
///
/// Note that these calculations are based on a count of cwnd as bytes,
/// not packets.
/// Unit of t (duration) and RTT are based on seconds (f64).
impl State {
    // K = cbrt(w_max * (1 - beta_cubic) / C) (Eq. 2)
    fn cubic_k(&self, max_datagram_size: u64) -> f64 {
        let w_max = self.w_max / max_datagram_size as f64;
        (w_max * (1.0 - BETA_CUBIC) / C).cbrt()
    }
}
impl Cubic {
    /// 1. Construct a state using the given `config` and current time `now`
    pub fn new(config: Arc<CubicConfig>, _now: Instant, current_mtu: u16) -> Self {
        Self {
            window: config.initial_window,
            config,
            recovery_start_time: None,
            current_mtu: current_mtu as u64,
            cubic_state: Default::default(),
            ssthresh: u64::MAX,
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
        if self
            .recovery_start_time
            .map(|recovery_start_time| sent <= recovery_start_time)
            .unwrap_or(false)
        {
            return;
        }

        self.recovery_start_time = Some(now);

        // Fast convergence
        #[allow(clippy::branches_sharing_code)]
        // https://github.com/rust-lang/rust-clippy/issues/7198
        if (self.window as f64) < self.cubic_state.w_max {
            self.cubic_state.w_max = self.window as f64 * (1.0 + BETA_CUBIC) / 2.0;
        } else {
            self.cubic_state.w_max = self.window as f64;
        }

        self.ssthresh = cmp::max(
            (self.cubic_state.w_max * BETA_CUBIC) as u64,
            self.minimum_window(),
        );
        self.window = self.ssthresh;
        self.cubic_state.k = self.cubic_state.cubic_k(self.current_mtu);

        if is_persistent_congestion {
            todo!()
        }
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
