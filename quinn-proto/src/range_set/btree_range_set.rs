use std::collections::BTreeMap;
use std::ops::Range;

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
}
