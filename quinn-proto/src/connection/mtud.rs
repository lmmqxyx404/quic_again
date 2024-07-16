use crate::{config::MtuDiscoveryConfig, MAX_UDP_PAYLOAD};

/// Implements Datagram Packetization Layer Path Maximum Transmission Unit Discovery
///
/// See [`MtuDiscoveryConfig`] for details
#[derive(Clone)]
pub(crate) struct MtuDiscovery {
    /// 1. Detected MTU for the path
    current_mtu: u16,
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
        Self { current_mtu }
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    /// We haven't started polling yet
    Initial,
}

/// Additional state for enabled MTU discovery
#[derive(Debug, Clone)]
struct EnabledMtuDiscovery {
    phase: Phase,
    peer_max_udp_payload_size: u16,
    config: MtuDiscoveryConfig,
}

impl EnabledMtuDiscovery {
    fn new(config: MtuDiscoveryConfig) -> Self {
        Self {
            phase: Phase::Initial,
            peer_max_udp_payload_size: MAX_UDP_PAYLOAD,
            config,
        }
    }
}
