use std::cmp;
use std::collections::BTreeMap;
use std::ops::{
    Bound::{Excluded, Included},
    Range,
};

/// A set of u64 values optimized for long runs and random insert/delete/contains
#[derive(Debug, Default, Clone)]
pub struct RangeSet(BTreeMap<u64, u64>);

impl RangeSet {
    /// 1.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// 2.
    pub fn pop_min(&mut self) -> Option<Range<u64>> {
        let result = self.peek_min()?;
        self.0.remove(&result.start);
        Some(result)
    }
    /// 3
    pub fn peek_min(&self) -> Option<Range<u64>> {
        let (&start, &end) = self.0.iter().next()?;
        Some(start..end)
    }
    /// 4 used for StreamsState::received_ack_of -> Send::ack -> SendBuffer::ack
    pub fn insert(&mut self, mut x: Range<u64>) -> bool {
        if x.is_empty() {
            return false;
        }
        if let Some((start, end)) = self.pred(x.start) {
            if end >= x.end {
                // Wholly contained
                return false;
            } else if end >= x.start {
                // Extend overlapping predecessor
                self.0.remove(&start);
                x.start = start;
            }
        }
        while let Some((next_start, next_end)) = self.succ(x.start) {
            if next_start > x.end {
                break;
            }
            // Overlaps with successor
            self.0.remove(&next_start);
            x.end = cmp::max(next_end, x.end);
        }
        self.0.insert(x.start, x.end);
        true
    }
    /// 5
    pub fn min(&self) -> Option<u64> {
        self.0.first_key_value().map(|(&start, _)| start)
    }
    /// 6. Find closest range to `x` that begins at or before it
    fn pred(&self, x: u64) -> Option<(u64, u64)> {
        self.0
            .range((Included(0), Included(x)))
            .next_back()
            .map(|(&x, &y)| (x, y))
    }
    /// 7.Find the closest range to `x` that begins after it
    fn succ(&self, x: u64) -> Option<(u64, u64)> {
        self.0
            .range((Excluded(x), Included(u64::MAX)))
            .next()
            .map(|(&x, &y)| (x, y))
    }
    /// 8
    pub fn new() -> Self {
        Default::default()
    }
}
