use std::time::Duration;

use crate::{config::AckFrequencyConfig, transport_parameters::TransportParameters, VarInt};

/// State associated to ACK frequency
pub(super) struct AckFrequencyState {}

impl AckFrequencyState {
    /// 1
    pub(super) fn new(default_max_ack_delay: Duration) -> Self {
        Self {}
    }

    /// 2. Returns the `max_ack_delay` for the purposes of calculating the PTO
    ///
    /// This `max_ack_delay` is defined as the maximum of the peer's current `max_ack_delay` and all
    /// in-flight `max_ack_delay`s (i.e. proposed values that haven't been acknowledged yet, but
    /// might be already in use by the peer).
    pub(super) fn max_ack_delay_for_pto(&self) -> Duration {
        // Note: we have at most one in-flight ACK_FREQUENCY frame
        todo!()
    }

    /// 3.Returns true if we should send an ACK_FREQUENCY frame
    pub(super) fn should_send_ack_frequency(
        &self,
        rtt: Duration,
        config: &AckFrequencyConfig,
        peer_params: &TransportParameters,
    ) -> bool {
        todo!()
    }
    /// 4.Returns the next sequence number for an ACK_FREQUENCY frame
    pub(super) fn next_sequence_number(&mut self) -> VarInt {
        todo!()
    }
    /// 5. Returns the `max_ack_delay` that should be requested of the peer when sending an
    /// ACK_FREQUENCY frame
    pub(super) fn candidate_max_ack_delay(
        &self,
        rtt: Duration,
        config: &AckFrequencyConfig,
        peer_params: &TransportParameters,
    ) -> Duration {
        todo!()
    }
    /// 6. Notifies the [`AckFrequencyState`] that a packet containing an ACK_FREQUENCY frame was sent
    pub(super) fn ack_frequency_sent(&mut self, pn: u64, requested_max_ack_delay: Duration) {
        todo!()
        // self.in_flight_ack_frequency_frame = Some((pn, requested_max_ack_delay));
    }
}
