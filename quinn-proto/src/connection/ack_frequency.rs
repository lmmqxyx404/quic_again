use std::time::Duration;

use crate::{
    config::AckFrequencyConfig, frame::AckFrequency, transport_parameters::TransportParameters,
    TransportError, VarInt,
};

use super::spaces::PendingAcks;

/// State associated to ACK frequency
pub(super) struct AckFrequencyState {
    /// 1
    pub(super) peer_max_ack_delay: Duration,
    /// 2. Sending ACK_FREQUENCY frames
    in_flight_ack_frequency_frame: Option<(u64, Duration)>,
    /// 3.
    pub(super) max_ack_delay: Duration,
    /// 4.
    next_outgoing_sequence_number: VarInt,
    // 5.Receiving ACK_FREQUENCY frames
    // last_ack_frequency_frame: Option<u64>,
}

impl AckFrequencyState {
    /// 1
    pub(super) fn new(default_max_ack_delay: Duration) -> Self {
        Self {
            peer_max_ack_delay: default_max_ack_delay,
            in_flight_ack_frequency_frame: None,
            max_ack_delay: default_max_ack_delay,

            next_outgoing_sequence_number: VarInt(0),
            // last_ack_frequency_frame: None,
        }
    }

    /// 2. Returns the `max_ack_delay` for the purposes of calculating the PTO
    ///
    /// This `max_ack_delay` is defined as the maximum of the peer's current `max_ack_delay` and all
    /// in-flight `max_ack_delay`s (i.e. proposed values that haven't been acknowledged yet, but
    /// might be already in use by the peer).
    pub(super) fn max_ack_delay_for_pto(&self) -> Duration {
        // Note: we have at most one in-flight ACK_FREQUENCY frame
        if let Some((_, max_ack_delay)) = self.in_flight_ack_frequency_frame {
            self.peer_max_ack_delay.max(max_ack_delay)
        } else {
            self.peer_max_ack_delay
        }
    }

    /// 3.Returns true if we should send an ACK_FREQUENCY frame
    pub(super) fn should_send_ack_frequency(
        &self,
        rtt: Duration,
        config: &AckFrequencyConfig,
        peer_params: &TransportParameters,
    ) -> bool {
        if self.next_outgoing_sequence_number.0 == 0 {
            // Always send at startup
            return true;
        }
        let current = self
            .in_flight_ack_frequency_frame
            .map_or(self.peer_max_ack_delay, |(_, pending)| pending);
        let desired = self.candidate_max_ack_delay(rtt, config, peer_params);
        let error = (desired.as_secs_f32() / current.as_secs_f32()) - 1.0;
        error.abs() > MAX_RTT_ERROR
    }
    /// 4.Returns the next sequence number for an ACK_FREQUENCY frame
    pub(super) fn next_sequence_number(&mut self) -> VarInt {
        assert!(self.next_outgoing_sequence_number <= VarInt::MAX);

        let seq = self.next_outgoing_sequence_number;
        self.next_outgoing_sequence_number.0 += 1;
        seq
    }
    /// 5. Returns the `max_ack_delay` that should be requested of the peer when sending an
    /// ACK_FREQUENCY frame
    pub(super) fn candidate_max_ack_delay(
        &self,
        rtt: Duration,
        config: &AckFrequencyConfig,
        peer_params: &TransportParameters,
    ) -> Duration {
        // Use the peer's max_ack_delay if no custom max_ack_delay was provided in the config
        let min_ack_delay =
            Duration::from_micros(peer_params.min_ack_delay.map_or(0, |x| x.into()));
        config
            .max_ack_delay
            .unwrap_or(self.peer_max_ack_delay)
            .clamp(min_ack_delay, rtt.max(MIN_AUTOMATIC_ACK_DELAY))
    }
    /// 6. Notifies the [`AckFrequencyState`] that a packet containing an ACK_FREQUENCY frame was sent
    pub(super) fn ack_frequency_sent(&mut self, pn: u64, requested_max_ack_delay: Duration) {
        self.in_flight_ack_frequency_frame = Some((pn, requested_max_ack_delay));
    }
    /// 7. Notifies the [`AckFrequencyState`] that a packet has been ACKed
    pub(super) fn on_acked(&mut self, pn: u64) {
        match self.in_flight_ack_frequency_frame {
            Some((number, requested_max_ack_delay)) if number == pn => {
                self.in_flight_ack_frequency_frame = None;
                self.peer_max_ack_delay = requested_max_ack_delay;
            }
            _ => {}
        }
    }
    /// Notifies the [`AckFrequencyState`] that an ACK_FREQUENCY frame was received
    ///
    /// Updates the endpoint's params according to the payload of the ACK_FREQUENCY frame, or
    /// returns an error in case the requested `max_ack_delay` is invalid.
    ///
    /// Returns `true` if the frame was processed and `false` if it was ignored because of being
    /// stale.
    pub(super) fn ack_frequency_received(
        &mut self,
        frame: &AckFrequency,
        pending_acks: &mut PendingAcks,
    ) -> Result<bool, TransportError> {
        todo!()
    }
}

/// Maximum proportion difference between the most recently requested max ACK delay and the
/// currently desired one before a new request is sent, when the peer supports the ACK frequency
/// extension and an explicit max ACK delay is not configured.
const MAX_RTT_ERROR: f32 = 0.2;

/// Minimum value to request the peer set max ACK delay to when the peer supports the ACK frequency
/// extension and an explicit max ACK delay is not configured.
// Keep in sync with `AckFrequencyConfig::max_ack_delay` documentation
const MIN_AUTOMATIC_ACK_DELAY: Duration = Duration::from_millis(25);
