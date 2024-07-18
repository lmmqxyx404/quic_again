use std::collections::VecDeque;

use crate::frame::Datagram;

#[derive(Default)]
pub(super) struct DatagramState {
    /// 1
    pub(super) outgoing: VecDeque<Datagram>,
}
