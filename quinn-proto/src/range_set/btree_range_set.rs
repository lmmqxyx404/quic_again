use std::collections::BTreeMap;

/// A set of u64 values optimized for long runs and random insert/delete/contains
#[derive(Debug, Default, Clone)]
pub struct RangeSet(BTreeMap<u64, u64>);

impl RangeSet {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
