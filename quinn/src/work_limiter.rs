use std::time::{Duration, Instant};

/// Limits the amount of time spent on a certain type of work in a cycle
///
/// The limiter works dynamically: For a sampled subset of cycles it measures
/// the time that is approximately required for fulfilling 1 work item, and
/// calculates the amount of allowed work items per cycle.
/// The estimates are smoothed over all cycles where the exact duration is measured.
///
/// In cycles where no measurement is performed the previously determined work limit
/// is used.
///
/// For the limiter the exact definition of a work item does not matter.
/// It could for example track the amount of transmitted bytes per cycle,
/// or the amount of transmitted datagrams per cycle.
/// It will however work best if the required time to complete a work item is
/// constant.
#[derive(Debug)]
pub(crate) struct WorkLimiter {
    /// Whether to measure the required work time, or to use the previous estimates
    mode: Mode,
    /// How many work items have been completed in the cycle
    completed: usize,
    /// The time the cycle started - only used in measurement mode
    start_time: Option<Instant>,
    /// The amount of work items which are allowed for a cycle
    allowed: usize,
    /// The desired cycle time
    desired_cycle_time: Duration,
}

impl WorkLimiter {
    pub(crate) fn new(desired_cycle_time: Duration) -> Self {
        Self {
            mode: Mode::Measure,
            completed: 0,
            start_time: None,
            allowed: 0,
            desired_cycle_time,
            /*
            cycle: 0,
            
            smoothed_time_per_work_item_nanos: 0.0, */
        }
    }

    /// Starts one work cycle
    pub(crate) fn start_cycle(&mut self, now: impl Fn() -> Instant) {
        self.completed = 0;
        if let Mode::Measure = self.mode {
            self.start_time = Some(now());
        }
    }

    /// Finishes one work cycle
    ///
    /// For cycles where the exact duration is measured this will update the estimates
    /// for the time per work item and the limit of allowed work items per cycle.
    /// The estimate is updated using the same exponential averaging (smoothing)
    /// mechanism which is used for determining QUIC path rtts: The last value is
    /// weighted by 1/8, and the previous average by 7/8.
    pub(crate) fn finish_cycle(&mut self, now: impl Fn() -> Instant) {
        // If no work was done in the cycle drop the measurement, it won't be useful
        if self.completed == 0 {
            return;
        }
        todo!()
    }

    /// Records that `work` additional work items have been completed inside the cycle
    ///
    /// Must be called between `start_cycle` and `finish_cycle`.
    pub(crate) fn record_work(&mut self, work: usize) {
        self.completed += work;
    }

    /// Returns whether more work can be performed inside the `desired_cycle_time`
    ///
    /// Requires that previous work was tracked using `record_work`.
    pub(crate) fn allow_work(&mut self, now: impl Fn() -> Instant) -> bool {
        match self.mode {
            Mode::Measure => (now() - self.start_time.unwrap()) < self.desired_cycle_time,
            Mode::HistoricData => self.completed < self.allowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Measure,
    HistoricData,
}
