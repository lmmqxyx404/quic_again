use std::time::Duration;

/// State associated to ACK frequency
pub(super) struct AckFrequencyState {}

impl AckFrequencyState {
    /// 1
    pub(super) fn new(default_max_ack_delay: Duration) -> Self {
        todo!()
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
}
