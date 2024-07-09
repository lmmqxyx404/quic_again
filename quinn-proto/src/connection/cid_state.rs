use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use rustc_hash::FxHashSet;

use crate::shared::IssuedCid;

/// 1. Local connection ID management
pub(super) struct CidState {
    /// Timestamp when issued cids should be retired
    retire_timestamp: VecDeque<CidTimestamp>,
    /// Number of local connection IDs that have been issued in NEW_CONNECTION_ID frames.
    issued: u64,
    /// Sequence numbers of local connection IDs not yet retired by the peer
    active_seq: FxHashSet<u64>,
    /// Sequence number the peer has already retired all CIDs below at our request via `retire_prior_to`
    prev_retire_seq: u64,
    /// Sequence number to set in retire_prior_to field in NEW_CONNECTION_ID frame
    retire_seq: u64,
    /// cid length used to decode short packet
    cid_len: usize,
    //// cid lifetime
    cid_lifetime: Option<Duration>,
}

impl CidState {
    /// 1.
    pub(crate) fn new(
        cid_len: usize,
        cid_lifetime: Option<Duration>,
        now: Instant,
        issued: u64,
    ) -> Self {
        let mut active_seq = FxHashSet::default();
        // Add sequence number of CIDs used in handshaking into tracking set
        for seq in 0..issued {
            active_seq.insert(seq);
        }
        let mut this = Self {
            retire_timestamp: VecDeque::new(),
            issued,
            active_seq,
            prev_retire_seq: 0,
            retire_seq: 0,
            cid_len,
            cid_lifetime,
        };
        // Track lifetime of CIDs used in handshaking
        for seq in 0..issued {
            this.track_lifetime(seq, now);
        }
        this
    }
    /// 2. Track the lifetime of issued cids in `retire_timestamp`
    fn track_lifetime(&mut self, new_cid_seq: u64, now: Instant) {
        todo!()
    }

    /// Update cid state when `NewIdentifiers` event is received
    pub(crate) fn new_cids(&mut self, ids: &[IssuedCid], now: Instant) {
        todo!()
    }
}

/// 2. Data structure that records when issued cids should be retired
#[derive(Copy, Clone, Eq, PartialEq)]
struct CidTimestamp {
    /// Highest cid sequence number created in a batch
    sequence: u64,
    /// Timestamp when cid needs to be retired
    timestamp: Instant,
}
