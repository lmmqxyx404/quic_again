use std::{cmp, time::Instant};

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

        // We might be migrating an existing connection to a new path, in which case the transport
        // parameters have already been transmitted, and we already know the value of
        // `peer_max_udp_payload_size`
        // used for `migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size`
        if let Some(peer_max_udp_payload_size) = peer_max_udp_payload_size {
            mtud.on_peer_max_udp_payload_size_received(peer_max_udp_payload_size);
        }

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
            self.black_hole_detector.on_probe_acked(pn, len);
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
    /// 9. Notifies the [`MtuDiscovery`] that a non-probe packet was lost
    ///
    /// When done notifying of lost packets, [`MtuDiscovery::black_hole_detected`] must be called, to
    /// ensure the last loss burst is properly processed and to trigger black hole recovery logic if
    /// necessary.
    pub(crate) fn on_non_probe_lost(&mut self, pn: u64, len: u16) {
        self.black_hole_detector.on_non_probe_lost(pn, len);
    }
    /// 10. Notifies the [`MtuDiscovery`] that the in-flight MTU probe was lost
    pub(crate) fn on_probe_lost(&mut self) {
        if let Some(state) = &mut self.state {
            state.on_probe_lost();
        }
    }
    /// 11. Returns true if a black hole was detected
    ///
    /// Calling this function will close the previous loss burst. If a black hole is detected, the
    /// current MTU will be reset to `min_mtu`.
    pub(crate) fn black_hole_detected(&mut self, now: Instant) -> bool {
        if !self.black_hole_detector.black_hole_detected() {
            return false;
        }

        self.current_mtu = self.black_hole_detector.min_mtu;

        if let Some(state) = &mut self.state {
            state.on_black_hole_detected(now);
        }

        true
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
            // Start a new search (we have reached the next activation time)
            self.phase = Phase::Searching(SearchState::new(
                current_mtu,
                self.peer_max_udp_payload_size,
                &self.config,
            ));
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
    /// 4. Called when the in-flight MTU probe was lost
    fn on_probe_lost(&mut self) {
        // We might no longer be searching, e.g. if a black hole was detected
        if let Phase::Searching(state) = &mut self.phase {
            state.in_flight_probe = None;
            state.lost_probe_count += 1;
        }
    }
    /// Called when a black hole is detected
    fn on_black_hole_detected(&mut self, now: Instant) {
        // Stop searching, if applicable, and reset the timer
        let next_mtud_activation = now + self.config.black_hole_cooldown;
        self.phase = Phase::Complete(next_mtud_activation);
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
    /// 1. The maximum of `min_mtu` and the size of `largest_post_loss_packet`, or exactly `min_mtu` if
    /// no larger packets have been received since the most recent loss burst.
    acked_mtu: u16,
    /// 2. Packet number of the biggest packet larger than `min_mtu` which we've received
    /// acknowledgment of more recently than any suspicious loss burst, if any
    largest_post_loss_packet: u64,
    /// 3. Packet loss bursts currently considered suspicious
    suspicious_loss_bursts: Vec<LossBurst>,
    /// 4. Loss burst currently being aggregated, if any
    current_loss_burst: Option<CurrentLossBurst>,
    /// 5. The UDP payload size guaranteed to be supported by the network
    min_mtu: u16,
}

impl BlackHoleDetector {
    /// 1
    fn new(min_mtu: u16) -> Self {
        Self {
            suspicious_loss_bursts: Vec::with_capacity(BLACK_HOLE_THRESHOLD + 1),
            largest_post_loss_packet: 0,
            acked_mtu: min_mtu,
            current_loss_burst: None,
            min_mtu,
        }
    }
    /// 2
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
    /// 3
    fn on_probe_acked(&mut self, pn: u64, len: u16) {
        // MTU probes are always larger than the previous MTU, so no previous loss bursts are
        // suspicious. At most one MTU probe is in flight at a time, so we don't need to worry about
        // reordering between them.
        self.suspicious_loss_bursts.clear();
        self.acked_mtu = len;
        // This might go backwards, but that's okay: a successful ACK means we haven't yet judged a
        // more recently sent packet lost, and we just want to track the largest packet that's been
        // successfully delivered more recently than a loss.
        self.largest_post_loss_packet = pn;
    }
    /// 4
    fn on_non_probe_lost(&mut self, pn: u64, len: u16) {
        // A loss burst is a group of consecutive packets that are declared lost, so a distance
        // greater than 1 indicates a new burst
        let end_last_burst = self
            .current_loss_burst
            .as_ref()
            .map_or(false, |current| pn - current.latest_non_probe != 1);

        if end_last_burst {
            self.finish_loss_burst();
        }

        self.current_loss_burst = Some(CurrentLossBurst {
            latest_non_probe: pn,
            smallest_packet_size: self
                .current_loss_burst
                .map_or(len, |prev| cmp::min(prev.smallest_packet_size, len)),
        });
    }
    /// 5. Marks the end of the current loss burst, checking whether it was suspicious
    fn finish_loss_burst(&mut self) {
        let Some(burst) = self.current_loss_burst.take() else {
            return;
        };
        // If a loss burst contains a packet smaller than the minimum MTU or a more recently
        // transmitted packet, it is not suspicious.
        if burst.smallest_packet_size < self.min_mtu
            || (burst.latest_non_probe < self.largest_post_loss_packet
                && burst.smallest_packet_size < self.acked_mtu)
        {
            return;
        }

        // A suspicious loss burst more recent than `largest_post_loss_packet` invalidates it. This
        // makes `acked_mtu` a conservative approximation. Ideally we'd update `safe_mtu` and
        // `largest_post_loss_packet` to describe the largest acknowledged packet sent later than
        // this burst, but that would require tracking the size of an unpredictable number of
        // recently acknowledged packets, and erring on the side of false positives is safe.
        if burst.latest_non_probe > self.largest_post_loss_packet {
            self.acked_mtu = self.min_mtu;
        }

        let burst = LossBurst {
            smallest_packet_size: burst.smallest_packet_size,
        };

        if self.suspicious_loss_bursts.len() <= BLACK_HOLE_THRESHOLD {
            self.suspicious_loss_bursts.push(burst);
            return;
        }
        todo!()
    }
    fn black_hole_detected(&mut self) -> bool {
        self.finish_loss_burst();

        if self.suspicious_loss_bursts.len() <= BLACK_HOLE_THRESHOLD {
            return false;
        }

        self.suspicious_loss_bursts.clear();

        true
    }
}

#[derive(Copy, Clone)]
struct LossBurst {
    smallest_packet_size: u16,
}

#[derive(Copy, Clone)]
struct CurrentLossBurst {
    /// 1.
    latest_non_probe: u64,
    /// 2.
    smallest_packet_size: u16,
}
