use std::cmp;
use std::collections::{btree_map, BTreeMap};
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
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter())
    }
    /// Add a range to the set, returning the intersection of current ranges with the new one
    pub fn replace(&mut self, mut range: Range<u64>) -> Replace<'_> {
        let pred = if let Some((prev_start, prev_end)) = self
            .pred(range.start)
            .filter(|&(_, end)| end >= range.start)
        {
            self.0.remove(&prev_start);
            let replaced_start = range.start;
            range.start = range.start.min(prev_start);
            let replaced_end = range.end.min(prev_end);
            range.end = range.end.max(prev_end);
            if replaced_start != replaced_end {
                Some(replaced_start..replaced_end)
            } else {
                None
            }
        } else {
            None
        };
        Replace {
            set: self,
            range,
            pred,
        }
    }
}

pub struct Iter<'a>(btree_map::Iter<'a, u64, u64>);

impl<'a> Iterator for Iter<'a> {
    type Item = Range<u64>;
    fn next(&mut self) -> Option<Range<u64>> {
        let (&start, &end) = self.0.next()?;
        Some(start..end)
    }
}

/// Iterator returned by `RangeSet::replace`
pub struct Replace<'a> {
    set: &'a mut RangeSet,
    /// Portion of the intersection arising from a range beginning at or before the newly inserted
    /// range
    pred: Option<Range<u64>>,
    /// Union of the input range and all ranges that have been visited by the iterator so far
    range: Range<u64>,
}

impl Iterator for Replace<'_> {
    type Item = Range<u64>;
    fn next(&mut self) -> Option<Range<u64>> {
        if let Some(pred) = self.pred.take() {
            // If a range starting before the inserted range overlapped with it, return the
            // corresponding overlap first
            return Some(pred);
        }

        let (next_start, next_end) = self.set.succ(self.range.start)?;
        if next_start > self.range.end {
            // If the next successor range starts after the current range ends, there can be no more
            // overlaps. This is sound even when `self.range.end` is increased because `RangeSet` is
            // guaranteed not to contain pairs of ranges that could be simplified.
            return None;
        }
        // Remove the redundant range...
        self.set.0.remove(&next_start);
        // ...and handle the case where the redundant range ends later than the new range.
        let replaced_end = self.range.end.min(next_end);
        self.range.end = self.range.end.max(next_end);
        if next_start == replaced_end {
            // If the redundant range started exactly where the new range ended, there was no
            // overlap with it or any later range.
            None
        } else {
            Some(next_start..replaced_end)
        }
    }
}
