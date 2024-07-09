use std::time::Instant;

use crate::{crypto::Keys, shared::IssuedCid};

/// 1.
pub(super) struct PacketSpace {
    /// 1.
    pub(super) crypto: Option<Keys>,
    /// 2. Data to send
    pub(super) pending: Retransmits,
}

impl PacketSpace {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            crypto: None,
            pending: Retransmits::default(),
        }
    }
}

/// 2. Retransmittable data queue
#[allow(unreachable_pub)] // fuzzing only
#[derive(Debug, Default, Clone)]
pub struct Retransmits {
    pub(super) new_cids: Vec<IssuedCid>,
}
