use std::time::Instant;

use tracing::trace;

use crate::{config::MtuDiscoveryConfig, packet::SpaceId, MAX_UDP_PAYLOAD};

/// Implements Datagram Packetization Layer Path Maximum Transmission Unit Discovery
///
/// See [`MtuDiscoveryConfig`] for details
#[derive(Clone)]
pub(crate) struct MtuDiscovery {
    /// 1. Detected MTU for the path
    current_mtu: u16,
    /// 2. The state of the MTU discovery, if enabled
    state: Option<EnabledMtuDiscovery>,
    /// 3. The state of the black hole detector
    black_hole_detector: BlackHoleDetector,
}

impl MtuDiscovery {
    /// 1
    pub(crate) fn new(
        initial_plpmtu: u16,
        min_mtu: u16,
        peer_max_udp_payload_size: Option<u16>,
        config: MtuDiscoveryConfig,
    ) -> Self {
        debug_assert!(
            initial_plpmtu >= min_mtu,
            "initial_max_udp_payload_size must be at least {min_mtu}"
        );

        let mut mtud = Self::with_state(
            initial_plpmtu,
            min_mtu,
            Some(EnabledMtuDiscovery::new(config)),
        );

        mtud
    }

    /// 2. MTU discovery will be disabled and the current MTU will be fixed to the provided value
    pub(crate) fn disabled(plpmtu: u16, min_mtu: u16) -> Self {
        Self::with_state(plpmtu, min_mtu, None)
    }

    /// 3. Returns the current MTU
    pub(crate) fn current_mtu(&self) -> u16 {
        self.current_mtu
    }

    /// 4.
    fn with_state(current_mtu: u16, min_mtu: u16, state: Option<EnabledMtuDiscovery>) -> Self {
        // todo
        Self {
            current_mtu,
            state,
            black_hole_detector: BlackHoleDetector::new(min_mtu),
        }
    }
    /// 5. Returns the amount of bytes that should be sent as an MTU probe, if any
    pub(crate) fn poll_transmit(&mut self, now: Instant, next_pn: u64) -> Option<u16> {
        self.state
            .as_mut()
            .and_then(|state| state.poll_transmit(now, self.current_mtu, next_pn))
    }

    /// 6. Notifies the [`MtuDiscovery`] that the peer's `max_udp_payload_size` transport parameter has
    /// been received
    pub(crate) fn on_peer_max_udp_payload_size_received(&mut self, peer_max_udp_payload_size: u16) {
        self.current_mtu = self.current_mtu.min(peer_max_udp_payload_size);

        if let Some(state) = self.state.as_mut() {
            // MTUD is only active after the connection has been fully established, so it is
            // guaranteed we will receive the peer's transport parameters before we start probing
            debug_assert!(matches!(state.phase, Phase::Initial));
            state.peer_max_udp_payload_size = peer_max_udp_payload_size;
        }
    }
    /// 7. Notifies the [`MtuDiscovery`] that a packet has been ACKed
    ///
    /// Returns true if the packet was an MTU probe
    pub(crate) fn on_acked(&mut self, space: SpaceId, pn: u64, len: u16) -> bool {
        // MTU probes are only sent in application data space
        if space != SpaceId::Data {
            return false;
        }
        // Update the state of the MTU search
        if let Some(new_mtu) = self
            .state
            .as_mut()
            .and_then(|state| state.on_probe_acked(pn))
        {
            self.current_mtu = new_mtu;
            trace!(current_mtu = self.current_mtu, "new MTU detected");
            todo!();
            //            self.black_hole_detector.on_probe_acked(pn, len);
            true
        } else {
            self.black_hole_detector.on_non_probe_acked(pn, len);
            false
        }
    }

    /// 8. Returns the packet number of the in-flight MTU probe, if any
    pub(crate) fn in_flight_mtu_probe(&self) -> Option<u64> {
        match &self.state {
            Some(EnabledMtuDiscovery {
                phase: Phase::Searching(search_state),
                ..
            }) => search_state.in_flight_probe,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    /// 1. We haven't started polling yet
    Initial,
    /// 2. We are currently searching for a higher PMTU
    Searching(SearchState),
    /// 3. Searching has completed and will be triggered again at the provided instant
    Complete(Instant),
}

#[derive(Debug, Clone, Copy)]
struct SearchState {
    /// 1. Packet number of an in-flight probe (if any)
    in_flight_probe: Option<u64>,
    /// 2. Lost probes at the current probe size
    lost_probe_count: usize,
    /// 3. The UDP payload size we last sent a probe for
    last_probed_mtu: u16,
    /// 4. The lower bound for the current binary search
    lower_bound: u16,
    /// 5. The upper bound for the current binary search
    upper_bound: u16,
    /// 6. The minimum change to stop the current binary search
    minimum_change: u16,
}

impl SearchState {
    /// 1. Creates a new search state, with the specified lower bound (the upper bound is derived from
    /// the config and the peer's `max_udp_payload_size` transport parameter)
    fn new(
        mut lower_bound: u16,
        peer_max_udp_payload_size: u16,
        config: &MtuDiscoveryConfig,
    ) -> Self {
        lower_bound = lower_bound.min(peer_max_udp_payload_size);
        let upper_bound = config
            .upper_bound
            .clamp(lower_bound, peer_max_udp_payload_size);

        Self {
            in_flight_probe: None,
            lost_probe_count: 0,
            // During initialization, we consider the lower bound to have already been
            // successfully probed
            last_probed_mtu: lower_bound,
            lower_bound,
            upper_bound,
            minimum_change: config.minimum_change,
        }
    }
    /// 2. Determines the next MTU to probe using binary search
    fn next_mtu_to_probe(&mut self, last_probe_succeeded: bool) -> Option<u16> {
        debug_assert_eq!(self.in_flight_probe, None);

        if last_probe_succeeded {
            self.lower_bound = self.last_probed_mtu;
        } else {
            self.upper_bound = self.last_probed_mtu - 1;
        }
        let next_mtu = (self.lower_bound as i32 + self.upper_bound as i32) / 2;

        // Binary search stopping condition
        if ((next_mtu - self.last_probed_mtu as i32).unsigned_abs() as u16) < self.minimum_change {
            // Special case: if the upper bound is far enough, we want to probe it as a last
            // step (otherwise we will never achieve the upper bound)
            if self.upper_bound.saturating_sub(self.last_probed_mtu) >= self.minimum_change {
                return Some(self.upper_bound);
            }

            return None;
        }

        Some(next_mtu as u16)
    }
}
/// Additional state for enabled MTU discovery
#[derive(Debug, Clone)]
struct EnabledMtuDiscovery {
    phase: Phase,
    peer_max_udp_payload_size: u16,
    config: MtuDiscoveryConfig,
}

impl EnabledMtuDiscovery {
    /// 1.
    fn new(config: MtuDiscoveryConfig) -> Self {
        Self {
            phase: Phase::Initial,
            peer_max_udp_payload_size: MAX_UDP_PAYLOAD,
            config,
        }
    }
    /// 2.Returns the amount of bytes that should be sent as an MTU probe, if any
    fn poll_transmit(&mut self, now: Instant, current_mtu: u16, next_pn: u64) -> Option<u16> {
        if let Phase::Initial = &self.phase {
            // Start the first search
            self.phase = Phase::Searching(SearchState::new(
                current_mtu,
                self.peer_max_udp_payload_size,
                &self.config,
            ));
        } else if let Phase::Complete(next_mtud_activation) = &self.phase {
            if now < *next_mtud_activation {
                return None;
            }
            todo!()
        }

        if let Phase::Searching(state) = &mut self.phase {
            // Nothing to do while there is a probe in flight
            if state.in_flight_probe.is_some() {
                return None;
            }

            // Retransmit lost probes, if any
            if 0 < state.lost_probe_count && state.lost_probe_count < MAX_PROBE_RETRANSMITS {
                state.in_flight_probe = Some(next_pn);
                return Some(state.last_probed_mtu);
            }

            let last_probe_succeeded = state.lost_probe_count == 0;

            // The probe is definitely lost (we reached the MAX_PROBE_RETRANSMITS threshold)
            if !last_probe_succeeded {
                state.lost_probe_count = 0;
                state.in_flight_probe = None;
            }

            if let Some(probe_udp_payload_size) = state.next_mtu_to_probe(last_probe_succeeded) {
                state.in_flight_probe = Some(next_pn);
                state.last_probed_mtu = probe_udp_payload_size;
                return Some(probe_udp_payload_size);
            } else {
                let next_mtud_activation = now + self.config.interval;
                self.phase = Phase::Complete(next_mtud_activation);
                return None;
            }
            todo!()
        }
        None
    }

    /// 3. Called when a packet is acknowledged in [`SpaceId::Data`]
    ///
    /// Returns the new `current_mtu` if the packet number corresponds to the in-flight MTU probe
    fn on_probe_acked(&mut self, pn: u64) -> Option<u16> {
        match &mut self.phase {
            Phase::Searching(state) if state.in_flight_probe == Some(pn) => {
                /*  tracing::debug!(
                    "pn is {} state.in_flight_probe is {:?}",
                    pn,
                    state.in_flight_probe
                ); */
                state.in_flight_probe = None;
                state.lost_probe_count = 0;
                Some(state.last_probed_mtu)
            }
            _ => None,
        }
    }
}

// Corresponds to the RFC's `MAX_PROBES` constant (see
// https://www.rfc-editor.org/rfc/rfc8899#section-5.1.2)
const MAX_PROBE_RETRANSMITS: usize = 3;
/// Maximum number of suspicious loss bursts that will not trigger black hole detection
const BLACK_HOLE_THRESHOLD: usize = 3;

/// Judges whether packet loss might indicate a drop in MTU
///
/// Our MTU black hole detection scheme is a heuristic based on the order in which packets were sent
/// (the packet number order), their sizes, and which are deemed lost.
///
/// First, contiguous groups of lost packets ("loss bursts") are aggregated, because a group of
/// packets all lost together were probably lost for the same reason.
///
/// A loss burst is deemed "suspicious" if it contains no packets that are (a) smaller than the
/// minimum MTU or (b) smaller than a more recent acknowledged packet, because such a burst could be
/// fully explained by a reduction in MTU.
///
/// When the number of suspicious loss bursts exceeds [`BLACK_HOLE_THRESHOLD`], we judge the
/// evidence for an MTU black hole to be sufficient.
#[derive(Clone)]
struct BlackHoleDetector {
    /// The maximum of `min_mtu` and the size of `largest_post_loss_packet`, or exactly `min_mtu` if
    /// no larger packets have been received since the most recent loss burst.
    acked_mtu: u16,
    /// Packet number of the biggest packet larger than `min_mtu` which we've received
    /// acknowledgment of more recently than any suspicious loss burst, if any
    largest_post_loss_packet: u64,
    /// Packet loss bursts currently considered suspicious
    suspicious_loss_bursts: Vec<LossBurst>,
}

impl BlackHoleDetector {
    fn new(min_mtu: u16) -> Self {
        Self {
            suspicious_loss_bursts: Vec::with_capacity(BLACK_HOLE_THRESHOLD + 1),
            largest_post_loss_packet: 0,
            acked_mtu: min_mtu,
            /*
            current_loss_burst: None,

            min_mtu, */
        }
    }
    fn on_non_probe_acked(&mut self, pn: u64, len: u16) {
        if len <= self.acked_mtu {
            // We've already seen a larger packet since the most recent suspicious loss burst;
            // nothing to do.
            return;
        }
        self.acked_mtu = len;
        // This might go backwards, but that's okay as described in `on_probe_acked`.
        self.largest_post_loss_packet = pn;
        // Loss bursts packets smaller than this are retroactively deemed non-suspicious.
        self.suspicious_loss_bursts
            .retain(|burst| burst.smallest_packet_size > len);
    }
}

#[derive(Copy, Clone)]
struct LossBurst {
    smallest_packet_size: u16,
}
